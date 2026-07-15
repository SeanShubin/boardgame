# Run the combat simulator — a clickable window that plays ONE combat encounter by hand, driving the
# pure rules::combat model (two formations, engagement by geometry). Pick an option; the board
# updates; each option shows the solver's verdict and how many lines win vs lose. Mirrors the on-screen
# state to fight-screen.txt in the repo root.
#
# Pass an encounter index (into the catalog's ENCOUNTERS); defaults to the first party encounter.
#   scripts\combat.ps1           # the default (first party) encounter
#   scripts\combat.ps1 4         # encounter #4
#
# Related: scripts\check.ps1 runs the tests; the balance/verification sims are cargo examples in
# deckbound-board (regions_diagonal = the 4/4-solos-4/4-corners ladder; regions_tune_corners = the
# warband tuner; explore = a text decision-tree walker).
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    cargo run --release -p boardgame --example fight -- @args
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
    Pop-Location
}
