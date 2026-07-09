//! Load a roster of character sheets from a RON file.

use crate::Character;
use std::path::Path;

/// Parse a `Vec<Character>` from a RON file.
pub fn load(path: impl AsRef<Path>) -> Result<Vec<Character>, String> {
    let path = path.as_ref();
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    ron::from_str::<Vec<Character>>(&text).map_err(|e| format!("parsing {}: {e}", path.display()))
}
