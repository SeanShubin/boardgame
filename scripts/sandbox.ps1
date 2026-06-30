# Launch the card-table renderer in **sandbox mode** — the renderer core driven by a hand-built deck
# table, with no game wired in. For prototyping UI features in isolation.
#
# Usage: scripts\sandbox.ps1 [extra cargo args]      e.g. scripts\sandbox.ps1 --release
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
cargo run -p cardtable --example sandbox @args
