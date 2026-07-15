//! **The generic decision-tree UI, over real encounters.** The canonical way to drive the game by hand: see the
//! state, see every option annotated with the solver's verdict and how many decisions lie beyond it, choose,
//! watch it unfold.
//!
//! It is the same generic runner as `rules/examples/explore.rs` - the dep-free demo - but pointed at the
//! catalog, so you can walk any of the eight encounters with the real kits and creatures. This is the "debug
//! view" the application would expose.
//!
//! Run: `cargo run --release -p deckbound-board --example explore -- [encounter#]`
//! (no argument lists the encounters and walks the first party encounter).

use std::io::{self, Write};

use deckbound_board::units::{beast, kit};
use deckbound_content::catalog::{self, Creature, Encounter};
use rules::combat::game::{Choice, Combat, State};
use rules::combat::regions::{Act, Board, Post};
use rules::combat::resolve::{Combatant, Side};
use rules::core::{Game, Solver, Verdict, decisions_within};

fn fight(e: &Encounter) -> State {
    let mut units: Vec<Combatant> = catalog::ROSTER.iter().copied().map(kit).collect();
    for (c, q) in catalog::encounter_foes(e) {
        for _ in 0..q {
            units.push(beast(c));
        }
    }
    State::new(units)
}

fn show_board(b: &Board) -> String {
    b.occupied()
        .iter()
        .map(|&r| {
            let tier = |p: Post| -> Vec<String> {
                (0..b.units.len())
                    .filter(|&i| b.regions[i] == r && b.posts[i] == p && !b.units[i].fallen)
                    .map(|i| {
                        let un = &b.units[i];
                        let mark = if un.side == Side::Party { "" } else { "*" };
                        format!("{mark}{}({})", un.name, un.health)
                    })
                    .collect()
            };
            let (f, k) = (tier(Post::Front), tier(Post::Back));
            let back = if k.is_empty() {
                String::new()
            } else {
                format!(" | {}", k.join(" "))
            };
            format!("[{}: {}{}]", (b'A' + r) as char, f.join(" "), back)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn label(b: &Board, c: &Choice) -> String {
    match c {
        Choice::Place { region, post } => format!(
            "stand in region {} at the {}",
            (b'A' + region) as char,
            if *post == Post::Front {
                "front"
            } else {
                "back"
            }
        ),
        Choice::Act(a) => match a {
            Act::Clash(t) => format!("Clash {}", b.units[*t].name),
            Act::Raid(t, ans) => format!("Raid {} ({ans:?})", b.units[*t].name),
            Act::Slip(r, ans) => format!("Slip to region {} ({ans:?})", (b'A' + r) as char),
            Act::Hold => "Hold".to_string(),
        },
    }
}

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
    for (i, e) in catalog::ENCOUNTERS.iter().enumerate() {
        println!(
            "  {i}  {:<20} {}",
            e.location,
            if e.party { "(party)" } else { "(solo)" }
        );
    }
    let idx: usize = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or_else(|| {
            catalog::ENCOUNTERS
                .iter()
                .position(|e| e.party)
                .unwrap_or(0)
        });
    let e = &catalog::ENCOUNTERS[idx % catalog::ENCOUNTERS.len()];
    println!("\n=== {} - {} ===", e.location, e.title);
    println!(
        "Read the state, pick an option by number. Each shows the solver's verdict from there.\n"
    );

    let mut state = fight(e);
    let stdin = io::stdin();
    loop {
        if let Some(o) = Combat::outcome(&state) {
            println!("\n*** {o:?} ***");
            return;
        }
        println!("round {}   {}", state.round(), show_board(state.board()));
        println!("  verdict here: {:?}", verdict(&state));
        let opts = Combat::options(&state);
        if opts.len() == 1 {
            state = Combat::apply(&state, &opts[0]);
            continue;
        }
        for (i, c) in opts.iter().enumerate() {
            let next = Combat::apply(&state, c);
            println!(
                "  [{i}] {:<32} -> {:?}, {} decisions within 6 plies",
                label(state.board(), c),
                verdict(&next),
                decisions_within::<Combat>(&next, 6)
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
