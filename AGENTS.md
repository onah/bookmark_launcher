# AGENTS.md

## Code style
- All source code must be written in English (ASCII code) only.
  - Japanese text needed in tests or fixtures is written as escapes:
    `"\u{30c6}"` in Rust, `$'\u30c6'` in zsh, with a comment saying what it
    reads as. Some file-writing tools turn escapes back into literal
    characters, so check new or edited files with
    `LC_ALL=C grep -n '[^ -~]' <file>`.
  - Documents under `docs/` (`*_ja.md`) are written in Japanese.
- Implement in small increments rather than making large changes at once.
- Prioritize quality and avoid accumulating technical debt as much as possible.

## Project context
- What the project is for, decisions made, current state, and open issues:
  `docs/new_function_design_ja.md`.
- How the romaji matching works, including how queries are decomposed:
  `docs/migemo_ja.md`.
- Main target is a low-spec Windows machine (PowerShell 5.1 and 7), so keep
  per-keystroke work small and PowerShell scripts thin (data handling
  belongs in the Rust binaries).

## Workflow
- Do each step on its own short-lived topic branch and commit there.
- Do not merge or push until the user asks. When asked, fast-forward merge
  into `main` (`git merge --ff-only`), delete the topic branch, and push
  `main` together with any new tags. History stays linear (no merge
  commits).

## Verification
- Before every commit: `cargo fmt --all`, `cargo clippy --workspace
  --all-targets` (no warnings), and `cargo test --workspace`. See also
  `docs/rust_checklist.md`.
- After any change to matching (`crates/migemo-core`), `kanaq complete`, or
  the zsh completion scripts (`shell/zsh/_cd`, `shell/zsh/_kanaq_files`),
  run the whole end-to-end suite, not only the cases the change targets:
  `cargo build --release -p kanaq && zsh tests/zsh-completion.zsh`.
  Add a case to its `cases` array for every regression found.
- After any change to `kanaq-bm` or `shell/zsh/kanaq-bm.zsh`:
  `cargo build --release -p kanaq -p kanaq-bm && zsh tests/zsh-bookmarks.zsh`.
- PowerShell scripts cannot be run on the development machine (no `pwsh`);
  say so explicitly when changing them.
