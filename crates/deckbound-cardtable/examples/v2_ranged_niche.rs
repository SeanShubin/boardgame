//! **Does high-damage-at-range have an efficiency niche?** (force not fiat). A hard-hitting, high-Toughness
//! enemy **Vanguard wall** is the test: a screen + a "cannon" must bring it down. We vary the cannon
//! (reach x Might) and ask the exact solver whether that party can force the win. If ranged-high-Might wins
//! where the alternatives (melee, or low-Might accumulation) lose, high-damage-at-range is the *efficient*
//! answer, and its necessity is emergent, not decreed.
//!
//! Run: `cargo run -p deckbound-cardtable --example v2_ranged_niche`

use deckbound::actor::Intention as Rank;
use deckbound_cardtable::combat::{Combatant, Side};
use deckbound_cardtable::solver::winnable;

/// A party unit from `[Might, Vitality, Toughness, Cadence, Finesse]` + reach (rank is the solver's choice).
fn ally(name: &str, stats: [u8; 5], melee: bool, ranged: bool) -> Combatant {
    Combatant::from_stats(name, Side::Party, Rank::Vanguard, stats, melee, ranged)
}

fn main() {
    // The enemy: one hard wall — a Vanguard that out-toughs and hits back hard. Might 4 (punches), Toughness
    // 3 + Vitality 6 (a wall), Cadence 2 (keeps swinging). Melee, Vanguard.
    let wall = Combatant::from_stats(
        "Wall",
        Side::Foe,
        Rank::Vanguard,
        [4, 6, 3, 2, 2],
        true,
        false,
    );

    // A tanky screen that soaks the wall while the cannon works, but can't crack it alone (Might 1).
    let screen = ally("Screen", [1, 8, 3, 1, 2], true, false);

    // The cannon variants: same 3-point damage budget spent differently, and different reach.
    let cannons = [
        (
            "ranged  Might 6, Cadence 1",
            ally("Cannon", [6, 3, 1, 1, 2], false, true),
        ),
        (
            "ranged  Might 2, Cadence 3",
            ally("Cannon", [2, 3, 1, 3, 2], false, true),
        ),
        (
            "melee   Might 6, Cadence 1",
            ally("Cannon", [6, 3, 1, 1, 2], true, false),
        ),
        (
            "melee   Might 2, Cadence 3",
            ally("Cannon", [2, 3, 1, 3, 2], true, false),
        ),
    ];

    println!(
        "=== Can [Screen + Cannon] force a win vs one hard-hitting tanky Vanguard wall? ===\n"
    );
    println!("(the solver picks each party unit's formation; the wall is Vanguard)\n");

    // Marginal-contribution baselines: neither piece alone should manage it.
    println!(
        "  {:<32} {}",
        "Screen alone",
        verdict(winnable(&[screen.clone()], &[wall.clone()]))
    );
    for (label, cannon) in &cannons {
        println!(
            "  {:<32} {}",
            format!("Cannon alone ({label})"),
            verdict(winnable(std::slice::from_ref(cannon), &[wall.clone()]))
        );
    }
    println!();
    for (label, cannon) in &cannons {
        let w = winnable(&[screen.clone(), cannon.clone()], &[wall.clone()]);
        println!("  {:<32} {}", format!("Screen + {label}"), verdict(w));
    }

    println!(
        "\nReading it: if 'Screen + ranged Might 6' wins while the melee and low-Might cannons do not, then\n\
         high-damage-at-range is the efficient answer to a hard wall - alternatives exist but fall short."
    );
}

fn verdict(win: bool) -> &'static str {
    if win {
        "WIN  (can force it)"
    } else {
        "lose (cannot force it)"
    }
}
