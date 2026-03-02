use crate::app::{App, Entry};

/// Abstraction trait for UI backends.
/// Implementations should provide a `run_app` entry point which runs the UI
/// and returns when the UI exits.
pub trait UiBackend {
    /// Run the UI for the provided bookmarks. Implementations should block
    /// until the UI exits and return a boxed error on failure.
    fn run_app(bookmarks: Vec<Entry>) -> Result<(), Box<dyn std::error::Error>>;
}

// Note: concrete backends can implement this trait or simply provide a
// `run_app` free function with compatible signature. This file defines the
// abstraction surface to guide implementations.
