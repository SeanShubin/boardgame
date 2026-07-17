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

use deckbound_board::units::{encounter_beasts, kit};
use deckbound_content::catalog::{self, Encounter};
use rules::combat::game::{Choice, Combat, Score, Scorer, State};
use rules::combat::regions::{Act, Board, Rank};
use rules::combat::resolve::{Combatant, Side};
use rules::core::{Game, Solver, Verdict, decisions_within};

fn fight(e: &Encounter) -> State {
    let mut units: Vec<Combatant> = catalog::ROSTER.iter().copied().map(kit).collect();
    units.extend(encounter_beasts(e)); // numbered when duplicated, so copies read apart
    State::new(units)
}

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

fn label(b: &Board, c: &Choice) -> String {
    let Choice::Act(a) = c;
    // Name the target with its hp (bodies, for a horde), so two same-named bodies in different states read apart.
    let who = |t: usize| {
        let u = &b.units[t];
        let kind = if u.horde { "bodies" } else { "hp" };
        format!("{} ({} {kind})", u.name, u.health)
    };
    match a {
        Act::Clash(t) => format!("Clash {}", who(*t)),
        Act::Raid(t, ans) => format!("Raid {} ({ans:?})", who(*t)),
        Act::Melee(t) => format!("Melee {}", who(*t)),
        Act::Slip(r, ans) => format!("Slip to region {} ({ans:?})", (b'A' + r) as char),
        Act::Hold => "Hold".to_string(),
    }
}

fn verdict(s: &State) -> Verdict {
    solve(s).0
}

/// The best winning route from `s` under the priority order (win, fewest downed, fewest rounds, least hp lost),
/// measured against the fight-start Vitality `start_hp`. `None` when there is no winning route.
fn best_route(s: &State, start_hp: &[u32]) -> Option<Score> {
    Scorer::new(start_hp.to_vec(), u64::MAX).best(s)
}

/// A best-route Score as a compact `Nd/Nr/Nhp` (downed / rounds / hp lost), or `no-win`.
fn fmt_score(s: Option<Score>) -> String {
    match s {
        Some(s) => format!("{}d/{}r/{}hp", s.downed, s.rounds, s.hp_lost),
        None => "no-win".to_string(),
    }
}

/// Solve `s` out completely (escalating the grant until it stops being Evaluating) and report the size of the
/// graph it took: `states` = distinct positions memoized (the DAG), `nodes` = total positions walked.
fn solve(s: &State) -> (Verdict, usize, u64) {
    let mut o = Solver::<Combat>::new();
    let mut grant = 1u64;
    loop {
        o.grant(grant);
        let v = o.verdict(s);
        if v != Verdict::Evaluating {
            return (v, o.states(), o.nodes());
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
    // The fight-start Vitality - the fixed reference the best-route scorer measures hp lost against.
    let start_hp: Vec<u32> = state.board().units.iter().map(|u| u.health).collect();
    let stdin = io::stdin();
    loop {
        if let Some(o) = Combat::outcome(&state) {
            println!("\n*** {o:?} ***");
            return;
        }
        println!("round {}   {}", state.round(), show_board(state.board()));
        let (v, states, nodes) = solve(&state);
        println!(
            "  verdict here: {v:?}   best route: {}   (solved graph: {states} distinct positions, {nodes} nodes walked)",
            fmt_score(best_route(&state, &start_hp))
        );
        println!("  (best route = downed / rounds / hp-lost, minimized in that priority order)");
        let opts = Combat::options(&state);
        if opts.len() == 1 {
            state = Combat::apply(&state, &opts[0]);
            continue;
        }
        for (i, c) in opts.iter().enumerate() {
            let next = Combat::apply(&state, c);
            println!(
                "  [{i}] {:<32} -> {:?}, best {}, {} decisions within 6 plies",
                label(state.board(), c),
                verdict(&next),
                fmt_score(best_route(&next, &start_hp)),
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
