# PowerShell functions for kanaq-bm: pick a bookmark with fzf (searching
# titles, tags, and urls by romaji through `kanaq filter`) and open it, or
# open the bookmark file in an editor. See kanaq-bm.zsh for the same thing
# in zsh.
#
# Install:
#   1. Make sure `fzf`, `kanaq`, and `kanaq-bm` are on $env:PATH (or set
#      $env:KANAQ_BIN / $env:KANAQ_BM_BIN to their absolute paths).
#   2. Dot-source this file from your $PROFILE and give the functions short
#      aliases, e.g.:
#        . /path/to/kanaq-bm.ps1
#        Set-Alias bm Invoke-KanaqBmPick
#        Set-Alias bme Invoke-KanaqBmEdit
#
# Usage:
#   Invoke-KanaqBmPick [query...]  pick a bookmark and open it in the
#       browser. With a query, fzf opens already filtered by it, and if
#       exactly one bookmark matches, it is opened without opening fzf.
#   Invoke-KanaqBmEdit             open the bookmark file in $env:EDITOR
#       (notepad if unset)
#
# Add and remove bookmarks with `kanaq-bm add <url>` / `kanaq-bm remove <url>`.

function Invoke-KanaqBmPick {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Query)

    $queryText = $Query -join ' '
    $kanaqBin = if ($env:KANAQ_BIN) { $env:KANAQ_BIN } else { 'kanaq' }
    $bmBin = if ($env:KANAQ_BM_BIN) { $env:KANAQ_BM_BIN } else { 'kanaq-bm' }

    foreach ($cmd in @($kanaqBin, $bmBin, 'fzf')) {
        if (-not (Get-Command $cmd -ErrorAction SilentlyContinue)) {
            Write-Error "kanaq-bm-pick: $cmd not found on `$env:PATH (see shell/powershell/kanaq-bm.ps1)"
            return
        }
    }

    $tmp = New-TemporaryFile
    $selected = $null
    try {
        # One `title<TAB>tags<TAB>url` line per bookmark, most opened first.
        & $bmBin list | Set-Content -LiteralPath $tmp.FullName
        if ($LASTEXITCODE -ne 0) {
            return
        }

        if ($queryText) {
            $hits = @(Get-Content -LiteralPath $tmp.FullName | & $kanaqBin filter -- $queryText)
            if ($hits.Count -eq 1) {
                $selected = $hits[0]
            }
        }

        if (-not $selected) {
            # Same scheme as Invoke-KanaqCd: the initial list is
            # pre-filtered by the query, and edits to the query re-rank the
            # full list via `reload`.
            $reloadCmd = "change:reload($kanaqBin filter --highlight -- {q} < `"$($tmp.FullName)`")"
            $fzfArgs = @('--disabled', '--ansi', '--height', '40%', '--reverse', '--prompt', 'bm> ', '--bind', $reloadCmd)
            # Only pass --query when there is one (see Invoke-KanaqCd).
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
        # The url is the last tab-separated field.
        & $bmBin open ($selected -split "`t")[-1]
    }
}

function Invoke-KanaqBmEdit {
    $bmBin = if ($env:KANAQ_BM_BIN) { $env:KANAQ_BM_BIN } else { 'kanaq-bm' }
    $editor = if ($env:EDITOR) { $env:EDITOR } else { 'notepad' }
    $file = & $bmBin path
    if ($LASTEXITCODE -eq 0) {
        & $editor $file
    }
}
