//! Guard: Rust *code* stays ASCII across the crates that render to the screen or write the debug logs (the
//! "app path"). Typographic Unicode in a string literal - arrows, middle dots, bullets, em-dashes, ellipses -
//! shows up as unprintable glyphs in the Bevy UI and in the log files, and it kept sneaking back in. So code
//! and its string literals stay ASCII: use "->" not an arrow, "--" not an em-dash, "x" not a multiply sign,
//! "*" not a bullet, "..." not an ellipsis. Doc comments are exempt.
//!
//! The `.githooks/pre-commit` gate blocks newly-added non-ASCII at commit time; this test is the CI / `cargo
//! test` backstop that catches anything that slips past (for example a `--no-verify` commit).

use std::path::{Path, PathBuf};

/// The crates whose code reaches the screen or the log files. `deckbound` balance/example tooling prints to a
/// terminal, not the app, so it is out of scope here (the commit hook still guards new additions everywhere).
const APP_CRATES: &[&str] = &[
    "boardgame",
    "cardtable",
    "cardtable-model",
    "cardtable-combat",
    "deckbound-cardtable",
    "contract",
    "engine",
];

#[test]
fn app_code_is_ascii_only() {
    let crates = workspace_root().join("crates");
    let mut offenders = Vec::new();
    for name in APP_CRATES {
        collect_rs(&crates.join(name), &mut |path| scan(path, &mut offenders));
    }
    assert!(
        offenders.is_empty(),
        "non-ASCII found in code - use ASCII in strings/logs (-> -- x * ...), not typographic Unicode:\n{}",
        offenders.join("\n")
    );
}

/// Flag any non-ASCII character outside a `//` comment (a crude but effective split: Rust identifiers are
/// ASCII, so non-ASCII in code is a string literal / char literal). Comment text is exempt.
fn scan(path: &Path, offenders: &mut Vec<String>) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    for (i, line) in text.lines().enumerate() {
        let code = line.split("//").next().unwrap_or("");
        if code.chars().any(|c| !c.is_ascii()) {
            offenders.push(format!("{}:{}: {}", path.display(), i + 1, line.trim()));
        }
    }
}

fn collect_rs(dir: &Path, f: &mut dyn FnMut(&Path)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs(&path, f);
        } else if path.extension().is_some_and(|x| x == "rs") {
            f(&path);
        }
    }
}

/// The workspace root: `CARGO_MANIFEST_DIR` is `.../crates/cardtable`, so the root is two directories up.
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/cardtable has a workspace root two levels up")
        .to_path_buf()
}
