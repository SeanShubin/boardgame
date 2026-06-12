# Build the whole workspace (debug). Extra args pass through to cargo,
# e.g. scripts\build.ps1 --release
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    cargo build @args
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
    Pop-Location
}
