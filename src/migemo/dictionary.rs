use super::romaji::hiragana_to_morae;
use std::collections::HashMap;

/// SKK-JISYO based dictionary mapping a word (typically containing kanji) to
/// the mora list computed from its first-seen reading. Used to look up kanji
/// runs while indexing item text in `MigemoSearcher::add_item`.
pub struct SkkDictionary {
    word_to_morae: HashMap<String, Vec<String>>,
}

impl SkkDictionary {
    /// Build a dictionary from the raw bytes of an EUC-JP encoded SKK-JISYO file.
    /// Only the "okuri-nasi" (no-okurigana) section is used.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let (text, _, _) = encoding_rs::EUC_JP.decode(bytes);
        let mut word_to_morae = HashMap::new();

        let mut in_okuri_nasi = false;
        for line in text.lines() {
            if !in_okuri_nasi {
                if line.starts_with(";; okuri-nasi entries") {
                    in_okuri_nasi = true;
                }
                continue;
            }
            if line.starts_with(';') || line.is_empty() {
                continue;
            }
            Self::parse_line(line, &mut word_to_morae);
        }

        Self { word_to_morae }
    }

    fn parse_line(line: &str, word_to_morae: &mut HashMap<String, Vec<String>>) {
        let Some((reading, candidates)) = line.split_once(' ') else {
            return;
        };
        let candidates = candidates.strip_prefix('/').unwrap_or(candidates);

        let morae = hiragana_to_morae(reading);
        if morae.is_empty() {
            return;
        }

        for candidate in candidates.split('/') {
            if candidate.is_empty() || candidate.starts_with('#') {
                continue;
            }
            let word = candidate.split(';').next().unwrap_or(candidate);
            if word.is_empty() {
                continue;
            }
            word_to_morae
                .entry(word.to_string())
                .or_insert_with(|| morae.clone());
        }
    }

    /// Find the mora list for the longest prefix of `chars` present in the
    /// dictionary. Returns the number of chars consumed and its mora list.
    pub(crate) fn longest_match(&self, chars: &[char]) -> Option<(usize, Vec<String>)> {
        for len in (1..=chars.len()).rev() {
            let word: String = chars[..len].iter().collect();
            if let Some(morae) = self.word_to_morae.get(&word) {
                return Some((len, morae.clone()));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_euc_jp(text: &str) -> Vec<u8> {
        let (bytes, _, had_errors) = encoding_rs::EUC_JP.encode(text);
        assert!(!had_errors, "test fixture must be EUC-JP encodable");
        bytes.into_owned()
    }

    #[test]
    fn parses_okuri_nasi_entries() {
        let text = "\
;; -*- mode: fundamental; coding: euc-jp -*-
;; okuri-ari entries.
;; some entry we must ignore
;; okuri-nasi entries.
げん /現/言/減/源;発生-/原;-住民/元/
げんご /言語/原語;original language/
";
        let bytes = encode_euc_jp(text);
        let dict = SkkDictionary::from_bytes(&bytes);

        let chars: Vec<char> = "言語".chars().collect();
        let (len, morae) = dict.longest_match(&chars).expect("言語 should be found");
        assert_eq!(len, 2);
        assert_eq!(morae, vec!["ge", "n", "go"]);
    }

    #[test]
    fn longest_match_prefers_longer_word() {
        let text = "\
;; okuri-nasi entries.
けん /件/
けんきゅう /研究/
";
        let bytes = encode_euc_jp(text);
        let dict = SkkDictionary::from_bytes(&bytes);

        let chars: Vec<char> = "研究".chars().collect();
        let (len, morae) = dict.longest_match(&chars).expect("研究 should be found");
        assert_eq!(len, 2);
        assert_eq!(morae, vec!["ke", "n", "kyu", "u"]);
    }

    #[test]
    fn strips_annotations_after_semicolon() {
        let text = "\
;; okuri-nasi entries.
げん /現/言;annotation/減/
";
        let bytes = encode_euc_jp(text);
        let dict = SkkDictionary::from_bytes(&bytes);

        let chars: Vec<char> = "言".chars().collect();
        let (len, morae) = dict.longest_match(&chars).expect("言 should be found");
        assert_eq!(len, 1);
        assert_eq!(morae, vec!["ge", "n"]);
    }

    #[test]
    fn first_seen_reading_wins() {
        let text = "\
;; okuri-nasi entries.
げん /言/
こと /言/
";
        let bytes = encode_euc_jp(text);
        let dict = SkkDictionary::from_bytes(&bytes);

        let chars: Vec<char> = "言".chars().collect();
        let (_, morae) = dict.longest_match(&chars).expect("言 should be found");
        assert_eq!(morae, vec!["ge", "n"]);
    }

    #[test]
    fn ignores_lines_before_okuri_nasi_section() {
        let text = "\
;; okuri-ari entries.
げん /現/
;; okuri-nasi entries.
げんご /言語/
";
        let bytes = encode_euc_jp(text);
        let dict = SkkDictionary::from_bytes(&bytes);

        let chars: Vec<char> = "現".chars().collect();
        assert!(dict.longest_match(&chars).is_none());
    }

    #[test]
    fn unknown_word_returns_none() {
        let text = "\
;; okuri-nasi entries.
げんご /言語/
";
        let bytes = encode_euc_jp(text);
        let dict = SkkDictionary::from_bytes(&bytes);

        let chars: Vec<char> = "未知".chars().collect();
        assert!(dict.longest_match(&chars).is_none());
    }
}
