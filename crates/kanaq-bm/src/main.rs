//! `kanaq-bm`: bookmarks for fzf. Searching and picking happen in fzf
//! through `kanaq filter`; this binary only reads and writes the bookmark
//! file.
//!
//! Contract:
//! - `kanaq-bm list`: print one `title<TAB>tags<TAB>url` line per bookmark,
//!   most opened first.
//! - `kanaq-bm path`: print the bookmark file's path (e.g. to open it in an
//!   editor for hand edits).
//! - `kanaq-bm open <url>`: open the bookmark with that url in the browser
//!   and count the visit (used to order `list`). Set `KANAQ_BM_OPENER` to a
//!   program to run with the url instead of the system default browser.

use kanaq_bm::list::{format_line, sorted_bookmarks};
use kanaq_bm::store;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

const USAGE: &str = "\
usage: kanaq-bm <command>

commands:
  list        print bookmarks as fzf lines (title, tags, url), most opened first
  path        print the bookmark file's path
  open <url>  open a bookmark in the browser and count the visit
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

fn open(url: &str) -> Result<(), String> {
    let path = bookmark_path()?;
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    // Check the url is a bookmark before launching anything.
    let updated = store::increment_access_count(&text, url)?;
    launch(url)?;
    store::write_atomically(&path, &updated)
}

fn launch(url: &str) -> Result<(), String> {
    match std::env::var_os("KANAQ_BM_OPENER").filter(|cmd| !cmd.is_empty()) {
        Some(opener) => {
            let status = Command::new(&opener)
                .arg(url)
                .status()
                .map_err(|e| format!("{}: {e}", opener.to_string_lossy()))?;
            if status.success() {
                Ok(())
            } else {
                Err(format!("{} exited with {status}", opener.to_string_lossy()))
            }
        }
        None => open::that(url).map_err(|e| format!("cannot open {url}: {e}")),
    }
}

/// Fail unless exactly `N` arguments were given.
fn expect_args<const N: usize>(args: Vec<String>) -> Result<[String; N], String> {
    let count = args.len();
    args.try_into()
        .map_err(|_| format!("expected {N} argument(s), got {count}"))
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        eprint!("{USAGE}");
        std::process::exit(1);
    };
    let args: Vec<String> = args.collect();

    let result = match command.as_str() {
        "list" => expect_args::<0>(args).and_then(|[]| list()),
        "path" => expect_args::<0>(args).and_then(|[]| path()),
        "open" => expect_args::<1>(args).and_then(|[url]| open(&url)),
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
