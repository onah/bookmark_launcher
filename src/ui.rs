use crate::app::{App, Bookmark};
use eframe::egui;

pub fn run_app(bookmarks: Vec<Bookmark>) -> eframe::Result<()> {
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

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        // リターンキーでアプリ終了
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Enter) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Bookmark Launcher");
                ui.add_space(10.0);
                ui.text_edit_singleline(&mut self.query);

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
            });
        });
    }
}
