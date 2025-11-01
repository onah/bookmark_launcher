use reqwest;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Clone)]
pub struct Bookmark {
    pub title: String,
    pub url: String,
    pub access_count: u32,
}

pub struct App {
    query: String,
    bookmarks: Vec<Bookmark>,
    filtered_bookmarks: Vec<Bookmark>,
    initial_focus: bool,
}

impl App {
    pub fn new(bookmarks: Vec<Bookmark>) -> Self {
        Self {
            query: String::new(),
            bookmarks,
            filtered_bookmarks: Vec::new(),
            initial_focus: true,
        }
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn query_mut(&mut self) -> &mut String {
        &mut self.query
    }

    pub fn bookmarks(&self) -> &[Bookmark] {
        &self.bookmarks
    }

    pub fn filtered_bookmarks(&self) -> &[Bookmark] {
        &self.filtered_bookmarks
    }

    pub fn set_filtered_bookmarks(&mut self, bookmarks: Vec<Bookmark>) {
        self.filtered_bookmarks = bookmarks;
    }

    pub fn initial_focus(&self) -> bool {
        self.initial_focus
    }

    pub fn set_initial_focus(&mut self, focus: bool) {
        self.initial_focus = focus;
    }

    pub fn increment_access_count(&mut self, url: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(bookmark) = self.bookmarks.iter_mut().find(|b| b.url == url) {
            bookmark.access_count += 1;
            self.save_bookmarks()?;
        }
        Ok(())
    }

    pub fn save_bookmarks(&self) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(&self.bookmarks)?;
        fs::write("bookmarks.json", json)?;
        Ok(())
    }

    pub fn add_bookmark(&mut self, url: String) -> Result<(), Box<dyn std::error::Error>> {
        // URLが既に存在するかチェック
        if self.bookmarks.iter().any(|b| b.url == url) {
            return Ok(());
        }

        // 新しいブックマークを追加
        let title = self.extract_title_from_url(&url);
        let bookmark = Bookmark {
            title,
            url,
            access_count: 0,
        };

        self.bookmarks.push(bookmark);
        self.save_bookmarks()?;
        Ok(())
    }

    fn extract_title_from_url(&self, url: &str) -> String {
        // Webページからタイトルを取得しようとする
        match self.fetch_page_title(url) {
            Ok(title) if !title.is_empty() => title,
            _ => {
                // 取得できない場合はドメイン名を使用
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

    fn fetch_page_title(&self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        // HTTPリクエストでHTMLを取得
        let response = reqwest::blocking::get(url)?;
        let html = response.text()?;

        // HTMLをパース
        let document = Html::parse_document(&html);
        let selector = Selector::parse("title")?;

        // titleタグの内容を取得
        if let Some(title_element) = document.select(&selector).next() {
            let title = title_element.text().collect::<String>().trim().to_string();
            Ok(title)
        } else {
            Ok(String::new())
        }
    }
}
