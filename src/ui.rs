use crate::app::{App, Bookmark};
use eframe::egui;
use egui::FontFamily;

pub fn run_app(bookmarks: Vec<Bookmark>) -> eframe::Result<()> {
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
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        "note_sans_jp".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/NotoSansJP-VariableFont_wght.ttf"))
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
                let mut clicked_url: Option<String> = None;

                // Fuzzy search results (ordered by relevance)
                let search_results = self.fuzzy_search(self.query());

                for (index, _) in &search_results {
                    if let Some(bm) = self.bookmarks().get(*index) {
                        // Buttons inside a horizontal block to ensure left alignment
                        ui.horizontal(|ui| {
                            if ui
                                .add(egui::Button::new(format!("{} ({})", bm.title, bm.url)))
                                .clicked()
                            {
                                clicked_url = Some(bm.url.clone());
                            }
                        });
                    }
                }

                if let Some(url) = clicked_url {
                    let _ = self.increment_access_count(&url);
                    let _ = open::that(&url);
                }

                self.set_filtered_bookmarks(
                    search_results
                        .iter()
                        .filter_map(|(index, _)| self.bookmarks().get(*index).cloned())
                        .collect(),
                );

                let mut enter_url: Option<String> = None;
                let mut should_add_bookmark = false;

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
                            enter_url = Some(bm.url.clone());
                        }
                    }
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }

                if let Some(url) = enter_url {
                    if should_add_bookmark {
                        let _ = self.add_bookmark(url.clone());
                    } else {
                        let _ = self.increment_access_count(&url);
                    }
                    let _ = open::that(&url);
                }
            });
        });
    }
}
