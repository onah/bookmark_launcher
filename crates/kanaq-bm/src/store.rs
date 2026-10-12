//! The bookmark file: `bookmarks.toml` in the shared kanaq data directory.
//!
//! The format is the one the retired ratatui launcher wrote (see the git tag
//! `tui-final`), so existing files load unchanged. Each entry carries a
//! `type` tag; only `"bookmark"` is supported, but the tag is kept so other
//! kinds of entries can be added later without changing the format.
//!
//! The file is also meant to be edited by hand, so updates go through
//! `toml_edit` and touch only the values they change: comments, ordering,
//! and formatting written by hand survive.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use toml_edit::{ArrayOfTables, DocumentMut, Item, Table};

const FILE_NAME: &str = "bookmarks.toml";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Entry {
    Bookmark(Bookmark),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bookmark {
    pub title: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default)]
    pub access_count: u32,
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookmarkFile {
    #[serde(default)]
    pub bookmarks: Vec<Entry>,
}

/// `bookmarks.toml` in the shared data directory, or `None` if the platform
/// has no home directory to put it under.
pub fn default_path() -> Option<PathBuf> {
    migemo_core::data_dir().map(|dir| dir.join(FILE_NAME))
}

/// Parse a bookmark file's contents.
pub fn parse(text: &str) -> Result<BookmarkFile, String> {
    toml::from_str(text).map_err(|e| e.to_string())
}

/// Load the bookmark file at `path`. A missing file is an empty list (no
/// bookmarks added yet); any other read or parse failure is an error, so a
/// damaged file is reported instead of being treated as empty and later
/// overwritten.
pub fn load(path: &Path) -> Result<BookmarkFile, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => parse(&text).map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BookmarkFile::default()),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// Return `text` with the `access_count` of the bookmark whose url is `url`
/// incremented by one, leaving everything else as written.
pub fn increment_access_count(text: &str, url: &str) -> Result<String, String> {
    edit(text, |tables| {
        let index = position(tables, url).ok_or_else(|| format!("no bookmark with url {url}"))?;
        let table = tables.get_mut(index).expect("index from position");
        let count = table
            .get("access_count")
            .and_then(|v| v.as_integer())
            .unwrap_or(0);
        table["access_count"] = toml_edit::value(count + 1);
        Ok(())
    })
}

/// Return `text` with `bookmark` appended, leaving existing entries as
/// written. A bookmark with the same url is an error.
pub fn add_bookmark(text: &str, bookmark: &Bookmark) -> Result<String, String> {
    edit(text, |tables| {
        if position(tables, &bookmark.url).is_some() {
            return Err(format!("already bookmarked: {}", bookmark.url));
        }
        let mut table = Table::new();
        table["type"] = toml_edit::value("bookmark");
        table["title"] = toml_edit::value(&bookmark.title);
        table["url"] = toml_edit::value(&bookmark.url);
        if !bookmark.tags.is_empty() {
            table["tags"] = toml_edit::value(bookmark.tags.iter().collect::<toml_edit::Array>());
        }
        table["access_count"] = toml_edit::value(i64::from(bookmark.access_count));
        if !tables.is_empty() {
            // Keep a blank line between entries, like the rest of the file.
            table.decor_mut().set_prefix("\n");
        }
        tables.push(table);
        Ok(())
    })
}

/// Return `text` without the bookmark whose url is `url`.
pub fn remove_bookmark(text: &str, url: &str) -> Result<String, String> {
    edit(text, |tables| {
        let index = position(tables, url).ok_or_else(|| format!("no bookmark with url {url}"))?;
        tables.remove(index);
        Ok(())
    })
}

/// Apply `change` to the `[[bookmarks]]` tables of `text` (created if the
/// file has none yet) and return the edited text. The result is checked to
/// still parse, so a bad edit is never handed back to be saved.
fn edit(
    text: &str,
    change: impl FnOnce(&mut ArrayOfTables) -> Result<(), String>,
) -> Result<String, String> {
    let mut doc: DocumentMut = text
        .parse()
        .map_err(|e: toml_edit::TomlError| e.to_string())?;
    let tables = doc
        .entry("bookmarks")
        .or_insert_with(|| Item::ArrayOfTables(ArrayOfTables::new()))
        .as_array_of_tables_mut()
        .ok_or("`bookmarks` must be written as [[bookmarks]] tables")?;
    change(tables)?;

    let updated = doc.to_string();
    parse(&updated)?;
    Ok(updated)
}

fn position(tables: &ArrayOfTables, url: &str) -> Option<usize> {
    tables
        .iter()
        .position(|t| t.get("url").and_then(|v| v.as_str()) == Some(url))
}

/// Write `contents` to `path` via a temporary file in the same directory and
/// a rename, so an interrupted write never leaves a truncated file behind.
pub fn write_atomically(path: &Path, contents: &str) -> Result<(), String> {
    let describe = |e: std::io::Error| format!("{}: {e}", path.display());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(describe)?;
    }
    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(format!(".{}.tmp", std::process::id()));
    let tmp = path.with_file_name(tmp_name);

    std::fs::write(&tmp, contents)
        .and_then(|()| std::fs::rename(&tmp, path))
        .map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            describe(e)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_format_written_by_the_old_launcher() {
        let text = r#"
[[bookmarks]]
type = "bookmark"
title = "onah GitHub"
url = "https://github.com/onah"
access_count = 4
"#;
        let file = parse(text).unwrap();
        assert_eq!(
            file.bookmarks,
            vec![Entry::Bookmark(Bookmark {
                title: "onah GitHub".into(),
                url: "https://github.com/onah".into(),
                tags: vec![],
                access_count: 4,
            })]
        );
    }

    #[test]
    fn parses_tags_and_defaults_missing_fields() {
        let text = r#"
[[bookmarks]]
type = "bookmark"
title = "Docs"
url = "https://example.com/docs"
tags = ["work", "ref"]
"#;
        let Entry::Bookmark(bookmark) = &parse(text).unwrap().bookmarks[0];
        assert_eq!(bookmark.tags, vec!["work", "ref"]);
        assert_eq!(bookmark.access_count, 0);
    }

    #[test]
    fn empty_text_is_an_empty_list() {
        assert_eq!(parse("").unwrap(), BookmarkFile::default());
    }

    #[test]
    fn unknown_entry_type_is_an_error() {
        let text = r#"
[[bookmarks]]
type = "app"
title = "Terminal"
command = "wt"
"#;
        assert!(parse(text).is_err());
    }

    const TWO_BOOKMARKS: &str = r#"# my bookmarks
[[bookmarks]]
type = "bookmark"
title = "A"
url = "https://a.example"
access_count = 4

[[bookmarks]]
type = "bookmark"   # no count yet
title = "B"
url = "https://b.example"
"#;

    fn access_counts(text: &str) -> Vec<u32> {
        parse(text)
            .unwrap()
            .bookmarks
            .iter()
            .map(|Entry::Bookmark(b)| b.access_count)
            .collect()
    }

    #[test]
    fn increment_bumps_only_the_matching_bookmark() {
        let updated = increment_access_count(TWO_BOOKMARKS, "https://a.example").unwrap();
        assert_eq!(access_counts(&updated), vec![5, 0]);
    }

    #[test]
    fn increment_adds_a_missing_count() {
        let updated = increment_access_count(TWO_BOOKMARKS, "https://b.example").unwrap();
        assert_eq!(access_counts(&updated), vec![4, 1]);
    }

    #[test]
    fn increment_keeps_hand_written_comments() {
        let updated = increment_access_count(TWO_BOOKMARKS, "https://b.example").unwrap();
        assert!(updated.starts_with("# my bookmarks\n"));
        assert!(updated.contains("# no count yet"));
    }

    #[test]
    fn increment_rejects_an_unknown_url() {
        assert!(increment_access_count(TWO_BOOKMARKS, "https://c.example").is_err());
    }

    fn new_bookmark(url: &str, tags: &[&str]) -> Bookmark {
        Bookmark {
            title: "New".into(),
            url: url.into(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            access_count: 0,
        }
    }

    #[test]
    fn add_appends_a_bookmark_and_keeps_the_rest() {
        let updated = add_bookmark(
            TWO_BOOKMARKS,
            &new_bookmark("https://c.example", &["work", "ref"]),
        )
        .unwrap();
        assert!(updated.starts_with(TWO_BOOKMARKS));
        let file = parse(&updated).unwrap();
        assert_eq!(
            file.bookmarks.last(),
            Some(&Entry::Bookmark(new_bookmark(
                "https://c.example",
                &["work", "ref"]
            )))
        );
    }

    #[test]
    fn add_to_an_empty_file_creates_the_list() {
        let updated = add_bookmark("", &new_bookmark("https://c.example", &[])).unwrap();
        assert_eq!(parse(&updated).unwrap().bookmarks.len(), 1);
        assert!(updated.starts_with("[[bookmarks]]"));
    }

    #[test]
    fn add_rejects_a_duplicate_url() {
        assert!(add_bookmark(TWO_BOOKMARKS, &new_bookmark("https://a.example", &[])).is_err());
    }

    #[test]
    fn remove_drops_only_the_matching_bookmark() {
        let updated = remove_bookmark(TWO_BOOKMARKS, "https://a.example").unwrap();
        let file = parse(&updated).unwrap();
        let urls: Vec<&str> = file
            .bookmarks
            .iter()
            .map(|Entry::Bookmark(b)| b.url.as_str())
            .collect();
        assert_eq!(urls, vec!["https://b.example"]);
        assert!(updated.contains("# no count yet"));
    }

    #[test]
    fn remove_rejects_an_unknown_url() {
        assert!(remove_bookmark(TWO_BOOKMARKS, "https://c.example").is_err());
    }

    #[test]
    fn write_atomically_replaces_the_file_and_leaves_no_temp_file() {
        let dir = std::env::temp_dir().join(format!("kanaq-bm-test-{}", std::process::id()));
        let path = dir.join("bookmarks.toml");
        write_atomically(&path, "first").unwrap();
        write_atomically(&path, "second").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");
        let names: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, vec!["bookmarks.toml"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn missing_file_loads_as_empty() {
        let path = std::env::temp_dir().join("kanaq-bm-test-no-such-file.toml");
        assert_eq!(load(&path).unwrap(), BookmarkFile::default());
    }
}
