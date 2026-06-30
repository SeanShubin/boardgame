#!/usr/bin/env bash
# Launch the card-table renderer in **sandbox mode** — the renderer core driven by a hand-built deck
# table, with no game wired in. For prototyping UI features in isolation.
#
# Usage: scripts/sandbox.sh [extra cargo args]      e.g. scripts/sandbox.sh --release
set -euo pipefail
cd "$(dirname "$0")/.."
exec cargo run -p cardtable --example sandbox "$@"
