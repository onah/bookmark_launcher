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

- `Ctrl+F`: Switch to fuzzy search mode
- `Ctrl+T`: Switch to migemo search mode

In migemo mode, romaji input can match Japanese text (for example, `puro` can match `プロ`).

### Migemo dictionary location

Place `migemo-compact-dict` in the same data directory as `bookmarks.toml`.

On macOS, this is typically under:

- `~/Library/Application Support/com/onah/bookmark_launcher/`

You can download the dictionary from:

- `https://github.com/oguna/migemo-compact-dict-latest`

### Backend selection

- Default backend is TUI:
  ```bash
  cargo run
  ```
- Run with eframe backend only:
  ```bash
  cargo run --no-default-features --features backend-eframe
  ```

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