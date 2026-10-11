//! CLI entry point: reads candidate lines from stdin, ranks them against a
//! query, and prints the survivors to stdout. Built to be driven by fzf's
//! reload hook:
//!
//! ```sh
//! fzf --disabled --ansi \
//!   --bind 'change:reload(migemo-fzf --highlight -- {q})' \
//!   < candidates.txt
//! ```
//!
//! (fzf reruns the reload command on every keystroke with the current query
//! substituted for `{q}`; stdin for that command needs to come from
//! somewhere that regenerates candidates each time too, e.g. piping a
//! listing command through this binary inside the `reload(...)`.)
//!
//! Contract: `migemo-fzf [--dict <path>] [--highlight] -- <query>`. Reads
//! candidates from stdin, one per line. Always exits 0 with zero or more
//! result lines on stdout; problems (bad args) go to stderr, and an
//! unreadable dictionary degrades to kana/ASCII-only matching rather than
//! failing, so a shell driving this on every keystroke never sees a hung or
//! crashed reload.

use migemo_core::SkkDictionary;
use migemo_fzf::filter_lines;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

struct Args {
    dict_path: Option<PathBuf>,
    highlight: bool,
    query: String,
}

fn parse_args() -> Result<Args, String> {
    let mut dict_path = None;
    let mut highlight = false;
    let mut query = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dict" => {
                dict_path = Some(PathBuf::from(args.next().ok_or("--dict requires a value")?))
            }
            "--highlight" => highlight = true,
            "--" => query = Some(args.next().unwrap_or_default()),
            other => return Err(format!("unrecognized argument: {other}")),
        }
    }

    Ok(Args {
        dict_path,
        highlight,
        query: query.unwrap_or_default(),
    })
}

/// Same data directory `migemo-complete` uses, so a
/// dictionary downloaded once works for every tool in this repo.
fn default_dict_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "onah", "bookmark_launcher")
        .map(|proj| proj.data_dir().join("SKK-JISYO.L"))
}

/// Missing or unreadable dictionary degrades to kana/ASCII-only matching
/// rather than failing the whole filter.
///
/// Parsing the ~170k-entry SKK-JISYO.L text takes ~100ms -- too slow to redo
/// on every keystroke, which is exactly how this binary is meant to be
/// invoked via fzf's `reload`. A compiled cache next to the source file (see
/// `migemo_core::SkkDictionary::compile`) cuts that to microseconds; it's
/// rebuilt whenever missing, corrupt, or older than the source dictionary.
/// Mirrors `migemo-complete/src/main.rs`, which has the same latency
/// requirement for the same reason (driven on every keystroke by a shell).
fn load_dictionary(explicit_path: Option<&Path>) -> SkkDictionary {
    let Some(source_path) = explicit_path.map(PathBuf::from).or_else(default_dict_path) else {
        return SkkDictionary::from_bytes(&[]);
    };

    let cache_path = compiled_cache_path(&source_path);
    if let Some(dict) = load_compiled_cache(&cache_path, &source_path) {
        return dict;
    }

    let Ok(bytes) = std::fs::read(&source_path) else {
        return SkkDictionary::from_bytes(&[]);
    };

    let dict = SkkDictionary::from_bytes(&bytes);
    // Best-effort: a read-only data dir or a race with another concurrent
    // filter process must not stop this run from working.
    let _ = std::fs::write(&cache_path, dict.compile());
    dict
}

fn compiled_cache_path(source_path: &Path) -> PathBuf {
    let mut file_name = source_path.file_name().unwrap_or_default().to_os_string();
    file_name.push(".compiled");
    source_path.with_file_name(file_name)
}

/// `None` on any problem (missing files, unreadable, stale, corrupt) so the
/// caller always falls back to rebuilding from the source dictionary.
fn load_compiled_cache(cache_path: &Path, source_path: &Path) -> Option<SkkDictionary> {
    let cache_modified = std::fs::metadata(cache_path)
        .and_then(|m| m.modified())
        .ok()?;
    let source_modified = std::fs::metadata(source_path)
        .and_then(|m| m.modified())
        .ok()?;
    if cache_modified < source_modified {
        return None;
    }

    let bytes = std::fs::read(cache_path).ok()?;
    SkkDictionary::from_compiled_bytes(&bytes).ok()
}

fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(message) => {
            eprintln!("migemo-fzf: {message}");
            std::process::exit(1);
        }
    };

    let stdin = io::stdin();
    let lines: Vec<String> = stdin.lock().lines().map_while(Result::ok).collect();

    let dict = load_dictionary(args.dict_path.as_deref());
    let results = filter_lines(&lines, &args.query, dict, args.highlight);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    for line in &results {
        let _ = writeln!(out, "{line}");
    }
}
