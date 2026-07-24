# Run the card-table app from a CLEAN SLATE - the same pristine table "Start Over"
# resets to, but from launch. It discards the persisted session by deleting the
# active save (the app then loads the fresh sample table); your own ".bak" backups
# next to it are left alone. Extra args pass through to cargo, e.g.
# scripts\run-fresh.ps1 --release
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# The native save location (see crates/boardgame/src/persistence.rs): the OS data
# dir, which on Windows is %APPDATA%\boardgame\data. Only the active .ron is
# removed - the .bak siblings are the player's own backups.
$save = Join-Path $env:APPDATA "boardgame\data\boardgame.tableau.ron"
if (Test-Path $save) {
    Remove-Item $save -Force
    Write-Host "cleared save: $save"
} else {
    Write-Host "no save to clear (already a clean slate): $save"
}

Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    cargo run -p boardgame @args
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
    Pop-Location
}
