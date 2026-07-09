//! Persist the card-table [`Tableau`] as **RON** — to the browser's `localStorage` on the web, and to a
//! file in the OS data directory natively (same serialization, two backends). The session **persists**
//! across launches on both platforms; the player resets with the **Start Over** button, so a save is kept
//! as long as it still deserializes (a fingerprint of the pristine shape is recorded for reference but no
//! longer gates the load). A save that can't be parsed — an incompatible struct change — falls back to a
//! fresh table rather than crashing. [`encode`] + [`write`] are split so the caller can dedupe (only write
//! when the RON changed), for the autosave loop.

use cardtable_model::Tableau;

/// The `localStorage` key (web) and file stem (native).
const KEY: &str = "boardgame.tableau";

/// The RON payload — the tableau plus the pristine-shape [`fingerprint`] it was written against.
#[derive(serde::Serialize, serde::Deserialize)]
struct Save {
    fingerprint: u64,
    tableau: Tableau,
}

/// A fingerprint of the pristine [`sample_table`](cardtable_model::sample_table) shape. It changes
/// whenever the fixture changes, so a save written against a different pristine is treated as stale.
/// Computed once and cached; `DefaultHasher`'s keys are fixed, so it is stable across builds.
fn fingerprint() -> u64 {
    use std::hash::{Hash, Hasher};
    static FP: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *FP.get_or_init(|| {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        ron::to_string(&cardtable_model::sample_table())
            .unwrap_or_default()
            .hash(&mut hasher);
        hasher.finish()
    })
}

/// Serialize the tableau to a RON string (stamped with the pristine fingerprint), or `None` on failure.
pub fn encode(tableau: &Tableau) -> Option<String> {
    ron::to_string(&Save {
        fingerprint: fingerprint(),
        tableau: tableau.clone(),
    })
    .ok()
}

/// Parse a RON string back to a tableau. The session **persists** across launches (and across builds):
/// the fingerprint is recorded but no longer gates the load, so a save is kept as long as it still
/// deserializes — use **Start Over** to reset. A save that can't be parsed (an incompatible struct
/// change) falls back to a fresh table rather than crashing.
fn decode(text: &str) -> Option<Tableau> {
    ron::from_str::<Save>(text).ok().map(|save| save.tableau)
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
