# Run the Deckbound sample scenario — the reference game (Deckbound + tabletop renderer), kept for
# compatibility and reference. Not the main product; that is scripts\run.ps1 (the card-table app).
#
# Usage: scripts\sample.ps1 [extra cargo args]     e.g. scripts\sample.ps1 --features cardtable
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    cargo run -p deckbound-sample @args
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
    Pop-Location
}
