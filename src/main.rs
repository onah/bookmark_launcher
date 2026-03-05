mod app;
mod ui;

use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = app::data_file_path();
    let bookmarks = match fs::read_to_string(&path) {
        Ok(s) => toml::from_str::<app::BookmarkFile>(&s)
            .unwrap_or_default()
            .bookmarks,
        Err(_) => Vec::new(),
    };

    ui::run_app(bookmarks)?;

    Ok(())
}
