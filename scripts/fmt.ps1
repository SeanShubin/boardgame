# Format all code in place. Pass --check to verify without writing,
# e.g. scripts\fmt.ps1 --check
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    cargo fmt --all @args
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
    Pop-Location
}
