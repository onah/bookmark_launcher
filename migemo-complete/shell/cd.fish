# fish completion adapter for migemo-complete.
#
# Install:
#   cp cd.fish ~/.config/fish/completions/cd.fish
#
# Make sure the migemo-complete binary is on $PATH, or set
# $MIGEMO_COMPLETE_BIN to its absolute path before fish loads this file
# (e.g. in ~/.config/fish/config.fish).
#
# Registration is skipped entirely if the binary can't be found, so a stale
# install never breaks `cd` -- fish's builtin directory completion for `cd`
# is untouched in that case.

function __migemo_complete_cd
    set -l bin $MIGEMO_COMPLETE_BIN
    if test -z "$bin"
        set bin migemo-complete
    end
    $bin --cwd (pwd) --kind dir -- (commandline -ct) 2>/dev/null
end

set -l migemo_bin $MIGEMO_COMPLETE_BIN
if test -z "$migemo_bin"
    set migemo_bin migemo-complete
end

if command -q $migemo_bin
    complete -c cd -f -a '(__migemo_complete_cd)'
end
