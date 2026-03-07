use crate::app::{App, Entry};
use eframe::egui;
use egui::FontFamily;

struct EframeApp {
    app: App,
    initial_focus: bool,
}

impl EframeApp {
    fn new(bookmarks: Vec<Entry>) -> Self {
        Self {
            app: App::new(bookmarks),
            initial_focus: true,
        }
    }
}

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
            Ok(Box::new(EframeApp::new(bookmarks)))
        }),
    )
    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    Ok(())
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let font_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("NotoSansJP-VariableFont_wght.ttf");

    if let Ok(font_bytes) = std::fs::read(font_path) {
        fonts.font_data.insert(
            "noto_sans_jp".to_owned(),
            egui::FontData::from_owned(font_bytes).into(),
        );

        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "noto_sans_jp".to_owned());
    }

    ctx.set_fonts(fonts);
}

impl eframe::App for EframeApp {
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

                response = Some(ui.text_edit_singleline(self.app.query_mut()));

                if self.initial_focus {
                    if let Some(r) = response.as_mut() {
                        r.request_focus();
                    }
                    self.initial_focus = false;
                }
            });

            ui.add_space(6.0);

            // Left-aligned list of bookmark results
            ui.vertical(|ui| {
                let mut clicked_index: Option<usize> = None;

                // Fuzzy search results (ordered by relevance)
                let search_results = self.app.fuzzy_search(self.app.query());

                for (index, _) in &search_results {
                    if let Some(entry) = self.app.bookmarks().get(*index) {
                        ui.horizontal(|ui| {
                            if ui.add(egui::Button::new(entry.display())).clicked() {
                                clicked_index = Some(*index);
                            }
                        });
                    }
                }

                if let Some(idx) = clicked_index {
                    let _ = self.app.increment_access_count_by_index(idx);
                    if let Some(entry) = self.app.bookmarks().get(idx) {
                        match entry {
                            Entry::Bookmark { url, .. } => {
                                let _ = open::that(url);
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

                let mut enter_url: Option<String> = None;
                let mut should_add_bookmark = false;
                let mut enter_index: Option<usize> = None;

                if response.as_ref().is_some_and(|r| r.lost_focus())
                    && ctx.input(|i| i.key_pressed(egui::Key::Enter))
                {
                    let query = self.app.query().trim();
                    if !query.is_empty() {
                        if query.starts_with("http://")
                            || query.starts_with("https://")
                            || query.contains('.')
                        {
                            should_add_bookmark = true;
                            enter_url = Some(query.to_string());
                        } else if let Some(&(idx, _)) = search_results.first() {
                            enter_index = Some(idx);
                        }
                    }
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }

                if let Some(url) = enter_url {
                    if should_add_bookmark {
                        let _ = self.app.add_bookmark(url.clone());
                    }
                    let _ = open::that(url);
                } else if let Some(idx) = enter_index {
                    let _ = self.app.increment_access_count_by_index(idx);
                    if let Some(entry) = self.app.bookmarks().get(idx) {
                        match entry {
                            Entry::Bookmark { url, .. } => {
                                let _ = open::that(url);
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
            });
        });
    }
}
