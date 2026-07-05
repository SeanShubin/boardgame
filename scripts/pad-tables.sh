#!/usr/bin/env bash
# Align all markdown tables in the repo so columns line up in monospace editors.
# Uses the pad_tables example which recursively finds .md files.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo run -p mdtable --example pad_tables "$@"
