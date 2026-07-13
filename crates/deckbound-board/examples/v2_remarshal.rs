//! **Does re-Marshalling mid-fight ever MATTER, and can a solver afford to model it?**
//!
//! Two questions, and the second only matters if the first says yes.
//!
//! 1. **Is a mid-combat formation change ever *required* to win?** The control is the **best fixed
//!    formation**, not a bad one: starting wrong and fixing it does not count, because the party could simply
//!    have started right. So the only interesting case is
//!
//!        no fixed formation wins   AND   re-Marshalling wins
//!
//!    If that never happens, mid-combat re-Marshalling is decoration - and we should simply **not allow it**,
//!    which makes the fixed-formation solver *correct about the game* and its "doomed" verdict honest.
//!
//! 2. **What does it cost?** Re-Marshalling branches 3^living-heroes at the top of every round, and puts the
//!    formation into the memo key (it is now mutable state). This reports the memo size either way, which is
//!    what an in-app solver would have to hold.
//!
//! Run: `cargo run --release -p deckbound-board --example v2_remarshal`

use std::time::Instant;

use deckbound_board::combat::{Combatant, Side};
use deckbound_board::solver::winnable_traced;
use deckbound_content::catalog::{self, Creature, Encounter};
use deckbound_content::rank::Intention as Rank;

fn rank_of(word: &str) -> Rank {
    match word {
        "Outrider" => Rank::Outrider,
        "Rearguard" => Rank::Rearguard,
        _ => Rank::Vanguard,
    }
}

fn kit_unit((name, stats, ability): (&'static str, [u8; 5], &'static str)) -> Combatant {
    let (melee, ranged) = catalog::ability_reach(ability);
    let (_r, aoe) = catalog::ability_shape(ability);
    Combatant::from_stats(name, Side::Party, Rank::Vanguard, stats, 0, melee, ranged).with_aoe(aoe)
}

fn creature_unit(c: &Creature) -> Combatant {
    let rank = rank_of(catalog::creature_intention(c));
    Combatant::from_stats(c.name, Side::Foe, rank, c.stats, 0, c.melee, c.ranged)
        .with_aoe(c.aoe)
        .as_horde(c.horde)
}

fn encounter_units(e: &Encounter) -> Vec<Combatant> {
    let mut foes = Vec::new();
    for (c, q) in catalog::encounter_foes(e) {
        for _ in 0..q {
            foes.push(creature_unit(c));
        }
    }
    foes
}

/// One matchup, both ways. Returns `(fixed, remarshal, required)`.
fn probe(label: &str, party: &[Combatant], foes: &[Combatant]) -> bool {
    let t0 = Instant::now();
    let (fixed, fixed_states) = winnable_traced(party, foes, false);
    let fixed_ms = t0.elapsed().as_millis();

    let t1 = Instant::now();
    let (remarshal, rm_states) = winnable_traced(party, foes, true);
    let rm_ms = t1.elapsed().as_millis();

    // The only interesting case: no fixed formation wins, but re-ranking mid-fight does.
    let required = !fixed && remarshal;
    let verdict = if required {
        "  <<< RE-MARSHAL REQUIRED"
    } else if fixed && !remarshal {
        "  <<< IMPOSSIBLE: re-marshal is a superset, this is a BUG"
    } else {
        ""
    };
    println!(
        "  {label:32}  fixed {:5}  ({fixed_states:>7} states, {fixed_ms:>5} ms)   \
         remarshal {:5}  ({rm_states:>8} states, {rm_ms:>6} ms){verdict}",
        yn(fixed),
        yn(remarshal),
    );
    required
}

fn yn(b: bool) -> &'static str {
    if b { "WIN" } else { "lose" }
}

fn main() {
    println!(
        "=== Does re-Marshalling mid-fight ever turn a guaranteed loss into a win? ===\n\
         (control = the BEST fixed formation. Starting wrong and fixing it does not count.)\n"
    );
    let mut required_anywhere = false;

    for e in catalog::ENCOUNTERS.iter() {
        let foes = encounter_units(e);
        println!("{} ({} foes)", e.location, foes.len());

        // Every single kit, and the full party.
        for kit in catalog::ROSTER {
            let party = vec![kit_unit(kit)];
            required_anywhere |= probe(&format!("solo {}", kit.0), &party, &foes);
        }
        let party: Vec<Combatant> = catalog::ROSTER.into_iter().map(kit_unit).collect();
        required_anywhere |= probe("the full party", &party, &foes);
        println!();
    }

    println!("=====================================================================");
    if required_anywhere {
        println!(
            "Re-Marshalling mid-fight IS sometimes required to win.\n\
             -> the game must keep it, and a fixed-formation solver LIES about being doomed."
        );
    } else {
        println!(
            "Re-Marshalling mid-fight is NEVER required to win.\n\
             -> it is decoration: forbid it, and the fixed-formation solver becomes correct\n\
                about the game - and cheap enough to run in the app."
        );
    }
}
