//! Play the **reference campaign** to completion headlessly, following the guide's scripted path
//! (the A → B[p] → C[p] → Final reference run), and print a step-by-step trace plus the final par.
//!
//! This is the no-UI driver of the same logic the tabletop runs: it exercises movement, the
//! role-card battles, reward assignment (§8.3), and the day clock end-to-end. Run with:
//!
//! ```text
//! cargo run -p deckbound --example reference_run
//! ```
//!
//! Add `--quiet` to print only the per-day milestones and the final par; the default prints every
//! scripted action and the new log lines it produced.

use contract::{Game, Outcome, PlayerId};
use deckbound::{CampAction, Campaign, reference_campaign};

fn main() {
    let quiet = std::env::args().any(|a| a == "--quiet");
    let game = Campaign;
    let mut s = reference_campaign();
    let mut printed = 0usize; // how many log lines we've already echoed

    println!("== Deckbound — reference campaign (guided, Clash off) ==\n");

    for step in 1..=100_000 {
        if game.outcome(&s).is_some() {
            break;
        }
        let Some(action) = game.suggest(&s) else {
            println!("!! the guide has no move and the run is unfinished — stuck.");
            break;
        };
        let label = game.action_label(&s, &action);
        // World-level actions are the interesting milestones; battle moves are many and noisy.
        let is_battle = matches!(action, CampAction::Battle(_));
        if !quiet && !is_battle {
            println!("[day {} · step {step}] {label}", s.run.day + 1);
        }
        if let Err(e) = game.apply(&mut s, &action) {
            println!("!! the suggested action was rejected: {e}");
            break;
        }
        // Echo any new log lines (skip in --quiet unless it's a clear/assign milestone).
        for line in &s.log[printed..] {
            let milestone = line.starts_with("Cleared")
                || line.starts_with("Assigned")
                || line.contains("run is won");
            if !quiet || milestone {
                println!("    {line}");
            }
        }
        printed = s.log.len();
    }

    println!();
    match game.outcome(&s) {
        Some(Outcome::Win(PlayerId(0))) => {
            println!("RESULT: the run is WON — par = {} days.", s.run.day + 1);
        }
        other => println!("RESULT: not won ({other:?}) after {} days.", s.run.day + 1),
    }
    // A compact coverage summary: what each member ended up holding.
    println!("\nFinal party (role-card rewards held):");
    for m in &s.party {
        let role = m.track.role().unwrap_or_else(|| m.track.label());
        println!("  {:16} {role:12} {} reward(s)", m.name, m.rewards.len());
    }
}
