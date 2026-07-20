use directories::ProjectDirs;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use migemo_core::{MigemoSearcher, SkkDictionary};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Entry {
    Bookmark {
        title: String,
        url: String,
        access_count: u32,
    },
    App {
        title: String,
        command: String,
        args: Vec<String>,
        access_count: u32,
    },
}

impl Entry {
    pub fn title(&self) -> &str {
        match self {
            Entry::Bookmark { title, .. } => title,
            Entry::App { title, .. } => title,
        }
    }

    pub fn access_count(&self) -> u32 {
        match self {
            Entry::Bookmark { access_count, .. } => *access_count,
            Entry::App { access_count, .. } => *access_count,
        }
    }

    pub fn secondary_text(&self) -> &str {
        match self {
            Entry::Bookmark { url, .. } => url,
            Entry::App { command, .. } => command,
        }
    }

    pub fn access_count_mut(&mut self) -> &mut u32 {
        match self {
            Entry::Bookmark { access_count, .. } => access_count,
            Entry::App { access_count, .. } => access_count,
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct BookmarkFile {
    #[serde(default)]
    pub bookmarks: Vec<Entry>,
}

pub fn data_file_path() -> PathBuf {
    if let Some(proj) = ProjectDirs::from("com", "onah", "bookmark_launcher") {
        let dir = proj.data_dir();
        let _ = std::fs::create_dir_all(dir);
        dir.join("bookmarks.toml")
    } else {
        PathBuf::from("bookmarks.toml")
    }
}

pub fn skk_dictionary_path() -> PathBuf {
    data_file_path().with_file_name("SKK-JISYO.L")
}

pub struct AppState {
    bookmarks: Vec<Entry>,
}

impl AppState {
    pub fn new(bookmarks: Vec<Entry>) -> Self {
        Self { bookmarks }
    }

    pub fn bookmarks(&self) -> &[Entry] {
        &self.bookmarks
    }

    pub fn save_bookmarks(&self) -> Result<(), Box<dyn std::error::Error>> {
        let file = BookmarkFile {
            bookmarks: self.bookmarks.clone(),
        };
        let content = toml::to_string_pretty(&file)?;
        let path = data_file_path();
        fs::write(path, content)?;
        Ok(())
    }

    pub fn increment_access_count_by_index(
        &mut self,
        index: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(entry) = self.bookmarks.get_mut(index) {
            *entry.access_count_mut() += 1;
            self.save_bookmarks()?;
        }
        Ok(())
    }

    pub fn delete_bookmark_by_index(
        &mut self,
        index: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if index < self.bookmarks.len() {
            self.bookmarks.remove(index);
            self.save_bookmarks()?;
        }
        Ok(())
    }

    pub fn add_bookmark(&mut self, url: String) -> Result<(), Box<dyn std::error::Error>> {
        if self.bookmarks.iter().any(|b| match b {
            Entry::Bookmark { url: u, .. } => u == &url,
            _ => false,
        }) {
            return Ok(());
        }

        let title = Self::extract_title_from_url(&url);
        let entry = Entry::Bookmark {
            title,
            url,
            access_count: 0,
        };

        self.bookmarks.push(entry);
        self.save_bookmarks()?;
        Ok(())
    }

    fn extract_title_from_url(url: &str) -> String {
        match Self::fetch_page_title(url) {
            Ok(title) if !title.is_empty() => title,
            _ => {
                if let Some(start) = url.find("://") {
                    let domain_part = &url[start + 3..];
                    if let Some(end) = domain_part.find('/') {
                        domain_part[..end].to_string()
                    } else if let Some(end) = domain_part.find('?') {
                        domain_part[..end].to_string()
                    } else {
                        domain_part.to_string()
                    }
                } else {
                    url.to_string()
                }
            }
        }
    }

    fn fetch_page_title(url: &str) -> Result<String, Box<dyn std::error::Error>> {
        let response = reqwest::blocking::get(url)?;
        let html = response.text()?;

        let document = Html::parse_document(&html);
        let selector = Selector::parse("title")?;

        if let Some(title_element) = document.select(&selector).next() {
            let title = title_element.text().collect::<String>().trim().to_string();
            Ok(title)
        } else {
            Ok(String::new())
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchMode {
    Fuzzy,
    Migemo,
    /// Fuses fuzzy and migemo matching: a bookmark is a candidate if either
    /// matches, ranked by a normalized 1:1:1 blend of fuzzy score, migemo
    /// score, and access count (see `App::combined_search`). The default.
    Combined,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortMode {
    ByCount,
    ByScore,
}

fn build_migemo_searcher(bookmarks: &[Entry]) -> Option<MigemoSearcher> {
    let bytes = fs::read(skk_dictionary_path()).ok()?;
    let dictionary = SkkDictionary::from_bytes(&bytes);
    let mut searcher = MigemoSearcher::new(dictionary);
    for entry in bookmarks {
        searcher.add_item(entry.title());
    }
    Some(searcher)
}

pub struct App {
    query: String,
    state: AppState,
    matcher: SkimMatcherV2,
    search_mode: SearchMode,
    sort_mode: SortMode,
    migemo: Option<MigemoSearcher>,
}

impl App {
    pub fn new(bookmarks: Vec<Entry>) -> Self {
        let migemo = build_migemo_searcher(&bookmarks);
        Self {
            query: String::new(),
            state: AppState::new(bookmarks),
            matcher: SkimMatcherV2::default(),
            search_mode: SearchMode::Combined,
            sort_mode: SortMode::ByScore,
            migemo,
        }
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn query_mut(&mut self) -> &mut String {
        &mut self.query
    }

    pub fn bookmarks(&self) -> &[Entry] {
        self.state.bookmarks()
    }

    pub fn search_mode(&self) -> SearchMode {
        self.search_mode
    }

    pub fn set_search_mode(&mut self, mode: SearchMode) {
        self.search_mode = mode;
    }

    pub fn search_mode_label(&self) -> &'static str {
        match self.search_mode {
            SearchMode::Fuzzy => "Fuzzy",
            SearchMode::Migemo => "Migemo",
            SearchMode::Combined => "Fuzzy+Migemo",
        }
    }

    pub fn toggle_sort_mode(&mut self) {
        self.sort_mode = match self.sort_mode {
            SortMode::ByCount => SortMode::ByScore,
            SortMode::ByScore => SortMode::ByCount,
        };
    }

    pub fn sort_mode_label(&self) -> &'static str {
        match self.sort_mode {
            SortMode::ByCount => "Count",
            SortMode::ByScore => "Score",
        }
    }

    pub fn is_migemo_ready(&self) -> bool {
        self.migemo.is_some()
    }

    pub fn search(&self, query: &str) -> Vec<(usize, i64)> {
        match self.search_mode {
            SearchMode::Fuzzy => self.fuzzy_search(query),
            SearchMode::Migemo => self.migemo_search(query),
            SearchMode::Combined => self.combined_search(query),
        }
    }

    pub fn fuzzy_search(&self, query: &str) -> Vec<(usize, i64)> {
        let bookmarks = self.state.bookmarks();

        if query.is_empty() {
            let mut results: Vec<(usize, i64)> =
                (0..bookmarks.len()).map(|i| (i, 0)).collect();
            results.sort_by(|a, b| {
                bookmarks[b.0].access_count().cmp(&bookmarks[a.0].access_count())
            });
            return results;
        }

        let mut results: Vec<(usize, i64)> = bookmarks
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                let title_score = self
                    .matcher
                    .fuzzy(entry.title(), query, false)
                    .map(|(s, _)| s);
                let secondary_score = self
                    .matcher
                    .fuzzy(entry.secondary_text(), query, false)
                    .map(|(s, _)| s);
                match (title_score, secondary_score) {
                    (None, None) => None,
                    (a, b) => Some((index, a.unwrap_or(i64::MIN).max(b.unwrap_or(i64::MIN)))),
                }
            })
            .collect();

        results.sort_by(|a, b| match self.sort_mode {
            SortMode::ByCount => bookmarks[b.0]
                .access_count()
                .cmp(&bookmarks[a.0].access_count())
                .then(b.1.cmp(&a.1)),
            SortMode::ByScore => b.1
                .cmp(&a.1)
                .then(bookmarks[b.0].access_count().cmp(&bookmarks[a.0].access_count())),
        });
        results
    }

    pub fn match_char_positions(&self, title: &str, query: &str) -> Vec<usize> {
        match self.search_mode {
            SearchMode::Fuzzy => self.fuzzy_match_positions(title, query),
            SearchMode::Migemo => self.migemo_match_positions(title, query),
            SearchMode::Combined => {
                let mut positions = self.fuzzy_match_positions(title, query);
                positions.extend(self.migemo_match_positions(title, query));
                positions.sort_unstable();
                positions.dedup();
                positions
            }
        }
    }

    pub fn match_secondary_positions(&self, entry: &Entry, query: &str) -> Vec<usize> {
        match self.search_mode {
            SearchMode::Fuzzy | SearchMode::Combined => {
                self.fuzzy_match_positions(entry.secondary_text(), query)
            }
            SearchMode::Migemo => Vec::new(),
        }
    }

    fn fuzzy_match_positions(&self, title: &str, query: &str) -> Vec<usize> {
        if query.is_empty() {
            return Vec::new();
        }
        self.matcher
            .fuzzy_indices(title, query)
            .map(|(_, indices)| indices)
            .unwrap_or_default()
    }

    fn migemo_search(&self, query: &str) -> Vec<(usize, i64)> {
        let bookmarks = self.state.bookmarks();

        if query.is_empty() {
            let mut results: Vec<(usize, i64)> =
                (0..bookmarks.len()).map(|i| (i, 0)).collect();
            results.sort_by(|a, b| {
                bookmarks[b.0].access_count().cmp(&bookmarks[a.0].access_count())
            });
            return results;
        }

        let Some(engine) = &self.migemo else {
            return Vec::new();
        };

        let mut results: Vec<(usize, i64)> = engine
            .search(query)
            .into_iter()
            .map(|r| (r.index, 0i64))
            .collect();

        results.sort_by(|a, b| {
            bookmarks[b.0].access_count().cmp(&bookmarks[a.0].access_count())
        });
        results
    }

    /// Fuses fuzzy and migemo matching. A bookmark is included if either
    /// matcher hits it (OR), then each of the three signals (fuzzy score,
    /// migemo score, access count) is min-max normalized to 0..=1 across the
    /// candidate set and summed with equal (1:1:1) weight. Normalizing
    /// first is what makes "equal weight" meaningful, since the raw signals
    /// live on unrelated scales (fuzzy_matcher scores, migemo's mora-based
    /// heuristic score, and a plain access counter).
    fn combined_search(&self, query: &str) -> Vec<(usize, i64)> {
        let bookmarks = self.state.bookmarks();

        if query.is_empty() {
            let mut results: Vec<(usize, i64)> =
                (0..bookmarks.len()).map(|i| (i, 0)).collect();
            results.sort_by(|a, b| {
                bookmarks[b.0].access_count().cmp(&bookmarks[a.0].access_count())
            });
            return results;
        }

        let fuzzy_raw: Vec<Option<i64>> = bookmarks
            .iter()
            .map(|entry| {
                let title_score = self
                    .matcher
                    .fuzzy(entry.title(), query, false)
                    .map(|(s, _)| s);
                let secondary_score = self
                    .matcher
                    .fuzzy(entry.secondary_text(), query, false)
                    .map(|(s, _)| s);
                match (title_score, secondary_score) {
                    (None, None) => None,
                    (a, b) => Some(a.unwrap_or(i64::MIN).max(b.unwrap_or(i64::MIN))),
                }
            })
            .collect();

        let mut migemo_raw: Vec<Option<i64>> = vec![None; bookmarks.len()];
        if let Some(engine) = &self.migemo {
            for r in engine.search(query) {
                if let Some(slot) = migemo_raw.get_mut(r.index) {
                    *slot = Some(r.score);
                }
            }
        }

        let candidate_indices: Vec<usize> = (0..bookmarks.len())
            .filter(|&i| fuzzy_raw[i].is_some() || migemo_raw[i].is_some())
            .collect();

        if candidate_indices.is_empty() {
            return Vec::new();
        }

        let fuzzy_max = candidate_indices
            .iter()
            .filter_map(|&i| fuzzy_raw[i])
            .map(|s| s.max(0))
            .max()
            .unwrap_or(0)
            .max(1) as f64;
        let migemo_max = candidate_indices
            .iter()
            .filter_map(|&i| migemo_raw[i])
            .map(|s| s.max(0))
            .max()
            .unwrap_or(0)
            .max(1) as f64;
        let access_max = candidate_indices
            .iter()
            .map(|&i| bookmarks[i].access_count())
            .max()
            .unwrap_or(0)
            .max(1) as f64;

        let mut results: Vec<(usize, i64)> = candidate_indices
            .iter()
            .map(|&index| {
                let fuzzy_norm = fuzzy_raw[index]
                    .map(|s| s.max(0) as f64 / fuzzy_max)
                    .unwrap_or(0.0);
                let migemo_norm = migemo_raw[index]
                    .map(|s| s.max(0) as f64 / migemo_max)
                    .unwrap_or(0.0);
                let access_norm = bookmarks[index].access_count() as f64 / access_max;
                let combined = fuzzy_norm + migemo_norm + access_norm;
                (index, (combined * 1_000_000.0).round() as i64)
            })
            .collect();

        results.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then(bookmarks[b.0].access_count().cmp(&bookmarks[a.0].access_count()))
        });
        results
    }

    fn migemo_match_positions(&self, title: &str, query: &str) -> Vec<usize> {
        if query.is_empty() {
            return Vec::new();
        }
        let Some(engine) = &self.migemo else {
            return Vec::new();
        };
        engine.highlight(title, query)
    }

    pub fn increment_access_count_by_index(
        &mut self,
        index: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.state.increment_access_count_by_index(index)
    }

    pub fn add_bookmark(&mut self, url: String) -> Result<(), Box<dyn std::error::Error>> {
        let count_before = self.state.bookmarks().len();
        self.state.add_bookmark(url)?;
        if self.state.bookmarks().len() > count_before
            && let Some(entry) = self.state.bookmarks().last()
        {
            let title = entry.title().to_string();
            if let Some(engine) = &mut self.migemo {
                engine.add_item(&title);
            }
        }
        Ok(())
    }

    pub fn delete_bookmark_by_index(
        &mut self,
        index: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.state.delete_bookmark_by_index(index)?;
        if let Some(engine) = &mut self.migemo {
            engine.remove_item(index);
        }
        Ok(())
    }
}

#[cfg(test)]
mod highlight_tests {
    use super::*;

    fn bookmark(title: &str) -> Entry {
        Entry::Bookmark {
            title: title.to_string(),
            url: "https://example.com".to_string(),
            access_count: 0,
        }
    }

    #[test]
    fn empty_query_returns_no_positions() {
        let app = App::new(vec![bookmark("GitHub")]);
        assert!(app.match_char_positions("GitHub", "").is_empty());
    }

    #[test]
    fn no_match_returns_empty_positions() {
        let app = App::new(vec![bookmark("GitHub")]);
        assert!(app.match_char_positions("GitHub", "xyz").is_empty());
    }

    #[test]
    fn positions_agree_with_search() {
        let app = App::new(vec![bookmark("GitHub")]);
        let query = "gh";
        let search_found_match = !app.fuzzy_search(query).is_empty();
        let positions = app.match_char_positions("GitHub", query);
        assert_eq!(
            search_found_match,
            !positions.is_empty(),
            "match_char_positions and fuzzy_search should agree on whether a match exists"
        );
    }

    #[test]
    fn all_returned_positions_within_title_length() {
        let app = App::new(vec![bookmark("GitHub")]);
        let positions = app.match_char_positions("GitHub", "gh");
        let title_len = "GitHub".chars().count();
        for &p in &positions {
            assert!(
                p < title_len,
                "position {} out of bounds (len={})",
                p,
                title_len
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bookmark(title: &str) -> Entry {
        Entry::Bookmark {
            title: title.to_string(),
            url: "https://example.com".to_string(),
            access_count: 0,
        }
    }

    #[test]
    fn migemo_mode_gen_hits_kanji_with_multi_mora_reading() {
        // This is the exact bug docs/migemo_ja.md rewrites the engine to fix:
        // a regex-alternation migemo can't match multi-mora romaji ("gen")
        // against kanji, only single-mora ("ge"). The new mora-index engine
        // can, so "gen" must hit "言".
        let mut app = App::new(vec![bookmark("言")]);
        assert!(
            app.is_migemo_ready(),
            "Migemo dictionary is not loaded at {}",
            skk_dictionary_path().display()
        );

        app.set_search_mode(SearchMode::Migemo);
        let results = app.search("gen");

        assert!(
            !results.is_empty(),
            "Expected 'gen' to hit '言' (reading げん), got: {:?}",
            results
        );
    }

    #[test]
    fn migemo_mode_jin_hits_jinji_via_kanji_reading() {
        // 人 alone is read じん (jin), and 人事 is read じんじ (jinji), so "jin"
        // is a genuine prefix match, not a false positive.
        let mut app = App::new(vec![bookmark("人事")]);
        assert!(app.is_migemo_ready());

        app.set_search_mode(SearchMode::Migemo);
        let results = app.search("jin");

        assert!(
            !results.is_empty(),
            "Expected 'jin' to hit '人事' (reading じんじ), got: {:?}",
            results
        );
    }

    #[test]
    fn combined_is_the_default_search_mode() {
        let app = App::new(vec![]);
        assert_eq!(app.search_mode(), SearchMode::Combined);
    }

    #[test]
    fn combined_mode_surfaces_migemo_only_hits() {
        // "gen" (latin) shares no characters with "言" (kanji), so fuzzy alone
        // can never match this pair; only migemo's romaji-mora reading match
        // can. Combined mode must still surface it via the migemo signal.
        let app = App::new(vec![bookmark("言")]);
        assert!(app.is_migemo_ready());
        assert_eq!(app.search_mode(), SearchMode::Combined);

        let results = app.search("gen");
        assert!(
            !results.is_empty(),
            "Expected combined mode to surface the migemo-only hit, got: {:?}",
            results
        );
    }

    #[test]
    fn combined_mode_surfaces_fuzzy_only_hits() {
        // "Fbr" is a non-contiguous subsequence of "Foobar" (fuzzy matches
        // it), but not a contiguous substring or a decodable romaji-mora
        // query, so migemo alone won't match it. Combined mode must still
        // surface it via the fuzzy signal.
        let app = App::new(vec![bookmark("Foobar")]);
        assert!(app.migemo_search("Fbr").is_empty());
        assert!(!app.fuzzy_search("Fbr").is_empty());

        let results = app.search("Fbr");
        assert!(
            !results.is_empty(),
            "Expected combined mode to surface the fuzzy-only hit, got: {:?}",
            results
        );
    }

    #[test]
    fn combined_mode_breaks_ties_by_access_count() {
        let mut low = bookmark("Test");
        let mut high = bookmark("Test");
        *low.access_count_mut() = 1;
        *high.access_count_mut() = 100;
        // Put the low-count entry first so a naive "first match wins" bug
        // wouldn't be caught by index order alone.
        let app = App::new(vec![low, high]);

        let results = app.search("Test");
        assert_eq!(
            results.first().map(|(idx, _)| *idx),
            Some(1),
            "Expected the higher access_count entry (index 1) to rank first, got: {:?}",
            results
        );
    }
}
