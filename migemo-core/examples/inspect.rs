//! Shows what migemo-core does internally: how an item's text is split into
//! segments and morae (`add_item`), how each query is decomposed into morae
//! (`query_to_morae`), and whether / where the query hits.
//!
//! Usage:
//!   cargo run --release -p migemo-core --example inspect -- [--dict <path>] <text> [query]...
//!
//! The dictionary defaults to tmp/SKK-JISYO.L at the workspace root (same as
//! `bench_dict_load`). A missing dictionary falls back to an empty one, so
//! kana and ASCII still work but kanji show up as unindexed.

use migemo_core::{MigemoSearcher, SkkDictionary, query_to_morae};
use std::time::Instant;

const DEFAULT_DICT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tmp/SKK-JISYO.L");

struct Args {
    dict_path: String,
    text: String,
    queries: Vec<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut dict_path = DEFAULT_DICT.to_string();
    let mut positional = Vec::new();

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dict" => dict_path = args.next().ok_or("--dict requires a value")?,
            _ => positional.push(arg),
        }
    }

    let mut positional = positional.into_iter();
    let text = positional.next().ok_or("missing <text>")?;
    Ok(Args {
        dict_path,
        text,
        queries: positional.collect(),
    })
}

fn load_dictionary(path: &str) -> SkkDictionary {
    let start = Instant::now();
    let dict = match std::fs::read(path) {
        Ok(bytes) => SkkDictionary::from_bytes(&bytes),
        Err(e) => {
            eprintln!("warning: cannot read dictionary {path}: {e} (using an empty one)");
            SkkDictionary::from_bytes(&[])
        }
    };
    println!(
        "dictionary: {path} ({} entries, {:?})",
        dict.len(),
        start.elapsed()
    );
    dict
}

fn print_reading(searcher: &MigemoSearcher, text: &str) {
    println!("\ntext: {text}");
    let segments = searcher.inspect(text);
    for segment in &segments {
        let morae = if segment.morae.is_empty() {
            "(not indexed)".to_string()
        } else {
            segment.morae.join(" ")
        };
        println!("  [{}]  {morae}", segment.text);
    }
    let all: Vec<&str> = segments
        .iter()
        .flat_map(|s| s.morae.iter().map(String::as_str))
        .collect();
    println!("  morae: {}", all.join(","));
}

fn print_query(searcher: &MigemoSearcher, text: &str, query: &str) {
    println!("\nquery: {query}");
    println!("  morae: {}", query_to_morae(query).join(","));

    match searcher.search(query).first() {
        Some(hit) => {
            let chars: Vec<char> = text.chars().collect();
            let matched: String = hit.highlight.iter().filter_map(|&i| chars.get(i)).collect();
            println!(
                "  hit: score={} highlight={:?} \"{matched}\"",
                hit.score, hit.highlight
            );
        }
        None => println!("  no hit"),
    }
}

fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(message) => {
            eprintln!("inspect: {message}");
            eprintln!("usage: inspect [--dict <path>] <text> [query]...");
            std::process::exit(2);
        }
    };

    let mut searcher = MigemoSearcher::new(load_dictionary(&args.dict_path));
    print_reading(&searcher, &args.text);

    searcher.add_item(&args.text);
    for query in &args.queries {
        print_query(&searcher, &args.text, query);
    }
}
