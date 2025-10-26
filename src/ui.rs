use crate::app::App;
use eframe::egui;

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Bookmark Launcher");
                ui.add_space(10.0);
                ui.text_edit_singleline(&mut self.query);

                // シンプルな検索
                for bm in &self.bookmarks {
                    if (self.query.is_empty() || bm.title.contains(&self.query))
                        && ui.button(format!("{} ({})", bm.title, bm.url)).clicked() {
                            let _ = open::that(&bm.url);
                            // アクセス数を更新
                            // ※小規模サンプルでは保存は省略可能
                        }
                }
            });
        });
    }
}
