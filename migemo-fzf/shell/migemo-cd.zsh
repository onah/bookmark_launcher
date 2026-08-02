# zsh widget: fuzzy/migemo search directories under $PWD and cd into the
# one you pick. The migemo-aware analog of fzf's own Alt+C (fzf-cd-widget).
#
# Install:
#   1. Make sure `fzf` and `migemo-fzf` are on $PATH (or set
#      MIGEMO_FZF_BIN to migemo-fzf's absolute path).
#   2. source this file from your .zshrc:
#        source /path/to/migemo-cd.zsh
#   3. Optionally bind it to a key, e.g. to reuse fzf's own Alt+C slot
#      (source this file *after* fzf's shell integration so this binding
#      wins):
#        bindkey '\ec' migemo-cd-widget
#
# Set MIGEMO_CD_FIND_OPTS to override which directories are listed (default
# excludes .git and node_modules); it's passed to `find` after `.`, e.g.:
#   MIGEMO_CD_FIND_OPTS='( -name .git -o -name .venv ) -prune -o -type d -print'

migemo-cd() {
    local bin="${MIGEMO_FZF_BIN:-migemo-fzf}"
    if ! command -v -- "$bin" >/dev/null 2>&1; then
        echo "migemo-cd: $bin not found on \$PATH (see migemo-fzf/shell/migemo-cd.zsh)" >&2
        return 1
    fi
    if ! command -v -- fzf >/dev/null 2>&1; then
        echo "migemo-cd: fzf not found on \$PATH" >&2
        return 1
    fi

    local tmp
    tmp=$(mktemp) || return 1

    # Walk the tree once per invocation, not once per keystroke -- fzf's
    # `reload` below just re-ranks this static snapshot through migemo-fzf,
    # which is what keeps each keystroke fast.
    local -a find_opts
    if [[ -n "$MIGEMO_CD_FIND_OPTS" ]]; then
        find_opts=(${(z)MIGEMO_CD_FIND_OPTS})
    else
        find_opts=(\( -name .git -o -name node_modules \) -prune -o -type d -print)
    fi
    find . "${find_opts[@]}" 2>/dev/null | sed -e 's|^\./||' -e '/^\.$/d' > "$tmp"

    local dir
    dir=$(fzf --disabled --ansi --height 40% --reverse --prompt 'cd> ' \
        --bind "change:reload($bin --highlight -- {q} < $tmp)" \
        < "$tmp")
    local ret=$?
    rm -f "$tmp"

    (( ret != 0 )) && return $ret
    [[ -n "$dir" ]] && cd -- "$dir"
}

migemo-cd-widget() {
    migemo-cd
    zle reset-prompt
}
zle -N migemo-cd-widget
