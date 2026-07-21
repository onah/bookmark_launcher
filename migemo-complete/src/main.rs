//! CLI entry point: parses arguments, reads the filesystem and dictionary,
//! and prints ranked completion candidates, one per line.
//!
//! Contract: `migemo-complete --cwd <path> [--kind dir|file|any] [--limit N]
//! [--dict <path>] -- <token>`. Always exits 0 with zero or more candidate
//! lines on stdout; problems (bad args, unreadable directory) go to stderr
//! so a shell driving this on every keystroke never sees a hung or crashed
//! completion.

use migemo_complete::{Entry, Kind, format_candidate, rank_candidates, split_token};
use migemo_core::SkkDictionary;
use std::path::{Path, PathBuf};

const DEFAULT_LIMIT: usize = 50;

struct Args {
    cwd: PathBuf,
    kind: Kind,
    limit: usize,
    dict_path: Option<PathBuf>,
    token: String,
}

fn parse_args() -> Result<Args, String> {
    let mut cwd = None;
    let mut kind = Kind::Any;
    let mut limit = DEFAULT_LIMIT;
    let mut dict_path = None;
    let mut token = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--cwd" => cwd = Some(PathBuf::from(args.next().ok_or("--cwd requires a value")?)),
            "--kind" => {
                let raw = args.next().ok_or("--kind requires a value")?;
                kind = Kind::parse(&raw).ok_or_else(|| format!("invalid --kind: {raw}"))?;
            }
            "--limit" => {
                let raw = args.next().ok_or("--limit requires a value")?;
                limit = raw.parse().map_err(|_| format!("invalid --limit: {raw}"))?;
            }
            "--dict" => {
                dict_path = Some(PathBuf::from(args.next().ok_or("--dict requires a value")?))
            }
            "--" => token = Some(args.next().unwrap_or_default()),
            other => return Err(format!("unrecognized argument: {other}")),
        }
    }

    Ok(Args {
        cwd: cwd.ok_or("--cwd is required")?,
        kind,
        limit,
        dict_path,
        token: token.unwrap_or_default(),
    })
}

/// Same data directory `bookmark_launcher` uses for its dictionary, so a
/// dictionary downloaded for one tool works for both without duplication.
fn default_dict_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "onah", "bookmark_launcher")
        .map(|proj| proj.data_dir().join("SKK-JISYO.L"))
}

/// Missing or unreadable dictionary degrades to kana/ASCII-only matching
/// (still handles a plain katakana/hiragana name like "アプリ") rather than
/// failing the whole completion.
///
/// Parsing the ~170k-entry SKK-JISYO.L text takes ~100ms (see
/// migemo-core/examples/bench_dict_load.rs) -- too slow to redo on every
/// keystroke. A compiled cache next to the source file cuts that to
/// microseconds; it's rebuilt whenever missing, corrupt, or older than the
/// source dictionary.
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
    // completion process must not stop this completion from working.
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

fn list_entries(dir: &Path) -> Vec<Entry> {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    read_dir
        .filter_map(|entry| entry.ok())
        .map(|entry| {
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            Entry {
                name: entry.file_name().to_string_lossy().into_owned(),
                is_dir,
            }
        })
        .collect()
}

fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(message) => {
            eprintln!("migemo-complete: {message}");
            std::process::exit(1);
        }
    };

    let (dir_prefix, leaf_query) = split_token(&args.token);
    let list_dir = if dir_prefix.is_empty() {
        args.cwd.clone()
    } else {
        args.cwd.join(dir_prefix)
    };

    let entries = list_entries(&list_dir);
    let dict = load_dictionary(args.dict_path.as_deref());
    let ranked = rank_candidates(&entries, leaf_query, args.kind, dict);

    let entries_by_name: std::collections::HashMap<&str, &Entry> =
        entries.iter().map(|e| (e.name.as_str(), e)).collect();

    for name in ranked.into_iter().take(args.limit) {
        if let Some(entry) = entries_by_name.get(name.as_str()) {
            println!("{}", format_candidate(dir_prefix, entry));
        }
    }
}
