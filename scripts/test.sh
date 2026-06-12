#!/usr/bin/env bash
# Run the test suite. By default the whole workspace; extra args pass through,
# e.g. scripts/test.sh -p engine
set -euo pipefail
cd "$(dirname "$0")/.."
cargo test --workspace "$@"
