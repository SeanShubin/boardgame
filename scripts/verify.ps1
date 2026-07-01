# The pre-push gauntlet: format check, clippy on the logic crates, the whole
# test suite, and a build of the card-table app (the product). Stops at the first
# failure. This is the one to run when you want a single green light.
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    Write-Host "==> Checking formatting"
    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    Write-Host "==> Clippy (logic + card-table product)"
    cargo clippy -p engine -p deckbound -p cardtable-model -p cardtable -p boardgame -- -D warnings
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    Write-Host "==> Tests"
    cargo test --workspace
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    Write-Host "==> Building the card-table app (boardgame)"
    cargo build -p boardgame
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    Write-Host ""
    Write-Host "All checks passed."
} finally {
    Pop-Location
}
