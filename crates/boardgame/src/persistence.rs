//! Persist the card-table [`Board`] as **RON** — to the browser's `localStorage` on the web, and to a
//! file in the OS data directory natively (same serialization, two backends). The session **persists**
//! across launches on both platforms; the player resets with the **Start Over** button.
//!
//! Loading **reconciles** the save against the pristine table the *current* code builds
//! ([`cardtable_model::reconcile`]): the save contributes the player's arrangement — where every card
//! and pile sits, facing, sizes, stack splits, session records — while all card *content* (rules text,
//! details, which cards exist at all) comes from the running build. So a session survives a software
//! update without ever showing cards the updated code could not have produced. A save that can't be
//! parsed at all (an incompatible struct change) falls back to a fresh table rather than crashing.
//!
//! [`encode`] + [`write`] are split so the caller can dedupe (only write when the RON changed), for the
//! autosave loop.

use cardtable_model::Board;

/// The `localStorage` key (web) and file stem (native).
const KEY: &str = "boardgame.tableau";

/// The RON payload. (Older saves also carried a `fingerprint` field; serde skips unknown fields, so
/// they still load — and then reconcile like any other save.)
#[derive(serde::Serialize, serde::Deserialize)]
struct Save {
    tableau: Board,
}

/// Serialize the tableau to a RON string, or `None` on failure.
pub fn encode(tableau: &Board) -> Option<String> {
    ron::to_string(&Save {
        tableau: tableau.clone(),
    })
    .ok()
}

/// Parse a RON string back to a tableau, or `None` if it can't be parsed (an incompatible struct
/// change) — the caller then falls back to a fresh table rather than crashing.
fn decode(text: &str) -> Option<Board> {
    ron::from_str::<Save>(text).ok().map(|save| save.tableau)
}

/// Load the saved session — the saved **arrangement** reconciled onto the pristine table the current
/// code builds, so content is always this build's — or `None` if there is no save (or it can't be
/// read / parsed).
pub fn load() -> Option<Board> {
    backend::read()
        .and_then(|text| decode(&text))
        .map(|saved| cardtable_model::reconcile(deckbound_board::sample_table(), &saved))
}

/// Persist an already-encoded RON string.
pub fn write(text: &str) {
    backend::write(text);
}

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use super::KEY;
    use std::path::PathBuf;

    /// `<data_dir>/boardgame/boardgame.tableau.ron` — the idiomatic per-OS save location.
    fn save_path() -> Option<PathBuf> {
        let dirs = directories::ProjectDirs::from("", "", "boardgame")?;
        Some(dirs.data_dir().join(format!("{KEY}.ron")))
    }

    pub fn read() -> Option<String> {
        std::fs::read_to_string(save_path()?).ok()
    }

    pub fn write(text: &str) {
        let Some(path) = save_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, text);
    }
}

#[cfg(target_arch = "wasm32")]
mod backend {
    use super::KEY;

    /// The window's `localStorage`, if the browser exposes it.
    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }

    pub fn read() -> Option<String> {
        storage()?.get_item(KEY).ok()?
    }

    pub fn write(text: &str) {
        if let Some(storage) = storage() {
            let _ = storage.set_item(KEY, text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A save round-trips through the RON payload.
    #[test]
    fn encode_decode_round_trips() {
        let board = deckbound_board::sample_table();
        let text = encode(&board).expect("serializes");
        let back = decode(&text).expect("parses");
        assert_eq!(back.card_count(), board.card_count());
    }

    /// Saves written by the previous format — which stamped a `fingerprint` field before the tableau —
    /// still parse: serde skips the unknown field, so an existing session survives this update too.
    #[test]
    fn old_saves_with_a_fingerprint_field_still_decode() {
        let board = deckbound_board::sample_table();
        let text = encode(&board).expect("serializes");
        let old_format = format!("(fingerprint:12345,{}", &text[1..]);
        let back = decode(&old_format).expect("old format still parses");
        assert_eq!(back.card_count(), board.card_count());
    }
}
