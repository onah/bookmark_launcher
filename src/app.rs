use directories::ProjectDirs;
use fuzzy_matcher::skim::SkimMatcherV2;
use reqwest;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
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

pub fn data_file_path() -> PathBuf {
    if let Some(proj) = ProjectDirs::from("com", "onah", "bookmark_launcher") {
        let dir = proj.data_dir();
        // ignore error if directory already exists or cannot be created
        let _ = std::fs::create_dir_all(dir);
        dir.join("bookmarks.json")
    } else {
        PathBuf::from("bookmarks.json")
    }
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
        let json = serde_json::to_string_pretty(&self.bookmarks)?;
        let path = data_file_path();
        fs::write(path, json)?;
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

    pub fn increment_access_count_by_entry(
        &mut self,
        target: &Entry,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let maybe_idx =
            self.bookmarks
                .iter_mut()
                .enumerate()
                .find_map(|(i, e)| match (e, target) {
                    (Entry::Bookmark { url: u1, .. }, Entry::Bookmark { url: u2, .. })
                        if u1 == u2 =>
                    {
                        Some(i)
                    }
                    (Entry::App { command: c1, .. }, Entry::App { command: c2, .. })
                        if c1 == c2 =>
                    {
                        Some(i)
                    }
                    _ => None,
                });

        if let Some(idx) = maybe_idx {
            let entry = &mut self.bookmarks[idx];
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

pub struct App {
    query: String,
    state: AppState,
    filtered_bookmarks: Vec<Entry>,
    initial_focus: bool,
    matcher: SkimMatcherV2,
}

impl App {
    pub fn new(bookmarks: Vec<Entry>) -> Self {
        Self {
            query: String::new(),
            state: AppState::new(bookmarks),
            filtered_bookmarks: Vec::new(),
            initial_focus: true,
            matcher: SkimMatcherV2::default(),
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

    pub fn filtered_bookmarks(&self) -> &[Entry] {
        &self.filtered_bookmarks
    }

    pub fn set_filtered_bookmarks(&mut self, bookmarks: Vec<Entry>) {
        self.filtered_bookmarks = bookmarks;
    }

    pub fn initial_focus(&self) -> bool {
        self.initial_focus
    }

    pub fn set_initial_focus(&mut self, focus: bool) {
        self.initial_focus = focus;
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

    pub fn increment_access_count_by_index(
        &mut self,
        index: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.state.increment_access_count_by_index(index)
    }

    pub fn increment_access_count_by_entry(
        &mut self,
        target: &Entry,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.state.increment_access_count_by_entry(target)
    }

    pub fn add_bookmark(&mut self, url: String) -> Result<(), Box<dyn std::error::Error>> {
        self.state.add_bookmark(url)
    }
}
