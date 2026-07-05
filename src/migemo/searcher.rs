use super::dictionary::SkkDictionary;
use super::romaji::{hiragana_to_morae_with_spans, katakana_to_hiragana, query_to_morae};
use std::ops::Range;

#[derive(Clone, Copy, PartialEq, Eq)]
enum CharKind {
    Ascii,
    Hiragana,
    Katakana,
    Kanji,
    Other,
}

fn classify(c: char, prev: Option<CharKind>) -> CharKind {
    // The long vowel mark (ー) belongs to whichever run it extends.
    if c == 'ー' {
        return prev.unwrap_or(CharKind::Katakana);
    }
    if c.is_ascii() {
        CharKind::Ascii
    } else if ('\u{3041}'..='\u{309F}').contains(&c) {
        CharKind::Hiragana
    } else if ('\u{30A1}'..='\u{30FF}').contains(&c) {
        CharKind::Katakana
    } else if ('\u{4E00}'..='\u{9FFF}').contains(&c) {
        CharKind::Kanji
    } else {
        CharKind::Other
    }
}

fn run_end(chars: &[char], start: usize, kind: CharKind) -> usize {
    let mut j = start + 1;
    while j < chars.len() && classify(chars[j], Some(kind)) == kind {
        j += 1;
    }
    j
}

/// One contiguous chunk of an indexed item's text, and how many morae (in
/// `IndexedItem::morae`) it contributed. Used to map a matched mora range
/// back to char positions in the original text for highlighting.
struct Segment {
    char_range: Range<usize>,
    morae_len: usize,
}

struct IndexedItem {
    text: String,
    morae: Vec<String>,
    segments: Vec<Segment>,
}

/// A search hit: the index the item was registered at (via `add_item`), and
/// the char positions in its text that should be highlighted.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub index: usize,
    // Part of the public API (see docs/migemo_ja.md section 5); callers that
    // only need the index, like this app's search-then-render-separately
    // flow, are free to ignore it.
    #[allow(dead_code)]
    pub highlight: Vec<usize>,
}

/// Romaji-mora index search engine. Text registered via `add_item` has its
/// reading pre-computed into a mora list; `search` matches a romaji query
/// against that list as a contiguous subsequence, so multi-mora queries can
/// match kanji the way single-mora queries always could.
pub struct MigemoSearcher {
    dictionary: SkkDictionary,
    items: Vec<IndexedItem>,
}

impl MigemoSearcher {
    pub fn new(dictionary: SkkDictionary) -> Self {
        Self {
            dictionary,
            items: Vec::new(),
        }
    }

    /// Register a searchable text, computing and storing its mora list.
    pub fn add_item(&mut self, text: &str) {
        let item = self.build_item(text);
        self.items.push(item);
    }

    fn build_item(&self, text: &str) -> IndexedItem {
        let chars: Vec<char> = text.chars().collect();
        let mut morae: Vec<String> = Vec::new();
        let mut segments: Vec<Segment> = Vec::new();

        let mut i = 0;
        let mut prev_kind: Option<CharKind> = None;
        while i < chars.len() {
            let kind = classify(chars[i], prev_kind);
            prev_kind = Some(kind);

            match kind {
                CharKind::Kanji => {
                    let end = run_end(&chars, i, kind);
                    while i < end {
                        if let Some((len, word_morae)) =
                            self.dictionary.longest_match(&chars[i..end])
                        {
                            let morae_len = word_morae.len();
                            morae.extend(word_morae);
                            segments.push(Segment {
                                char_range: i..i + len,
                                morae_len,
                            });
                            i += len;
                        } else {
                            // Word not in dictionary: skip, this char stays unsearchable.
                            i += 1;
                        }
                    }
                }
                CharKind::Hiragana | CharKind::Katakana => {
                    let end = run_end(&chars, i, kind);
                    let run: String = chars[i..end].iter().collect();
                    let hira = if kind == CharKind::Katakana {
                        katakana_to_hiragana(&run)
                    } else {
                        run
                    };
                    for (s, e, mora) in hiragana_to_morae_with_spans(&hira) {
                        morae.push(mora);
                        segments.push(Segment {
                            char_range: (i + s)..(i + e),
                            morae_len: 1,
                        });
                    }
                    i = end;
                }
                CharKind::Ascii | CharKind::Other => {
                    let end = run_end(&chars, i, kind);
                    for (offset, &c) in chars[i..end].iter().enumerate() {
                        let pos = i + offset;
                        for lower in c.to_lowercase() {
                            morae.push(lower.to_string());
                        }
                        segments.push(Segment {
                            char_range: pos..pos + 1,
                            morae_len: 1,
                        });
                    }
                    i = end;
                }
            }
        }

        IndexedItem {
            text: text.to_string(),
            morae,
            segments,
        }
    }

    /// Compute highlight positions for `text` against `query` without
    /// registering it as an item. Useful for UIs that already know the text
    /// (e.g. from a separately stored list) and just need highlight ranges.
    pub fn highlight(&self, text: &str, query: &str) -> Vec<usize> {
        let item = self.build_item(text);
        let query_morae = query_to_morae(query);
        let query_lower = query.to_lowercase();
        match_item(&item, &query_morae, &query_lower).unwrap_or_default()
    }

    /// Drop the item registered at `index`, shifting later indices down by
    /// one (mirrors `Vec::remove`), to keep in sync with an external list.
    pub fn remove_item(&mut self, index: usize) {
        if index < self.items.len() {
            self.items.remove(index);
        }
    }

    /// Search with a romaji query. Matches either as a contiguous mora
    /// subsequence (handles multi-mora kanji queries like "gen" -> 言) or,
    /// failing that, as a plain case-insensitive substring (keeps plain
    /// ASCII search working for latin item text).
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        let query_morae = query_to_morae(query);
        let query_lower = query.to_lowercase();

        self.items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                match_item(item, &query_morae, &query_lower)
                    .map(|highlight| SearchResult { index, highlight })
            })
            .collect()
    }
}

/// Match a single already-indexed item against a decomposed query, returning
/// its highlight positions on a hit. Shared by `search` (over stored items)
/// and `highlight` (over a freshly-built, unstored item).
fn match_item(item: &IndexedItem, query_morae: &[String], query_lower: &str) -> Option<Vec<usize>> {
    if let Some(start) = find_mora_window(&item.morae, query_morae) {
        return Some(highlight_positions(item, start, start + query_morae.len()));
    }

    if !query_lower.is_empty() && item.text.to_lowercase().contains(query_lower) {
        return Some(plain_highlight_positions(item, query_lower));
    }

    None
}

fn find_mora_window(item_morae: &[String], query_morae: &[String]) -> Option<usize> {
    if query_morae.is_empty() || query_morae.len() > item_morae.len() {
        return None;
    }
    item_morae
        .windows(query_morae.len())
        .position(|w| w == query_morae)
}

fn highlight_positions(item: &IndexedItem, mora_start: usize, mora_end: usize) -> Vec<usize> {
    let mut positions = Vec::new();
    let mut cursor = 0;
    for segment in &item.segments {
        let seg_start = cursor;
        let seg_end = cursor + segment.morae_len;
        if seg_end > mora_start && seg_start < mora_end {
            positions.extend(segment.char_range.clone());
        }
        cursor = seg_end;
    }
    positions
}

fn plain_highlight_positions(item: &IndexedItem, query_lower: &str) -> Vec<usize> {
    let lower_chars: Vec<char> = item.text.to_lowercase().chars().collect();
    let query_chars: Vec<char> = query_lower.chars().collect();
    if query_chars.is_empty() || query_chars.len() > lower_chars.len() {
        return Vec::new();
    }
    let text_len = item.text.chars().count();
    match lower_chars
        .windows(query_chars.len())
        .position(|w| w == query_chars)
    {
        Some(start) => (start..start + query_chars.len())
            .filter(|&i| i < text_len)
            .collect(),
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_euc_jp(text: &str) -> Vec<u8> {
        let (bytes, _, had_errors) = encoding_rs::EUC_JP.encode(text);
        assert!(!had_errors);
        bytes.into_owned()
    }

    fn test_dictionary() -> SkkDictionary {
        let text = "\
;; okuri-nasi entries.
げん /現/言/減/源/
げんご /言語/
じんじ /人事/
かんり /管理/
けんきゅう /研究/
";
        SkkDictionary::from_bytes(&encode_euc_jp(text))
    }

    #[test]
    fn multi_mora_query_hits_kanji() {
        let mut searcher = MigemoSearcher::new(test_dictionary());
        searcher.add_item("言語");

        let results = searcher.search("gen");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].index, 0);
    }

    #[test]
    fn full_word_query_hits_kanji() {
        let mut searcher = MigemoSearcher::new(test_dictionary());
        searcher.add_item("言語");

        let results = searcher.search("gengo");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn jin_hits_jinji_as_a_prefix_of_its_reading() {
        // 人事 reads "jinji" (morae ["ji","n","ji"]); "jin" -> ["ji","n"] is a
        // genuine contiguous prefix of that mora list, so it should match —
        // this is the same class of multi-mora match this design adds for
        // "gen" -> 言, just landing on a prefix instead of a whole word.
        let mut searcher = MigemoSearcher::new(test_dictionary());
        searcher.add_item("人事");

        let results = searcher.search("jin");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn kanri_hits_management() {
        let mut searcher = MigemoSearcher::new(test_dictionary());
        searcher.add_item("プロジェクト管理");

        let results = searcher.search("kanri");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn mora_boundary_is_respected() {
        // "ogu" must not match inside "プログラム" (pu-ro-gu-ra-mu):
        // "og" spans the boundary between "ro" and "gu", not a mora itself.
        let mut searcher = MigemoSearcher::new(test_dictionary());
        searcher.add_item("プログラム");

        assert!(searcher.search("ogu").is_empty());
        assert!(!searcher.search("rogu").is_empty());
    }

    #[test]
    fn ascii_substring_still_matches() {
        let mut searcher = MigemoSearcher::new(test_dictionary());
        searcher.add_item("GitHub");

        let results = searcher.search("git");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].highlight, vec![0, 1, 2]);
    }

    #[test]
    fn highlight_covers_matched_kanji_segment() {
        let mut searcher = MigemoSearcher::new(test_dictionary());
        searcher.add_item("研究会");

        let results = searcher.search("kenkyuu");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].highlight, vec![0, 1]);
    }

    #[test]
    fn remove_item_shifts_indices() {
        let mut searcher = MigemoSearcher::new(test_dictionary());
        searcher.add_item("言語");
        searcher.add_item("管理");
        searcher.remove_item(0);

        let results = searcher.search("kanri");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].index, 0);
    }

    #[test]
    fn unknown_kanji_is_skipped_not_crashing() {
        let mut searcher = MigemoSearcher::new(test_dictionary());
        searcher.add_item("鰯");
        assert!(searcher.search("iwashi").is_empty());
    }
}
