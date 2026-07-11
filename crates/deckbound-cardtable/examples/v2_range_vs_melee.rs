//! **Can the screen alone force RANGE (no riposte)?** Against a hard-hitting **armored** wall, compare party
//! shapes: a melee bruiser trades with the wall (V->V, mutual), a ranged cannon fires from safety behind a
//! screen. If [screen + ranged cannon] wins where the melee options lose, then range is forced by the screen
//! + attrition alone - a riposte mechanic would be redundant. If melee-with-a-screen also wins, the screen
//! isn't enough and a melee-punish (riposte) is what makes range decisive.
//!
//! Run: `cargo run -p deckbound-cardtable --example v2_range_vs_melee`

use deckbound::actor::Intention as Rank;
use deckbound_cardtable::combat::{Combatant, Side};
use deckbound_cardtable::solver::winnable;

fn party(name: &str, stats: [u8; 5], melee: bool, ranged: bool) -> Combatant {
    Combatant::from_stats(name, Side::Party, Rank::Vanguard, stats, 0, melee, ranged)
}

fn main() {
    // A hard-hitting ARMORED wall: Might 4 (punishes whoever trades), Armor 3 (needs high Might to dent),
    // Vitality 6 + Toughness 2 (a wall), Cadence 2 (keeps swinging).
    let wall = Combatant::from_stats(
        "Wall",
        Side::Foe,
        Rank::Vanguard,
        [4, 6, 2, 2, 2],
        3,
        true,
        false,
    );

    // A tanky screen (soaks, can't dent the wall itself: Might 1 < Armor 3 -> 0 damage).
    let screen = party("Screen", [1, 9, 3, 1, 2], true, false);
    // A high-Might cracker (Might 7 -> 4 through Armor 3). Same stats, two reaches.
    let melee_cracker = party("MeleeCracker", [7, 4, 1, 2, 2], true, false);
    let ranged_cracker = party("RangedCannon", [7, 4, 1, 2, 2], false, true);

    let cases: [(&str, Vec<Combatant>); 4] = [
        ("lone melee cracker", vec![melee_cracker.clone()]),
        (
            "lone ranged cannon (no screen!)",
            vec![ranged_cracker.clone()],
        ),
        (
            "screen + melee cracker",
            vec![screen.clone(), melee_cracker.clone()],
        ),
        (
            "screen + ranged cannon",
            vec![screen.clone(), ranged_cracker.clone()],
        ),
    ];

    println!("=== vs one hard-hitting ARMORED wall: does the party win? ===\n");
    for (label, p) in &cases {
        println!(
            "  {:<36} {}",
            label,
            if winnable(p, &[wall.clone()]) {
                "WIN"
            } else {
                "lose"
            }
        );
    }

    println!(
        "\nReading it:\n\
         - If only 'screen + ranged cannon' wins -> the screen + attrition force RANGE on their own; riposte\n\
           is redundant for this gate.\n\
         - If 'screen + melee cracker' also wins -> a screen protects melee too, so range is only preferred;\n\
           a melee-punish (riposte) is what would make range decisive.\n\
         - 'lone ranged cannon' should lose: with no screen the Rearguard is exposed and the wall shoots back."
    );
}
