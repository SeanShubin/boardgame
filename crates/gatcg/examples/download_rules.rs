//! Download the rules text of every Grand Archive card into a local directory.
//!
//! Usage:
//!   cargo run -p gatcg --example download_rules
//!   cargo run -p gatcg --example download_rules -- <output-dir>
//!
//! Default output root is `local/grand-archive` (the repo's gitignored scratch
//! area). Writes one Markdown file per printing at `rules/<set-prefix>/<slug>.md`
//! — a **sibling of `images/` with the identical structure**, so a card's rules
//! and its art sit at matching paths (e.g. `rules/P24/verdant-slime-p24.md`
//! beside `images/P24/verdant-slime-p24.jpg`).
//!
//! The rules text lives in the same search-API pages used to enumerate the
//! library, so this makes no per-card requests — it is fast (~45 API calls) and
//! writes thousands of small files.
//!
//! **Safe to interrupt and restart:** existing files are skipped, and each file
//! is written via a `.part` temp then atomically renamed, so a stop never leaves
//! a truncated page. A card's text is identical across its reprints, so it
//! repeats per printing by design to keep the layout paired with `images/`.

use std::path::PathBuf;

use gatcg::{Fetched, fetch_all_records, write_card_text};

fn main() {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("local/grand-archive"));

    if let Err(err) = std::fs::create_dir_all(&root) {
        eprintln!("could not create output dir {}: {err}", root.display());
        std::process::exit(1);
    }

    let agent = gatcg::agent();

    println!("Enumerating card rules from {} ...", gatcg::API_HOST);
    let records = match fetch_all_records(&agent, |page, total, running| {
        println!("  page {page}/{total} — {running} printings so far");
    }) {
        Ok(r) => r,
        Err(err) => {
            eprintln!("enumeration failed: {err}");
            std::process::exit(1);
        }
    };
    println!("Found {} card printings.", records.len());

    let total = records.len();
    let (mut wrote, mut skip, mut fail) = (0usize, 0usize, 0usize);

    for (i, rec) in records.iter().enumerate() {
        match write_card_text(&root, rec) {
            Fetched::Downloaded => wrote += 1,
            Fetched::Skipped => skip += 1,
            Fetched::Failed(msg) => {
                fail += 1;
                eprintln!("  FAIL {}: {msg}", rec.edition.slug);
            }
        }
        if (i + 1) % 500 == 0 || i + 1 == total {
            println!(
                "  [{}/{total}] wrote={wrote} skipped={skip} failed={fail}",
                i + 1
            );
        }
    }

    println!(
        "\nDone. wrote={wrote} skipped={skip} failed={fail}\nOutput: {}",
        root.join("rules").display()
    );
    if fail > 0 {
        std::process::exit(2);
    }
}
