#!/usr/bin/env bash
# Run the same gauntlet CI runs, before you push: format check, clippy on the
# logic crates, the test suite, and a build of the launcher. Stops at the first
# failure. This is the one to run when you want a single green light.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> Checking formatting"
cargo fmt --all -- --check

echo "==> Clippy (engine, treasure-dive)"
cargo clippy -p engine -p treasure-dive -- -D warnings

echo "==> Tests"
cargo test --workspace

echo "==> Building boardgame"
cargo build -p boardgame

echo
echo "All checks passed."
