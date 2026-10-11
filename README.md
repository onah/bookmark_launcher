# kanaq

Tools for searching Japanese text with romaji (alphabet) input, built around
fzf. For example, typing `keihi` finds a directory named `経費精算`.

> Work in progress: this repository is being restructured and renamed to
> `kanaq` (see `docs/new_function_design_ja.md`). The former ratatui
> bookmark launcher has been retired; its last state is kept at the git tag
> `tui-final`.

## Workspace

Crates live under `crates/`; shell integration scripts live under
`shell/{powershell,zsh,fish}/`.

- `migemo-core`: library that indexes text into romaji morae and matches
  romaji queries against it (design: `docs/migemo_ja.md`)
- `kanaq`: the command-line tool, with subcommands
  - `kanaq filter`: stdin -> stdout filter driven by fzf's `reload` hook
  - `kanaq complete`: shell completion helper (zsh / fish / PowerShell
    adapters in `shell/`)

Build it with `cargo build --release -p kanaq` and put
`target/release/kanaq` on `$PATH`, or point the shell scripts at it with
the `KANAQ_BIN` environment variable.

## Dictionary

Place `SKK-JISYO.L` (EUC-JP encoded) in the shared data directory:

- macOS: `~/Library/Application Support/com.onah.kanaq/`
- Windows: `%APPDATA%\onah\kanaq\data\`
- Linux: `~/.local/share/kanaq/`

A compiled cache (`SKK-JISYO.L.compiled`) is created next to it on first
use. The dictionary can be downloaded from
`https://github.com/skk-dev/dict/blob/master/SKK-JISYO.L`.

Upgrading from the old layout: the data directory used to be named after
`bookmark_launcher` (e.g. `~/Library/Application Support/com.onah.bookmark_launcher/`
on macOS). Move its contents into the directory above; it is not read
anymore.

## Directory jump: kanaq filter + fzf

`kanaq filter` is a thin stdin -> stdout filter built for fzf's `--bind change:reload(...)` hook: it reads candidate lines from stdin, ranks them against a query with the `migemo-core` matcher, and prints the survivors highest-score-first (optionally ANSI-highlighted). Paired with the zsh widget in `shell/zsh/migemo-cd.zsh`, it's a migemo-aware replacement for fzf's own Alt+C (`fzf-cd-widget`): search subdirectories under the current directory and `cd` into the one you pick.

### Setup

1. Build it and put it on `$PATH`:
   ```bash
   cargo build --release -p kanaq
   ```
2. Wire up the widget for your shell:

   **zsh** -- source `migemo-cd.zsh` from `.zshrc` and bind it to a key, e.g. to reuse fzf's own Alt+C slot (source this *after* fzf's shell integration so this binding wins):
   ```sh
   source /path/to/kanaq/shell/zsh/migemo-cd.zsh
   bindkey '\ec' migemo-cd-widget
   ```

   **PowerShell** -- dot-source `migemo-cd.ps1` from `$PROFILE`, then optionally bind it with PSReadLine:
   ```powershell
   . /path/to/kanaq/shell/powershell/migemo-cd.ps1

   Set-PSReadLineKeyHandler -Chord 'Alt+c' -ScriptBlock {
       [Microsoft.PowerShell.PSConsoleReadLine]::RevertLine()
       Invoke-MigemoCd
       [Microsoft.PowerShell.PSConsoleReadLine]::InvokePrompt()
   }
   ```

### Usage

Press `Alt+C` (or run `migemo-cd` in zsh / `Invoke-MigemoCd` in PowerShell directly) in any directory: fzf opens with every subdirectory listed, and typing romaji or kana filters it live via `kanaq filter` (e.g. `puro` matches `プロ`). Selecting one `cd`s into it.

`.git` and `node_modules` are excluded from the listing by default:

- zsh: override with `MIGEMO_CD_FIND_OPTS` (passed to `find` after `.`), e.g. to also exclude `.venv`:
  ```sh
  MIGEMO_CD_FIND_OPTS='( -name .git -o -name .venv ) -prune -o -type d -print'
  ```
- PowerShell: override with `$env:MIGEMO_CD_EXCLUDE_DIRS` (semicolon-separated directory names), e.g.:
  ```powershell
  $env:MIGEMO_CD_EXCLUDE_DIRS = '.git;.venv'
  ```

Either way, an excluded directory is pruned -- never descended into -- not just hidden from the results, so excluding `node_modules` also skips walking everything underneath it.

Under the hood: the directory tree is walked once per invocation into a temp file, and fzf runs with `--disabled` (its own filter off), re-ranking that same snapshot through `kanaq filter` on every keystroke -- so filtering stays fast even on a large tree, since only the (cheap) ranking step repeats, not the (expensive) tree walk.

## Development

```bash
cargo build --release
cargo test --workspace
cargo clippy --workspace --all-targets
```

To see how `migemo-core` reads a text and decomposes a query:

```bash
cargo run --release -p migemo-core --example inspect -- 経費精算 keihi
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.
