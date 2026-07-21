//! Measures `SkkDictionary::from_bytes` wall time against the real
//! SKK-JISYO.L, to give a concrete before/after number for dictionary
//! load optimization work.
//!
//! Usage: cargo run --release -p migemo-core --example bench_dict_load [path]
//! Defaults to tmp/SKK-JISYO.L at the workspace root.

use std::time::Instant;

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/../tmp/SKK-JISYO.L").to_string());

    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
    println!("dictionary file: {path} ({} bytes)", bytes.len());

    let start = Instant::now();
    let dict = migemo_core::SkkDictionary::from_bytes(&bytes);
    let elapsed = start.elapsed();
    println!("from_bytes:          {elapsed:?}  ({} entries)", dict.len());

    let start = Instant::now();
    let compiled_bytes = dict.compile();
    println!(
        "compile:             {:?}  ({} bytes on disk)",
        start.elapsed(),
        compiled_bytes.len()
    );

    let start = Instant::now();
    let compiled = migemo_core::SkkDictionary::from_compiled_bytes(&compiled_bytes)
        .expect("bench-generated compiled bytes must be valid");
    println!(
        "from_compiled_bytes: {:?}  ({} entries)",
        start.elapsed(),
        compiled.len()
    );
}
