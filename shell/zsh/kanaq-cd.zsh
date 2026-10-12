# zsh widget: fuzzy/migemo search directories under $PWD and cd into the
# one you pick. The migemo-aware analog of fzf's own Alt+C (fzf-cd-widget).
#
# Install:
#   1. Make sure `fzf` and `kanaq` are on $PATH (or set KANAQ_BIN to
#      kanaq's absolute path).
#   2. source this file from your .zshrc:
#        source /path/to/kanaq-cd.zsh
#   3. Optionally give it a short alias and/or bind it to a key, e.g. to
#      reuse fzf's own Alt+C slot (source this file *after* fzf's shell
#      integration so this binding wins):
#        alias j=kanaq-cd
#        bindkey '\ec' kanaq-cd-widget
#
# Usage: `kanaq-cd [query...]`. With a query, fzf opens already filtered by
# it, and if exactly one directory matches, kanaq-cd jumps there without
# opening fzf at all (like zoxide's `z foo`).
#
# Set KANAQ_CD_FIND_OPTS to override which directories are listed (default
# excludes .git and node_modules); it's passed to `find` after `.`, e.g.:
#   KANAQ_CD_FIND_OPTS='( -name .git -o -name .venv ) -prune -o -type d -print'

kanaq-cd() {
    local bin="${KANAQ_BIN:-kanaq}"
    if ! command -v -- "$bin" >/dev/null 2>&1; then
        echo "kanaq-cd: $bin not found on \$PATH (see shell/zsh/kanaq-cd.zsh)" >&2
        return 1
    fi
    if ! command -v -- fzf >/dev/null 2>&1; then
        echo "kanaq-cd: fzf not found on \$PATH" >&2
        return 1
    fi

    local tmp
    tmp=$(mktemp) || return 1

    # Walk the tree once per invocation, not once per keystroke -- fzf's
    # `reload` below just re-ranks this static snapshot through `kanaq filter`,
    # which is what keeps each keystroke fast.
    local -a find_opts
    if [[ -n "$KANAQ_CD_FIND_OPTS" ]]; then
        find_opts=(${(z)KANAQ_CD_FIND_OPTS})
    else
        find_opts=(\( -name .git -o -name node_modules \) -prune -o -type d -print)
    fi
    find . "${find_opts[@]}" 2>/dev/null | sed -e 's|^\./||' -e '/^\.$/d' > "$tmp"

    local query="$*" dir ret=0
    if [[ -n "$query" ]]; then
        local -a hits
        hits=("${(@f)$("$bin" filter -- "$query" < "$tmp")}")
        # (@f) on empty output still yields one empty element.
        if (( ${#hits} == 1 )) && [[ -n "${hits[1]}" ]]; then
            dir="${hits[1]}"
        fi
    fi

    if [[ -z "$dir" ]]; then
        # The initial list is pre-filtered by the query (an empty query
        # passes every line through); edits to the query re-rank the full
        # snapshot via `reload`.
        dir=$("$bin" filter --highlight -- "$query" < "$tmp" |
            fzf --disabled --ansi --height 40% --reverse --prompt 'cd> ' \
                --query "$query" \
                --bind "change:reload($bin filter --highlight -- {q} < $tmp)")
        ret=$?
    fi
    rm -f "$tmp"

    (( ret != 0 )) && return $ret
    [[ -n "$dir" ]] && cd -- "$dir"
}

kanaq-cd-widget() {
    kanaq-cd
    zle reset-prompt
}
zle -N kanaq-cd-widget
