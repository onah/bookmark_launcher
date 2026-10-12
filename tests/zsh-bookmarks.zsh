#!/usr/bin/env zsh
#
# End-to-end test for shell/zsh/kanaq-bm.zsh: runs kanaq-bm-pick through a
# pty (fzf needs a terminal) against a fixture bookmark file and checks
# which url gets opened and how the visit counts change.
#
# Usage (from anywhere):
#   cargo build --release -p kanaq -p kanaq-bm
#   zsh tests/zsh-bookmarks.zsh
#
# KANAQ_BIN / KANAQ_BM_BIN override the binaries under test (default:
# target/release/ in this repository). The test is hermetic: it uses its own
# HOME, so your bookmarks are never read or changed, and a recording
# KANAQ_BM_OPENER instead of the browser. fzf picks the first line by itself
# via `--bind load:accept`.
#
# Japanese text is written as \u escapes to keep this file ASCII-only (see
# AGENTS.md).

emulate -L zsh
setopt err_return no_unset pipe_fail

zmodload zsh/zpty || { print -u2 "zsh/zpty is required"; exit 1 }

repo=${0:A:h:h}
kanaq_bin=${KANAQ_BIN:-$repo/target/release/kanaq}
kanaq_bm_bin=${KANAQ_BM_BIN:-$repo/target/release/kanaq-bm}
for bin in $kanaq_bin $kanaq_bm_bin; do
    if [[ ! -x $bin ]]; then
        print -u2 "binary not found at $bin"
        print -u2 "build first: cargo build --release -p kanaq -p kanaq-bm"
        exit 1
    fi
done
command -v fzf >/dev/null || { print -u2 "fzf is required"; exit 1 }

puroguramu=$'\u30d7\u30ed\u30b0\u30e9\u30e0'    # katakana "puroguramu"

work=$(mktemp -d)
cleanup() {
    zpty -d kanaq_bm_test 2>/dev/null || true
    [[ -n $work && -d $work ]] && rm -rf -- "$work"
}
trap cleanup EXIT INT TERM

# --- Fixture --------------------------------------------------------------

fake_home=$work/home
case $OSTYPE in
    darwin*) data_dir="$fake_home/Library/Application Support/com.onah.kanaq" ;;
    *) data_dir="$fake_home/.local/share/kanaq" ;;
esac
mkdir -p "$data_dir"
cat > "$data_dir/bookmarks.toml" <<EOF
[[bookmarks]]
type = "bookmark"
title = "Ruby docs"
url = "https://ruby.example/docs"
access_count = 5

[[bookmarks]]
type = "bookmark"
title = "$puroguramu notes"
url = "https://prog.example/"
tags = ["work"]
access_count = 1

[[bookmarks]]
type = "bookmark"
title = "Rust docs"
url = "https://rust.example/docs"
access_count = 2
EOF

opened=$work/opened
opener=$work/opener
print -r -- "#!/bin/sh
printf '%s\n' \"\$1\" >> ${(q)opened}" > "$opener"
chmod +x "$opener"

# --- Driver ---------------------------------------------------------------

# Run `kanaq-bm-pick $1` in a pty and print the url it opened ("" if none).
pick() {
    local query=$1 i
    : > "$opened"
    local script="source ${(q)repo}/shell/zsh/kanaq-bm.zsh; kanaq-bm-pick $query"
    zpty -b kanaq_bm_test \
        "HOME=${(q)fake_home} KANAQ_BIN=${(q)kanaq_bin} KANAQ_BM_BIN=${(q)kanaq_bm_bin}" \
        "KANAQ_BM_OPENER=${(q)opener} FZF_DEFAULT_OPTS='--bind load:accept'" \
        "zsh -f -c ${(q)script}"
    # Wait (up to 10s) for it to finish, reading its output so it never
    # blocks on a full pty.
    for (( i = 0; i < 100; i++ )); do
        zpty -r -t kanaq_bm_test chunk 2>/dev/null || true
        zpty -t kanaq_bm_test 2>/dev/null || break
        sleep 0.1
    done
    zpty -d kanaq_bm_test 2>/dev/null || true
    print -r -- "$(<$opened)"
}

list_titles_and_counts() {
    HOME=$fake_home $kanaq_bm_bin list | cut -f1 | tr '\n' '|'
}

failures=0
check() {
    local label=$1 actual=$2 expected=$3
    if [[ $actual == "$expected" ]]; then
        print -r -- "ok   $label -> $actual"
    else
        print -r -- "FAIL $label -> $actual (expected: $expected)"
        (( failures++ )) || true
    fi
}

# --- Cases ----------------------------------------------------------------

# A single match by reading opens directly, without fzf.
check 'pick puro' "$(pick puro)" 'https://prog.example/'
# Tags are searchable.
check 'pick work' "$(pick work)" 'https://prog.example/'
# Several matches go through fzf; equal scores keep the most-opened first.
check 'pick docs' "$(pick docs)" 'https://ruby.example/docs'
# No query lists everything, most opened first.
check 'pick' "$(pick '')" 'https://ruby.example/docs'
# No match: nothing is opened.
check 'pick zzz' "$(pick zzz)" ''
# Visits were counted: prog.example (1 + 2) now outranks Rust docs (2).
check 'list order' "$(list_titles_and_counts)" "Ruby docs|$puroguramu notes|Rust docs|"

# kanaq-bm-edit opens the bookmark file in $EDITOR.
edited=$(HOME=$fake_home KANAQ_BM_BIN=$kanaq_bm_bin EDITOR=print \
    zsh -f -c "source ${(q)repo}/shell/zsh/kanaq-bm.zsh; kanaq-bm-edit")
check 'edit' "$edited" "$data_dir/bookmarks.toml"

print -r -- "$(( 7 - failures )) passed, $failures failed"
(( failures == 0 ))
