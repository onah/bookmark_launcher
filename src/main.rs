mod app;
mod ui;

use app::Entry;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = app::data_file_path();
    let bookmarks = match fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str::<Vec<Entry>>(&s).unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    ui::run_app(bookmarks)?;

    Ok(())
}
