//! **Are the GAME's eight encounters ready?** Runs the exact solver over `catalog::ENCOUNTERS` - the real foe
//! lists the arena deals (a solo = one keystone; a corner = all four creatures with the keystone doubled) -
//! against the real `catalog::ROSTER` kits. Checks:
//!   - each SOLO (adjacent) is won by EXACTLY its keystone's counter kit and no other, and
//!   - each CORNER falls to the full four-kit party but to NO single kit (it needs the party).
//! (Solver = optimal-play winnability; the arena shares the same combat brain + greedy foe.)
//!
//! Run: `cargo run --release -p deckbound-board --example v2_encounters`

use deckbound_board::combat::{Combatant, Side};
use deckbound_board::solver::winnable;
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

fn main() {
    let kits: Vec<Combatant> = catalog::ROSTER.iter().map(|&k| kit_unit(k)).collect();

    println!("=== SOLOS (adjacent): want exactly the keystone's counter kit ===\n");
    let mut ok = true;
    for e in catalog::ENCOUNTERS.iter().filter(|e| !e.party) {
        let foes = encounter_units(e);
        let want = catalog::creature_counter(catalog::creature(e.keystone).unwrap());
        let winners: Vec<&str> = catalog::ROSTER
            .iter()
            .filter(|&&k| winnable(std::slice::from_ref(&kit_unit(k)), &foes))
            .map(|k| k.0)
            .collect();
        let good = winners == [want];
        ok &= good;
        println!(
            "  {:<20} {:<11} -> won by [{}]  (want {want}) {}",
            e.location,
            e.keystone,
            winners.join(", "),
            if good { "OK" } else { "MISMATCH" }
        );
    }

    println!("\n=== CORNERS: want the full party to WIN, and NO single kit to solo ===\n");
    for e in catalog::ENCOUNTERS.iter().filter(|e| e.party) {
        let foes = encounter_units(e);
        let full = winnable(&kits, &foes);
        let soloers: Vec<&str> = catalog::ROSTER
            .iter()
            .filter(|&&k| winnable(std::slice::from_ref(&kit_unit(k)), &foes))
            .map(|k| k.0)
            .collect();
        let good = full && soloers.is_empty();
        ok &= good;
        println!(
            "  {:<20} (keystone {:<11}) full party {}, solo-able by [{}] {}",
            e.location,
            e.keystone,
            if full { "WIN" } else { "LOSE" },
            soloers.join(", "),
            if good { "OK" } else { "NEEDS TUNING" }
        );
    }

    println!(
        "\n{}",
        if ok {
            "ALL EIGHT ENCOUNTERS READY: solos gated to one kit, corners need the party."
        } else {
            "NOT ready - the flagged encounters need tuning (see catalog CREATURES / encounter_foes)."
        }
    );
}
