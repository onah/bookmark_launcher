# PowerShell widget: fuzzy/migemo search directories under the current
# location and cd into the one you pick. The migemo-aware analog of fzf's
# own Alt+C (fzf-cd-widget); see kanaq-cd.zsh for the same thing in zsh.
#
# Install:
#   1. Make sure `fzf` and `kanaq` are on $env:PATH (or set
#      $env:KANAQ_BIN to kanaq's absolute path).
#   2. Dot-source this file from your $PROFILE:
#        . /path/to/kanaq-cd.ps1
#   3. Optionally give it a short alias and/or bind it to a key with
#      PSReadLine, e.g.:
#        Set-Alias j Invoke-KanaqCd
#        Set-PSReadLineKeyHandler -Chord 'Alt+c' -ScriptBlock {
#            [Microsoft.PowerShell.PSConsoleReadLine]::RevertLine()
#            Invoke-KanaqCd
#            [Microsoft.PowerShell.PSConsoleReadLine]::InvokePrompt()
#        }
#
# Usage: `Invoke-KanaqCd [query...]`. With a query, fzf opens already
# filtered by it, and if exactly one directory matches, it jumps there
# without opening fzf at all (like zoxide's `z foo`).
#
# Set $env:KANAQ_CD_EXCLUDE_DIRS to override which directory names are
# pruned from the walk (semicolon-separated, default ".git;node_modules").
# A pruned directory is never descended into, same as `find -prune` in the
# zsh version -- so excluding node_modules also skips walking everything
# underneath it, not just hiding it from the results.

function Get-KanaqCdCandidate {
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

function Invoke-KanaqCd {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Query)

    $queryText = $Query -join ' '
    $kanaqBin = if ($env:KANAQ_BIN) { $env:KANAQ_BIN } else { 'kanaq' }

    if (-not (Get-Command $kanaqBin -ErrorAction SilentlyContinue)) {
        Write-Error "kanaq-cd: $kanaqBin not found on `$env:PATH (see shell/powershell/kanaq-cd.ps1)"
        return
    }
    if (-not (Get-Command fzf -ErrorAction SilentlyContinue)) {
        Write-Error 'kanaq-cd: fzf not found on $env:PATH'
        return
    }

    $excludeDirs = if ($env:KANAQ_CD_EXCLUDE_DIRS) {
        $env:KANAQ_CD_EXCLUDE_DIRS -split ';'
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
        Get-KanaqCdCandidate -ExcludeDirs $excludeDirs | Set-Content -LiteralPath $tmp.FullName

        if ($queryText) {
            $hits = @(Get-Content -LiteralPath $tmp.FullName | & $kanaqBin filter -- $queryText)
            if ($hits.Count -eq 1) {
                $selected = $hits[0]
            }
        }

        if (-not $selected) {
            # The initial list is pre-filtered by the query (an empty query
            # passes every line through); edits to the query re-rank the
            # full snapshot via `reload`.
            $reloadCmd = "change:reload($kanaqBin filter --highlight -- {q} < `"$($tmp.FullName)`")"
            $fzfArgs = @('--disabled', '--ansi', '--height', '40%', '--reverse', '--prompt', 'cd> ', '--bind', $reloadCmd)
            # Only pass --query when there is one: older PowerShell drops
            # empty-string arguments to native commands, which would shift
            # the next argument into --query's value.
            if ($queryText) {
                $fzfArgs += @('--query', $queryText)
            }
            $selected = Get-Content -LiteralPath $tmp.FullName |
                & $kanaqBin filter --highlight -- $queryText |
                fzf @fzfArgs
        }
    }
    finally {
        Remove-Item -LiteralPath $tmp.FullName -ErrorAction SilentlyContinue
    }

    if ($selected) {
        Set-Location -LiteralPath $selected
    }
}
