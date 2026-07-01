# Run the card-table app — the product (the boardgame bin). Extra args pass
# through to cargo, e.g. scripts\run.ps1 --release
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    cargo run -p boardgame @args
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
    Pop-Location
}
