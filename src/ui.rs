// include the UI abstraction and platform backends
mod backend;
#[cfg(feature = "backend-eframe")]
mod backend_eframe;
#[cfg(feature = "backend-tui")]
mod backend_tui;

#[cfg(feature = "backend-eframe")]
pub use backend_eframe::run_app;

#[cfg(all(feature = "backend-tui", not(feature = "backend-eframe")))]
pub use crate::ui::backend_tui::run_app;

#[cfg(not(any(feature = "backend-eframe", feature = "backend-tui")))]
pub fn run_app(_bookmarks: Vec<crate::app::Entry>) -> Result<(), Box<dyn std::error::Error>> {
    Err("No UI backend feature enabled".into())
}
