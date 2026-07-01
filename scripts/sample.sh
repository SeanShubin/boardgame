#!/usr/bin/env bash
# Run the Deckbound sample scenario — the reference game (Deckbound + tabletop renderer), kept for
# compatibility and reference. Not the main product; that is `scripts/run.sh` (the card-table app).
#
# Usage: scripts/sample.sh [extra cargo args]      e.g. scripts/sample.sh --features cardtable
set -euo pipefail
cd "$(dirname "$0")/.."
exec cargo run -p deckbound-sample "$@"
