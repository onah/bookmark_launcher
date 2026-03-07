use directories::ProjectDirs;
use fuzzy_matcher::skim::SkimMatcherV2;
#[cfg(feature = "backend-tui")]
use regex::Regex;
#[cfg(feature = "backend-tui")]
use rustmigemo::migemo::compact_dictionary::CompactDictionary;
#[cfg(feature = "backend-tui")]
use rustmigemo::migemo::query::query as migemo_query;
#[cfg(feature = "backend-tui")]
use rustmigemo::migemo::regex_generator::RegexOperator;
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

    pub fn display(&self) -> String {
        match self {
            Entry::Bookmark { title, url, .. } => format!("{} ({})", title, url),
            Entry::App {
                title,
                command,
                args,
                ..
            } => {
                if args.is_empty() {
                    format!("{} ({})", title, command)
                } else {
                    format!("{} ({} {})", title, command, args.join(" "))
                }
            }
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
        // ignore error if directory already exists or cannot be created
        let _ = std::fs::create_dir_all(dir);
        dir.join("bookmarks.toml")
    } else {
        PathBuf::from("bookmarks.toml")
    }
}

#[cfg(feature = "backend-tui")]
pub fn migemo_dict_path() -> PathBuf {
    data_file_path().with_file_name("migemo-compact-dict")
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

#[cfg(feature = "backend-tui")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchMode {
    Fuzzy,
    Migemo,
}

#[cfg(feature = "backend-tui")]
struct MigemoEngine {
    dictionary: CompactDictionary,
}

#[cfg(feature = "backend-tui")]
impl MigemoEngine {
    fn load() -> Option<Self> {
        let dict_path = migemo_dict_path();
        let bytes = fs::read(dict_path).ok()?;
        Some(Self {
            dictionary: CompactDictionary::new(&bytes),
        })
    }

    fn build_regex(&self, query: &str) -> Option<Regex> {
        let pattern = migemo_query(query.to_string(), &self.dictionary, &RegexOperator::Default);
        if pattern.is_empty() {
            return None;
        }
        Regex::new(&pattern).ok()
    }
}

pub struct App {
    query: String,
    state: AppState,
    matcher: SkimMatcherV2,
    #[cfg(feature = "backend-tui")]
    search_mode: SearchMode,
    #[cfg(feature = "backend-tui")]
    migemo: Option<MigemoEngine>,
}

impl App {
    pub fn new(bookmarks: Vec<Entry>) -> Self {
        Self {
            query: String::new(),
            state: AppState::new(bookmarks),
            matcher: SkimMatcherV2::default(),
            #[cfg(feature = "backend-tui")]
            search_mode: SearchMode::Fuzzy,
            #[cfg(feature = "backend-tui")]
            migemo: MigemoEngine::load(),
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

    #[cfg(feature = "backend-tui")]
    pub fn search_mode(&self) -> SearchMode {
        self.search_mode
    }

    #[cfg(feature = "backend-tui")]
    pub fn set_search_mode(&mut self, mode: SearchMode) {
        self.search_mode = mode;
    }

    #[cfg(feature = "backend-tui")]
    pub fn search_mode_label(&self) -> &'static str {
        match self.search_mode {
            SearchMode::Fuzzy => "Fuzzy",
            SearchMode::Migemo => "Migemo",
        }
    }

    #[cfg(feature = "backend-tui")]
    pub fn is_migemo_ready(&self) -> bool {
        self.migemo.is_some()
    }

    #[cfg(feature = "backend-tui")]
    pub fn search(&self, query: &str) -> Vec<(usize, i64)> {
        match self.search_mode {
            SearchMode::Fuzzy => self.fuzzy_search(query),
            SearchMode::Migemo => self.migemo_search(query),
        }
    }

    pub fn fuzzy_search(&self, query: &str) -> Vec<(usize, i64)> {
        if query.is_empty() {
            return (0..self.state.bookmarks().len()).map(|i| (i, 0)).collect();
        }

        let mut results: Vec<(usize, i64)> = self
            .state
            .bookmarks()
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                self.matcher
                    .fuzzy(entry.title(), query, false)
                    .map(|(score, _)| (index, score))
            })
            .collect();

        results.sort_by(|a, b| b.1.cmp(&a.1));
        results
    }

    #[cfg(feature = "backend-tui")]
    fn migemo_search(&self, query: &str) -> Vec<(usize, i64)> {
        if query.is_empty() {
            return (0..self.state.bookmarks().len()).map(|i| (i, 0)).collect();
        }

        let Some(engine) = &self.migemo else {
            return Vec::new();
        };

        let Some(regex) = engine.build_regex(query) else {
            return Vec::new();
        };

        self.state
            .bookmarks()
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                if regex.is_match(entry.title()) {
                    Some((index, 0))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn increment_access_count_by_index(
        &mut self,
        index: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.state.increment_access_count_by_index(index)
    }

    pub fn add_bookmark(&mut self, url: String) -> Result<(), Box<dyn std::error::Error>> {
        self.state.add_bookmark(url)
    }
}
