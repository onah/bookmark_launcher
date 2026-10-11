use super::romaji::hiragana_to_morae;
use std::collections::HashMap;
use std::fmt;

/// SKK-JISYO based dictionary mapping a word (typically containing kanji) to
/// the mora list computed from its first-seen reading. Used to look up kanji
/// runs while indexing item text in `MigemoSearcher::add_item`.
///
/// Has two interchangeable backends with identical lookup behavior:
/// - `Map`, built by [`SkkDictionary::from_bytes`], parses the source
///   SKK-JISYO text into a `HashMap`. Straightforward, but building the map
///   allocates every one of its ~170k entries up front even if a caller
///   (e.g. a shell-completion process matching a handful of directory
///   names) only ever looks up a few of them.
/// - `Compiled`, built by [`SkkDictionary::compile`] /
///   [`SkkDictionary::from_compiled_bytes`], stores an `fst::Map` (a
///   compact trie serialized to bytes, interpreted directly with no
///   per-entry allocation) plus a side buffer of comma-joined mora
///   strings. A lookup is one FST walk; only a hit's mora string gets
///   allocated into a `Vec<String>`.
pub struct SkkDictionary {
    backend: Backend,
}

enum Backend {
    Map(HashMap<String, Vec<String>>),
    Compiled {
        fst: fst::Map<Vec<u8>>,
        morae_blob: Vec<u8>,
    },
}

/// A compiled-format value packs the matched entry's byte range within
/// `morae_blob` into a single u64: the high 32 bits are the offset, the low
/// 32 bits are the length. The dictionary (a few MB of SKK-JISYO text) never
/// comes close to either half overflowing.
fn pack_range(offset: usize, len: usize) -> u64 {
    ((offset as u64) << 32) | (len as u64)
}

fn unpack_range(value: u64) -> (usize, usize) {
    ((value >> 32) as usize, (value & 0xFFFF_FFFF) as usize)
}

/// A morae list is stored as its tokens joined by `,`; romaji mora tokens
/// (see `romaji.rs`) are always plain lowercase ASCII letters, so `,` can't
/// collide with real content.
const MORA_SEPARATOR: char = ',';

/// Error returned by [`SkkDictionary::from_compiled_bytes`] when the input
/// isn't a valid compiled dictionary (e.g. a truncated or corrupted cache
/// file). Callers driving a cache (like `migemo-complete`) should treat this
/// as "rebuild the cache", not a fatal error.
#[derive(Debug)]
pub enum CompiledDictError {
    Truncated,
    InvalidFst(fst::Error),
}

impl fmt::Display for CompiledDictError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompiledDictError::Truncated => write!(f, "compiled dictionary bytes are truncated"),
            CompiledDictError::InvalidFst(e) => write!(f, "invalid compiled dictionary fst: {e}"),
        }
    }
}

impl std::error::Error for CompiledDictError {}

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

        Self {
            backend: Backend::Map(word_to_morae),
        }
    }

    /// Serialize this dictionary into the compact format consumed by
    /// [`SkkDictionary::from_compiled_bytes`]: an `fst::Map` of word ->
    /// packed `(offset, len)` into a trailing blob of comma-joined mora
    /// strings, an 8-byte little-endian length prefix in front of the fst
    /// bytes so a reader knows where the fst ends and the blob begins.
    pub fn compile(&self) -> Vec<u8> {
        let (fst_bytes, morae_blob): (Vec<u8>, Vec<u8>) = match &self.backend {
            Backend::Map(map) => {
                let mut entries: Vec<(&String, &Vec<String>)> = map.iter().collect();
                entries.sort_by(|a, b| a.0.cmp(b.0));

                let mut morae_blob = Vec::new();
                let mut builder = fst::MapBuilder::memory();
                for (word, morae) in entries {
                    let joined = morae.join(&MORA_SEPARATOR.to_string());
                    let offset = morae_blob.len();
                    morae_blob.extend_from_slice(joined.as_bytes());
                    builder
                        .insert(word.as_bytes(), pack_range(offset, joined.len()))
                        .expect("keys are sorted and unique, from a HashMap");
                }
                let fst_bytes = builder.into_inner().expect("in-memory fst build");
                (fst_bytes, morae_blob)
            }
            Backend::Compiled { fst, morae_blob } => {
                (fst.as_fst().as_bytes().to_vec(), morae_blob.clone())
            }
        };

        let mut out = Vec::with_capacity(8 + fst_bytes.len() + morae_blob.len());
        out.extend_from_slice(&(fst_bytes.len() as u64).to_le_bytes());
        out.extend_from_slice(&fst_bytes);
        out.extend_from_slice(&morae_blob);
        out
    }

    /// Load a dictionary previously produced by [`SkkDictionary::compile`].
    pub fn from_compiled_bytes(bytes: &[u8]) -> Result<Self, CompiledDictError> {
        let header: [u8; 8] = bytes
            .get(..8)
            .and_then(|s| s.try_into().ok())
            .ok_or(CompiledDictError::Truncated)?;
        let fst_len = u64::from_le_bytes(header) as usize;
        let rest = &bytes[8..];
        let fst_bytes = rest.get(..fst_len).ok_or(CompiledDictError::Truncated)?;
        let morae_blob = rest[fst_len..].to_vec();

        let fst = fst::Map::new(fst_bytes.to_vec()).map_err(CompiledDictError::InvalidFst)?;
        Ok(Self {
            backend: Backend::Compiled { fst, morae_blob },
        })
    }

    /// Number of distinct words the dictionary holds a reading for.
    pub fn len(&self) -> usize {
        match &self.backend {
            Backend::Map(map) => map.len(),
            Backend::Compiled { fst, .. } => fst.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
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
        match &self.backend {
            Backend::Map(map) => {
                for len in (1..=chars.len()).rev() {
                    let word: String = chars[..len].iter().collect();
                    if let Some(morae) = map.get(&word) {
                        return Some((len, morae.clone()));
                    }
                }
                None
            }
            Backend::Compiled { fst, morae_blob } => {
                for len in (1..=chars.len()).rev() {
                    let word: String = chars[..len].iter().collect();
                    let Some(value) = fst.get(word.as_bytes()) else {
                        continue;
                    };
                    let (offset, blob_len) = unpack_range(value);
                    // A cache file can be corrupted or truncated by something
                    // outside our control (disk full, killed mid-write, ...);
                    // treat an out-of-range or non-UTF-8 slice as "no match
                    // at this length" rather than panicking.
                    let Some(slice) = offset
                        .checked_add(blob_len)
                        .and_then(|end| morae_blob.get(offset..end))
                    else {
                        continue;
                    };
                    let Ok(joined) = std::str::from_utf8(slice) else {
                        continue;
                    };
                    let morae = joined.split(MORA_SEPARATOR).map(str::to_string).collect();
                    return Some((len, morae));
                }
                None
            }
        }
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

    fn sample_text() -> &'static str {
        "\
;; okuri-nasi entries.
げん /現/言/減/源/
げんご /言語/
じんじ /人事/
かんり /管理/
けんきゅう /研究/
"
    }

    #[test]
    fn compiled_round_trip_matches_source_dictionary() {
        let source = SkkDictionary::from_bytes(&encode_euc_jp(sample_text()));
        let compiled_bytes = source.compile();
        let compiled =
            SkkDictionary::from_compiled_bytes(&compiled_bytes).expect("valid compiled bytes");

        assert_eq!(compiled.len(), source.len());

        for word in ["言語", "言", "人事", "管理", "研究"] {
            let chars: Vec<char> = word.chars().collect();
            assert_eq!(
                compiled.longest_match(&chars),
                source.longest_match(&chars),
                "mismatch for {word}"
            );
        }

        let unknown: Vec<char> = "未知".chars().collect();
        assert!(compiled.longest_match(&unknown).is_none());
    }

    #[test]
    fn recompiling_a_compiled_dictionary_is_idempotent() {
        let source = SkkDictionary::from_bytes(&encode_euc_jp(sample_text()));
        let once = source.compile();
        let compiled = SkkDictionary::from_compiled_bytes(&once).expect("valid compiled bytes");
        let twice = compiled.compile();

        assert_eq!(once, twice);
    }

    #[test]
    fn from_compiled_bytes_rejects_truncated_header() {
        assert!(matches!(
            SkkDictionary::from_compiled_bytes(&[1, 2, 3]),
            Err(CompiledDictError::Truncated)
        ));
    }

    #[test]
    fn from_compiled_bytes_rejects_bytes_shorter_than_declared_fst() {
        let source = SkkDictionary::from_bytes(&encode_euc_jp(sample_text()));
        let bytes = source.compile();
        let fst_len = u64::from_le_bytes(bytes[..8].try_into().unwrap()) as usize;
        assert!(
            fst_len > 4,
            "fixture too small for this test to be meaningful"
        );

        // Cut off partway through the fst region itself (not just the
        // trailing mora blob), so `rest.len() < fst_len` and the header's
        // claim can't be satisfied.
        let truncated = &bytes[..8 + fst_len / 2];
        assert!(matches!(
            SkkDictionary::from_compiled_bytes(truncated),
            Err(CompiledDictError::Truncated)
        ));
    }

    #[test]
    fn truncated_mora_blob_is_tolerated_at_load_and_misses_at_lookup() {
        // Chopping only the trailing blob (not the fst) leaves the header's
        // claim about fst_len satisfiable, so loading succeeds; a lookup
        // landing on the missing tail must return None, not panic.
        let source = SkkDictionary::from_bytes(&encode_euc_jp(sample_text()));
        let mut bytes = source.compile();
        bytes.pop();
        let compiled = SkkDictionary::from_compiled_bytes(&bytes)
            .expect("blob truncation alone must still load");

        let chars: Vec<char> = "言語".chars().collect();
        // Either still matches (if the dropped byte wasn't part of this
        // entry's slice) or misses cleanly -- must not panic either way.
        let _ = compiled.longest_match(&chars);
    }
}
