//! Custom migemo implementation: see docs/migemo_ja.md for the design.
//!
//! Unlike a regex-based migemo, item text is indexed into a romaji mora list
//! up front (`add_item`), and a query is matched as a contiguous mora
//! subsequence against it. This lets multi-mora romaji queries (e.g. "gen")
//! match kanji with the same reading (言, 現, ...), which a per-character
//! regex alternation cannot do.

mod dictionary;
mod romaji;
mod searcher;

pub use dictionary::{CompiledDictError, SkkDictionary};
// SearchResult is part of the public API (see docs/migemo_ja.md section 5)
// even though current callers only destructure it through inference.
#[allow(unused_imports)]
pub use searcher::{MigemoSearcher, SearchResult};
