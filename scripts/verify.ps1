# Run the same gauntlet CI runs, before you push: format check, clippy on the
# logic crates, the test suite, and a build of the launcher. Stops at the first
# failure. This is the one to run when you want a single green light.
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    Write-Host "==> Checking formatting"
    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    Write-Host "==> Clippy (engine, deckbound)"
    cargo clippy -p engine -p deckbound -- -D warnings
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    Write-Host "==> Tests"
    cargo test --workspace
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    Write-Host "==> Building boardgame"
    cargo build -p boardgame
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    Write-Host ""
    Write-Host "All checks passed."
} finally {
    Pop-Location
}
