# zsh functions for kanaq-bm: pick a bookmark with fzf (searching titles,
# tags, and urls by romaji through `kanaq filter`) and open it, or open the
# bookmark file in an editor.
#
# Install:
#   1. Make sure `fzf`, `kanaq`, and `kanaq-bm` are on $PATH (or set
#      KANAQ_BIN / KANAQ_BM_BIN to their absolute paths).
#   2. source this file from your .zshrc and give the functions short
#      aliases, e.g.:
#        source /path/to/kanaq-bm.zsh
#        alias bm=kanaq-bm-pick
#        alias bme=kanaq-bm-edit
#
# Usage:
#   kanaq-bm-pick [query...]  pick a bookmark and open it in the browser.
#       With a query, fzf opens already filtered by it, and if exactly one
#       bookmark matches, it is opened without opening fzf at all.
#   kanaq-bm-edit             open the bookmark file in $EDITOR (vi if unset)
#
# Add and remove bookmarks with `kanaq-bm add <url>` / `kanaq-bm remove <url>`.

kanaq-bm-pick() {
    local bin="${KANAQ_BIN:-kanaq}" bm="${KANAQ_BM_BIN:-kanaq-bm}" cmd
    for cmd in "$bin" "$bm" fzf; do
        if ! command -v -- "$cmd" >/dev/null 2>&1; then
            echo "kanaq-bm-pick: $cmd not found on \$PATH (see shell/zsh/kanaq-bm.zsh)" >&2
            return 1
        fi
    done

    local tmp
    tmp=$(mktemp) || return 1
    # One `title<TAB>tags<TAB>url` line per bookmark, most opened first.
    if ! "$bm" list > "$tmp"; then
        rm -f "$tmp"
        return 1
    fi

    local query="$*" line ret=0
    if [[ -n "$query" ]]; then
        local -a hits
        hits=(${(f)"$("$bin" filter -- "$query" < "$tmp")"})
        (( ${#hits} == 1 )) && line="${hits[1]}"
    fi

    if [[ -z "$line" ]]; then
        # Same scheme as kanaq-cd: the initial list is pre-filtered by the
        # query, and edits to the query re-rank the full list via `reload`.
        line=$("$bin" filter --highlight -- "$query" < "$tmp" |
            fzf --disabled --ansi --height 40% --reverse --prompt 'bm> ' \
                --query "$query" \
                --bind "change:reload($bin filter --highlight -- {q} < ${(q)tmp})")
        ret=$?
    fi
    rm -f "$tmp"

    (( ret != 0 )) && return $ret
    [[ -n "$line" ]] && "$bm" open "${line##*$'\t'}"
}

kanaq-bm-edit() {
    local bm="${KANAQ_BM_BIN:-kanaq-bm}" file
    file=$("$bm" path) || return 1
    "${EDITOR:-vi}" "$file"
}
