//! **The balance diagonal, measured in the regions model.**
//!
//! Two properties, and they are the whole design of the encounter set:
//!
//! 1. **Each solo (the four map cells adjacent to the inn) is soloable by EXACTLY ONE kit** - the kit its
//!    keystone creature is built to be weak to. Not "the counter kit can do it": *only* the counter kit can.
//!    A solo any two kits can beat teaches nothing; a solo no kit can beat is not a tutorial.
//!
//! 2. **Each corner (the four party encounters) is UNIQUELY VULNERABLE to a single kit.** The full party wins;
//!    drop that one kit and the fight becomes unwinnable; drop any *other* kit and it still falls. So exactly
//!    one hand is *necessary*, and the encounter is a lesson about that hand.
//!
//! The counters ([`catalog::creature_counter`]):
//!
//! | creature | its lock | the kit that answers it |
//! |---|---|---|
//! | The Wall | armored - a per-strike floor | **Raider** (one huge blow) |
//! | The Duelist | ripostes anything that closes | **Marksman** (answer from range) |
//! | The Swarm | a back-line horde | **Bastion** (a tough melee area survives the exchange) |
//! | The Storm | a front horde | **Bombardier** (ranged area, first-strike) |
//!
//! This measures where we ARE, so tuning has a target rather than a feeling. It changes no numbers itself.
//!
//! Run: `cargo run --release -p deckbound-board --example v2_regions_diagonal`

use std::time::Instant;

use deckbound_board::combat::{Combatant, Side};
use deckbound_board::regions::{Board, Oracle, Post};
use deckbound_content::catalog::{self, Creature, Encounter};
use deckbound_content::rank::Intention as Rank;

const BUDGET: u64 = 20_000_000;

fn kit(spec: (&'static str, [u8; 5], &'static str)) -> Combatant {
    let (name, stats, ability) = spec;
    let (melee, ranged) = catalog::ability_reach(ability);
    let (_r, aoe) = catalog::ability_shape(ability);
    Combatant::from_stats(name, Side::Party, Rank::Vanguard, stats, 0, melee, ranged).with_aoe(aoe)
}

fn beast(c: &Creature) -> Combatant {
    Combatant::from_stats(
        c.name,
        Side::Foe,
        Rank::Vanguard,
        c.stats,
        0,
        c.melee,
        c.ranged,
    )
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

/// Every partition of `n` heroes, as a restricted-growth string - each *partition* exactly once, never a mere
/// relabelling of one.
fn partitions(n: usize) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut rgs = vec![0u8; n];
    let mut going = true;
    while going {
        out.push(rgs.clone());
        going = false;
        for k in (1..n).rev() {
            let ceiling = rgs[..k].iter().copied().max().unwrap_or(0) + 1;
            if rgs[k] < ceiling {
                rgs[k] += 1;
                for x in rgs.iter_mut().skip(k + 1) {
                    *x = 0;
                }
                going = true;
                break;
            }
        }
    }
    out
}

/// **Can this party win this fight, given its BEST round-1 setup?**
///
/// The setup is a secret simultaneous commit, so the party gets to pick the best formation it has - the honest
/// control. Foes take one region of their own and post the natural way: shoot-only bodies behind the line.
fn winnable(heroes: &[Combatant], foes: &[Combatant]) -> bool {
    let n = heroes.len();
    let mut us: Vec<Combatant> = heroes.to_vec();
    us.extend_from_slice(foes);

    for p in partitions(n) {
        let foe_region = p.iter().copied().max().unwrap_or(0) + 1;
        for mask in 0..(1u32 << n) {
            let mut posts: Vec<Post> = (0..n)
                .map(|k| {
                    if (mask >> k) & 1 == 1 {
                        Post::Back
                    } else {
                        Post::Front
                    }
                })
                .collect();
            posts.extend(foes.iter().map(|f| {
                if f.ranged && !f.melee {
                    Post::Back
                } else {
                    Post::Front
                }
            }));

            let mut regions = p.clone();
            regions.extend(std::iter::repeat_n(foe_region, foes.len()));

            let b = Board::new(us.clone(), regions, posts);
            if Oracle::new(BUDGET).winnable(&b, 0, false) {
                return true;
            }
        }
    }
    false
}

fn roster() -> Vec<Combatant> {
    catalog::ROSTER.iter().copied().map(kit).collect()
}

fn main() {
    println!("v2_regions_diagonal - does the encounter set still teach what it is meant to?\n");
    let t0 = Instant::now();
    let kits = roster();
    let names: Vec<&str> = catalog::ROSTER.iter().map(|k| k.0).collect();

    // ---- the four solos: each must be soloable by EXACTLY ONE kit, and it must be the counter -----------
    println!(
        "SOLOS - each must be soloable by exactly ONE kit: the one its lock is built to be weak to.\n"
    );
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
            "TOO HARD - no kit can solo it; there is nothing to learn".to_string()
        } else if winners.len() > 1 {
            format!(
                "TOO SOFT - {} kits solo it, so it teaches nothing",
                winners.len()
            )
        } else {
            format!(
                "WRONG LESSON - only {} solos it, but it should be {want}",
                winners[0]
            )
        };
        println!(
            "  {:<20} keystone {:<14} answer: {want}",
            e.location, e.keystone
        );
        println!("      solos it: {:<40} {verdict}", format!("{winners:?}"));
    }
    println!("\n  {solo_ok}/4 solos teach the lesson they are built to teach.\n");

    // ---- the four corners: each must be UNIQUELY vulnerable to a single kit -----------------------------
    println!("----------------------------------------------------------------");
    println!("CORNERS - each must be UNIQUELY vulnerable to one kit: drop it and the fight is");
    println!("lost; drop any other and it still falls. Exactly one hand is NECESSARY.\n");

    let mut corner_ok = 0;
    for e in catalog::ENCOUNTERS.iter().filter(|e| e.party) {
        let foes = foes_of(e);
        let want = catalog::creature(e.keystone)
            .map(catalog::creature_counter)
            .unwrap_or("");

        let whole = winnable(&kits, &foes);

        // Which kits are NECESSARY - the ones whose absence loses the fight?
        let needed: Vec<&str> = (0..kits.len())
            .filter(|&drop| {
                let short: Vec<Combatant> = kits
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != drop)
                    .map(|(_, k)| k.clone())
                    .collect();
                !winnable(&short, &foes)
            })
            .map(|i| names[i])
            .collect();

        let verdict = if !whole {
            "BROKEN - the full party cannot win it at all".to_string()
        } else if needed == vec![want] {
            corner_ok += 1;
            "OK".to_string()
        } else if needed.is_empty() {
            "TOO SOFT - no kit is necessary; any three of them clear it".to_string()
        } else if needed.len() > 1 {
            format!("NOT UNIQUE - {} kits are each necessary", needed.len())
        } else {
            format!(
                "WRONG LESSON - it leans on {}, but should lean on {want}",
                needed[0]
            )
        };
        println!(
            "  {:<20} keystone {:<14} answer: {want}",
            e.location, e.keystone
        );
        println!(
            "      full party wins: {:<6} necessary kits: {:<32} {verdict}",
            whole,
            format!("{needed:?}")
        );
    }
    println!("\n  {corner_ok}/4 corners lean on exactly the hand they are meant to.\n");

    println!("----------------------------------------------------------------");
    println!(
        "SCORE: {solo_ok}/4 solos, {corner_ok}/4 corners   ({} ms)",
        t0.elapsed().as_millis()
    );
    println!();
    println!("A solo that several kits beat teaches nothing. A corner that needs no particular");
    println!("kit is a fight, not a lesson. Both failures read the same way from the outside -");
    println!("the player never discovers why the roster has four hands in it.");
}
