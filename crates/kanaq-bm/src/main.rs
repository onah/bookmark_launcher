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
//! - `kanaq-bm add <url> [--title <title>] [--tag <tag>]...`: add a
//!   bookmark. Without `--title`, the page's title is fetched; if that
//!   fails, the url's host name is used.
//! - `kanaq-bm remove <url>`: remove the bookmark with that url.

use kanaq_bm::list::{format_line, sorted_bookmarks};
use kanaq_bm::store::{self, Bookmark};
use kanaq_bm::title::{fetch_title, host_of};
use std::io::{self, Write};
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

const USAGE: &str = "\
usage: kanaq-bm <command>

commands:
  list        print bookmarks as fzf lines (title, tags, url), most opened first
  path        print the bookmark file's path
  open <url>  open a bookmark in the browser and count the visit
  add <url> [--title <title>] [--tag <tag>]...
              add a bookmark (the title is fetched from the page if omitted)
  remove <url>
              remove a bookmark
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

/// The bookmark file's text, or "" if it does not exist yet.
fn read_text(path: &Path) -> Result<String, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

fn open(url: &str) -> Result<(), String> {
    let path = bookmark_path()?;
    let text = read_text(&path)?;
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

#[derive(Debug, PartialEq)]
struct AddArgs {
    url: String,
    title: Option<String>,
    tags: Vec<String>,
}

fn parse_add_args(args: Vec<String>) -> Result<AddArgs, String> {
    let mut url = None;
    let mut title = None;
    let mut tags = Vec::new();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--title" => title = Some(args.next().ok_or("--title requires a value")?),
            "--tag" => tags.push(args.next().ok_or("--tag requires a value")?),
            option if option.starts_with("--") => return Err(format!("unknown option: {option}")),
            _ if url.is_some() => return Err(format!("unexpected argument: {arg}")),
            _ => url = Some(arg),
        }
    }
    Ok(AddArgs {
        url: url.ok_or("missing <url>")?,
        title,
        tags,
    })
}

fn add(args: AddArgs) -> Result<(), String> {
    if reqwest::Url::parse(&args.url).is_err() {
        return Err(format!(
            "not a url: {} (include the scheme, e.g. https://)",
            args.url
        ));
    }
    let path = bookmark_path()?;
    let text = read_text(&path)?;
    // Refuse a duplicate before spending a network round trip on its title.
    let file = store::parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    if file
        .bookmarks
        .iter()
        .any(|store::Entry::Bookmark(b)| b.url == args.url)
    {
        return Err(format!("already bookmarked: {}", args.url));
    }

    let title = match args.title {
        Some(title) => title,
        None => fetch_title(&args.url).unwrap_or_else(|e| {
            eprintln!("kanaq-bm add: could not get the page title ({e}); using the host name");
            host_of(&args.url).unwrap_or_else(|| args.url.clone())
        }),
    };
    let bookmark = Bookmark {
        title,
        url: args.url,
        tags: args.tags,
        access_count: 0,
    };
    let updated = store::add_bookmark(&text, &bookmark)?;
    store::write_atomically(&path, &updated)?;
    println!("{}", format_line(&bookmark));
    Ok(())
}

fn remove(url: &str) -> Result<(), String> {
    let path = bookmark_path()?;
    let updated = store::remove_bookmark(&read_text(&path)?, url)?;
    store::write_atomically(&path, &updated)
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
        "add" => parse_add_args(args).and_then(add),
        "remove" => expect_args::<1>(args).and_then(|[url]| remove(&url)),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|a| a.to_string()).collect()
    }

    #[test]
    fn add_args_accept_options_in_any_order() {
        assert_eq!(
            parse_add_args(args(&[
                "--tag",
                "a",
                "https://x.example",
                "--title",
                "X",
                "--tag",
                "b"
            ])),
            Ok(AddArgs {
                url: "https://x.example".into(),
                title: Some("X".into()),
                tags: vec!["a".into(), "b".into()],
            })
        );
    }

    #[test]
    fn add_args_reject_bad_input() {
        assert!(parse_add_args(args(&[])).is_err());
        assert!(parse_add_args(args(&["https://x.example", "--title"])).is_err());
        assert!(parse_add_args(args(&["https://x.example", "--bogus"])).is_err());
        assert!(parse_add_args(args(&["https://x.example", "https://y.example"])).is_err());
    }
}
