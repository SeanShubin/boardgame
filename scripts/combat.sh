#!/usr/bin/env bash
# Run the combat simulator - a clickable window that plays ONE combat encounter by hand, driving the
# pure `rules::combat` model (the round-sequence model: two declare waves - acts, then catches -
# resolved through the Inner/Crossing/Outer rings). Pick an option; the board updates; each option
# shows the solver's verdict and how many lines win vs lose. Mirrors the on-screen state to
# `fight-screen.txt`, and the WHOLE running log to `fight-log.txt`, in the repo root.
#
# Pass an encounter index (into the catalog's ENCOUNTERS); defaults to the first party encounter. A SOLO
# encounter (0-3) is fielded by exactly one kit - the keystone's counter by default; pass a kit name as a
# second arg to override. A party encounter (4-8) always musters the full roster (7 = The Hollow
# Rampart, the raid lesson - the best tour of crossings, catches and withdrawal).
#   scripts/combat.sh            # the default (first party) encounter
#   scripts/combat.sh 3          # solo #3 (The Brood), fielded by its counter (Bastion)
#   scripts/combat.sh 3 Raider   # solo #3, fielded by the Raider instead
#   scripts/combat.sh 7          # The Hollow Rampart: raid past the Walls to the Sniper
#
# Related: scripts/check.sh runs the tests; the balance/verification sims are cargo examples in
# deckbound-board (regions_diagonal = the 4/4-solos + 5/5-corners ladder; regions_tune_corners = the
# warband tuner; explore = a text decision-tree walker).
set -euo pipefail
cd "$(dirname "$0")/.."
exec cargo run --release -p boardgame --example fight -- "$@"
