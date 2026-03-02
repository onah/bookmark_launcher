use crate::app::Entry;
use native_windows_derive as nwd;
use native_windows_gui as nwg;
use native_windows_gui::NativeUi;
use nwd::NwgUi;

#[derive(Default, NwgUi)]
pub struct BasicUi {
    #[nwg_control(size: (520, 360), position: (300, 200), title: "Bookmark Launcher")]
    #[nwg_events(OnWindowClose: [BasicUi::exit])]
    window: nwg::Window,

    #[nwg_control(parent: window, size: (500, 300), position: (10, 10))]
    #[nwg_events(OnListBoxDoubleClick: [BasicUi::on_double_click])]
    listbox: nwg::ListBox<String>,

    #[nwg_control(parent: window, text: "Open", position: (10, 320))]
    #[nwg_events(OnButtonClick: [BasicUi::on_open_click])]
    open_btn: nwg::Button,

    #[nwg_control(parent: window, text: "Close", position: (100, 320))]
    #[nwg_events(OnButtonClick: [BasicUi::exit])]
    close_btn: nwg::Button,

    ui_data: UiDat
    fn on_double_click(&self) {
        self.open_selected();
    }

    fn on_open_click(&self) {
        self.open_selected();
    }

    fn open_selected(&self) {
        if let Some(i) = self.listbox.selection() {
            if let Some(entry) = self.ui_data.bookmarks.get(i) {
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

    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }
}

pub fn run_app(bookmarks: Vec<Entry>) -> Result<(), Box<dyn std::error::Error>> {
    nwg::init()?;

    // prepare initial state with bookmarks
    let mut initial = BasicUi::default();
    initial.ui_data.bookmarks = bookmarks;

    // build the UI from initial state
    let ui = BasicUi::build_ui(initial)?;

    // populate listbox
    let items = ui.ui_data.display_items();
    for it in items.iter() {
        ui.listbox.push(it.clone());
    }

    nwg::dispatch_thread_events();

    Ok(())
}
