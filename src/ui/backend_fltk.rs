use crate::app::Entry;

pub fn run_app(bookmarks: Vec<Entry>) -> Result<(), Box<dyn std::error::Error>> {
    use fltk::{
        app, browser::HoldBrowser, button::Button, frame::Frame, prelude::*, window::Window,
    };

    let a = app::App::default();
    let mut wind = Window::new(100, 100, 600, 400, "Bookmark Launcher (fltk)");

    let mut browser = HoldBrowser::new(10, 10, 580, 330, "");
    for b in bookmarks.iter() {
        let text = match b {
            Entry::Bookmark { title, url, .. } => format!("{} ({})", title, url),
            Entry::App { title, command, .. } => format!("{} (app: {})", title, command),
        };
        browser.add(&text);
    }

    let mut open_btn = Button::new(10, 350, 80, 30, "Open");
    let mut close_btn = Button::new(100, 350, 80, 30, "Close");
    let mut status = Frame::new(200, 350, 390, 30, "");

    wind.end();
    wind.show();

    let bmarks = bookmarks; // move into closure if needed

    open_btn.set_callback(move |_| {
        if let Some(idx) = browser.value() {
            // 1-based index
            if idx > 0 {
                let i = (idx - 1) as usize;
                if let Some(entry) = bmarks.get(i) {
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
        }
    });

    close_btn.set_callback(move |_| {
        app::quit();
    });

    while a.wait() {
        // event loop
    }

    Ok(())
}
