use crate::app::{App, Bookmark};
use eframe::egui;

pub fn run_app(bookmarks: Vec<Bookmark>) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false) // タイトルバーなし
            .with_transparent(true) // 背景透過（Windowsでフローティング感）
            .with_inner_size([500.0, 300.0])
            .with_position(egui::Pos2::new(400.0, 100.0)) // 中央少し上に配置
            .with_always_on_top(), // 常に前面
        ..Default::default()
    };

    eframe::run_native(
        "Bookmark Launcher",
        options,
        Box::new(|_| Ok(Box::new(App::new(bookmarks)))),
    )
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Bookmark Launcher");
                ui.add_space(10.0);

                // フォントサイズを大きくする
                ui.style_mut().text_styles.insert(
                    egui::TextStyle::Body,
                    egui::FontId::new(20.0, egui::FontFamily::Proportional),
                );

                let response = ui.text_edit_singleline(&mut self.query);

                if self.initial_focus {
                    response.request_focus();
                    self.initial_focus = false;
                }

                // シンプルな検索
                for bm in &self.bookmarks {
                    if (self.query.is_empty() || bm.title.contains(&self.query))
                        && ui.button(format!("{} ({})", bm.title, bm.url)).clicked()
                    {
                        let _ = open::that(&bm.url);
                        // アクセス数を更新
                        // ※小規模サンプルでは保存は省略可能
                    }
                }

                self.filtered_bookmarks = self
                    .bookmarks
                    .iter()
                    .filter(|bm| self.query.is_empty() || bm.title.contains(&self.query))
                    .cloned()
                    .collect();

                if response.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                    if let Some(bm) = self.filtered_bookmarks.first() {
                        let _ = open::that(&bm.url);
                    }
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    }
}
