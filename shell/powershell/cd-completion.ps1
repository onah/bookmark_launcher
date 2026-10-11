# PowerShell completion adapter for `kanaq complete`.
#
# Install: add this line to your $PROFILE:
#   . /path/to/cd-completion.ps1
#
# Set $env:KANAQ_BIN to an absolute path if the binary isn't on
# $env:PATH. Registration is skipped entirely if the binary can't be found,
# so a stale install never breaks `cd` / `Set-Location`.

$KanaqBin = if ($env:KANAQ_BIN) { $env:KANAQ_BIN } else { 'kanaq' }

if (Get-Command $KanaqBin -ErrorAction SilentlyContinue) {
    Register-ArgumentCompleter -Native -CommandName cd, Set-Location -ScriptBlock {
        param($wordToComplete, $commandAst, $cursorPosition)

        $cwd = (Get-Location).ProviderPath
        $lines = & $KanaqBin complete --cwd $cwd --kind dir -- $wordToComplete 2>$null

        foreach ($line in $lines) {
            $text = if ($line -match '\s') { "'$line'" } else { $line }
            [System.Management.Automation.CompletionResult]::new($text, $line, 'ParameterValue', $line)
        }
    }
}
