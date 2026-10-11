//! `kanaq`: search Japanese text with romaji input. Each subcommand is a
//! small CLI meant to be driven by a shell or fzf; see `commands/` for their
//! contracts.

mod commands;

const USAGE: &str = "\
usage: kanaq <command> [options]

commands:
  filter    rank stdin lines against a romaji query (for fzf's reload hook)
  complete  print shell completion candidates for a path token
";

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        eprint!("{USAGE}");
        std::process::exit(1);
    };

    let result = match command.as_str() {
        "filter" => commands::filter::run(args),
        "complete" => commands::complete::run(args),
        "help" | "-h" | "--help" => {
            print!("{USAGE}");
            return;
        }
        other => {
            eprintln!("kanaq: unknown command: {other}");
            eprint!("{USAGE}");
            std::process::exit(1);
        }
    };

    if let Err(message) = result {
        eprintln!("kanaq {command}: {message}");
        std::process::exit(1);
    }
}
