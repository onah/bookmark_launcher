use serde::{Deserialize, Serialize};

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
}
