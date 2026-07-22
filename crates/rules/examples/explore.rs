//! **The decision-tree explorer** - walk any [`Game`] by hand: see the state, see the options (each annotated
//! with the solver's verdict and how many decisions lie beyond it), pick one, repeat.
//!
//! This is the tool that makes a speculative rule cheap to judge: change `options`/`apply` in one file, run
//! this, and *watch* the consequence a choice at a time - long before a probe or a UI is built around it. The
//! same walk is what a debug view in the application would show.
//!
//! It drives the eight-step combat game ([`StepCombat`]) over a small hand-built fight (no catalog - this
//! crate has no deps).
//!
//! Run: `cargo run -p rules --example explore`

use std::io::{self, Write};

use rules::combat::regions::{Board, Rank};
use rules::combat::resolve::{Combatant, Side};
use rules::combat::step_game::{Step, StepChoice, StepCombat as Combat, StepState as State};
use rules::core::{Game, Solver, Verdict, decisions_within};

fn u(name: &str, side: Side, stats: [u8; 5], melee: bool, ranged: bool) -> Combatant {
    Combatant::from_stats(name, side, stats, 0, melee, ranged)
}

/// One line describing the board: each region, its front line, then its back line after `|`.
fn show_board(b: &Board) -> String {
    b.occupied()
        .iter()
        .map(|&r| {
            let tier = |rank: Rank| -> Vec<String> {
                (0..b.units.len())
                    .filter(|&i| b.regions[i] == r && b.ranks[i] == rank && !b.units[i].fallen)
                    .map(|i| {
                        let un = &b.units[i];
                        let mark = if un.side == Side::Party { "" } else { "*" };
                        format!("{mark}{}({})", un.name, un.health)
                    })
                    .collect()
            };
            let mut front = tier(Rank::Vanguard);
            // A loose outrider stands in no line; show it in the region marked with a leading ~.
            front.extend(tier(Rank::Outrider).into_iter().map(|s| format!("~{s}")));
            let k = tier(Rank::Rearguard);
            let back = if k.is_empty() {
                String::new()
            } else {
                format!(" | {}", k.join(" "))
            };
            format!("[{}: {}{}]", (b'A' + r) as char, front.join(" "), back)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// What this choice does at this step, in words.
fn label(step: Step, b: &Board, c: &StepChoice) -> String {
    let who = |t: usize| format!("{}({})", b.units[t].name, b.units[t].health);
    match (step, c) {
        (Step::Havoc, StepChoice::Strike(Some(t))) => format!("Melee {}", who(*t)),
        (Step::Skirmish, StepChoice::Strike(Some(t))) => format!("Skirmish {}", who(*t)),
        (Step::Volley, StepChoice::Strike(Some(t))) => format!("Volley the crossing {}", who(*t)),
        (Step::Raid, StepChoice::Strike(Some(t))) => format!("Raid {}", who(*t)),
        (Step::Advance, StepChoice::Strike(Some(t))) => {
            format!("Advance on the exposed {}", who(*t))
        }
        (_, StepChoice::Strike(Some(t))) => format!("Strike {}", who(*t)),
        (_, StepChoice::Strike(None)) => "Hold (pass this step)".to_string(),
        (Step::Withdraw, StepChoice::Move(true)) => "Withdraw to your own line".to_string(),
        (Step::Withdraw, StepChoice::Move(false)) => "Stay loose in their ranks".to_string(),
        (Step::Cross, StepChoice::Move(true)) => "Cross into their line".to_string(),
        (Step::Cross, StepChoice::Move(false)) => "Hold the line (do not cross)".to_string(),
        (_, StepChoice::Move(go)) => if *go { "Go" } else { "Stay" }.to_string(),
    }
}

/// The solver's verdict for a state, ground out to certainty.
fn verdict(s: &State) -> Verdict {
    let mut o = Solver::<Combat>::new();
    let mut grant = 1u64;
    loop {
        o.grant(grant);
        let v = o.verdict(s);
        if v != Verdict::Evaluating {
            return v;
        }
        grant = grant.saturating_mul(2);
    }
}

fn main() {
    let mut state = State::new(vec![
        u("Raider", Side::Party, [7, 6, 1, 3, 2], true, false),
        u("Marksman", Side::Party, [5, 2, 1, 2, 2], false, true),
        u("Wall", Side::Foe, [1, 6, 4, 1, 2], true, false),
        u("Cannon", Side::Foe, [4, 2, 1, 2, 2], false, true),
    ]);

    println!("Decision-tree explorer. Read the state, pick an option by number, watch it unfold.");
    println!(
        "Each option shows the solver's verdict from there, and how many decisions lie beyond.\n"
    );

    let stdin = io::stdin();
    loop {
        if let Some(o) = Combat::outcome(&state) {
            println!("\n=== {o:?} ===");
            return;
        }
        let deciding = state
            .deciding()
            .map(|i| state.board().units[i].name.clone())
            .unwrap_or_default();
        println!(
            "round {}  step {:?}  {} declares   {}",
            state.round(),
            state.step(),
            deciding,
            show_board(state.board())
        );
        println!("  position verdict: {:?}", verdict(&state));

        let opts = Combat::options(&state);
        if opts.len() == 1 {
            state = Combat::apply(&state, &opts[0]); // auto-advance a forced choice
            continue;
        }
        for (i, c) in opts.iter().enumerate() {
            let next = Combat::apply(&state, c);
            let v = verdict(&next);
            let beyond = decisions_within::<Combat>(&next, 8);
            println!(
                "  [{i}] {:<34} -> {v:?}, {beyond} decisions within 8 plies",
                label(state.step(), state.board(), c)
            );
        }
        print!("choose (or q): ");
        io::stdout().flush().ok();
        let mut line = String::new();
        if stdin.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        let line = line.trim();
        if line == "q" {
            return;
        }
        match line.parse::<usize>() {
            Ok(i) if i < opts.len() => state = Combat::apply(&state, &opts[i]),
            _ => println!("  ? not an option\n"),
        }
    }
}
