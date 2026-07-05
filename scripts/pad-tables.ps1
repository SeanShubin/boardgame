# Align all markdown tables in the repo so columns line up in monospace editors.
# Uses the pad_tables example which recursively finds .md files.
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    cargo run -p mdtable --example pad_tables @args
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
    Pop-Location
}
