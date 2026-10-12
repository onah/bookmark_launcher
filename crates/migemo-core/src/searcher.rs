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

/// A search hit: the index the item was registered at (via `add_item`), the
/// char positions in its text that should be highlighted, and a relevance
/// score (higher is better) usable for ranking and for fusing with other
/// search signals (e.g. a fuzzy matcher's score).
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub index: usize,
    // Part of the public API (see docs/migemo_ja.md section 5); callers that
    // only need the index, like this app's search-then-render-separately
    // flow, are free to ignore it.
    #[allow(dead_code)]
    pub highlight: Vec<usize>,
    pub score: i64,
    /// Whether the query also matches starting at the item's first char
    /// (by reading or as plain text), e.g. "kei" on "経費精算" but not on
    /// "abc-keihi". Lets callers like shell completion prefer prefix hits.
    pub at_start: bool,
}

/// One piece of an item's indexed reading, as reported by
/// [`MigemoSearcher::inspect`]: a run of source chars and the morae it
/// produced. An empty `morae` means the run is not searchable by reading
/// (e.g. a kanji missing from the dictionary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadingSegment {
    pub text: String,
    pub morae: Vec<String>,
}

/// The outcome of matching an item against a query: highlight positions plus
/// a relevance score. Longer matches and matches closer to the start of the
/// text score higher, mirroring the ordering fuzzy_matcher's `SkimMatcherV2`
/// already favors, so the two signals stay comparable after normalization.
struct MatchOutcome {
    positions: Vec<usize>,
    score: i64,
    at_start: bool,
}

fn score_match(matched_len: usize, start: usize, total_len: usize) -> i64 {
    let matched_len = matched_len as i64;
    let earliness = (total_len as i64 - start as i64).max(0);
    matched_len * 100 + earliness
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
        match_item(&item, &query_morae, &query_lower)
            .map(|outcome| outcome.positions)
            .unwrap_or_default()
    }

    /// Break `text` down exactly the way `add_item` indexes it, for
    /// debugging and learning (see `examples/inspect.rs`). The returned
    /// segments cover all of `text` in order; chars the index skips come
    /// back as segments with no morae.
    pub fn inspect(&self, text: &str) -> Vec<ReadingSegment> {
        let item = self.build_item(text);
        let chars: Vec<char> = text.chars().collect();
        let slice = |range: Range<usize>| -> String { chars[range].iter().collect() };
        let unindexed = |range: Range<usize>| ReadingSegment {
            text: slice(range),
            morae: Vec::new(),
        };

        let mut result = Vec::new();
        let mut char_pos = 0;
        let mut mora_pos = 0;
        for segment in &item.segments {
            if segment.char_range.start > char_pos {
                result.push(unindexed(char_pos..segment.char_range.start));
            }
            let mora_end = mora_pos + segment.morae_len;
            result.push(ReadingSegment {
                text: slice(segment.char_range.clone()),
                morae: item.morae[mora_pos..mora_end].to_vec(),
            });
            char_pos = segment.char_range.end;
            mora_pos = mora_end;
        }
        if char_pos < chars.len() {
            result.push(unindexed(char_pos..chars.len()));
        }
        result
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
                match_item(item, &query_morae, &query_lower).map(|outcome| SearchResult {
                    index,
                    highlight: outcome.positions,
                    score: outcome.score,
                    at_start: outcome.at_start,
                })
            })
            .collect()
    }
}

/// Match a single already-indexed item against a decomposed query, returning
/// its highlight positions and a relevance score on a hit. Shared by
/// `search` (over stored items) and `highlight` (over a freshly-built,
/// unstored item).
fn match_item(
    item: &IndexedItem,
    query_morae: &[String],
    query_lower: &str,
) -> Option<MatchOutcome> {
    let at_start = matches_at_start(item, query_morae, query_lower);

    if let Some(start) = find_mora_window(&item.morae, query_morae) {
        let end = start + query_morae.len();
        return Some(MatchOutcome {
            positions: highlight_positions(item, start, end),
            score: score_match(query_morae.len(), start, item.morae.len()),
            at_start,
        });
    }

    if !query_lower.is_empty() && item.text.to_lowercase().contains(query_lower) {
        let query_len = query_lower.chars().count();
        let start = plain_match_start(item, query_lower)?;
        return Some(MatchOutcome {
            positions: plain_highlight_positions(item, query_lower),
            score: score_match(query_len, start, item.text.chars().count()),
            at_start,
        });
    }

    None
}

/// True if the query matches from the item's first char, either by reading
/// (its morae start the item's morae, and those morae begin at char 0, so a
/// leading unindexed char like an unknown kanji does not count) or as a
/// plain case-insensitive prefix. Checked independently of which match
/// `match_item` reports, since e.g. "keihi_経費" hits "kei" by reading at
/// 経費 but also as a plain prefix.
fn matches_at_start(item: &IndexedItem, query_morae: &[String], query_lower: &str) -> bool {
    let by_reading = !query_morae.is_empty()
        && item.morae.starts_with(query_morae)
        && item
            .segments
            .first()
            .is_some_and(|s| s.char_range.start == 0);
    let as_plain = !query_lower.is_empty() && item.text.to_lowercase().starts_with(query_lower);
    by_reading || as_plain
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

fn plain_match_start(item: &IndexedItem, query_lower: &str) -> Option<usize> {
    let lower_chars: Vec<char> = item.text.to_lowercase().chars().collect();
    let query_chars: Vec<char> = query_lower.chars().collect();
    if query_chars.is_empty() || query_chars.len() > lower_chars.len() {
        return None;
    }
    lower_chars
        .windows(query_chars.len())
        .position(|w| w == query_chars)
}

fn plain_highlight_positions(item: &IndexedItem, query_lower: &str) -> Vec<usize> {
    let query_len = query_lower.chars().count();
    let text_len = item.text.chars().count();
    match plain_match_start(item, query_lower) {
        Some(start) => (start..start + query_len)
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

    fn segment(text: &str, morae: &[&str]) -> ReadingSegment {
        ReadingSegment {
            text: text.to_string(),
            morae: morae.iter().map(|m| m.to_string()).collect(),
        }
    }

    #[test]
    fn inspect_pairs_each_source_run_with_its_morae() {
        let searcher = MigemoSearcher::new(test_dictionary());
        assert_eq!(
            searcher.inspect("言語か"),
            vec![segment("言語", &["ge", "n", "go"]), segment("か", &["ka"])]
        );
    }

    #[test]
    fn inspect_reports_unindexed_chars_as_empty_segments() {
        // 猫 is deliberately absent from `test_dictionary()` (the real
        // SKK-JISYO.L reads it as ne,ko), standing in for any kanji the
        // dictionary doesn't know.
        let searcher = MigemoSearcher::new(test_dictionary());
        assert_eq!(
            searcher.inspect("猫管理"),
            vec![segment("猫", &[]), segment("管理", &["ka", "n", "ri"])]
        );
    }

    fn at_start_of(searcher: &MigemoSearcher, query: &str) -> Vec<(usize, bool)> {
        let mut hits: Vec<(usize, bool)> = searcher
            .search(query)
            .iter()
            .map(|r| (r.index, r.at_start))
            .collect();
        hits.sort();
        hits
    }

    #[test]
    fn at_start_distinguishes_prefix_hits_from_inner_hits() {
        let mut searcher = MigemoSearcher::new(test_dictionary());
        searcher.add_item("管理表"); // reading starts with kanri
        searcher.add_item("abc-kanri"); // plain text, inner hit
        searcher.add_item("新管理"); // 新 is not in the test dictionary
        searcher.add_item("Kanri-memo"); // plain prefix, any case
        assert_eq!(
            at_start_of(&searcher, "kanri"),
            vec![(0, true), (1, false), (2, false), (3, true)]
        );
    }

    #[test]
    fn at_start_holds_for_a_plain_prefix_even_when_reading_hits_later() {
        let mut searcher = MigemoSearcher::new(test_dictionary());
        searcher.add_item("kanri_管理");
        assert_eq!(at_start_of(&searcher, "kanri"), vec![(0, true)]);
    }

    #[test]
    fn digits_and_symbols_can_lead_a_reading_query() {
        let mut searcher = MigemoSearcher::new(test_dictionary());
        searcher.add_item("202610_テスト");
        searcher.add_item("202610_管理");
        searcher.add_item("2026-10_管理");
        assert_eq!(at_start_of(&searcher, "202610_te"), vec![(0, true)]);
        assert_eq!(at_start_of(&searcher, "202610_kanri"), vec![(1, true)]);
        assert_eq!(at_start_of(&searcher, "2026-10_ka"), vec![(2, true)]);
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
        // 人事 reads "jinji" (morae ["zi","n","zi"]); "jin" -> ["zi","n"] is a
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

    #[test]
    fn ascii_and_katakana_homophones_both_stay_matchable() {
        // "test.txt" (ASCII) and "テスト.txt" (katakana, reads "tesuto")
        // coexisting must not make either one drop out of the results for
        // a shared query -- each is matched by an independent code path
        // (plain substring vs. mora window) and must not interfere with
        // the other.
        let mut searcher = MigemoSearcher::new(test_dictionary());
        searcher.add_item("test.txt");
        searcher.add_item("テスト.txt");

        for query in ["te", "tes", "test"] {
            let results = searcher.search(query);
            let indices: Vec<usize> = results.iter().map(|r| r.index).collect();
            assert!(
                indices.contains(&0),
                "query {query:?} should still match test.txt, got {indices:?}"
            );
            assert!(
                indices.contains(&1),
                "query {query:?} should still match テスト.txt, got {indices:?}"
            );
        }
    }
}
