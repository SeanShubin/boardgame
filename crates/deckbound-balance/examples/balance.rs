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

use deckbound_balance::{
    DuelLocks, Level, RegionLocks, duel_locks, duel_locks_report, region_locks_report, run_level,
};

fn main() {
    let path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            // Default relative to this crate, so it works from the workspace root or the crate dir.
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/balance/level-1.ron")
        });

    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read balance file {}: {e}", path.display()));

    // Three file shapes share this runner: a composition-matrix `Level` (`level-N.ron`), a `DuelLocks`
    // set (`duel-locks.ron`), and a `RegionLocks` set (`region-locks.ron`). Try each in turn.
    if let Ok(level) = ron::from_str::<Level>(&text) {
        print!("{}", run_level(&level));
    } else if let Ok(locks) = ron::from_str::<DuelLocks>(&text) {
        print!("{}", duel_locks_report(&locks));
    } else {
        let regions: RegionLocks = ron::from_str(&text)
            .unwrap_or_else(|e| panic!("cannot parse balance file {}: {e}", path.display()));
        const BUDGET: u64 = 2_000_000;
        print!(
            "{}",
            region_locks_report(&regions, &duel_locks().kits, BUDGET)
        );
    }
}
