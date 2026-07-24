# Run the card-table app in DEBUG mode: a clean slate AND the doom oracle turned on.
# Two things in one: it discards the persisted session by deleting the active save
# (the app then loads the fresh sample table, exactly like "Start Over"), and it sets
# BOARDGAME_ORACLE so the System deck's "Doom oracle" toggle opens ON - the combat
# foresight badges (winnable / doomed) are visible from launch. A plain scripts\run.ps1
# opens with the oracle off; you can also flip the toggle in-app at any time. Your own
# ".bak" backups next to the save are left alone. Extra args pass through to cargo, e.g.
# scripts\run-debug.ps1 --release
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
    $env:BOARDGAME_ORACLE = "1"
    cargo run -p boardgame @args
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
    Pop-Location
}
