#!/usr/bin/env bash
# Run the game (the boardgame launcher). Extra args pass through to cargo,
# e.g. scripts/run.sh --release
set -euo pipefail
cd "$(dirname "$0")/.."
cargo run -p boardgame "$@"
