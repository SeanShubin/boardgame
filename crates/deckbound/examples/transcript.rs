//! Write **headless battle transcripts** to a git-ignored `transcripts/` directory, so a fight can be
//! read and discussed without the UI (see [`deckbound::transcript`]).
//!
//! ```text
//! cargo run -p deckbound --example transcript                # every scenario, seed 1
//! cargo run -p deckbound --example transcript -- 7           # every scenario, seed 7
//! cargo run -p deckbound --example transcript -- rules-tour  # one scenario by name
//! ```
//!
//! Each run writes `transcripts/<scenario>.<seed>.txt` and echoes it to stdout.

use deckbound::{transcribe, transcript_scenarios};

fn main() {
    // Args: an optional numeric seed and/or an optional scenario name (in any order).
    let args: Vec<String> = std::env::args().skip(1).collect();
    let seed: u64 = args.iter().find_map(|a| a.parse().ok()).unwrap_or(1);
    let only: Option<&str> = args
        .iter()
        .find(|a| a.parse::<u64>().is_err())
        .map(|s| s.as_str());

    let dir = std::path::Path::new("transcripts");
    std::fs::create_dir_all(dir).expect("create transcripts/ directory");

    let mut wrote = 0;
    for scn in transcript_scenarios() {
        if only.is_some_and(|n| n != scn.name) {
            continue;
        }
        let text = transcribe(&scn, seed);
        let path = dir.join(format!("{}.{seed}.txt", scn.name));
        std::fs::write(&path, &text).expect("write transcript");
        println!("=== wrote {} ===\n{text}", path.display());
        wrote += 1;
    }
    if wrote == 0 {
        eprintln!("no scenario matched {only:?}");
        std::process::exit(1);
    }
}
