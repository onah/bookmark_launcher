#[cfg(feature = "backend-eframe")]
mod backend_eframe;
#[cfg(feature = "backend-tui")]
mod backend_tui;

#[cfg(feature = "backend-tui")]
pub use backend_tui::run_app;

#[cfg(all(feature = "backend-eframe", not(feature = "backend-tui")))]
pub use backend_eframe::run_app;

#[cfg(not(any(feature = "backend-eframe", feature = "backend-tui")))]
pub fn run_app(_bookmarks: Vec<crate::app::Entry>) -> Result<(), Box<dyn std::error::Error>> {
    Err("No UI backend feature enabled".into())
}
