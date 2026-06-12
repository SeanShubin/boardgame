#!/usr/bin/env bash
# Lint the whole workspace with clippy, treating warnings as errors.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo clippy --workspace --all-targets "$@" -- -D warnings
