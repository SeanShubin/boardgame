#!/usr/bin/env bash
# The pre-push gauntlet: format check, clippy on the logic crates, the whole
# test suite, and a build of the card-table app (the product). Stops at the first
# failure. This is the one to run when you want a single green light.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> Checking formatting"
cargo fmt --all -- --check

echo "==> Clippy (logic + card-table product)"
cargo clippy -p engine -p deckbound -p cardtable-model -p cardtable -p boardgame -- -D warnings

echo "==> Tests"
cargo test --workspace

echo "==> Building the card-table app (boardgame)"
cargo build -p boardgame

echo
echo "All checks passed."
