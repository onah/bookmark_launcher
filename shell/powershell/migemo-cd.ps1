# PowerShell widget: fuzzy/migemo search directories under the current
# location and cd into the one you pick. The migemo-aware analog of fzf's
# own Alt+C (fzf-cd-widget); see migemo-cd.zsh for the same thing in zsh.
#
# Install:
#   1. Make sure `fzf` and `kanaq` are on $env:PATH (or set
#      $env:KANAQ_BIN to kanaq's absolute path).
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
    $kanaqBin = if ($env:KANAQ_BIN) { $env:KANAQ_BIN } else { 'kanaq' }

    if (-not (Get-Command $kanaqBin -ErrorAction SilentlyContinue)) {
        Write-Error "migemo-cd: $kanaqBin not found on `$env:PATH (see shell/powershell/migemo-cd.ps1)"
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
        # `kanaq filter`, which is what keeps each keystroke fast.
        Get-MigemoCdCandidate -ExcludeDirs $excludeDirs | Set-Content -LiteralPath $tmp.FullName

        $reloadCmd = "change:reload($kanaqBin filter --highlight -- {q} < `"$($tmp.FullName)`")"
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
