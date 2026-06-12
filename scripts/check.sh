#!/usr/bin/env bash
# Fast type-check of the whole workspace without producing binaries.
# Extra args pass through to cargo.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo check --workspace "$@"
