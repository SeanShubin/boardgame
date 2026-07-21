//! **The STEP-MACHINE diagonal probe** - the same balance grid as `regions_diagonal`, driven through the
//! in-flight step machine ([`rules::combat::step_game`]) instead of the shipped wave model, printed SIDE BY
//! SIDE with the wave model's answers. **A probe, not a gate**: it asserts nothing - it exists so the
//! step-machine restructure is judged by measurement before any consumer switches (stage C of the plan in
//! `needs-merge/round-sequence.md`).
//!
//! Reading the grid: `wave -> step` per cell, `OK` when they agree, `DIFF` when the step machine changes the
//! answer. The wave column is the measured, shipped balance (4/4 solos, 5/5 party fights); every `DIFF` is a
//! real behavioral consequence of the reorder (early trade, uncontested walk, same-round advance).
//!
//! Run: `cargo run --release -p deckbound-board --example steps_diagonal`

use std::time::Instant;

use deckbound_board::units::{beast, kit};
use deckbound_board::verify::{
    greedy_wins_steps, insight_class, insight_class_steps, solver_wins, solver_wins_steps,
};
use deckbound_content::catalog::{self, Encounter};
use rules::combat::game::{ClashOnly, Combat};
use rules::combat::resolve::Combatant;
use rules::combat::step_game::{StepClashOnly, StepCombat};

fn foes_of(e: &Encounter) -> Vec<Combatant> {
    let mut out = Vec::new();
    for (c, q) in catalog::encounter_foes(e) {
        for _ in 0..q {
            out.push(beast(c));
        }
    }
    out
}

fn mark(same: bool) -> &'static str {
    if same { "OK" } else { "DIFF" }
}

fn main() {
    let t0 = Instant::now();
    println!("steps_diagonal - the step machine vs the shipped wave model, cell by cell.");
    println!("(wave -> step; OK = agreement, DIFF = the reorder changed the answer)\n");

    let kits: Vec<Combatant> = catalog::ROSTER.iter().copied().map(kit).collect();
    let names: Vec<&str> = catalog::ROSTER.iter().map(|(n, _, _)| *n).collect();

    // ---- SOLOS: who can solo each keystone, wave vs step. ----------------------------------------------
    println!("SOLOS - which kits can solo each keystone (expected: exactly its counter).");
    for e in catalog::ENCOUNTERS.iter().filter(|e| !e.party) {
        let foes = foes_of(e);
        let wave: Vec<&str> = names
            .iter()
            .zip(&kits)
            .filter(|(_, k)| solver_wins::<Combat>(std::slice::from_ref(k), &foes))
            .map(|(n, _)| *n)
            .collect();
        let step: Vec<&str> = names
            .iter()
            .zip(&kits)
            .filter(|(_, k)| solver_wins_steps::<StepCombat>(std::slice::from_ref(k), &foes))
            .map(|(n, _)| *n)
            .collect();
        println!(
            "  {:<20} {:<6} wave {:?} -> step {:?}",
            e.location,
            mark(wave == step),
            wave,
            step
        );
    }

    // ---- CORNERS + CAPSTONE: insight classes per sub-party, wave vs step. ------------------------------
    let melee: Vec<Combatant> = kits.iter().filter(|k| k.melee).cloned().collect();
    let ranged: Vec<Combatant> = kits
        .iter()
        .filter(|k| k.ranged && !k.melee)
        .cloned()
        .collect();
    let single: Vec<Combatant> = kits.iter().filter(|k| !k.aoe).cloned().collect();
    let area: Vec<Combatant> = kits.iter().filter(|k| k.aoe).cloned().collect();

    println!(
        "\nCORNERS + CAPSTONE - insight class per sub-party (T greedy wins / I solver only / X neither)."
    );
    let cells: Vec<(&str, &str, &[Combatant])> = vec![
        ("Emberfall Hollow", "full", &kits),
        ("Emberfall Hollow", "single", &single),
        ("Emberfall Hollow", "area", &area),
        ("Greywater Ford", "full", &kits),
        ("Greywater Ford", "ranged", &ranged),
        ("Greywater Ford", "melee", &melee),
        ("Ninefold Deep", "full", &kits),
        ("Ninefold Deep", "area", &area),
        ("Ninefold Deep", "single", &single),
        ("The Hollow Rampart", "full", &kits),
        ("Ashfen Crossing", "full", &kits),
        ("Ashfen Crossing", "melee", &melee),
        ("Ashfen Crossing", "ranged", &ranged),
        ("Ashfen Crossing", "single", &single),
    ];
    for (loc, label, heroes) in cells {
        let e = catalog::ENCOUNTERS
            .iter()
            .find(|e| e.location == loc)
            .unwrap();
        let foes = foes_of(e);
        let w = insight_class(heroes, &foes);
        let s = insight_class_steps(heroes, &foes);
        println!("  {loc:<20} {label:<7} {:<6} {w} -> {s}", mark(w == s));
    }

    // ---- The clash-only controls: the raid must stay load-bearing. -------------------------------------
    println!("\nCLASH-ONLY CONTROLS - must LOSE (the raid carries these fights).");
    for loc in ["The Hollow Rampart", "Ashfen Crossing"] {
        let e = catalog::ENCOUNTERS
            .iter()
            .find(|e| e.location == loc)
            .unwrap();
        let foes = foes_of(e);
        let w = solver_wins::<ClashOnly>(&kits, &foes);
        let s = solver_wins_steps::<StepClashOnly>(&kits, &foes);
        println!(
            "  {loc:<20} {:<6} wave {} -> step {}",
            mark(w == s),
            if w { "WINS (bad)" } else { "loses" },
            if s { "WINS (bad)" } else { "loses" }
        );
    }

    // ---- The greedy baseline, for texture. -------------------------------------------------------------
    println!("\nGREEDY BASELINE (step machine) - the self-play line per party fight.");
    for e in catalog::ENCOUNTERS.iter().filter(|e| e.party) {
        let foes = foes_of(e);
        println!(
            "  {:<20} greedy {}",
            e.location,
            if greedy_wins_steps(&kits, &foes) {
                "wins"
            } else {
                "loses"
            }
        );
    }

    println!("\n({} ms)", t0.elapsed().as_millis());
}
