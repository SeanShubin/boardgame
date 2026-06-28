//! Data-driven balance runner. Loads a **level** (roster stats + which rules are on + party sizes) from
//! a RON file at runtime and prints its composition matrix — so iterating on numbers needs no rebuild:
//! edit the `.ron`, re-run, read the matrix.
//!
//! Usage:
//!   cargo run -p deckbound --example balance                       # defaults to data/balance/level-1.ron
//!   cargo run -p deckbound --example balance -- path/to/level.ron  # any level file
//!
//! The matrix is the solver-optimized player (every composition) vs the deterministic AI (every
//! composition), per size, under the level's ruleset. Each cell is W (proven win) / L (proven loss) /
//! ? (budget-limited). The header records which rules were enabled — the result's provenance.

use std::path::PathBuf;

use deckbound::balance::{Level, run_level};

fn main() {
    let path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            // Default relative to this crate, so it works from the workspace root or the crate dir.
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/balance/level-1.ron")
        });

    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read level file {}: {e}", path.display()));
    let level: Level = ron::from_str(&text)
        .unwrap_or_else(|e| panic!("cannot parse level file {}: {e}", path.display()));

    print!("{}", run_level(&level));
}
