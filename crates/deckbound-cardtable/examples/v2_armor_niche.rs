//! **Does armor make high per-strike damage *necessary*?** The exact solver, run against the same target with
//! and without armor, answers it decisively. Under v2's toughness-accumulate model, `Might` (per strike) and
//! `Cadence` (number of strikes) are fungible for total damage - so no target *requires* high Might. **Armor**
//! (a flat per-strike reduction) breaks that fungibility: a strike whose Might does not exceed the armor deals
//! nothing, and no amount of Cadence changes that. If low-Might-high-Cadence beats the UNarmored target but
//! *loses* to the ARMORED one while high-Might wins, armor is the mechanic that gates the high-damage role -
//! the decisive "unsolvable until you bring the right tool" a tutorial wants.
//!
//! Run: `cargo run -p deckbound-cardtable --example v2_armor_niche`

use deckbound::actor::Intention as Rank;
use deckbound_cardtable::combat::{Combatant, Side};
use deckbound_cardtable::solver::winnable;

fn hero(name: &str, stats: [u8; 5]) -> Combatant {
    // Melee attacker; the solver picks its rank. (Armor gates *damage*; range is a separate lever - riposte.)
    Combatant::from_stats(name, Side::Party, Rank::Vanguard, stats, 0, true, false)
}

/// A soft target that can't win the attrition (Might 1), so the ONLY variable is whether the attacker's
/// strikes penetrate. `armor` is the knob.
fn target(armor: u32) -> Combatant {
    Combatant::from_stats(
        "Target",
        Side::Foe,
        Rank::Vanguard,
        [1, 4, 1, 1, 1],
        armor,
        true,
        false,
    )
}

fn main() {
    // Same damage budget, spent as per-strike Might vs number-of-strikes (Cadence).
    let attackers = [
        ("Might 2, Cadence 5  (volume)", hero("A", [2, 3, 1, 5, 1])),
        ("Might 3, Cadence 5  (volume)", hero("A", [3, 3, 1, 5, 1])),
        ("Might 5, Cadence 1  (punch) ", hero("A", [5, 3, 1, 1, 1])),
        ("Might 6, Cadence 1  (punch) ", hero("A", [6, 3, 1, 1, 1])),
    ];

    println!("=== 1v1: does the attacker's damage get through? ===\n");
    println!(
        "{:<32}{:>14}{:>16}",
        "attacker", "vs unarmored", "vs Armor 3"
    );
    for (label, atk) in &attackers {
        let bare = winnable(std::slice::from_ref(atk), &[target(0)]);
        let armored = winnable(std::slice::from_ref(atk), &[target(3)]);
        println!("{label:<32}{:>14}{:>16}", yn(bare), yn(armored));
    }

    println!(
        "\nReading it: under v2, volume (Cadence) beats the unarmored target as well as punch (Might) -\n\
         damage is fungible, so no target NEEDS high Might. Armor 3 flips that: Might <= 3 deals nothing no\n\
         matter the Cadence (loses), while Might > 3 penetrates (wins). Armor is the per-strike floor that\n\
         makes high single-target damage a hard requirement - the decisive tutorial gate.\n\
         (To force it to be RANGED too, pair armor with a melee-punish like riposte: then the answer is\n\
         specifically a high-damage RANGED strike.)"
    );
}

fn yn(win: bool) -> &'static str {
    if win { "WIN" } else { "lose" }
}
