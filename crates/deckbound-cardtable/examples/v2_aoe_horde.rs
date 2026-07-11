//! **AoE vs the horde (and why single-target still exists).** The four generic attacks are the same unit with
//! a different reach x spread: Jab (melee single), Shot (ranged single), Sweep (melee area), Salvo (ranged
//! area). An **area** strike hits every enemy in a rank at once, unevadable, for one tempo card - but it
//! cannot concentrate (no extra strikes). A **horde** is a pack of one-Health bodies whose offense scales
//! with the pack: a single strike cuts down one body at a time (you drown), an area strike clears the pack.
//!
//! So the two decisive gates should be *opposite*: only an area attack answers a big horde, and only a
//! concentrated attack answers one armored wall. If the solver agrees, spread is a real axis - neither attack
//! dominates.
//!
//! Run: `cargo run -p deckbound-cardtable --example v2_aoe_horde`

use deckbound::actor::Intention as Rank;
use deckbound_cardtable::combat::{Combatant, Side};
use deckbound_cardtable::solver::winnable;

/// One attack, as a party unit with a fixed stat line; only the reach x spread differs (the solver picks the
/// rank). `[Might, Vitality, Toughness, Cadence, Finesse]`.
fn attack(name: &str, melee: bool, ranged: bool, aoe: bool) -> Combatant {
    Combatant::from_stats(
        name,
        Side::Party,
        Rank::Vanguard,
        [5, 12, 2, 4, 2],
        0,
        melee,
        ranged,
    )
    .with_aoe(aoe)
}

fn main() {
    let attacks = [
        ("Jab   (melee  single)", attack("Jab", true, false, false)),
        ("Shot  (ranged single)", attack("Shot", false, true, false)),
        ("Sweep (melee  area)  ", attack("Sweep", true, false, true)),
        ("Salvo (ranged area)  ", attack("Salvo", false, true, true)),
    ];

    // A big horde: 24 one-Health bodies (Might 1 each). It swarms with one card per body, so single-file
    // chipping never keeps up - you drown. Placed as a Vanguard so any attack can reach it.
    let horde = Combatant::from_stats(
        "Horde",
        Side::Foe,
        Rank::Vanguard,
        [1, 24, 1, 1, 1],
        0,
        true,
        false,
    )
    .as_horde(true);

    // One stubborn wall: Toughness 6 means a single sweep of Might 5 is always sub-bar and wiped, so only a
    // *concentrated* attack (many strikes accumulating) cracks it. Weak offense (Might 1) so the lone
    // attacker survives - the only variable is whether its damage sticks.
    let wall = Combatant::from_stats(
        "Wall",
        Side::Foe,
        Rank::Vanguard,
        [1, 10, 6, 1, 2],
        0,
        true,
        false,
    );

    println!("=== which attack forces the win? (the solver picks each attacker's rank) ===\n");
    println!(
        "{:<26}{:>18}{:>18}",
        "attack", "vs 24-body horde", "vs toughness wall"
    );
    for (label, atk) in &attacks {
        let h = winnable(std::slice::from_ref(atk), &[horde.clone()]);
        let w = winnable(std::slice::from_ref(atk), &[wall.clone()]);
        println!("{label:<26}{:>18}{:>18}", yn(h), yn(w));
    }

    println!(
        "\nReading it: an **area** attack (Sweep / Salvo) clears the horde in one sweep but cannot concentrate,\n\
         so each sub-bar sweep is wiped by the wall's Toughness; a **single** attack (Jab / Shot) concentrates\n\
         many strikes past the Toughness to crack the wall but drowns against the horde one body at a time.\n\
         Spread is a real axis - the answer to a pack is the wrong answer to a wall, and vice versa."
    );
}

fn yn(win: bool) -> &'static str {
    if win { "WIN" } else { "lose" }
}
