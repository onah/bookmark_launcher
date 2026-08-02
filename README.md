# Bookmark Launcher

A fast and intuitive bookmark launcher built with Rust and egui. Features fuzzy search, automatic title fetching, and access count tracking.

## Features

- **Fuzzy Search**: Intelligent fuzzy matching for bookmark titles
- **Auto Title Fetching**: Automatically fetches page titles from URLs
- **Access Count Tracking**: Tracks how often each bookmark is accessed
- **Persistent Storage**: Saves bookmarks to JSON file
- **Keyboard Shortcuts**:
  - `Enter`: Open selected bookmark or add new URL
  - `Escape`: Close application

## Installation

### Prerequisites

- Rust 1.70 or later
- Windows/Linux/macOS

### Build from Source

```bash
git clone https://github.com/onah/bookmark_launcher.git
cd bookmark_launcher
cargo build --release
```

The executable will be available at `target/release/bookmark_launcher`.

## Usage

1. Run the application:
   ```bash
   cargo run
   ```

2. The search input will be focused automatically

3. Type to search existing bookmarks using fuzzy matching

4. Press `Enter` to:
   - Open the first matching bookmark, or
   - Add a new URL as a bookmark (if input contains a URL)

5. Press `Escape` to close the application

### Search modes (TUI)

- `Ctrl+B`: Switch to combined fuzzy + migemo search mode (the default)
- `Ctrl+F`: Switch to fuzzy-only search mode
- `Ctrl+T`: Switch to migemo-only search mode

In migemo mode (and the migemo half of combined mode), romaji input can match Japanese text (for example, `puro` can match `プロ`).

The default combined mode includes a bookmark if either the fuzzy or the migemo matcher hits it, then ranks results by a 1:1:1 blend of the fuzzy score, the migemo score, and the access count (each normalized to the same 0–1 scale before summing, so the three signals carry equal weight).

### Migemo dictionary location

Place `SKK-JISYO.L` (EUC-JP encoded) in the same data directory as `bookmarks.toml`.

On macOS, this is typically under:

- `~/Library/Application Support/com.onah.bookmark_launcher/`

You can download the dictionary from:

- `https://github.com/skk-dev/dict/blob/master/SKK-JISYO.L`

### Backend selection

- Default backend is TUI:
  ```bash
  cargo run
  ```
- Run with eframe backend only:
  ```bash
  cargo run --no-default-features --features backend-eframe
  ```

## migemo-fzf: fuzzy directory jump

`migemo-fzf` (a separate binary in this workspace) is a thin stdin -> stdout filter built for fzf's `--bind change:reload(...)` hook: it reads candidate lines from stdin, ranks them against a query with the same migemo matcher this app uses, and prints the survivors highest-score-first (optionally ANSI-highlighted). Paired with the zsh widget in `migemo-fzf/shell/migemo-cd.zsh`, it's a migemo-aware replacement for fzf's own Alt+C (`fzf-cd-widget`): search subdirectories under the current directory and `cd` into the one you pick.

### Setup

1. Build it and put it on `$PATH`:
   ```bash
   cargo build --release -p migemo-fzf
   ```
2. Wire up the widget for your shell:

   **zsh** -- source `migemo-cd.zsh` from `.zshrc` and bind it to a key, e.g. to reuse fzf's own Alt+C slot (source this *after* fzf's shell integration so this binding wins):
   ```sh
   source /path/to/bookmark_launcher/migemo-fzf/shell/migemo-cd.zsh
   bindkey '\ec' migemo-cd-widget
   ```

   **PowerShell** -- dot-source `migemo-cd.ps1` from `$PROFILE`, then optionally bind it with PSReadLine:
   ```powershell
   . /path/to/bookmark_launcher/migemo-fzf/shell/migemo-cd.ps1

   Set-PSReadLineKeyHandler -Chord 'Alt+c' -ScriptBlock {
       [Microsoft.PowerShell.PSConsoleReadLine]::RevertLine()
       Invoke-MigemoCd
       [Microsoft.PowerShell.PSConsoleReadLine]::InvokePrompt()
   }
   ```

### Usage

Press `Alt+C` (or run `migemo-cd` in zsh / `Invoke-MigemoCd` in PowerShell directly) in any directory: fzf opens with every subdirectory listed, and typing romaji or kana filters it live via `migemo-fzf` (e.g. `puro` matches `プロ`). Selecting one `cd`s into it.

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

Under the hood: the directory tree is walked once per invocation into a temp file, and fzf runs with `--disabled` (its own filter off), re-ranking that same snapshot through `migemo-fzf` on every keystroke -- so filtering stays fast even on a large tree, since only the (cheap) ranking step repeats, not the (expensive) tree walk.

## Bookmark Management

### Adding Bookmarks

- Enter a URL (starting with `http://` or `https://`) and press `Enter`
- The application will automatically fetch the page title
- Bookmarks are saved to `bookmarks.json`

### Searching Bookmarks

- Type any part of a bookmark title
- Fuzzy matching allows for typos and partial matches
- Results are sorted by relevance score

### Access Tracking

- Each time a bookmark is opened, its access count increases
- Data is automatically saved to `bookmarks.json`

## Configuration

Bookmarks are stored in `bookmarks.json` in the following format:

```json
[
  {
    "title": "GitHub",
    "url": "https://github.com",
    "access_count": 5
  }
]
```

## Dependencies

- `eframe`: GUI framework
- `serde` & `serde_json`: Serialization
- `open`: Cross-platform URL opening
- `reqwest`: HTTP client for title fetching
- `scraper`: HTML parsing
- `fuzzy-matcher`: Fuzzy search algorithm

## Development

### Running in Debug Mode

```bash
cargo run
```

### Building for Release

```bash
cargo build --release
```

### Running Tests

```bash
cargo test
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run `cargo clippy` and `cargo test`
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Built with [egui](https://github.com/emilk/egui) - an easy-to-use GUI library for Rust
- Fuzzy search powered by [fuzzy-matcher](https://github.com/lotabout/fuzzy-matcher)