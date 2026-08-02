# PowerShell widget: fuzzy/migemo search directories under the current
# location and cd into the one you pick. The migemo-aware analog of fzf's
# own Alt+C (fzf-cd-widget); see migemo-cd.zsh for the same thing in zsh.
#
# Install:
#   1. Make sure `fzf` and `migemo-fzf` are on $env:PATH (or set
#      $env:MIGEMO_FZF_BIN to migemo-fzf's absolute path).
#   2. Dot-source this file from your $PROFILE:
#        . /path/to/migemo-cd.ps1
#   3. Optionally bind it to a key with PSReadLine, e.g. Alt+C:
#        Set-PSReadLineKeyHandler -Chord 'Alt+c' -ScriptBlock {
#            [Microsoft.PowerShell.PSConsoleReadLine]::RevertLine()
#            Invoke-MigemoCd
#            [Microsoft.PowerShell.PSConsoleReadLine]::InvokePrompt()
#        }
#
# Set $env:MIGEMO_CD_EXCLUDE_DIRS to override which directory names are
# pruned from the walk (semicolon-separated, default ".git;node_modules").
# A pruned directory is never descended into, same as `find -prune` in the
# zsh version -- so excluding node_modules also skips walking everything
# underneath it, not just hiding it from the results.

function Get-MigemoCdCandidate {
    param([string[]]$ExcludeDirs)

    $root = (Get-Location).ProviderPath
    $stack = [System.Collections.Generic.Stack[System.IO.DirectoryInfo]]::new()
    $stack.Push([System.IO.DirectoryInfo]::new($root))

    while ($stack.Count -gt 0) {
        $dir = $stack.Pop()
        $children = $null
        try {
            $children = $dir.GetDirectories()
        }
        catch {
            continue
        }
        foreach ($child in $children) {
            if ($ExcludeDirs -contains $child.Name) {
                continue
            }
            Write-Output ([System.IO.Path]::GetRelativePath($root, $child.FullName))
            $stack.Push($child)
        }
    }
}

function Invoke-MigemoCd {
    $migemoFzfBin = if ($env:MIGEMO_FZF_BIN) { $env:MIGEMO_FZF_BIN } else { 'migemo-fzf' }

    if (-not (Get-Command $migemoFzfBin -ErrorAction SilentlyContinue)) {
        Write-Error "migemo-cd: $migemoFzfBin not found on `$env:PATH (see migemo-fzf/shell/migemo-cd.ps1)"
        return
    }
    if (-not (Get-Command fzf -ErrorAction SilentlyContinue)) {
        Write-Error 'migemo-cd: fzf not found on $env:PATH'
        return
    }

    $excludeDirs = if ($env:MIGEMO_CD_EXCLUDE_DIRS) {
        $env:MIGEMO_CD_EXCLUDE_DIRS -split ';'
    }
    else {
        @('.git', 'node_modules')
    }

    $tmp = New-TemporaryFile
    $selected = $null
    try {
        # Walk the tree once per invocation, not once per keystroke -- fzf's
        # `reload` below just re-ranks this static snapshot through
        # migemo-fzf, which is what keeps each keystroke fast.
        Get-MigemoCdCandidate -ExcludeDirs $excludeDirs | Set-Content -LiteralPath $tmp.FullName

        $reloadCmd = "change:reload($migemoFzfBin --highlight -- {q} < `"$($tmp.FullName)`")"
        $selected = Get-Content -LiteralPath $tmp.FullName |
            fzf --disabled --ansi --height 40% --reverse --prompt 'cd> ' --bind $reloadCmd
    }
    finally {
        Remove-Item -LiteralPath $tmp.FullName -ErrorAction SilentlyContinue
    }

    if ($selected) {
        Set-Location -LiteralPath $selected
    }
}
