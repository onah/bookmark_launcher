use crate::app::{App, Entry};

// include the UI abstraction and platform backends
mod backend;
#[cfg(feature = "backend-fltk")]
mod backend_fltk;
#[cfg(feature = "backend-tui")]
mod backend_tui;
#[cfg(feature = "backend-windows")]
mod backend_windows;
#[cfg(feature = "backend-windows-rs")]
mod backend_windows_rs;

#[cfg(feature = "backend-eframe")]
mod eframe_ui {
    use super::*;
    use eframe::egui;
    use egui::FontFamily;

    pub fn run_app(bookmarks: Vec<Entry>) -> Result<(), Box<dyn std::error::Error>> {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_decorations(false)
                .with_transparent(true)
                .with_inner_size([500.0, 300.0])
                .with_position(egui::Pos2::new(400.0, 100.0))
                .with_always_on_top(),
            ..Default::default()
        };

        eframe::run_native(
            "Bookmark Launcher",
            options,
            Box::new(|cc| {
                setup_custom_fonts(&cc.egui_ctx);
                Ok(Box::new(App::new(bookmarks)))
            }),
        )
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        Ok(())
    }

    fn setup_custom_fonts(ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();

        fonts.font_data.insert(
            "note_sans_jp".to_owned(),
            egui::FontData::from_static(include_bytes!(
                "../assets/NotoSansJP-VariableFont_wght.ttf"
            ))
            .into(),
        );

        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "note_sans_jp".to_owned());

        ctx.set_fonts(fonts);
    }

    impl eframe::App for App {
        fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }

            egui::CentralPanel::default().show(ctx, |ui| {
                // Keep heading and input centered
                let mut response: Option<egui::Response> = None;
                ui.vertical_centered(|ui| {
                    ui.heading("Bookmark Launcher");
                    ui.add_space(10.0);

                    ui.style_mut().text_styles.insert(
                        egui::TextStyle::Body,
                        egui::FontId::new(20.0, egui::FontFamily::Proportional),
                    );

                    response = Some(ui.text_edit_singleline(self.query_mut()));

                    if self.initial_focus() {
                        if let Some(r) = response.as_mut() {
                            r.request_focus();
                        }
                        self.set_initial_focus(false);
                    }
                });

                ui.add_space(6.0);

                // Left-aligned list of bookmark results
                ui.vertical(|ui| {
                    let mut clicked_index: Option<usize> = None;

                    // Fuzzy search results (ordered by relevance)
                    let search_results = self.fuzzy_search(self.query());

                    for (index, _) in &search_results {
                        if let Some(entry) = self.bookmarks().get(*index) {
                            ui.horizontal(|ui| {
                                if ui.add(egui::Button::new(entry.display())).clicked() {
                                    clicked_index = Some(*index);
                                }
                            });
                        }
                    }

                    if let Some(idx) = clicked_index {
                        let _ = self.increment_access_count_by_index(idx);
                        if let Some(entry) = self.bookmarks().get(idx) {
                            match entry {
                                Entry::Bookmark { url, .. } => {
                                    let _ = open::that(&url);
                                }
                                Entry::App { command, args, .. } => {
                                    let mut cmd = std::process::Command::new(command);
                                    if !args.is_empty() {
                                        cmd.args(args);
                                    }
                                    let _ = cmd.spawn();
                                }
                            }
                        }
                    }

                    self.set_filtered_bookmarks(
                        search_results
                            .iter()
                            .filter_map(|(index, _)| self.bookmarks().get(*index).cloned())
                            .collect(),
                    );

                    let mut enter_url: Option<String> = None;
                    let mut should_add_bookmark = false;
                    let mut enter_entry: Option<Entry> = None;

                    if response.as_ref().map_or(false, |r| r.lost_focus())
                        && ctx.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        let query = self.query().trim();
                        if !query.is_empty() {
                            if query.starts_with("http://")
                                || query.starts_with("https://")
                                || query.contains('.')
                            {
                                should_add_bookmark = true;
                                enter_url = Some(query.to_string());
                            } else if let Some(bm) = self.filtered_bookmarks().first() {
                                enter_entry = Some(bm.clone());
                            }
                        }
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }

                    if let Some(url) = enter_url {
                        if should_add_bookmark {
                            let _ = self.add_bookmark(url.clone());
                        }
                        let _ = open::that(&url);
                    } else if let Some(entry) = enter_entry {
                        let _ = self.increment_access_count_by_entry(&entry);
                        match entry {
                            Entry::Bookmark { url, .. } => {
                                let _ = open::that(&url);
                            }
                            Entry::App { command, args, .. } => {
                                let mut cmd = std::process::Command::new(command);
                                if !args.is_empty() {
                                    cmd.args(&args);
                                }
                                let _ = cmd.spawn();
                            }
                        }
                    }
                });
            });
        }
    }
}

#[cfg(feature = "backend-eframe")]
pub use eframe_ui::run_app;

#[cfg(feature = "backend-fltk")]
pub use crate::ui::backend_fltk::run_app;
#[cfg(feature = "backend-windows")]
pub use crate::ui::backend_windows::run_app;

#[cfg(feature = "backend-windows-rs")]
pub use crate::ui::backend_windows_rs::run_app;

#[cfg(not(any(
    feature = "backend-eframe",
    feature = "backend-windows",
    feature = "backend-windows-rs",
    feature = "backend-fltk",
    feature = "backend-tui",
)))]
pub fn run_app(_bookmarks: Vec<Entry>) -> Result<(), Box<dyn std::error::Error>> {
    Err("No UI backend feature enabled".into())
}

#[cfg(feature = "backend-tui")]
pub use crate::ui::backend_tui::run_app;
