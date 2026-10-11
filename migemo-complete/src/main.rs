//! CLI entry point: parses arguments, reads the filesystem and dictionary,
//! and prints ranked completion candidates, one per line.
//!
//! Contract: `migemo-complete --cwd <path> [--kind dir|file|any] [--limit N]
//! [--dict <path>] -- <token>`. Always exits 0 with zero or more candidate
//! lines on stdout; problems (bad args, unreadable directory) go to stderr
//! so a shell driving this on every keystroke never sees a hung or crashed
//! completion.

use migemo_complete::{Entry, Kind, format_candidate, rank_candidates, split_token};
use migemo_core::load_dictionary;
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
