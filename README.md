# kanaq

Tools for searching Japanese text with romaji (alphabet) input, built around
fzf. For example, typing `keihi` finds a directory named `経費精算`.

Formerly `bookmark_launcher`; the design behind the rename is in
`docs/new_function_design_ja.md`. The old ratatui bookmark launcher has
been retired; its last state is kept at the git tag `tui-final`.

## Workspace

Crates live under `crates/`; shell integration scripts live under
`shell/{powershell,zsh,fish}/`.

- `migemo-core`: library that indexes text into romaji morae and matches
  romaji queries against it (design: `docs/migemo_ja.md`)
- `kanaq`: the command-line tool, with subcommands
  - `kanaq filter`: stdin -> stdout filter driven by fzf's `reload` hook
  - `kanaq complete`: shell completion helper (zsh / fish / PowerShell
    adapters in `shell/`)
- `kanaq-bm`: bookmarks picked with fzf (see [Bookmarks](#bookmarks-kanaq-bm));
  a separate binary because fetching page titles needs HTTP/TLS

Build them with `cargo build --release -p kanaq -p kanaq-bm` and put
`target/release/kanaq` and `target/release/kanaq-bm` on `$PATH`, or point
the shell scripts at them with the `KANAQ_BIN` / `KANAQ_BM_BIN` environment
variables.

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

## zsh setup

The zsh integration is made of four files, each installed differently:

| File | What it does | Where it goes |
|---|---|---|
| `kanaq` (binary) | the tool itself | a directory on `$PATH`, e.g. `~/.local/bin` |
| `shell/zsh/_cd` | Tab completion for `cd` / `pushd` | a directory on `$fpath` (not `$PATH`) |
| `shell/zsh/_kanaq_files` | Tab completion for `vim` / `vi` / `less` | a directory on `$fpath` |
| `shell/zsh/kanaq-cd.zsh` | `kanaq-cd` directory jump (needs fzf) | anywhere; `source`d from `.zshrc` |

1. Build and install the binary, and put the dictionary in place (see
   [Dictionary](#dictionary)):
   ```sh
   cargo build --release -p kanaq
   cp target/release/kanaq ~/.local/bin/
   ```
2. Link the completion files into a completion directory. Symlinks keep
   them in sync with the repository:
   ```sh
   mkdir -p ~/.zsh/completions
   ln -sf /path/to/kanaq/shell/zsh/_cd ~/.zsh/completions/_cd
   ln -sf /path/to/kanaq/shell/zsh/_kanaq_files ~/.zsh/completions/_kanaq_files
   ```
3. Install fzf (only needed for `kanaq-cd`), e.g. `brew install fzf`.
4. Add this to `.zshrc`. The `fpath` line must come before `compinit`. If
   you bind Alt+C, source `kanaq-cd.zsh` after fzf's own shell integration
   (if you use it) so this binding wins:
   ```zsh
   unsetopt LIST_AMBIGUOUS
   fpath=(~/.zsh/completions $fpath)
   autoload -Uz compinit && compinit

   source /path/to/kanaq/shell/zsh/kanaq-cd.zsh
   alias j=kanaq-cd                 # run it as a command: j, j keihi
   # bindkey '\ec' kanaq-cd-widget  # optional: also bind it to Alt+C
   ```
   `unsetopt LIST_AMBIGUOUS` is recommended because a romaji match usually
   shares no literal prefix with what you typed; without it, the first Tab
   shows nothing (see the comments in `shell/zsh/_cd`).
5. Restart zsh and try it. If completion still behaves like before, clear
   the completion cache first:
   ```sh
   rm -f ~/.zcompdump*
   exec zsh
   ```
   Then `cd tesu<Tab>` should offer a directory named `テスト`, and `j`
   should open fzf.

If the binary is not on `$PATH`, set `KANAQ_BIN` to its absolute path. If
you bind Alt+C on macOS, the terminal must send Option as Meta (Terminal.app:
Settings > Profiles > Keyboard > "Use Option as Meta key"; iTerm2: Profiles >
Keys > Left Option key: Esc+).

Upgrading from the migemo-named scripts: remove the old
`~/.zsh/completions/_migemo_files` and the old `migemo-complete` binary, and
replace `migemo-cd.zsh` / `migemo-cd-widget` in `.zshrc` with the names
above.

## Directory jump: kanaq filter + fzf

`kanaq filter` is a thin stdin -> stdout filter built for fzf's `--bind change:reload(...)` hook: it reads candidate lines from stdin, ranks them against a query with the `migemo-core` matcher, and prints the survivors highest-score-first (optionally ANSI-highlighted). Paired with the zsh widget in `shell/zsh/kanaq-cd.zsh`, it's a migemo-aware replacement for fzf's own Alt+C (`fzf-cd-widget`): search subdirectories under the current directory and `cd` into the one you pick.

### Setup

1. Build it and put it on `$PATH`:
   ```bash
   cargo build --release -p kanaq
   ```
2. Wire up the widget for your shell:

   **zsh** -- see [zsh setup](#zsh-setup).

   **PowerShell** -- dot-source `kanaq-cd.ps1` from `$PROFILE`, then give it an alias and/or bind it with PSReadLine:
   ```powershell
   . /path/to/kanaq/shell/powershell/kanaq-cd.ps1
   Set-Alias j Invoke-KanaqCd

   # Optional: also bind it to Alt+C
   Set-PSReadLineKeyHandler -Chord 'Alt+c' -ScriptBlock {
       [Microsoft.PowerShell.PSConsoleReadLine]::RevertLine()
       Invoke-KanaqCd
       [Microsoft.PowerShell.PSConsoleReadLine]::InvokePrompt()
   }
   ```

### Usage

Run `j` (the alias for `kanaq-cd` in zsh / `Invoke-KanaqCd` in PowerShell, or the Alt+C binding) in any directory: fzf opens with every subdirectory listed, and typing romaji or kana filters it live via `kanaq filter` (e.g. `puro` matches `プロ`). Selecting one `cd`s into it.

Give it a query to start from, like zoxide's `z foo`: `j keihi` opens fzf already filtered by `keihi`, and if exactly one directory matches, it jumps there directly without opening fzf.

`.git` and `node_modules` are excluded from the listing by default:

- zsh: override with `KANAQ_CD_FIND_OPTS` (passed to `find` after `.`), e.g. to also exclude `.venv`:
  ```sh
  KANAQ_CD_FIND_OPTS='( -name .git -o -name .venv ) -prune -o -type d -print'
  ```
- PowerShell: override with `$env:KANAQ_CD_EXCLUDE_DIRS` (semicolon-separated directory names), e.g.:
  ```powershell
  $env:KANAQ_CD_EXCLUDE_DIRS = '.git;.venv'
  ```

Either way, an excluded directory is pruned -- never descended into -- not just hidden from the results, so excluding `node_modules` also skips walking everything underneath it.

Under the hood: the directory tree is walked once per invocation into a temp file, and fzf runs with `--disabled` (its own filter off), re-ranking that same snapshot through `kanaq filter` on every keystroke -- so filtering stays fast even on a large tree, since only the (cheap) ranking step repeats, not the (expensive) tree walk.

## Bookmarks: kanaq-bm

Bookmarks live in `bookmarks.toml` in the same data directory as the
dictionary (`kanaq-bm path` prints its location). Searching and picking
happen in fzf through `kanaq filter`, so titles, tags, and urls can all be
searched by romaji (e.g. `kintai` finds a page titled 勤怠...).

```sh
kanaq-bm add https://example.com/              # title fetched from the page
kanaq-bm add https://example.com/ --title "Example" --tag work --tag ref
kanaq-bm remove https://example.com/
kanaq-bm list                                  # title<TAB>tags<TAB>url, most opened first
kanaq-bm open https://example.com/             # open in the browser, count the visit
```

When `--title` is omitted, the page title is fetched; if that fails (offline,
proxy, ...), the url's host name is used instead. Set `KANAQ_BM_OPENER` to a
program to open urls with instead of the system default browser.

To edit bookmarks (rename, retag, reorder), edit the file directly: updates
made by `kanaq-bm` only touch the values they change, so comments and
formatting you write are kept.

### Shell setup

**zsh** -- source `kanaq-bm.zsh` from `.zshrc` and give it aliases:
```zsh
source /path/to/kanaq/shell/zsh/kanaq-bm.zsh
alias bm=kanaq-bm-pick     # bm, bm kintai
alias bme=kanaq-bm-edit    # open bookmarks.toml in $EDITOR
```

**PowerShell** -- dot-source `kanaq-bm.ps1` from `$PROFILE`:
```powershell
. /path/to/kanaq/shell/powershell/kanaq-bm.ps1
Set-Alias bm Invoke-KanaqBmPick
Set-Alias bme Invoke-KanaqBmEdit     # $env:EDITOR, or notepad
```

`bm` opens fzf with every bookmark, most opened first; pick one to open it.
Like `j`, `bm <query>` starts already filtered and opens the bookmark
directly when exactly one matches.

## Development

```bash
cargo build --release
cargo test --workspace
cargo clippy --workspace --all-targets
```

The zsh completion adapters have an end-to-end test that drives a real
interactive zsh through a pty (about 40 seconds; it uses its own throwaway
HOME and dictionary, so your shell config is not touched):

```bash
cargo build --release -p kanaq
zsh tests/zsh-completion.zsh
```

Run it after any change to matching in `migemo-core`, to `kanaq complete`,
or to `shell/zsh/`.

The zsh bookmark picker has a similar test (a few seconds; its own HOME and
a recording opener instead of the browser):

```bash
cargo build --release -p kanaq -p kanaq-bm
zsh tests/zsh-bookmarks.zsh
```

To see how `migemo-core` reads a text and decomposes a query:

```bash
cargo run --release -p migemo-core --example inspect -- 経費精算 keihi
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.
