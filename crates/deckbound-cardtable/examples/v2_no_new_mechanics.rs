//! **Can the clean 4x4 diagonal be reached WITHOUT armor or riposte?** Each of the four creatures should be
//! soloable by exactly one kit. Two gates are intrinsically exclusive already (area vs a horde, Finesse vs an
//! evasive foe). This probe tries to co-tune the other two using only current mechanics:
//!   - Anvil (Executioner-only): lean on the Toughness *wipe* as a per-sub-phase burst threshold - raise the
//!     Anvil's Toughness into the gap ABOVE every other kit's burst but below the Executioner's (so the
//!     Executioner is re-tuned to out-burst the rest; its "one big blow" identity is dropped, only the
//!     diagonal is sought).
//!   - Coil (Marksman-only): lean on the Clash(R->V, sub-phase 4)-before-Breach(V->R, sub-phase 5) timing -
//!     a Rearguard strikes the enemy front a sub-phase before it is struck back, while a melee attacker
//!     trades simultaneously in V->V. Make the Coil hit hard + fast enough that the melee trade kills the
//!     melee kits, but the ranged Marksman's delayed exposure lets it outlast.
//!
//! If every column has exactly one WIN, current mechanics suffice. Any column with 0 or >1 WINs is a gate
//! that needs a new mechanic (armor / riposte) or more tuning.
//!
//! Run: `cargo run -p deckbound-cardtable --example v2_no_new_mechanics`

use deckbound::actor::Intention as Rank;
use deckbound::catalog;
use deckbound_cardtable::combat::{Combatant, Side};
use deckbound_cardtable::solver::winnable;

/// A party kit: `[Might, Vitality, Toughness, Cadence, Finesse]` + its attack's reach x spread (from the
/// catalog). Rank is the solver's choice.
fn kit(name: &str, stats: [u8; 5], ability: &str) -> Combatant {
    let (melee, ranged) = catalog::ability_reach(ability);
    let (_r, aoe) = catalog::ability_shape(ability);
    Combatant::from_stats(name, Side::Party, Rank::Vanguard, stats, 0, melee, ranged).with_aoe(aoe)
}

/// A candidate creature at an explicit rank (current mechanics only: no armor, no riposte).
fn foe(name: &str, rank: Rank, stats: [u8; 5], horde: bool) -> Combatant {
    Combatant::from_stats(name, Side::Foe, rank, stats, 0, true, false).as_horde(horde)
}

fn main() {
    // Co-tuned kits: Executioner re-tuned to the HIGHEST burst (Might 6 x Cadence 2 = 12); the rest keep
    // burst 8 or less. Phantom keeps the highest Finesse.
    let kits = [
        kit("Executioner", [6, 4, 1, 2, 1], "Jab"), // burst 12, Finesse 1
        kit("Broadsider", [2, 3, 3, 1, 1], "Sweep"), // area
        kit("Marksman", [4, 4, 1, 2, 2], "Shot"),   // ranged, burst 8
        kit("Phantom", [4, 3, 1, 2, 4], "Jab"),     // burst 8, Finesse 4
    ];

    // Candidate creatures, current mechanics only. Tuned to be JUST beatable by their one kit.
    let foes = [
        // Toughness 9: above every burst-8 kit, at/below the Executioner's burst 12. Low Vitality so the
        // Executioner's slow 1-card-per-Clash grind still finishes inside the round limit.
        foe("Anvil", Rank::Vanguard, [1, 3, 9, 1, 1], false),
        // Horde as an Outrider: Broadsider sweeps it in Intercept before it can raid; single strikes drown.
        foe("Swarm", Rank::Outrider, [1, 12, 1, 1, 1], true),
        // Low HP but high, fast offense. A melee kit trades in V->V (simultaneous) and mutual-dies (a loss);
        // the ranged Marksman strikes in Clash (R->V) and kills it before its Breach (V->R) counter ever
        // comes - so only range survives.
        foe("Coil", Rank::Vanguard, [5, 3, 1, 2, 2], false),
        // Finesse 5 so only a fast hand LANDS (Executioner/Marksman can't reach its Finesse); Toughness 7 so
        // sub-bar blows are wiped - that shuts out Broadsider's area sweep AND the Executioner's unevadable
        // strike-back (Might 6 < 7). Only Phantom both lands (Finesse 4) and out-bursts the Toughness (8 > 7).
        foe("Mirage", Rank::Vanguard, [1, 4, 7, 1, 5], false),
    ];

    // The DESIGNATED counter for each creature column (the identity diagonal we actually want).
    let designated = ["Executioner", "Broadsider", "Marksman", "Phantom"];

    println!("=== 4x4 solo winnability with NO armor / NO riposte (co-tuned) ===\n");
    print!("{:<14}", "kit \\ foe");
    for f in &foes {
        print!("{:>12}", f.name);
    }
    println!();

    // winners[col] = the kit names that beat that creature.
    let mut winners: [Vec<&str>; 4] = [vec![], vec![], vec![], vec![]];
    for k in &kits {
        print!("{:<14}", k.name);
        for (col, f) in foes.iter().enumerate() {
            let w = winnable(std::slice::from_ref(k), std::slice::from_ref(f));
            print!("{:>12}", if w { "WIN" } else { "--" });
            if w {
                winners[col].push(&k.name);
            }
        }
        println!();
    }

    println!("\nper creature (want: beaten ONLY by its designated kit):");
    let mut clean = true;
    for (col, f) in foes.iter().enumerate() {
        let ws = &winners[col];
        let want = designated[col];
        let ok = ws.len() == 1 && ws[0] == want;
        if !ok {
            clean = false;
        }
        let note = if ok {
            format!("OK - only {want}")
        } else if ws.is_empty() {
            "UNBEATABLE (over-tuned)".to_string()
        } else {
            format!("want {want}, got [{}]", ws.join(", "))
        };
        println!("  {:<8} {note}", f.name);
    }
    println!(
        "\n{}",
        if clean {
            "CLEAN IDENTITY DIAGONAL with current mechanics - armor / riposte NOT needed for the regions."
        } else {
            "NOT clean - a mismatched column needs more tuning, or a mechanic (armor / riposte) it lacks."
        }
    );
}
