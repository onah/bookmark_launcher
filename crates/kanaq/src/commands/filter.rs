//! `kanaq filter`: reads candidate lines from stdin, ranks them against a
//! query, and prints the survivors to stdout. Built to be driven by fzf's
//! reload hook:
//!
//! ```sh
//! fzf --disabled --ansi \
//!   --bind 'change:reload(kanaq filter --highlight -- {q})' \
//!   < candidates.txt
//! ```
//!
//! (fzf reruns the reload command on every keystroke with the current query
//! substituted for `{q}`; stdin for that command needs to come from
//! somewhere that regenerates candidates each time too, e.g. piping a
//! listing command through this subcommand inside the `reload(...)`.)
//!
//! Contract: `kanaq filter [--dict <path>] [--highlight] -- <query>`. Reads
//! candidates from stdin, one per line. Always exits 0 with zero or more
//! result lines on stdout; problems (bad args) go to stderr, and an
//! unreadable dictionary degrades to kana/ASCII-only matching rather than
//! failing, so a shell driving this on every keystroke never sees a hung or
//! crashed reload.

use kanaq::filter::filter_lines;
use migemo_core::load_dictionary;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

struct Args {
    dict_path: Option<PathBuf>,
    highlight: bool,
    query: String,
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut dict_path = None;
    let mut highlight = false;
    let mut query = None;

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

pub fn run(args: impl Iterator<Item = String>) -> Result<(), String> {
    let args = parse_args(args)?;

    let stdin = io::stdin();
    let lines: Vec<String> = stdin.lock().lines().map_while(Result::ok).collect();

    let dict = load_dictionary(args.dict_path.as_deref());
    let results = filter_lines(&lines, &args.query, dict, args.highlight);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    for line in &results {
        let _ = writeln!(out, "{line}");
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
    fn parses_query_and_flags() {
        let args = parse(&["--highlight", "--dict", "d.txt", "--", "keihi"]).unwrap();
        assert!(args.highlight);
        assert_eq!(args.dict_path, Some(PathBuf::from("d.txt")));
        assert_eq!(args.query, "keihi");
    }

    #[test]
    fn rejects_unknown_argument() {
        assert!(parse(&["--bogus"]).is_err());
    }
}
