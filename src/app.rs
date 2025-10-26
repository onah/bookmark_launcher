use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Bookmark {
    pub title: String,
    pub url: String,
    pub access_count: u32,
}

pub struct App {
    pub query: String,
    pub bookmarks: Vec<Bookmark>,
}

impl App {
    pub fn new(bookmarks: Vec<Bookmark>) -> Self {
        Self {
            query: String::new(),
            bookmarks,
        }
    }
}
