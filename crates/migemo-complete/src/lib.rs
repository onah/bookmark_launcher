//! Core logic for the `migemo-complete` shell completion helper.
//!
//! Kept independent of stdin/stdout/filesystem so it can be unit tested
//! directly; `main.rs` is the thin CLI wrapper that does I/O and calls into
//! this module.

use migemo_core::{MigemoSearcher, SkkDictionary};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Dir,
    File,
    Any,
}

impl Kind {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "dir" => Some(Kind::Dir),
            "file" => Some(Kind::File),
            "any" => Some(Kind::Any),
            _ => None,
        }
    }
}

/// One directory entry as seen by the shell, decoupled from `std::fs` so
/// tests can supply fixtures without touching the filesystem.
#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub is_dir: bool,
}

/// Split a raw completion token (as typed by the user, e.g. `"Documents/ap"`)
/// into the directory prefix to keep verbatim (`"Documents/"`) and the leaf
/// fragment to match against entry names (`"ap"`). A token with no separator
/// has an empty prefix.
pub fn split_token(token: &str) -> (&str, &str) {
    match token.rfind(['/', '\\']) {
        Some(i) => token.split_at(i + 1),
        None => ("", token),
    }
}

/// Filter `entries` by `kind` and rank them against `leaf_query`.
///
/// An empty query (the user has typed nothing past the directory separator
/// yet) returns every matching entry, alphabetically, matching plain shell
/// completion behavior. Otherwise ranks by `MigemoSearcher` score,
/// highest first, breaking ties alphabetically for determinism.
pub fn rank_candidates(
    entries: &[Entry],
    leaf_query: &str,
    kind: Kind,
    dict: SkkDictionary,
) -> Vec<String> {
    let filtered: Vec<&Entry> = entries
        .iter()
        .filter(|e| match kind {
            Kind::Dir => e.is_dir,
            Kind::File => !e.is_dir,
            Kind::Any => true,
        })
        .collect();

    if leaf_query.is_empty() {
        let mut names: Vec<String> = filtered.iter().map(|e| e.name.clone()).collect();
        names.sort();
        return names;
    }

    let mut searcher = MigemoSearcher::new(dict);
    for entry in &filtered {
        searcher.add_item(&entry.name);
    }

    let mut results = searcher.search(leaf_query);
    results.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| filtered[a.index].name.cmp(&filtered[b.index].name))
    });

    results
        .into_iter()
        .map(|r| filtered[r.index].name.clone())
        .collect()
}

/// Combine the verbatim directory prefix with a matched entry name, adding a
/// trailing separator for directories so the shell can chain another Tab
/// press straight into the next path segment.
pub fn format_candidate(dir_prefix: &str, entry: &Entry) -> String {
    let mut out = format!("{dir_prefix}{}", entry.name);
    if entry.is_dir {
        out.push('/');
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

    #[test]
    fn split_token_separates_prefix_and_leaf() {
        assert_eq!(split_token("Documents/ap"), ("Documents/", "ap"));
        assert_eq!(split_token("ap"), ("", "ap"));
        assert_eq!(split_token("a/b/c"), ("a/b/", "c"));
    }

    #[test]
    fn split_token_handles_backslash_for_windows() {
        assert_eq!(split_token(r"Documents\ap"), (r"Documents\", "ap"));
    }

    #[test]
    fn empty_query_lists_all_matching_kind_alphabetically() {
        let entries = vec![
            Entry {
                name: "zeta".into(),
                is_dir: true,
            },
            Entry {
                name: "alpha".into(),
                is_dir: true,
            },
            Entry {
                name: "readme.txt".into(),
                is_dir: false,
            },
        ];
        let names = rank_candidates(&entries, "", Kind::Dir, empty_dict());
        assert_eq!(names, vec!["alpha", "zeta"]);
    }

    #[test]
    fn katakana_directory_matches_romaji_prefix() {
        // "アプリ" (apuri) is pure katakana, so this must work without any
        // kanji dictionary entries -- the exact "cd ap<TAB>" scenario.
        let entries = vec![
            Entry {
                name: "アプリ".into(),
                is_dir: true,
            },
            Entry {
                name: "Documents".into(),
                is_dir: true,
            },
        ];
        let names = rank_candidates(&entries, "ap", Kind::Dir, empty_dict());
        assert_eq!(names, vec!["アプリ"]);
    }

    #[test]
    fn kind_filter_excludes_files_when_dir_requested() {
        let entries = vec![
            Entry {
                name: "apple.txt".into(),
                is_dir: false,
            },
            Entry {
                name: "app".into(),
                is_dir: true,
            },
        ];
        let names = rank_candidates(&entries, "app", Kind::Dir, empty_dict());
        assert_eq!(names, vec!["app"]);
    }

    #[test]
    fn ascii_and_katakana_homophone_files_both_survive_ranking() {
        // Regression: "test.txt" and "テスト.txt" (katakana, reads "tesuto")
        // coexisting in the same directory must not make either one vanish
        // from the ranked results.
        let entries = vec![
            Entry {
                name: "test.txt".into(),
                is_dir: false,
            },
            Entry {
                name: "テスト.txt".into(),
                is_dir: false,
            },
        ];
        for query in ["te", "tes", "test"] {
            let names = rank_candidates(&entries, query, Kind::Any, empty_dict());
            assert!(
                names.contains(&"test.txt".to_string()),
                "query {query:?} dropped test.txt, got {names:?}"
            );
            assert!(
                names.contains(&"テスト.txt".to_string()),
                "query {query:?} dropped テスト.txt, got {names:?}"
            );
        }
    }

    #[test]
    fn ascii_and_katakana_homophone_directories_both_survive_ranking() {
        let entries = vec![
            Entry {
                name: "test".into(),
                is_dir: true,
            },
            Entry {
                name: "テスト".into(),
                is_dir: true,
            },
        ];
        for query in ["te", "tes", "test"] {
            let names = rank_candidates(&entries, query, Kind::Dir, empty_dict());
            assert_eq!(
                names.len(),
                2,
                "query {query:?} should keep both directories, got {names:?}"
            );
        }
    }

    #[test]
    fn format_candidate_appends_separator_for_directories_only() {
        let dir = Entry {
            name: "アプリ".into(),
            is_dir: true,
        };
        let file = Entry {
            name: "notes.txt".into(),
            is_dir: false,
        };
        assert_eq!(format_candidate("Documents/", &dir), "Documents/アプリ/");
        assert_eq!(format_candidate("", &file), "notes.txt");
    }
}
