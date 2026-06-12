#!/usr/bin/env bash
# Format all code in place. Pass --check to verify without writing,
# e.g. scripts/fmt.sh --check
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --all "$@"
