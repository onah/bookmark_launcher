//! `kanaq complete`: parses arguments, reads the filesystem and dictionary,
//! and prints ranked completion candidates, one per line.
//!
//! Contract: `kanaq complete --cwd <path> [--kind dir|file|any] [--limit N]
//! [--dict <path>] -- <token>`. Always exits 0 with zero or more candidate
//! lines on stdout; problems (bad args, unreadable directory) go to stderr
//! so a shell driving this on every keystroke never sees a hung or crashed
//! completion.

use kanaq::complete::{Entry, Kind, format_candidate, rank_candidates, split_token};
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

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut cwd = None;
    let mut kind = Kind::Any;
    let mut limit = DEFAULT_LIMIT;
    let mut dict_path = None;
    let mut token = None;

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

pub fn run(args: impl Iterator<Item = String>) -> Result<(), String> {
    let args = parse_args(args)?;

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
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Args, String> {
        parse_args(args.iter().map(|a| a.to_string()))
    }

    #[test]
    fn parses_options_and_token() {
        let args = parse(&[
            "--cwd", "/tmp", "--kind", "dir", "--limit", "5", "--", "kan",
        ])
        .unwrap();
        assert_eq!(args.cwd, PathBuf::from("/tmp"));
        assert_eq!(args.kind, Kind::Dir);
        assert_eq!(args.limit, 5);
        assert_eq!(args.token, "kan");
    }

    #[test]
    fn requires_cwd() {
        assert!(parse(&["--", "kan"]).is_err());
    }
}
