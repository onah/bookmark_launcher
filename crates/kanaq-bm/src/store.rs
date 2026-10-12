//! The bookmark file: `bookmarks.toml` in the shared kanaq data directory.
//!
//! The format is the one the retired ratatui launcher wrote (see the git tag
//! `tui-final`), so existing files load unchanged. Each entry carries a
//! `type` tag; only `"bookmark"` is supported, but the tag is kept so other
//! kinds of entries can be added later without changing the format.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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

    #[test]
    fn missing_file_loads_as_empty() {
        let path = std::env::temp_dir().join("kanaq-bm-test-no-such-file.toml");
        assert_eq!(load(&path).unwrap(), BookmarkFile::default());
    }
}
