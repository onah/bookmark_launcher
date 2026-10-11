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

use migemo_core::load_dictionary;
use migemo_fzf::filter_lines;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

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
