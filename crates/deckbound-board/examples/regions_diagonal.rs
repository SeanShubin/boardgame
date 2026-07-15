//! **The balance diagonal, over the `rules` crate.** Does the encounter set still teach what it is meant to?
//!
//! This is the verification that the pure `rules` port reproduces the balance work: it drives
//! [`rules::combat`] (the regions model, behind the generic `Game`) via the generic [`Solver`], instead of the
//! old `deckbound_board::regions` copy. If it still reads 4/4 solos, the migration is faithful.
//!
//! - Each **solo** must be soloable by EXACTLY ONE kit - the one its keystone is built to be weak to.
//! - Each **corner** must be UNIQUELY vulnerable to a single kit: the full party wins, dropping that kit loses
//!   the fight, and dropping any other still clears it.
//!
//! Run: `cargo run --release -p deckbound-board --example regions_diagonal`

use std::time::Instant;

use deckbound_content::catalog::{self, Creature, Encounter};
use rules::combat::game::{Combat, State};
use rules::combat::resolve::{Combatant, Side};
use rules::core::{Solver, Verdict};

fn kit(spec: (&'static str, [u8; 5], &'static str)) -> Combatant {
    let (name, stats, ability) = spec;
    let (melee, ranged) = catalog::ability_reach(ability);
    let (_r, aoe) = catalog::ability_shape(ability);
    Combatant::from_stats(name, Side::Party, stats, 0, melee, ranged).with_aoe(aoe)
}

fn beast(c: &Creature) -> Combatant {
    Combatant::from_stats(c.name, Side::Foe, c.stats, 0, c.melee, c.ranged)
        .with_aoe(c.aoe)
        .as_horde(c.horde)
}

fn foes_of(e: &Encounter) -> Vec<Combatant> {
    let mut out = Vec::new();
    for (c, q) in catalog::encounter_foes(e) {
        for _ in 0..q {
            out.push(beast(c));
        }
    }
    out
}

/// **Can this party win, given its best formation?** The `Combat` game makes setup part of the search, so the
/// solver finds the best formation itself - one shared memo across the whole tree, no external formation loop.
/// The verdict is ground out to certainty (an escalating grant, doubling on `Evaluating`).
fn winnable(heroes: &[Combatant], foes: &[Combatant]) -> bool {
    let mut units: Vec<Combatant> = heroes.to_vec();
    units.extend_from_slice(foes);
    let s = State::new(units);

    let mut o = Solver::<Combat>::new();
    let mut grant = 1u64;
    loop {
        o.grant(grant);
        match o.verdict(&s) {
            Verdict::Winnable => return true,
            Verdict::Doomed => return false,
            Verdict::Evaluating => grant = grant.saturating_mul(2),
        }
    }
}

fn without(kits: &[Combatant], drop: usize) -> Vec<Combatant> {
    kits.iter()
        .enumerate()
        .filter(|(i, _)| *i != drop)
        .map(|(_, k)| k.clone())
        .collect()
}

fn main() {
    println!("regions_diagonal - does the encounter set still teach what it is meant to?");
    println!("(driving the pure `rules` crate through the generic Game + Solver)\n");
    let t0 = Instant::now();
    let kits: Vec<Combatant> = catalog::ROSTER.iter().copied().map(kit).collect();
    let names: Vec<&str> = catalog::ROSTER.iter().map(|k| k.0).collect();

    println!("SOLOS - each must be soloable by exactly ONE kit (its keystone's counter).\n");
    let mut solo_ok = 0;
    for e in catalog::ENCOUNTERS.iter().filter(|e| !e.party) {
        let foes = foes_of(e);
        let want = catalog::creature(e.keystone)
            .map(catalog::creature_counter)
            .unwrap_or("");
        let winners: Vec<&str> = kits
            .iter()
            .zip(&names)
            .filter(|(k, _)| winnable(std::slice::from_ref(*k), &foes))
            .map(|(_, n)| *n)
            .collect();
        let verdict = if winners == vec![want] {
            solo_ok += 1;
            "OK".to_string()
        } else if winners.is_empty() {
            "TOO HARD - no kit solos it".to_string()
        } else if winners.len() > 1 {
            format!("TOO SOFT - {} kits solo it", winners.len())
        } else {
            format!("WRONG - {} solos it, want {want}", winners[0])
        };
        println!(
            "  {:<20} {:<12} answer {want:<11} solos: {:<24} {verdict}",
            e.location,
            e.keystone,
            format!("{winners:?}")
        );
    }
    println!("\n  {solo_ok}/4 solos.\n");

    println!("CORNERS - each must be UNIQUELY vulnerable to one kit.\n");
    let mut corner_ok = 0;
    for e in catalog::ENCOUNTERS.iter().filter(|e| e.party) {
        let foes = foes_of(e);
        let want = catalog::creature(e.keystone)
            .map(catalog::creature_counter)
            .unwrap_or("");
        let whole = winnable(&kits, &foes);
        let needed: Vec<&str> = (0..kits.len())
            .filter(|&d| !winnable(&without(&kits, d), &foes))
            .map(|i| names[i])
            .collect();
        let verdict = if !whole {
            "BROKEN - full party loses".to_string()
        } else if needed == vec![want] {
            corner_ok += 1;
            "OK".to_string()
        } else if needed.is_empty() {
            "TOO SOFT - no kit necessary".to_string()
        } else if needed.len() > 1 {
            format!("NOT UNIQUE - {} needed", needed.len())
        } else {
            format!("WRONG - leans on {}, want {want}", needed[0])
        };
        println!(
            "  {:<20} {:<12} answer {want:<11} needs: {:<24} {verdict}",
            e.location,
            e.keystone,
            format!("{needed:?}")
        );
    }
    println!("\n  {corner_ok}/4 corners.");
    println!(
        "\nSCORE: {solo_ok}/4 solos, {corner_ok}/4 corners   ({} ms)",
        t0.elapsed().as_millis()
    );
}
