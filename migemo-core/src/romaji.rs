// Hiragana / katakana / romaji-mora conversion.
//
// A "mora" here is the romaji spelling of one Japanese sound unit (e.g. "ge", "n", "kya").
// Item text is converted hiragana/katakana -> mora list at index time (add_item).
// User queries are converted ascii romaji -> mora list at search time (query_to_morae).
// Both use the same canonical spellings so a Vec<String> equality check finds matches.

use std::collections::HashMap;
use std::sync::LazyLock;

/// Convert katakana characters to their hiragana equivalents; everything else is untouched.
pub fn katakana_to_hiragana(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\u{30A1}'..='\u{30F6}' => char::from_u32(c as u32 - 0x60).unwrap_or(c),
            other => other,
        })
        .collect()
}

/// Convert a hiragana string into its mora list, expanding sokuon (っ) and
/// the long vowel mark (ー) along the way.
pub fn hiragana_to_morae(s: &str) -> Vec<String> {
    hiragana_to_morae_with_spans(s)
        .into_iter()
        .map(|(_, _, mora)| mora)
        .collect()
}

/// Same as [`hiragana_to_morae`] but also returns, for each mora, the char
/// range (in `s`) of the kana that produced it. Used to map a matched mora
/// back to the original text for highlighting.
pub fn hiragana_to_morae_with_spans(s: &str) -> Vec<(usize, usize, String)> {
    let chars: Vec<char> = s.chars().collect();
    let mut result: Vec<(usize, usize, String)> = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c == 'ー' {
            if let Some((_, _, last)) = result.last() {
                let vowel = last_vowel(last);
                result.push((i, i + 1, vowel.to_string()));
            }
            i += 1;
            continue;
        }

        if c == 'っ' {
            if i + 1 < chars.len() {
                let (unit_len, unit_mora) = next_hiragana_unit(&chars, i + 1);
                if let Some(base) = unit_mora {
                    result.push((i, i + 1 + unit_len, double_leading_consonant(&base)));
                    i += 1 + unit_len;
                    continue;
                }
            }
            i += 1;
            continue;
        }

        let (unit_len, unit_mora) = next_hiragana_unit(&chars, i);
        if let Some(m) = unit_mora {
            result.push((i, i + unit_len, m));
        }
        i += unit_len;
    }

    result
}

/// Read one kana unit (a youon digraph or a single kana) starting at `i`.
/// Returns the number of chars consumed and its romaji, if known.
fn next_hiragana_unit(chars: &[char], i: usize) -> (usize, Option<String>) {
    if i + 1 < chars.len() {
        let combo: String = chars[i..i + 2].iter().collect();
        if let Some(r) = youon_mora(&combo) {
            return (2, Some(r.to_string()));
        }
    }
    (1, single_mora(chars[i]).map(|s| s.to_string()))
}

fn double_leading_consonant(base: &str) -> String {
    match base.chars().next() {
        Some(c) => format!("{c}{base}"),
        None => base.to_string(),
    }
}

fn last_vowel(mora: &str) -> char {
    match mora.chars().last() {
        Some(c) if "aiueo".contains(c) => c,
        Some(c) => c,
        None => 'a',
    }
}

/// One kana (single character or youon digraph) and its canonical romaji
/// spelling. `aliases` lists other romaji spellings a user might type for
/// this same mora (Hepburn/Kunrei fluctuations); [`lookup_plain_token`]
/// normalizes any of them to `canonical`.
///
/// This is the single source of truth for kana<->romaji: [`single_mora`],
/// [`youon_mora`], and [`lookup_plain_token`] all read from [`MORAE`]
/// instead of keeping three hand-maintained tables in sync.
///
/// `じ`/`ぢ` share the canonical spelling "zi" (with "ji" as an alias),
/// mirroring how `ず`/`づ` already share "zu" — they're homophones in
/// modern Japanese, so a query should hit either kana. `てぃ`/`でぃ`/`とぅ`
/// use the extended spellings "thi"/"dhi"/"twu" rather than "ti"/"di"/"tu",
/// which stay reserved for the `ち`/`ぢ`/`つ` aliases below; this avoids a
/// token meaning two different morae depending on context.
struct MoraDef {
    kana: &'static str,
    canonical: &'static str,
    aliases: &'static [&'static str],
}

const fn m(
    kana: &'static str,
    canonical: &'static str,
    aliases: &'static [&'static str],
) -> MoraDef {
    MoraDef {
        kana,
        canonical,
        aliases,
    }
}

const MORAE: &[MoraDef] = &[
    m("あ", "a", &[]),
    m("い", "i", &[]),
    m("う", "u", &[]),
    m("え", "e", &[]),
    m("お", "o", &[]),
    m("か", "ka", &[]),
    m("き", "ki", &[]),
    m("く", "ku", &[]),
    m("け", "ke", &[]),
    m("こ", "ko", &[]),
    m("さ", "sa", &[]),
    m("し", "shi", &["si"]),
    m("す", "su", &[]),
    m("せ", "se", &[]),
    m("そ", "so", &[]),
    m("た", "ta", &[]),
    m("ち", "chi", &["ti"]),
    m("つ", "tsu", &["tu"]),
    m("て", "te", &[]),
    m("と", "to", &[]),
    m("な", "na", &[]),
    m("に", "ni", &[]),
    m("ぬ", "nu", &[]),
    m("ね", "ne", &[]),
    m("の", "no", &[]),
    m("は", "ha", &[]),
    m("ひ", "hi", &[]),
    m("ふ", "fu", &["hu"]),
    m("へ", "he", &[]),
    m("ほ", "ho", &[]),
    m("ま", "ma", &[]),
    m("み", "mi", &[]),
    m("む", "mu", &[]),
    m("め", "me", &[]),
    m("も", "mo", &[]),
    m("や", "ya", &[]),
    m("ゆ", "yu", &[]),
    m("よ", "yo", &[]),
    m("ら", "ra", &[]),
    m("り", "ri", &[]),
    m("る", "ru", &[]),
    m("れ", "re", &[]),
    m("ろ", "ro", &[]),
    m("わ", "wa", &[]),
    m("を", "wo", &[]),
    m("ん", "n", &[]),
    m("が", "ga", &[]),
    m("ぎ", "gi", &[]),
    m("ぐ", "gu", &[]),
    m("げ", "ge", &[]),
    m("ご", "go", &[]),
    m("ざ", "za", &[]),
    m("じ", "zi", &["ji"]),
    m("ず", "zu", &[]),
    m("ぜ", "ze", &[]),
    m("ぞ", "zo", &[]),
    m("だ", "da", &[]),
    m("ぢ", "zi", &["di"]),
    m("づ", "zu", &[]),
    m("で", "de", &[]),
    m("ど", "do", &[]),
    m("ば", "ba", &[]),
    m("び", "bi", &[]),
    m("ぶ", "bu", &[]),
    m("べ", "be", &[]),
    m("ぼ", "bo", &[]),
    m("ぱ", "pa", &[]),
    m("ぴ", "pi", &[]),
    m("ぷ", "pu", &[]),
    m("ぺ", "pe", &[]),
    m("ぽ", "po", &[]),
    m("ゔ", "vu", &[]),
    m("ぁ", "a", &[]),
    m("ぃ", "i", &[]),
    m("ぅ", "u", &[]),
    m("ぇ", "e", &[]),
    m("ぉ", "o", &[]),
    m("ゃ", "ya", &[]),
    m("ゅ", "yu", &[]),
    m("ょ", "yo", &[]),
    m("ゎ", "wa", &[]),
    m("きゃ", "kya", &[]),
    m("きゅ", "kyu", &[]),
    m("きょ", "kyo", &[]),
    m("ぎゃ", "gya", &[]),
    m("ぎゅ", "gyu", &[]),
    m("ぎょ", "gyo", &[]),
    m("しゃ", "sha", &[]),
    m("しゅ", "shu", &[]),
    m("しょ", "sho", &[]),
    m("じゃ", "ja", &[]),
    m("じゅ", "ju", &[]),
    m("じょ", "jo", &[]),
    m("ちゃ", "cha", &[]),
    m("ちゅ", "chu", &[]),
    m("ちょ", "cho", &[]),
    m("ぢゃ", "ja", &[]),
    m("ぢゅ", "ju", &[]),
    m("ぢょ", "jo", &[]),
    m("にゃ", "nya", &[]),
    m("にゅ", "nyu", &[]),
    m("にょ", "nyo", &[]),
    m("ひゃ", "hya", &[]),
    m("ひゅ", "hyu", &[]),
    m("ひょ", "hyo", &[]),
    m("びゃ", "bya", &[]),
    m("びゅ", "byu", &[]),
    m("びょ", "byo", &[]),
    m("ぴゃ", "pya", &[]),
    m("ぴゅ", "pyu", &[]),
    m("ぴょ", "pyo", &[]),
    m("みゃ", "mya", &[]),
    m("みゅ", "myu", &[]),
    m("みょ", "myo", &[]),
    m("りゃ", "rya", &[]),
    m("りゅ", "ryu", &[]),
    m("りょ", "ryo", &[]),
    m("ふぁ", "fa", &[]),
    m("ふぃ", "fi", &[]),
    m("ふぇ", "fe", &[]),
    m("ふぉ", "fo", &[]),
    m("うぃ", "wi", &[]),
    m("うぇ", "we", &[]),
    m("うぉ", "wo", &[]),
    m("ゔぁ", "va", &[]),
    m("ゔぃ", "vi", &[]),
    m("ゔぇ", "ve", &[]),
    m("ゔぉ", "vo", &[]),
    m("てぃ", "thi", &[]),
    m("でぃ", "dhi", &[]),
    m("とぅ", "twu", &[]),
    m("どぅ", "du", &[]),
    m("つぁ", "tsa", &[]),
    m("つぃ", "tsi", &[]),
    m("つぇ", "tse", &[]),
    m("つぉ", "tso", &[]),
    m("しぇ", "she", &[]),
    m("じぇ", "je", &[]),
    m("ちぇ", "che", &[]),
    m("くぁ", "kwa", &[]),
    m("くぃ", "kwi", &[]),
    m("くぇ", "kwe", &[]),
    m("くぉ", "kwo", &[]),
];

/// `MORAE` keyed by `kana`, for [`single_mora`] and [`youon_mora`]. Built
/// once on first use rather than searched linearly on every call:
/// `hiragana_to_morae` runs per dictionary entry when loading SKK-JISYO.L
/// (~170k entries), where a linear scan over ~140 morae measurably slowed
/// down `SkkDictionary::from_bytes` (see `examples/bench_dict_load.rs`).
static KANA_TO_MORA: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| MORAE.iter().map(|m| (m.kana, m.canonical)).collect());

/// `MORAE` keyed by romaji token (canonical spellings first, then aliases
/// inserted on top so an alias always wins if it happens to collide with
/// some other entry's canonical spelling), for [`lookup_plain_token`].
static TOKEN_TO_MORA: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut map: HashMap<&'static str, &'static str> = HashMap::new();
    for entry in MORAE {
        map.entry(entry.canonical).or_insert(entry.canonical);
    }
    for entry in MORAE {
        for alias in entry.aliases {
            map.insert(alias, entry.canonical);
        }
    }
    map
});

fn single_mora(c: char) -> Option<&'static str> {
    let mut buf = [0u8; 4];
    let key: &str = c.encode_utf8(&mut buf);
    KANA_TO_MORA.get(key).copied()
}

fn youon_mora(combo: &str) -> Option<&'static str> {
    KANA_TO_MORA.get(combo).copied()
}

/// Look up a plain (non-doubled) romaji token, normalizing common spelling
/// fluctuations (si -> shi, ti -> chi, ...) to the canonical mora used by
/// [`hiragana_to_morae`].
fn lookup_plain_token(s: &str) -> Option<&'static str> {
    TOKEN_TO_MORA.get(s).copied()
}

/// Try to match one mora token at `chars[pos..]`, either a plain token (up to 4
/// chars, longest match first) or a sokuon-doubled consonant (e.g. "kka").
/// Returns the number of chars consumed and the canonical mora string.
fn match_token(chars: &[char], pos: usize) -> Option<(usize, String)> {
    for len in [4usize, 3, 2, 1] {
        if pos + len <= chars.len() {
            let s: String = chars[pos..pos + len].iter().collect();
            if let Some(canon) = lookup_plain_token(&s) {
                return Some((len, canon.to_string()));
            }
        }
    }

    if pos + 1 < chars.len() && chars[pos] == chars[pos + 1] && chars[pos] != 'n' {
        for len in [3usize, 2, 1] {
            if pos + 1 + len <= chars.len() {
                let s: String = chars[pos + 1..pos + 1 + len].iter().collect();
                if let Some(canon) = lookup_plain_token(&s) {
                    let doubled = double_leading_consonant(canon);
                    return Some((1 + len, doubled));
                }
            }
        }
    }

    None
}

/// Decompose a user-typed romaji query into a mora list, stopping (dropping the
/// rest) as soon as an unrecognized/incomplete fragment is hit. For example
/// "geng" -> ["ge", "n"], the trailing "g" is dropped rather than guessed at.
pub fn query_to_morae(query: &str) -> Vec<String> {
    let chars: Vec<char> = query.to_lowercase().chars().collect();
    let mut morae = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        match match_token(&chars, i) {
            Some((len, mora)) => {
                morae.push(mora);
                i += len;
            }
            None => break,
        }
    }

    morae
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_basic_word() {
        assert_eq!(hiragana_to_morae("げんご"), vec!["ge", "n", "go"]);
    }

    #[test]
    fn converts_sokuon() {
        assert_eq!(hiragana_to_morae("がっこう"), vec!["ga", "kko", "u"]);
    }

    #[test]
    fn converts_chouon() {
        assert_eq!(
            hiragana_to_morae("そーすこーど"),
            vec!["so", "o", "su", "ko", "o", "do"]
        );
    }

    #[test]
    fn katakana_run_converts_via_hiragana() {
        let hira = katakana_to_hiragana("ソースコード");
        assert_eq!(
            hiragana_to_morae(&hira),
            vec!["so", "o", "su", "ko", "o", "do"]
        );
    }

    #[test]
    fn query_full_word() {
        assert_eq!(query_to_morae("gengo"), vec!["ge", "n", "go"]);
    }

    #[test]
    fn query_incomplete_trailing_mora_is_dropped() {
        assert_eq!(query_to_morae("geng"), vec!["ge", "n"]);
    }

    #[test]
    fn query_word_final_n_is_confirmed() {
        assert_eq!(query_to_morae("gen"), vec!["ge", "n"]);
    }

    #[test]
    fn query_normalizes_spelling_fluctuation() {
        assert_eq!(query_to_morae("si"), vec!["shi"]);
        assert_eq!(query_to_morae("ti"), vec!["chi"]);
    }

    #[test]
    fn query_kanri() {
        assert_eq!(query_to_morae("kanri"), vec!["ka", "n", "ri"]);
    }

    #[test]
    fn ji_and_zi_are_homophones_of_di() {
        // じ/ぢ share the canonical spelling "zi" (like ず/づ share "zu"),
        // so "ji" (alias of じ) and "di" (alias of ぢ) both normalize to it.
        assert_eq!(query_to_morae("ji"), vec!["zi"]);
        assert_eq!(query_to_morae("zi"), vec!["zi"]);
        assert_eq!(query_to_morae("di"), vec!["zi"]);
        assert_eq!(hiragana_to_morae("じ"), vec!["zi"]);
        assert_eq!(hiragana_to_morae("ぢ"), vec!["zi"]);
    }

    #[test]
    fn extended_spellings_reach_thi_dhi_twu_without_shadowing_ti_di_tu() {
        // てぃ/でぃ/とぅ use "thi"/"dhi"/"twu" precisely so these tokens don't
        // collide with the "ti"/"di"/"tu" aliases of ち/ぢ/つ below.
        assert_eq!(hiragana_to_morae("てぃ"), vec!["thi"]);
        assert_eq!(hiragana_to_morae("でぃ"), vec!["dhi"]);
        assert_eq!(hiragana_to_morae("とぅ"), vec!["twu"]);
        assert_eq!(query_to_morae("thi"), vec!["thi"]);
        assert_eq!(query_to_morae("dhi"), vec!["dhi"]);
        assert_eq!(query_to_morae("twu"), vec!["twu"]);

        assert_eq!(query_to_morae("ti"), vec!["chi"]);
        assert_eq!(query_to_morae("di"), vec!["zi"]);
        assert_eq!(query_to_morae("tu"), vec!["tsu"]);
    }
}
