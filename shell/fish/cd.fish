# fish completion adapter for `kanaq complete`.
#
# Install:
#   cp cd.fish ~/.config/fish/completions/cd.fish
#
# Make sure the kanaq binary is on $PATH, or set $KANAQ_BIN to its
# absolute path before fish loads this file
# (e.g. in ~/.config/fish/config.fish).
#
# Registration is skipped entirely if the binary can't be found, so a stale
# install never breaks `cd` -- fish's builtin directory completion for `cd`
# is untouched in that case.

function __kanaq_complete_cd
    set -l bin $KANAQ_BIN
    if test -z "$bin"
        set bin kanaq
    end
    $bin complete --cwd (pwd) --kind dir -- (commandline -ct) 2>/dev/null
end

set -l kanaq_bin $KANAQ_BIN
if test -z "$kanaq_bin"
    set kanaq_bin kanaq
end

if command -q $kanaq_bin
    complete -c cd -f -a '(__kanaq_complete_cd)'
end
