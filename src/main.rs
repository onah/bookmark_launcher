mod app;
mod ui;

use app::Bookmark;
use std::fs;

fn main() -> eframe::Result<()> {
    let bookmarks = serde_json::from_str::<Vec<Bookmark>>(
        &fs::read_to_string("bookmarks.json").unwrap_or("[]".into()),
    )
    .unwrap_or_default();

    ui::run_app(bookmarks)
}
