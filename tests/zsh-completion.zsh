#!/usr/bin/env zsh
#
# End-to-end test for the zsh completion adapters (shell/zsh/_cd and
# shell/zsh/_kanaq_files): drives a real interactive zsh through a pty,
# types a command line, presses Tab, and checks what ends up in the line
# buffer.
#
# Usage (from anywhere):
#   cargo build --release -p kanaq
#   zsh tests/zsh-completion.zsh
#
# KANAQ_BIN overrides the binary under test (default: target/release/kanaq
# in this repository). The test is hermetic: it runs in a throwaway
# directory with its own HOME, ZDOTDIR, and a tiny SKK dictionary, so your
# own shell config and data directory are never read.
#
# Japanese names are written as \u escapes to keep this file ASCII-only
# (see AGENTS.md).

emulate -L zsh
setopt err_return no_unset pipe_fail

zmodload zsh/zpty || { print -u2 "zsh/zpty is required"; exit 1 }

repo=${0:A:h:h}
kanaq_bin=${KANAQ_BIN:-$repo/target/release/kanaq}
if [[ ! -x $kanaq_bin ]]; then
    print -u2 "kanaq binary not found at $kanaq_bin"
    print -u2 "build it first: cargo build --release -p kanaq (or set KANAQ_BIN)"
    exit 1
fi

# Seconds to wait after each key so zsh can finish completing.
step=${KANAQ_TEST_STEP:-0.5}

tesuto=$'\u30c6\u30b9\u30c8'    # katakana "tesuto"
kaihatsu=$'\u958b\u767a'        # kanji read "kaihatsu"
kaihatsu_kana=$'\u304b\u3044\u306f\u3064'

work=$(mktemp -d)
cleanup() {
    if zpty -d kanaq_test 2>/dev/null; then
        sleep 0.2    # let the killed shell exit before removing its files
    fi
    [[ -n $work && -d $work ]] && rm -rf -- "$work"
}
trap cleanup EXIT INT TERM

# --- Fixture --------------------------------------------------------------

fixture=$work/fixture
mkdir -p "$fixture"/{test,$tesuto,${tesuto}2,kanri-a,kanri-b} \
    "$fixture"/{202610_$tesuto,202610_$kaihatsu,.claude,.cache}
: > "$fixture/.zshrc"
: > "$fixture/.zprofile"

# kanaq looks for SKK-JISYO.L in its per-user data directory, which the
# fake HOME below redirects into $work.
fake_home=$work/home
case $OSTYPE in
    darwin*) data_dir="$fake_home/Library/Application Support/com.onah.kanaq" ;;
    *) data_dir="$fake_home/.local/share/kanaq" ;;
esac
mkdir -p "$data_dir"
print -r -- ";; okuri-nasi entries.
$kaihatsu_kana /$kaihatsu/" | iconv -f UTF-8 -t EUC-JP > "$data_dir/SKK-JISYO.L"

zdotdir=$work/zdotdir
mkdir -p "$zdotdir"
buffer_file=$work/buffer
cat > "$zdotdir/.zshrc" <<EOF
PS1='> '
unsetopt LIST_AMBIGUOUS
fpath=(${(q)repo}/shell/zsh \$fpath)
autoload -Uz compinit && compinit -D
export KANAQ_BIN=${(q)kanaq_bin}
# ^O writes the current line buffer to a file so the test can read it.
dump-buffer() { print -r -- "\$BUFFER" > ${(q)buffer_file} }
zle -N dump-buffer
bindkey '^O' dump-buffer
cd ${(q)fixture}
EOF

# --- Driver ---------------------------------------------------------------

drain() {
    local chunk
    while zpty -r -t kanaq_test chunk 2>/dev/null; do :; done
}

# Type $1, press Tab $2 times, and print the resulting line buffer.
complete_line() {
    local typed=$1 tabs=$2 i
    : > "$buffer_file"
    zpty -w -n kanaq_test "$typed"; sleep $step; drain
    for (( i = 0; i < tabs; i++ )); do
        zpty -w -n kanaq_test $'\t'; sleep $step; drain
    done
    zpty -w -n kanaq_test $'\x0f'; sleep $step; drain    # ^O: dump buffer
    zpty -w -n kanaq_test $'\x15'; sleep $step; drain    # ^U: clear line
    print -r -- "$(<$buffer_file)"
}

# zpty runs its command through the shell, so pass the environment inline.
zpty -b kanaq_test "HOME=${(q)fake_home} ZDOTDIR=${(q)zdotdir} zsh -i"
sleep 1; drain

# --- Cases: typed text, Tab presses, expected line ------------------------

cases=(
    # A reading match with no literal common prefix keeps the typed word,
    # then Tab cycles through the candidates.
    'cd te'          1 'cd te'
    'cd te'          2 'cd test/'
    'cd te'          3 "cd $tesuto/"
    # A common prefix found by reading is inserted.
    'cd tesu'        1 "cd $tesuto"
    # Plain prefix completion still works.
    'cd kanri'       1 'cd kanri-'
    # No candidates: the line is left alone.
    'cd zzz'         1 'cd zzz'
    # Digits and punctuation mixed with romaji.
    'cd 20'          1 'cd 202610_'
    'cd 202610_te'   1 "cd 202610_$tesuto/"
    'cd 202610_ka'   1 "cd 202610_$kaihatsu/"
    'cd 202610_x'    1 'cd 202610_x'
    # Dotfiles: offered only for a leading ".", matched as plain text.
    'cd .cla'        1 'cd .claude/'
    'cd .c'          1 'cd .c'
    'vim .zsh'       1 'vim .zshrc'
    'vim .zp'        1 'vim .zprofile'
    # _kanaq_files behaves like _cd.
    'vim te'         1 'vim te'
    'vim te'         2 'vim test/'
)

failures=0
for typed tabs expected in "${cases[@]}"; do
    actual=$(complete_line "$typed" $tabs)
    label="${(r:16:)typed} x$tabs"
    if [[ $actual == "$expected" ]]; then
        print -r -- "ok   $label -> $actual"
    else
        print -r -- "FAIL $label -> $actual (expected: $expected)"
        (( failures++ )) || true
    fi
done

print -r -- "$(( ${#cases} / 3 - failures )) passed, $failures failed"
(( failures == 0 ))
