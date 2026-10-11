//! Core logic for the `migemo-fzf` reload filter.
//!
//! Kept independent of stdin/stdout/filesystem so it can be unit tested
//! directly; `main.rs` is the thin CLI wrapper that does I/O and dictionary
//! loading.

use migemo_core::{MigemoSearcher, SkkDictionary};
use std::collections::HashSet;

const HIGHLIGHT_ON: &str = "\x1b[1;31m";
const HIGHLIGHT_OFF: &str = "\x1b[0m";

/// Rank `lines` against `query`, returning the survivors highest-score-first
/// with ties broken by original position (stable, so e.g. shell-history
/// recency order survives among equal scores).
///
/// An empty query returns every line unchanged, in input order -- matching
/// fzf's own unfiltered `--disabled` state, so the first reload (before the
/// user has typed anything) shows the full candidate list.
///
/// When `highlight` is set, each surviving line has its matched char
/// positions wrapped in ANSI bold-red escapes, for display with fzf's
/// `--ansi`.
pub fn filter_lines(
    lines: &[String],
    query: &str,
    dict: SkkDictionary,
    highlight: bool,
) -> Vec<String> {
    if query.is_empty() {
        return lines.to_vec();
    }

    let mut searcher = MigemoSearcher::new(dict);
    for line in lines {
        searcher.add_item(line);
    }

    let mut results = searcher.search(query);
    results.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.index.cmp(&b.index)));

    results
        .into_iter()
        .map(|r| {
            let line = &lines[r.index];
            if highlight {
                apply_highlight(line, &r.highlight)
            } else {
                line.clone()
            }
        })
        .collect()
}

fn apply_highlight(line: &str, highlight: &[usize]) -> String {
    if highlight.is_empty() {
        return line.to_string();
    }
    let marked: HashSet<usize> = highlight.iter().copied().collect();
    let mut out = String::with_capacity(line.len());
    let mut in_run = false;
    for (i, c) in line.chars().enumerate() {
        let hit = marked.contains(&i);
        if hit && !in_run {
            out.push_str(HIGHLIGHT_ON);
            in_run = true;
        } else if !hit && in_run {
            out.push_str(HIGHLIGHT_OFF);
            in_run = false;
        }
        out.push(c);
    }
    if in_run {
        out.push_str(HIGHLIGHT_OFF);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_euc_jp(text: &str) -> Vec<u8> {
        let (bytes, _, had_errors) = encoding_rs::EUC_JP.encode(text);
        assert!(!had_errors);
        bytes.into_owned()
    }

    fn empty_dict() -> SkkDictionary {
        SkkDictionary::from_bytes(&encode_euc_jp(";; okuri-nasi entries.\n"))
    }

    fn test_dictionary() -> SkkDictionary {
        let text = "\
;; okuri-nasi entries.
げんご /言語/
";
        SkkDictionary::from_bytes(&encode_euc_jp(text))
    }

    #[test]
    fn empty_query_passes_lines_through_in_input_order() {
        let lines = vec!["zeta".to_string(), "alpha".to_string()];
        let out = filter_lines(&lines, "", empty_dict(), false);
        assert_eq!(out, vec!["zeta", "alpha"]);
    }

    #[test]
    fn ranks_by_score_dropping_non_matches() {
        // score_match rewards a longer `total_len` at the same match start
        // (see crates/migemo-core/src/searcher.rs), so "apple" outranks "app" here;
        // this test's job is just to confirm "documents" (no match) is gone.
        let lines = vec![
            "documents".to_string(),
            "app".to_string(),
            "apple".to_string(),
        ];
        let out = filter_lines(&lines, "app", empty_dict(), false);
        assert_eq!(out.len(), 2);
        assert!(out.contains(&"app".to_string()));
        assert!(out.contains(&"apple".to_string()));
    }

    #[test]
    fn ties_break_by_input_order() {
        let lines = vec!["banana".to_string(), "apple".to_string()];
        let out = filter_lines(&lines, "a", empty_dict(), false);
        assert_eq!(out, vec!["banana", "apple"]);
    }

    #[test]
    fn romaji_query_matches_kanji_reading() {
        let lines = vec!["言語".to_string(), "other".to_string()];
        let out = filter_lines(&lines, "gengo", test_dictionary(), false);
        assert_eq!(out, vec!["言語"]);
    }

    #[test]
    fn highlight_wraps_matched_run_in_ansi_codes() {
        let lines = vec!["GitHub".to_string()];
        let out = filter_lines(&lines, "git", empty_dict(), true);
        assert_eq!(out, vec!["\x1b[1;31mGit\x1b[0mHub"]);
    }

    #[test]
    fn no_highlight_leaves_line_untouched() {
        let lines = vec!["GitHub".to_string()];
        let out = filter_lines(&lines, "git", empty_dict(), false);
        assert_eq!(out, vec!["GitHub"]);
    }

    #[test]
    fn non_matching_lines_are_dropped() {
        let lines = vec!["apple".to_string(), "banana".to_string()];
        let out = filter_lines(&lines, "zzz", empty_dict(), false);
        assert!(out.is_empty());
    }
}
