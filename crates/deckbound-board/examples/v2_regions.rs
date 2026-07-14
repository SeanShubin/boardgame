//! **Does a PRICED move ever matter? The mirror of `v2_remarshal`.**
//!
//! `v2_remarshal` asked whether a mid-fight re-rank is ever *required* to win, against the honest control
//! (the **best fixed formation**, not a bad one). The answer was no, exhaustively - so re-Marshalling was cut
//! as decoration and the formation was frozen for the whole fight.
//!
//! That result is sound, and it is *why* this probe exists. Read it again:
//!
//!     re-ranking was worthless BECAUSE re-ranking was free.
//!
//! A repositioning that costs nothing and is offered every round can always be pre-empted by simply starting
//! in the right place. It can therefore never be *necessary* - which is exactly what the probe measured. It
//! did not show that position does not matter. It showed that **costless** position does not matter.
//!
//! So: re-ask the identical question, with the identical control, in a model where **moving costs**.
//!
//!     Does there exist a position where NO fixed setup wins, AND moving wins?
//!
//! - **Yes** -> movement is load-bearing. The model buys something real.
//! - **No**  -> the model is DEAD, and we learned it for the price of one example program instead of an arena
//!   rewrite.
//!
//! The model under test is [`deckbound_board::regions`] - read its module docs, they are the design. This
//! program only *drives* it: the control, the treatment, the cost report, a transcript, and the per-move
//! verdict table the UI has to surface.
//!
//! Run: `cargo run --release -p deckbound-board --example v2_regions`
//!
//! # What this does NOT search (stated honestly)
//!
//! It searches the **aim layer exhaustively** and holds the **tempo allocation at greedy** for both sides (the
//! same greedy tension `battle.rs` uses). That is deliberate - it isolates the *positional* question by
//! comparing like with like, and it is what makes the probe finish. So a **"no" would be evidence, not proof**
//! (a fixed setup might still lose under optimal allocation where it wins under greedy). A **"yes" is proof**,
//! and "yes" is the answer that costs money. Support (buffs) is omitted.

use std::time::Instant;

use deckbound_board::combat::{Combatant, Side};
use deckbound_board::regions::{
    self, Aim, Board, Oracle, SubPhase, Verdict, best_fixed, legal_aims,
};
use deckbound_content::catalog::{self, Creature, Encounter};
use deckbound_content::rank::Intention as Rank;

const BUDGET: u64 = 4_000_000;

fn kit_unit((name, stats, ability): (&'static str, [u8; 5], &'static str)) -> Combatant {
    let (melee, ranged) = catalog::ability_reach(ability);
    let (_r, aoe) = catalog::ability_shape(ability);
    Combatant::from_stats(name, Side::Party, Rank::Vanguard, stats, 0, melee, ranged).with_aoe(aoe)
}

fn creature_unit(c: &Creature) -> Combatant {
    Combatant::from_stats(
        c.name,
        Side::Foe,
        Rank::Vanguard,
        c.stats,
        0,
        c.melee,
        c.ranged,
    )
    .with_aoe(c.aoe)
    .as_horde(c.horde)
}

fn setup(e: &Encounter) -> Board {
    let mut units: Vec<Combatant> = catalog::ROSTER.iter().copied().map(kit_unit).collect();
    for (c, q) in catalog::encounter_foes(e) {
        for _ in 0..q {
            units.push(creature_unit(c));
        }
    }
    Board::opening(units)
}

/// The board as one line: the regions, and who stands in each. `*` = foe, `(n)` = health.
fn board_line(b: &Board) -> String {
    b.occupied()
        .iter()
        .map(|&r| {
            let who: Vec<String> = b
                .in_region(r)
                .iter()
                .map(|&i| {
                    let u = &b.units[i];
                    let mark = if u.side == Side::Party { "" } else { "*" };
                    format!("{}{}({})", mark, u.name, u.health)
                })
                .collect();
            format!("[{}: {}]", (b'A' + r) as char, who.join(" "))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The `pick`-th joint declaration over the living heroes, or `None` once they are exhausted.
fn nth_line(b: &Board, heroes: &[usize], pick: usize) -> Option<Vec<Aim>> {
    let choices: Vec<Vec<Aim>> = heroes.iter().map(|&i| legal_aims(b, i)).collect();
    let total: usize = choices.iter().map(|c| c.len()).product::<usize>().max(1);
    if pick >= total {
        return None;
    }
    let mut aims: Vec<Aim> = vec![Aim::WAIT; b.units.len()];
    for (k, &i) in heroes.iter().enumerate() {
        let radix: usize = choices[..k]
            .iter()
            .map(|c| c.len())
            .product::<usize>()
            .max(1);
        aims[i] = choices[k][(pick / radix) % choices[k].len()];
    }
    for (i, a) in regions::foe_aims(b).iter().enumerate() {
        if let Some(a) = a {
            aims[i] = *a;
        }
    }
    Some(aims)
}

/// The line a player following the doom oracle would take: the first joint declaration it still certifies as
/// winnable. Falls back to the first legal one if the position is already lost - which is the honest thing to
/// show, because a doomed board still has to be played out.
fn certified_line(b: &Board, round: usize) -> Vec<Aim> {
    let heroes: Vec<usize> = (0..b.units.len())
        .filter(|&i| b.units[i].side == Side::Party && !b.units[i].fallen)
        .collect();
    let mut o = Oracle::new(BUDGET);
    let mut fallback: Option<Vec<Aim>> = None;
    for pick in 0.. {
        let Some(aims) = nth_line(b, &heroes, pick) else {
            break;
        };
        if fallback.is_none() {
            fallback = Some(aims.clone());
        }
        let mut probe = b.clone();
        regions::play_round(&mut probe, &aims);
        if o.winnable(&probe, round + 1, None) {
            return aims;
        }
    }
    fallback.expect("at least one legal declaration")
}

fn main() {
    println!("v2_regions - does a PRICED move ever turn a loss into a win?");
    println!("the mirror of v2_remarshal, which proved a FREE move never does.\n");

    let mut rescued = Vec::new();
    let (mut nodes, mut worst_memo, mut ms) = (0u64, 0usize, 0u128);

    for e in catalog::ENCOUNTERS.iter() {
        let board = setup(e);

        let t0 = Instant::now();
        let (fixed_wins, fo) = best_fixed(&board, BUDGET);
        let fixed_ms = t0.elapsed().as_millis();

        let t1 = Instant::now();
        let mut mo = Oracle::new(BUDGET);
        let moving = mo.verdict(&board, 0);
        let move_ms = t1.elapsed().as_millis();

        nodes += mo.nodes();
        worst_memo = worst_memo.max(mo.states());
        ms += move_ms;

        let fixed_verdict = match (fixed_wins, fo.aborted()) {
            (true, _) => "WINNABLE",
            (false, true) => "evaluating (budget)",
            (false, false) => "DOOMED",
        };

        println!("{} - {}", e.location, e.title);
        println!(
            "   fixed setup, never moves : {:<20} ({} nodes, {} memo, {} ms)",
            fixed_verdict,
            fo.nodes(),
            fo.states(),
            fixed_ms
        );
        println!(
            "   re-declare every round   : {:<20} ({} nodes, {} memo, {} ms)",
            format!("{moving:?}").to_uppercase(),
            mo.nodes(),
            mo.states(),
            move_ms
        );

        // The whole question, in one line.
        if moving == Verdict::Winnable && !fixed_wins && !fo.aborted() {
            println!("   >>> MOVEMENT IS LOAD-BEARING HERE: no fixed setup wins, and moving does.");
            rescued.push(e.location);
        }
        println!();
    }

    println!("----------------------------------------------------------------");
    if rescued.is_empty() {
        println!("VERDICT: movement is DECORATION. No encounter is rescued by moving that a fixed");
        println!("         setup could not already win - the same result v2_remarshal got for a");
        println!(
            "         FREE re-rank. Pricing the move did not make it matter. Do not build it."
        );
    } else {
        println!(
            "VERDICT: movement is LOAD-BEARING. {} encounter(s) are winnable ONLY by moving,",
            rescued.len()
        );
        println!("         and unwinnable from every fixed setup:");
        for r in &rescued {
            println!("           - {r}");
        }
        println!("         This is what v2_remarshal could NOT find with a free move. Pricing the");
        println!("         move created a decision that did not exist before.");
    }
    println!(
        "\nCOST (v2_remarshal measured 24x for a per-round re-rank; that is what to beat):\n  \
         {nodes} nodes total, {worst_memo} states in the worst memo, {ms} ms total"
    );

    // ---- the transcript: can you READ the fight at round 4? --------------------------------------------
    println!("\n----------------------------------------------------------------");
    println!("TRANSCRIPT - the board after every sub-phase. `*` = foe, `(n)` = health.");
    println!("The judgment the numbers cannot give: at round 4, can you still say what is");
    println!("happening, and why?\n");

    let e = catalog::ENCOUNTERS
        .iter()
        .find(|e| e.location == "Greywater Ford")
        .expect("Greywater Ford");
    let mut b = setup(e);
    println!(
        "{} - {}\n  start:   {}\n",
        e.location,
        e.title,
        board_line(&b)
    );

    for round in 0..regions::MAX_ROUNDS {
        if b.outcome().is_some() {
            break;
        }
        let aims = certified_line(&b, round);
        println!("Round {}:", round + 1);
        for i in 0..b.units.len() {
            if b.units[i].side == Side::Party && !b.units[i].fallen {
                println!("    {:<12} {}", b.units[i].name, aims[i].label(&b));
            }
        }
        let logs = regions::play_round(&mut b, &aims);
        for (phase, log) in SubPhase::ALL.iter().zip(&logs) {
            let mut notes = Vec::new();
            for &i in &log.caught {
                notes.push(format!("{} is CAUGHT crossing", b.units[i].name));
            }
            for &i in &log.arrived {
                notes.push(format!("{} gets through", b.units[i].name));
            }
            for &i in &log.fallen {
                notes.push(format!("{} FALLS", b.units[i].name));
            }
            let note = if notes.is_empty() {
                String::new()
            } else {
                format!("   ({})", notes.join("; "))
            };
            println!(
                "  {:<9}{}{}",
                format!("{}:", phase.label()),
                board_line(&b),
                note
            );
        }
        println!();
    }
    println!(
        "  result: {}\n",
        match b.outcome() {
            Some(true) => "party wins",
            Some(false) => "party falls",
            None => "draw at the round cap",
        }
    );

    // ---- the per-move verdict table: exactly the doom-oracle data the UI must surface -------------------
    println!("----------------------------------------------------------------");
    println!("PER-MOVE VERDICT TABLE (the doom oracle, as the UI would chart it)");
    println!("For each hero, each move it could open with, and whether the position is still");
    println!("winnable if it makes it. It asks what FORECLOSES the win - not what is optimal.\n");

    for e in catalog::ENCOUNTERS.iter().filter(|e| e.party) {
        let b = setup(e);
        println!("{} - {}", e.location, e.title);
        println!("  board: {}", board_line(&b));
        for i in 0..b.units.len() {
            if b.units[i].side != Side::Party {
                continue;
            }
            let mut o = Oracle::new(BUDGET);
            let lines: Vec<(String, Verdict)> = legal_aims(&b, i)
                .into_iter()
                .map(|a| (a.label(&b), o.verdict_for(&b, 0, i, a)))
                .collect();
            // Only print a hero whose choice actually discriminates. One whose every move keeps the win has no
            // decision to make, and listing seven identical verdicts is noise (spec 4.1: count-adaptivity - a
            // choice is shown iff it has >= 2 meaningfully different options).
            if lines.iter().all(|(_, v)| *v == Verdict::Winnable) {
                println!(
                    "  {}: every move keeps the win - no decision here",
                    b.units[i].name
                );
                continue;
            }
            println!("  {}:", b.units[i].name);
            for (l, v) in lines {
                println!("      {l:<22} {v:?}");
            }
        }
        println!();
    }
}
