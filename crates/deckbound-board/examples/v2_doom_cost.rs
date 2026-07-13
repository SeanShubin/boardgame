//! **What does it cost to map a fight out completely, from the initial Marshal?**
//!
//! The four adjacent locations, each with the one kit that is allowed to win it, and the four corners with the
//! full party. For each: every formation, every allocation, every line, to the end of every branch, with **no
//! short-circuit anywhere**.
//!
//! This is the **ceiling**, and it is deliberately harsher than what the game does:
//!
//! - The doom oracle in the app starts *after* the formation is chosen. This starts at the Marshal, so the
//!   `3^heroes` formation fan-out is inside the tree - 81 formations for a corner. That is precisely the cost a
//!   Marshal-screen indicator would have to pay, which is why this probe answers whether we can ever build one.
//! - The oracle stops a subtree the moment it finds a winning line. This never stops. So the numbers here bound
//!   the real cost from above, and by a wide margin on any fight that is winnable early.
//!
//! `nodes` is positions *evaluated* (a memo hit is free); `states` is what the memo ends up holding - the
//! memory an in-app oracle would carry for that fight.
//!
//! Run: `cargo run --release -p deckbound-board --example v2_doom_cost`

use std::time::Instant;

use deckbound_board::combat::{Combatant, Side};
use deckbound_board::solver::{formation_ranks, map_out, map_out_formation};
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

fn kit(name: &str) -> Combatant {
    kit_unit(
        catalog::ROSTER
            .into_iter()
            .find(|(n, _, _)| *n == name)
            .expect("a kit by that name"),
    )
}

/// The per-frame node allowance the app runs at (see `board_game::NODE_BUDGET`), so the frame count below is
/// the real one, not a hypothetical.
const NODE_BUDGET: u64 = 2_500;

fn letter(r: Rank) -> char {
    match r {
        Rank::Vanguard => 'V',
        Rank::Outrider => 'O',
        Rank::Rearguard => 'R',
    }
}

fn probe(place: &str, who: &str, party: &[Combatant], foes: &[Combatant]) -> f64 {
    // The whole map, one shared memo - the total cost of knowing everything about this fight.
    let t0 = Instant::now();
    let all = map_out(party, foes);
    let ms = t0.elapsed().as_secs_f64() * 1000.0;

    // ...and every formation ALONE, from a fresh memo. The shared-memo total flatters whichever formation went
    // first; this is the number a Marshal-screen indicator would actually pay, per formation, and the worst of
    // them is the frame-hitch risk.
    let mut worst = (0usize, 0f64, 0u64);
    let mut total_alone = 0.0;
    for f in 0..all.formations {
        let t = Instant::now();
        let one = map_out_formation(party, foes, f);
        let one_ms = t.elapsed().as_secs_f64() * 1000.0;
        total_alone += one_ms;
        if one_ms > worst.1 {
            worst = (f, one_ms, one.nodes);
        }
    }
    let ranks: String = formation_ranks(party.len(), worst.0)
        .into_iter()
        .map(letter)
        .collect();
    let frames = worst.2.div_ceil(NODE_BUDGET);

    println!(
        "  {place:20} {who:24} {:>9} nodes  {:>8} states  {:>3} forms  {ms:>8.1} ms   |  worst single formation: {ranks:<4} {:>8} nodes {:>8.1} ms ({frames} frames)  {}",
        all.nodes,
        all.states,
        all.formations,
        worst.2,
        worst.1,
        if all.winnable { "WIN" } else { "lose" },
    );
    let _ = total_alone;
    ms
}

fn main() {
    println!(
        "=== Mapping a fight out completely, from the initial Marshal ===\n\
         Every formation, every allocation, every line - no short-circuit anywhere.\n\
         This is the CEILING: the app's oracle starts after the formation and stops at the first win.\n"
    );

    println!("SOLOS (adjacent) - the one kit allowed to win each:");
    let mut solo_ms = 0.0;
    for e in catalog::ENCOUNTERS.iter().filter(|e| !e.party) {
        let foes = encounter_units(e);
        let counter = catalog::creature_counter(catalog::creature(e.keystone).expect("keystone"));
        let party = vec![kit(counter)];
        solo_ms += probe(e.location, counter, &party, &foes);
    }

    println!("\nCORNERS - the full party:");
    let party: Vec<Combatant> = catalog::ROSTER.into_iter().map(kit_unit).collect();
    let mut corner_ms = 0.0;
    for e in catalog::ENCOUNTERS.iter().filter(|e| e.party) {
        let foes = encounter_units(e);
        corner_ms += probe(e.location, "the full party", &party, &foes);
    }

    println!("\n---------------------------------------------------------------");
    println!("  four solos:   {solo_ms:9.1} ms");
    println!("  four corners: {corner_ms:9.1} ms");
    println!("  all eight:    {:9.1} ms", solo_ms + corner_ms);
}
