//! **v2 balance harness** — runs the exact winnability solver across the duel-locks kit x creature matrix,
//! reporting whether the clean diagonal (each creature beaten by exactly one kit) still holds under the v2
//! combat mechanics (target / react / extra sub-phases). This is the first check of whether the numbers that
//! balanced the OLD model survive the new one.
//!
//! Run: `cargo run -p deckbound-cardtable --example v2_balance`

use deckbound::actor::Intention as Rank;
use deckbound::catalog::{self, Creature};
use deckbound_cardtable::combat::{Combatant, Side};
use deckbound_cardtable::solver::winnable;

/// Map a creature's intention word to a rank.
fn rank_of(word: &str) -> Rank {
    match word {
        "Outrider" => Rank::Outrider,
        "Rearguard" => Rank::Rearguard,
        _ => Rank::Vanguard,
    }
}

/// A party kit as a combatant (rank is a placeholder — the solver enumerates formations).
fn kit_unit(name: &str, stats: [u8; 5], ability: &str) -> Combatant {
    let (melee, ranged) = catalog::ability_reach(ability);
    Combatant::from_stats(name, Side::Party, Rank::Vanguard, stats, melee, ranged)
}

/// A creature as a scripted foe combatant (rank derived from its intention).
fn creature_unit(c: &Creature) -> Combatant {
    let rank = rank_of(catalog::creature_intention(c));
    Combatant::from_stats(c.name, Side::Foe, rank, c.stats, c.melee, c.ranged)
}

fn main() {
    println!("=== v2 solo winnability: can the KIT (row) beat the CREATURE (col) 1v1? ===\n");

    // Header row of creature names.
    print!("{:<14}", "kit \\ foe");
    for c in &catalog::CREATURES {
        print!("{:>14}", c.name);
    }
    println!();

    let mut mismatches = Vec::new();
    for &(kname, kstats, kability) in &catalog::ROSTER {
        let kit = kit_unit(kname, kstats, kability);
        print!("{kname:<14}");
        for c in &catalog::CREATURES {
            let win = winnable(std::slice::from_ref(&kit), &[creature_unit(c)]);
            let expected = catalog::creature_counter(c) == kname; // the designed answer
            let mark = match (win, expected) {
                (true, true) => "  WIN*",  // designed win, present
                (true, false) => "  win ", // an unexpected win (kit also beats this lock)
                (false, true) => "  MISS", // designed answer FAILS under v2 - a balance break
                (false, false) => "  --  ",
            };
            print!("{mark:>14}");
            if win != expected {
                mismatches.push((kname, c.name, win, expected));
            }
        }
        println!();
    }

    println!("\n  WIN* = the designed counter (creature_counter) wins, as intended");
    println!("  MISS = the designed counter FAILS under v2 (a balance break)");
    println!("  win  = a kit that ALSO beats a lock it wasn't the designed answer for\n");

    if mismatches.is_empty() {
        println!(
            "DIAGONAL HOLDS: every creature is beaten by exactly its designed kit, and no others."
        );
    } else {
        println!(
            "DIAGONAL DIVERGES from the old model under v2 ({} cells):",
            mismatches.len()
        );
        for (kit, foe, win, expected) in &mismatches {
            let what = if *win {
                "unexpected win"
            } else {
                "designed answer fails"
            };
            println!("  {kit} vs {foe}: {what} (v2={win}, designed={expected})");
        }
    }
}
