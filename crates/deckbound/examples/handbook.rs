//! Generate the player-facing reference docs (card library + rules encyclopedia) to
//! `docs/games/deckbound/reference/`. Run after any change to the cards (booklet) or the Spec terms:
//!
//! ```text
//! cargo run -p deckbound --example handbook
//! ```
//!
//! The golden tests in `deckbound::handbook` fail the build if the committed docs drift from the
//! generated content, so this is the one command that keeps the reference in sync.

use std::fs;
use std::path::Path;

use deckbound::handbook::{card_library_md, rules_reference_md};

fn main() {
    // The crate sits at `crates/deckbound`; the reference lives at the repo's docs tree.
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/games/deckbound/reference");
    fs::create_dir_all(&dir).expect("create reference dir");

    let docs = [
        ("card-library.md", card_library_md()),
        ("rules-reference.md", rules_reference_md()),
        ("combat-phases.md", deckbound::rules::appendix()),
    ];
    for (name, body) in docs {
        let path = dir.join(name);
        fs::write(&path, body).expect("write reference doc");
        println!("wrote {}", path.display());
    }
}
