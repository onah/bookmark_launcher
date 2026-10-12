//! The logic behind `kanaq-bm`, the bookmark tool driven by fzf, kept
//! apart from argument handling so it can be unit tested. The CLI wrapper
//! lives in `main.rs`; the only network access is `title::fetch_title`.

pub mod list;
pub mod store;
pub mod title;
