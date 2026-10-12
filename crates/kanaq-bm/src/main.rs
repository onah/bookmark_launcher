//! `kanaq-bm`: bookmarks for fzf. Searching and picking happen in fzf
//! through `kanaq filter`; this binary only reads and writes the bookmark
//! file.
//!
//! Contract:
//! - `kanaq-bm list`: print one `title<TAB>tags<TAB>url` line per bookmark,
//!   most opened first.
//! - `kanaq-bm path`: print the bookmark file's path (e.g. to open it in an
//!   editor for hand edits).

use kanaq_bm::list::{format_line, sorted_bookmarks};
use kanaq_bm::store;
use std::io::{self, Write};
use std::path::PathBuf;

const USAGE: &str = "\
usage: kanaq-bm <command>

commands:
  list   print bookmarks as fzf lines (title, tags, url), most opened first
  path   print the bookmark file's path
";

fn bookmark_path() -> Result<PathBuf, String> {
    store::default_path().ok_or_else(|| "no home directory to keep bookmarks in".to_string())
}

fn list() -> Result<(), String> {
    let file = store::load(&bookmark_path()?)?;
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for bookmark in sorted_bookmarks(&file.bookmarks) {
        let _ = writeln!(out, "{}", format_line(bookmark));
    }
    Ok(())
}

fn path() -> Result<(), String> {
    println!("{}", bookmark_path()?.display());
    Ok(())
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        eprint!("{USAGE}");
        std::process::exit(1);
    };
    if let Some(extra) = args.next() {
        eprintln!("kanaq-bm {command}: unexpected argument: {extra}");
        std::process::exit(1);
    }

    let result = match command.as_str() {
        "list" => list(),
        "path" => path(),
        "help" | "-h" | "--help" => {
            print!("{USAGE}");
            return;
        }
        other => {
            eprintln!("kanaq-bm: unknown command: {other}");
            eprint!("{USAGE}");
            std::process::exit(1);
        }
    };

    if let Err(message) = result {
        eprintln!("kanaq-bm {command}: {message}");
        std::process::exit(1);
    }
}
