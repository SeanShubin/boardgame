//! Persist the card-table [`Tableau`] as **RON** — to the browser's `localStorage` on the web, and to a
//! file in the OS data directory natively (same serialization, two backends). [`load`] returns `None` on
//! absence or any parse / schema-version mismatch, so a corrupt or outdated save falls back to a fresh
//! table rather than crashing. [`encode`] + [`write`] are split so the caller can dedupe (only write
//! when the RON actually changed), which the autosave loop relies on.

use cardtable_model::Tableau;

/// Bump when the persisted shape changes incompatibly; a save whose `version` differs is discarded.
const SCHEMA_VERSION: u32 = 1;
/// The `localStorage` key (web) and file stem (native).
const KEY: &str = "boardgame.tableau";

/// The versioned RON payload — the tableau plus a schema tag so an old save is rejected, not mis-read.
#[derive(serde::Serialize, serde::Deserialize)]
struct Save {
    version: u32,
    tableau: Tableau,
}

/// Serialize the tableau to a RON string (with the schema version), or `None` if serialization fails.
pub fn encode(tableau: &Tableau) -> Option<String> {
    ron::to_string(&Save {
        version: SCHEMA_VERSION,
        tableau: tableau.clone(),
    })
    .ok()
}

/// Parse a RON string back to a tableau, rejecting a mismatched schema version.
fn decode(text: &str) -> Option<Tableau> {
    let save: Save = ron::from_str(text).ok()?;
    (save.version == SCHEMA_VERSION).then_some(save.tableau)
}

/// Load the saved tableau, or `None` if there is none (or it can't be read / parsed).
pub fn load() -> Option<Tableau> {
    backend::read().and_then(|text| decode(&text))
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
