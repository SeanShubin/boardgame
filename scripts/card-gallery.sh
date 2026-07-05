#!/usr/bin/env bash
# Open the card gallery / text audit: renders every card in the sample table at all
# three sizes, prints an overflow report to the terminal, and frames overflowing cards
# in red. A window opens — close it when done. Use it when finalizing card text.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo run -p cardtable --example card_gallery "$@"
