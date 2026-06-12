# Lint the whole workspace with clippy, treating warnings as errors.
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    cargo clippy --workspace --all-targets @args -- -D warnings
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
    Pop-Location
}
