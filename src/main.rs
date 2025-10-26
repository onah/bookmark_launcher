mod app;
mod ui;

use app::{App, Bookmark};
use eframe::egui;
use std::fs;

fn main() -> eframe::Result<()> {
    // bookmarks.json 読み込み
    let bookmarks = serde_json::from_str::<Vec<Bookmark>>(
        &fs::read_to_string("bookmarks.json").unwrap_or("[]".into()),
    )
    .unwrap_or_default();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false) // タイトルバーなし
            .with_transparent(true) // 背景透過（Windowsでフローティング感）
            .with_inner_size([500.0, 300.0])
            .with_always_on_top(), // 常に前面
        ..Default::default()
    };

    eframe::run_native(
        "Bookmark Launcher",
        options,
        Box::new(|_| Ok(Box::new(App::new(bookmarks)))),
    )
}
