#!/usr/bin/env bash
# Run the card-table app — the product (the `boardgame` bin). Extra args pass
# through to cargo, e.g. scripts/run.sh --release
set -euo pipefail
cd "$(dirname "$0")/.."
cargo run -p boardgame "$@"
