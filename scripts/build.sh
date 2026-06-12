#!/usr/bin/env bash
# Build the whole workspace (debug). Extra args pass through to cargo,
# e.g. scripts/build.sh --release
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build "$@"
