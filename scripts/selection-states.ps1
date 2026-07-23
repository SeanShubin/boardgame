# Open the selection-states demo: a window showing the four attention states a card takes during a
# source -> action -> target gesture - background / in-the-selection / completing (the rotating ring) /
# selectable - each drawn by the REAL combat tile code (draw_scene_tile + the real ring animation), so it
# looks exactly as it does in a fight. All four are on screen at once. A window opens - close it when done.
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    cargo run -p cardtable --example selection_states @args
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
    Pop-Location
}
