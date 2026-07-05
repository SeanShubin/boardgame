//! Pads all markdown tables in `.md` files so columns align in monospace editors.
//!
//! Run with: `cargo run -p mdtable --example pad_tables` (or `scripts/pad-tables`).
//!
//! Recursively scans the current directory for `.md` files, finds markdown tables,
//! and pads each cell so pipes align. Skips `.git`, `target`, `node_modules`, and
//! any dot-prefixed directory. Only writes files that actually change.
//!
//! The padding logic itself lives in [`mdtable::pad_tables`] so generated docs
//! (e.g. deckbound's handbook) can emit tables already in this exact form.

use mdtable::pad_tables;
use std::fs;
use std::path::Path;

fn main() {
    let mut changed_files = Vec::new();
    visit_dir(Path::new("."), &mut changed_files);

    if changed_files.is_empty() {
        println!("All tables are already padded.");
    } else {
        for path in &changed_files {
            println!("  Padded: {}", path);
        }
        println!("\nChanged {} file(s).", changed_files.len());
    }
}

fn visit_dir(dir: &Path, changed: &mut Vec<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            visit_dir(&path, changed);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            if process_file(&path) {
                changed.push(path.display().to_string());
            }
        }
    }
}

/// Pad one file in place, returning whether its contents changed.
fn process_file(path: &Path) -> bool {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let padded = pad_tables(&content);
    if padded != content {
        fs::write(path, padded).expect("Failed to write file");
        true
    } else {
        false
    }
}
