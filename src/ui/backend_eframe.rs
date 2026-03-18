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

fn build_highlighted_job(
    entry: &Entry,
    matched: &std::collections::HashSet<usize>,
    font_id: egui::FontId,
    normal_color: egui::Color32,
) -> egui::text::LayoutJob {
    use egui::text::{LayoutJob, TextFormat};

    let highlight_color = egui::Color32::YELLOW;

    let title = entry.title();
    let suffix = match entry {
        Entry::Bookmark { url, .. } => format!(" ({})", url),
        Entry::App { command, args, .. } => {
            if args.is_empty() {
                format!(" ({})", command)
            } else {
                format!(" ({} {})", command, args.join(" "))
            }
        }
    };

    let mut job = LayoutJob::default();
    let mut buf = String::new();
    let mut cur_highlighted = false;

    for (i, c) in title.chars().enumerate() {
        let is_highlighted = matched.contains(&i);
        if is_highlighted != cur_highlighted && !buf.is_empty() {
            let color = if cur_highlighted { highlight_color } else { normal_color };
            job.append(
                &buf,
                0.0,
                TextFormat {
                    font_id: font_id.clone(),
                    color,
                    ..Default::default()
                },
            );
            buf.clear();
        }
        cur_highlighted = is_highlighted;
        buf.push(c);
    }
    if !buf.is_empty() {
        let color = if cur_highlighted { highlight_color } else { normal_color };
        job.append(
            &buf,
            0.0,
            TextFormat {
                font_id: font_id.clone(),
                color,
                ..Default::default()
            },
        );
    }
    job.append(
        &suffix,
        0.0,
        TextFormat {
            font_id,
            color: normal_color,
            ..Default::default()
        },
    );
    job
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Color32, FontFamily, FontId};
    use std::collections::HashSet;

    fn bookmark_entry(title: &str) -> Entry {
        Entry::Bookmark {
            title: title.to_string(),
            url: "https://example.com".to_string(),
            access_count: 0,
        }
    }

    fn test_font() -> FontId {
        FontId::new(14.0, FontFamily::Proportional)
    }

    #[test]
    fn no_matched_chars_no_yellow_sections() {
        let entry = bookmark_entry("GitHub");
        let matched: HashSet<usize> = HashSet::new();
        let job = build_highlighted_job(&entry, &matched, test_font(), Color32::WHITE);
        for section in &job.sections {
            assert_ne!(
                section.format.color,
                Color32::YELLOW,
                "no section should be yellow when nothing matches"
            );
        }
    }

    #[test]
    fn matched_first_char_produces_yellow_section() {
        let entry = bookmark_entry("GitHub");
        let matched: HashSet<usize> = [0].into_iter().collect(); // 'G'
        let job = build_highlighted_job(&entry, &matched, test_font(), Color32::WHITE);

        let yellow_sections: Vec<_> = job
            .sections
            .iter()
            .filter(|s| s.format.color == Color32::YELLOW)
            .collect();
        assert!(!yellow_sections.is_empty(), "expected at least one yellow section");
        // 'G' is ASCII so byte 0 is char 0
        assert!(
            yellow_sections[0].byte_range.contains(&0),
            "byte offset 0 should be covered by a yellow section"
        );
    }

    #[test]
    fn suffix_section_is_not_yellow() {
        let entry = bookmark_entry("GitHub");
        let matched: HashSet<usize> = [0, 1, 2, 3, 4, 5].into_iter().collect(); // all chars
        let job = build_highlighted_job(&entry, &matched, test_font(), Color32::WHITE);

        // suffix " (https://example.com)" starts after "GitHub" (6 bytes)
        let suffix_start = "GitHub".len();
        for section in &job.sections {
            if section.byte_range.start >= suffix_start {
                assert_ne!(
                    section.format.color,
                    Color32::YELLOW,
                    "suffix section should not be yellow"
                );
            }
        }
    }
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
                        let positions: std::collections::HashSet<usize> = self
                            .app
                            .match_char_positions(entry.title(), self.app.query())
                            .into_iter()
                            .collect();
                        let font_id = egui::FontId::new(20.0, egui::FontFamily::Proportional);
                        let normal_color = ui.visuals().text_color();
                        let job = build_highlighted_job(entry, &positions, font_id, normal_color);
                        ui.horizontal(|ui| {
                            if ui.add(egui::Button::new(job)).clicked() {
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
