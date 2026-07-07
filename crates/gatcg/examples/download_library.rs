//! Download every Grand Archive card image into a local directory.
//!
//! Usage:
//!   cargo run -p gatcg --example download_library
//!   cargo run -p gatcg --example download_library -- <output-dir>
//!
//! Default output root is `local/grand-archive` (the repo's gitignored scratch
//! area). Layout mirrors the manual sample: `images/<set-prefix>/<slug>.jpg`,
//! plus a `manifest.csv` mapping every file back to its card and source URL.
//!
//! **Safe to interrupt and restart.** Already-downloaded images are skipped on
//! the next run, and each image is written to a `.part` file that is only
//! renamed into place once fully downloaded — so Ctrl-C (or a crash) never
//! leaves a truncated image behind. Re-running resumes with only what's missing.
//!
//! The whole library is ~2200 cards / several thousand editions, so a cold run
//! takes a while and a few hundred MB. Point it at a smaller run by interrupting
//! whenever you like; progress is preserved.

use std::path::PathBuf;
use std::time::Duration;

use gatcg::{Fetched, download_edition, fetch_all_editions, write_manifest};

/// Politeness delay between image requests. Also ensures Ctrl-C lands cleanly
/// between files rather than mid-write.
const REQUEST_SPACING: Duration = Duration::from_millis(40);

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

    println!("Enumerating card library from {} ...", gatcg::API_HOST);
    let editions = match fetch_all_editions(&agent, |page, total, running| {
        println!("  page {page}/{total} — {running} editions so far");
    }) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("enumeration failed: {err}");
            std::process::exit(1);
        }
    };
    println!("Found {} card editions.", editions.len());

    if let Err(err) = write_manifest(&root, &editions) {
        eprintln!("warning: could not write manifest.csv: {err}");
    }

    let total = editions.len();
    let (mut done, mut skip, mut fail) = (0usize, 0usize, 0usize);

    for (i, edition) in editions.iter().enumerate() {
        match download_edition(&agent, &root, edition) {
            Fetched::Downloaded => {
                done += 1;
                std::thread::sleep(REQUEST_SPACING); // only pace real fetches
            }
            Fetched::Skipped => skip += 1,
            Fetched::Failed(msg) => {
                fail += 1;
                eprintln!("  FAIL {}: {msg}", edition.slug);
            }
        }

        if (i + 1) % 50 == 0 || i + 1 == total {
            println!(
                "  [{}/{total}] downloaded={done} skipped={skip} failed={fail}",
                i + 1
            );
        }
    }

    println!(
        "\nDone. downloaded={done} skipped={skip} failed={fail}\nOutput: {}",
        root.display()
    );
    if fail > 0 {
        // Non-zero exit signals partial completion; just re-run to retry failures.
        std::process::exit(2);
    }
}
