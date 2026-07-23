#!/usr/bin/env bash
# Open the selection-states demo: a window showing the four attention states a card takes during a
# source -> action -> target gesture - background / in-the-selection / completing (the rotating ring) /
# selectable - each drawn by the REAL combat tile code (draw_scene_tile + the real ring animation), so it
# looks exactly as it does in a fight. All four are on screen at once. A window opens - close it when done.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo run -p cardtable --example selection_states "$@"
