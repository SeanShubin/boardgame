# Open the card gallery / text audit: renders every card in the sample table at all
# three sizes, prints an overflow report to the terminal, and frames overflowing cards
# in red. A window opens — close it when done. Use it when finalizing card text.
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    cargo run -p cardtable --example card_gallery @args
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
    Pop-Location
}
