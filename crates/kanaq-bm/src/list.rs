//! Formatting bookmarks as lines for fzf (`kanaq-bm list`).
//!
//! Each line is `title<TAB>tags<TAB>url`. The shell side shows the first two
//! fields and searches all three (so a URL like "github" is findable too),
//! then hands the third field to `kanaq-bm open`.

use crate::store::{Bookmark, Entry};

/// Bookmarks in display order: most opened first, ties kept in file order.
/// `kanaq filter` breaks equal match scores by input order, so this order
/// survives filtering.
pub fn sorted_bookmarks(entries: &[Entry]) -> Vec<&Bookmark> {
    let mut bookmarks: Vec<&Bookmark> = entries
        .iter()
        .map(|entry| match entry {
            Entry::Bookmark(bookmark) => bookmark,
        })
        .collect();
    bookmarks.sort_by_key(|b| std::cmp::Reverse(b.access_count));
    bookmarks
}

/// One fzf line for `bookmark`. Tabs and line breaks inside a field are
/// replaced by spaces so they cannot break the line format.
pub fn format_line(bookmark: &Bookmark) -> String {
    let tags: Vec<String> = bookmark
        .tags
        .iter()
        .map(|tag| format!("#{}", clean(tag)))
        .collect();
    format!(
        "{}\t{}\t{}",
        clean(&bookmark.title),
        tags.join(" "),
        clean(&bookmark.url)
    )
}

fn clean(field: &str) -> String {
    field.replace(['\t', '\r', '\n'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bookmark(title: &str, access_count: u32) -> Bookmark {
        Bookmark {
            title: title.into(),
            url: format!("https://example.com/{title}"),
            tags: vec![],
            access_count,
        }
    }

    #[test]
    fn most_opened_first_with_ties_in_file_order() {
        let entries = vec![
            Entry::Bookmark(bookmark("a", 1)),
            Entry::Bookmark(bookmark("b", 5)),
            Entry::Bookmark(bookmark("c", 1)),
        ];
        let titles: Vec<&str> = sorted_bookmarks(&entries)
            .iter()
            .map(|b| b.title.as_str())
            .collect();
        assert_eq!(titles, vec!["b", "a", "c"]);
    }

    #[test]
    fn line_has_title_tags_and_url_separated_by_tabs() {
        let mut b = bookmark("Docs", 0);
        b.tags = vec!["work".into(), "ref".into()];
        assert_eq!(
            format_line(&b),
            "Docs\t#work #ref\thttps://example.com/Docs"
        );
    }

    #[test]
    fn line_without_tags_keeps_an_empty_field() {
        assert_eq!(
            format_line(&bookmark("Docs", 0)),
            "Docs\t\thttps://example.com/Docs"
        );
    }

    #[test]
    fn tabs_and_newlines_in_fields_become_spaces() {
        let mut b = bookmark("a\tb", 0);
        b.title = "line1\nline2\ttab".into();
        assert!(format_line(&b).starts_with("line1 line2 tab\t"));
    }
}
