//! Regenerate `docs/games/deckbound/reference/combat-targets.md` — the denormalized target table.
//!
//! The schedule, joined with the range rule and the screen rule, so "who can hit whom, when?" is answerable by
//! looking rather than by holding three files in your head. Generated, never hand-written: a reference that
//! can drift from the engine is worse than none, and `targets::tests::the_committed_table_is_current` fails if
//! this has not been run.
//!
//! Run: `cargo run -p deckbound-board --example targets`

use std::fs;
use std::path::Path;

fn main() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/games/deckbound/reference/combat-targets.md");
    fs::create_dir_all(path.parent().expect("has a parent")).expect("create reference dir");
    fs::write(&path, deckbound_board::targets::table_md()).expect("write the target table");
    println!("wrote {}", path.display());
}
