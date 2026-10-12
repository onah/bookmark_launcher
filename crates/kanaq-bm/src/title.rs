//! Finding a page's title for `kanaq-bm add`.
//!
//! Only the `<title>` element is needed, so instead of a full HTML parser
//! this does a small, forgiving scan. The part that matters more is the
//! character encoding: many Japanese pages are Shift_JIS or EUC-JP and only
//! say so in a `<meta>` tag, so the charset is taken from the Content-Type
//! header, then from the page itself, then defaults to UTF-8.

use encoding_rs::{Encoding, UTF_8};
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(10);
/// How much of the page to scan for a `<meta charset>` declaration. HTML
/// requires it within the first 1024 bytes; allow some slack.
const META_SCAN_BYTES: usize = 4096;

/// Download `url` and return its `<title>`.
pub fn fetch_title(url: &str) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(TIMEOUT)
        .user_agent(concat!("kanaq-bm/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| describe(&e))?;
    let response = client
        .get(url)
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| describe(&e))?;
    let header_charset = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(charset_from_content_type);
    let body = response.bytes().map_err(|e| describe(&e))?;

    let charset = header_charset.or_else(|| charset_from_meta(&body));
    title_from_html(&decode(&body, charset.as_deref()))
        .ok_or_else(|| "no <title> in the page".to_string())
}

/// `error` followed by its chain of causes: reqwest's own message alone
/// ("error sending request for url ...") does not say what went wrong.
fn describe(error: &dyn std::error::Error) -> String {
    let mut message = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    message
}

/// The host part of `url`, used as a title when the page cannot be fetched.
pub fn host_of(url: &str) -> Option<String> {
    reqwest::Url::parse(url)
        .ok()?
        .host_str()
        .map(str::to_string)
}

fn charset_from_content_type(content_type: &str) -> Option<String> {
    charset_after(&content_type.to_ascii_lowercase())
}

/// The charset declared by `<meta charset="...">` or
/// `<meta http-equiv="Content-Type" content="...; charset=...">` near the
/// start of `html`.
fn charset_from_meta(html: &[u8]) -> Option<String> {
    let head = &html[..html.len().min(META_SCAN_BYTES)];
    // Declarations are ASCII, so a lossy view is enough to find them.
    let head = String::from_utf8_lossy(head).to_ascii_lowercase();
    let mut from = 0;
    while let Some(offset) = head[from..].find("<meta") {
        let start = from + offset;
        let end = head[start..].find('>').map_or(head.len(), |e| start + e);
        if let Some(charset) = charset_after(&head[start..end]) {
            return Some(charset);
        }
        from = end;
    }
    None
}

/// The value following the first `charset=` in `text` (already lowercased),
/// with optional quotes stripped.
fn charset_after(text: &str) -> Option<String> {
    let start = text.find("charset=")? + "charset=".len();
    let value: String = text[start..]
        .trim_start_matches(['"', '\''])
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':' | '.'))
        .collect();
    (!value.is_empty()).then_some(value)
}

/// Decode `bytes` as `charset` (UTF-8 when unknown). A byte-order mark, if
/// present, wins over the declared charset.
fn decode(bytes: &[u8], charset: Option<&str>) -> String {
    let encoding = charset
        .and_then(|label| Encoding::for_label(label.as_bytes()))
        .unwrap_or(UTF_8);
    encoding.decode(bytes).0.into_owned()
}

/// The text of the first `<title>` element, with character references
/// decoded and whitespace collapsed; `None` if there is no non-empty title.
fn title_from_html(html: &str) -> Option<String> {
    // ASCII lowercasing keeps byte offsets, so indexes carry over to `html`.
    let lower = html.to_ascii_lowercase();
    let open = lower.find("<title")?;
    let content_start = open + lower[open..].find('>')? + 1;
    let content_end = content_start + lower[content_start..].find("</title")?;
    let title = decode_entities(&html[content_start..content_end]);
    let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    (!title.is_empty()).then_some(title)
}

/// Decode the character references a title commonly contains: the basic
/// named ones and numeric ones. Anything else is left as written.
fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let decoded = rest
            .find(';')
            .filter(|&semi| semi <= 10)
            .and_then(|semi| entity_char(&rest[1..semi]).map(|c| (c, semi)));
        match decoded {
            Some((c, semi)) => {
                out.push(c);
                rest = &rest[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn entity_char(name: &str) -> Option<char> {
    let code = if let Some(hex) = name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
        u32::from_str_radix(hex, 16).ok()?
    } else if let Some(dec) = name.strip_prefix('#') {
        dec.parse().ok()?
    } else {
        return match name {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some(' '),
            _ => None,
        };
    };
    char::from_u32(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    // "Japanese" in kanji (nihongo), written as escapes to keep the source
    // ASCII-only.
    const NIHONGO: &str = "\u{65e5}\u{672c}\u{8a9e}";

    #[test]
    fn title_is_found_case_insensitively_and_cleaned_up() {
        let html = "<html><HEAD><Title lang=en>\n  Hello &amp; welcome\n </TITLE></head>";
        assert_eq!(title_from_html(html).as_deref(), Some("Hello & welcome"));
    }

    #[test]
    fn missing_or_empty_title_is_none() {
        assert_eq!(title_from_html("<html><body>hi</body></html>"), None);
        assert_eq!(title_from_html("<title>  </title>"), None);
    }

    #[test]
    fn numeric_and_unknown_entities() {
        assert_eq!(decode_entities("a&#39;b&#x41;c"), "a'bAc");
        assert_eq!(decode_entities("Q&A &copy; x & y"), "Q&A &copy; x & y");
    }

    #[test]
    fn charset_from_content_type_header() {
        assert_eq!(
            charset_from_content_type("text/html; charset=Shift_JIS").as_deref(),
            Some("shift_jis")
        );
        assert_eq!(charset_from_content_type("text/html"), None);
    }

    #[test]
    fn charset_from_meta_tags() {
        assert_eq!(
            charset_from_meta(br#"<head><meta charset="EUC-JP"><title>"#).as_deref(),
            Some("euc-jp")
        );
        let http_equiv =
            br#"<meta http-equiv="Content-Type" content="text/html; charset=Shift_JIS">"#;
        assert_eq!(charset_from_meta(http_equiv).as_deref(), Some("shift_jis"));
        assert_eq!(charset_from_meta(b"<meta name=viewport>"), None);
    }

    #[test]
    fn shift_jis_page_declared_only_in_meta_decodes_correctly() {
        let page = format!(r#"<html><head><meta charset="Shift_JIS"><title>{NIHONGO}</title>"#);
        let (bytes, _, had_errors) = encoding_rs::SHIFT_JIS.encode(&page);
        assert!(!had_errors);
        let charset = charset_from_meta(&bytes);
        let html = decode(&bytes, charset.as_deref());
        assert_eq!(title_from_html(&html).as_deref(), Some(NIHONGO));
    }

    #[test]
    fn unknown_charset_falls_back_to_utf8() {
        let html = decode(
            format!("<title>{NIHONGO}</title>").as_bytes(),
            Some("bogus"),
        );
        assert_eq!(title_from_html(&html).as_deref(), Some(NIHONGO));
    }

    #[test]
    fn host_of_url() {
        assert_eq!(
            host_of("https://www.example.com:8080/a?b").as_deref(),
            Some("www.example.com")
        );
        assert_eq!(host_of("not a url"), None);
    }
}
