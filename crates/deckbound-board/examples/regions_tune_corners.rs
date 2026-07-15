//! **Tune the corners: the smallest warband that leans on exactly one hand** - over the `rules` crate.
//!
//! The four solos lock the creature stats (a solo is one creature), so the only corner lever is the
//! **composition**: which creatures, how many. That is also the low-numbers lever - it raises counts, not stats.
//! Compositions are walked smallest-first, so whatever comes back is the fewest bodies that teach the lesson.
//!
//! Drives [`rules::combat`] through the generic [`Solver`]; it changes no files, it prints what to write.
//!
//! Run: `cargo run --release -p deckbound-board --example regions_tune_corners`

use std::time::Instant;

use deckbound_content::catalog::{self, Creature};
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

/// Can the party win, given its best formation? (The `Combat` game searches the formation itself.)
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

/// Is this warband uniquely vulnerable to `want`? The counter is checked SECOND so a warband that does not need
/// it is rejected after two searches, not five (most candidates are rejections).
fn uniquely_needs(kits: &[Combatant], want: usize, foes: &[Combatant]) -> bool {
    if !winnable(kits, foes) {
        return false;
    }
    if winnable(&without(kits, want), foes) {
        return false;
    }
    (0..kits.len())
        .filter(|&i| i != want)
        .all(|i| winnable(&without(kits, i), foes))
}

/// Every warband worth trying, smallest first, keystone always present.
fn warbands(keystone: usize, n: usize, max_bodies: u32) -> Vec<Vec<u32>> {
    let mut out: Vec<Vec<u32>> = Vec::new();
    fn walk(
        k: usize,
        n: usize,
        c: &mut Vec<u32>,
        keystone: usize,
        cap: u32,
        out: &mut Vec<Vec<u32>>,
    ) {
        if k == n {
            let total: u32 = c.iter().sum();
            if c[keystone] >= 1 && (2..=cap).contains(&total) {
                out.push(c.clone());
            }
            return;
        }
        for q in 0..=4u32 {
            c[k] = q;
            walk(k + 1, n, c, keystone, cap, out);
        }
        c[k] = 0;
    }
    walk(0, n, &mut vec![0; n], keystone, max_bodies, &mut out);
    out.sort_by_key(|c| (c.iter().sum::<u32>(), c.iter().filter(|&&q| q > 0).count()));
    out
}

fn main() {
    println!("regions_tune_corners - the smallest warband that leans on exactly one hand\n");
    let t0 = Instant::now();
    let kits: Vec<Combatant> = catalog::ROSTER.iter().copied().map(kit).collect();
    let names: Vec<&str> = catalog::ROSTER.iter().map(|k| k.0).collect();
    let creatures: Vec<&Creature> = catalog::CREATURES.iter().collect();

    let mut solved = 0;
    for e in catalog::ENCOUNTERS.iter().filter(|e| e.party) {
        let keystone = creatures
            .iter()
            .position(|c| c.name == e.keystone)
            .expect("keystone");
        let want = catalog::creature_counter(creatures[keystone]);
        let want_i = names.iter().position(|n| *n == want).expect("counter kit");
        println!(
            "{} - keystone {} -> must need {want}",
            e.location, e.keystone
        );

        let mut found = None;
        let mut tried = 0;
        for counts in warbands(keystone, creatures.len(), 6) {
            let foes: Vec<Combatant> = creatures
                .iter()
                .zip(&counts)
                .flat_map(|(c, &q)| std::iter::repeat_n(beast(c), q as usize))
                .collect();
            tried += 1;
            if uniquely_needs(&kits, want_i, &foes) {
                found = Some((counts, foes.len()));
                break;
            }
        }
        match found {
            Some((counts, bodies)) => {
                solved += 1;
                let spec: Vec<String> = creatures
                    .iter()
                    .zip(counts.iter())
                    .filter(|&(_, &q)| q > 0)
                    .map(|(c, &q)| format!("(\"{}\", {q})", c.name))
                    .collect();
                println!(
                    "      FOUND after {tried} tries, {bodies} bodies:  foes: &[{}],",
                    spec.join(", ")
                );
            }
            None => println!("      NOTHING WORKS in {tried} warbands of up to 6 bodies."),
        }
    }
    println!(
        "\n{solved}/4 corners solved   ({} ms)",
        t0.elapsed().as_millis()
    );
}
