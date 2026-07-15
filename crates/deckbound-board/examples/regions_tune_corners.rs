//! **Tune the corners: find the smallest warband that leans on exactly one hand.**
//!
//! The four solos are locked at 4/4 (`v2_regions_diagonal`), and they lock the **creature stats** with them -
//! a solo is one creature, so changing a creature's numbers changes its solo. So the only lever left for the
//! corners is the **composition**: which creatures, and how many. That is also the *low-numbers* lever, because
//! it raises **counts** rather than stats.
//!
//! A corner is right when it is **uniquely vulnerable to one kit**: the full party wins, dropping that kit
//! loses the fight, and dropping any *other* kit still clears it. Exactly one hand is **necessary**, and the
//! encounter is a lesson about that hand.
//!
//! The search walks compositions in order of **increasing total bodies** and takes the first that works - so
//! whatever it finds is the smallest warband that teaches the lesson. It changes no files; it prints what to
//! write.
//!
//! Run: `cargo run --release -p deckbound-board --example regions_tune_corners`

use std::time::Instant;

use deckbound_board::combat::{Combatant, Side};
use deckbound_board::regions::{Board, Oracle, Post};
use deckbound_content::catalog::{self, Creature};
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

/// Can this party win, given its best round-1 setup? Foes take one region, posted the natural way: shoot-only
/// bodies hold the back.
///
/// `split` decides whether the party may **partition itself across several regions**, or must field one line.
///
/// The search runs with `split = false` - one region, posts only: **16 formations instead of 240**, and the
/// search is 15x faster for it. That is safe *in one direction only*, and the direction matters: restricting
/// the party's options can only make a fight look **harder**, never easier. So a kit can be falsely reported
/// **necessary**, never falsely reported unnecessary - which means the fast pass can only ever hand us a
/// **candidate**, and every candidate is then re-checked with `split = true` before it is believed.
fn winnable_with(heroes: &[Combatant], foes: &[Combatant], split: bool) -> bool {
    let n = heroes.len();
    let mut us: Vec<Combatant> = heroes.to_vec();
    us.extend_from_slice(foes);

    let parts = if split {
        partitions(n)
    } else {
        vec![vec![0u8; n]]
    };

    // **ONE oracle across every formation.** The memo is what makes the second question cheap, and a fresh
    // Oracle per formation throws it away up to 240 times per call - which is exactly the mistake that made the
    // in-app doom chart slow. Different formations key differently at the root, but the positions they collapse
    // into a round or two later are largely the SAME positions, and each one now gets settled once.
    let mut oracle = Oracle::new(BUDGET);
    for p in parts {
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
            oracle.grant(BUDGET);
            if oracle.winnable(&b, 0, false) {
                return true;
            }
        }
    }
    false
}

fn without(kits: &[Combatant], drop: usize) -> Vec<Combatant> {
    kits.iter()
        .enumerate()
        .filter(|(i, _)| *i != drop)
        .map(|(_, k)| k.clone())
        .collect()
}

/// **Is this warband uniquely vulnerable to `want`?** - the full party wins, dropping `want` loses it, and
/// dropping any *other* kit still clears it.
///
/// Ordered to reject fast, because nearly every candidate is a rejection: the **counter is checked second**, so
/// a warband that does not actually need it is thrown out after two searches instead of five. On a search that
/// tries hundreds of warbands and accepts one, that is most of the running time.
fn uniquely_needs(kits: &[Combatant], want: usize, foes: &[Combatant], split: bool) -> bool {
    if !winnable_with(kits, foes, split) {
        return false; // the party cannot win it at all - not a lesson, just a wall
    }
    if winnable_with(&without(kits, want), foes, split) {
        return false; // it wins WITHOUT the hand it is supposed to lean on: no lesson
    }
    // ...and it must not lean on anything else.
    (0..kits.len())
        .filter(|&i| i != want)
        .all(|i| winnable_with(&without(kits, i), foes, split))
}

/// Every warband worth trying, **smallest first**: the keystone is always present (it is the encounter's
/// identity), and we prefer few bodies to many - low numbers, as asked.
fn warbands(keystone: usize, n_creatures: usize, max_bodies: u32) -> Vec<Vec<u32>> {
    let mut out: Vec<Vec<u32>> = Vec::new();
    let cap = 4u32;
    let mut counts = vec![0u32; n_creatures];
    fn walk(
        k: usize,
        n: usize,
        cap: u32,
        counts: &mut Vec<u32>,
        keystone: usize,
        max_bodies: u32,
        out: &mut Vec<Vec<u32>>,
    ) {
        if k == n {
            let total: u32 = counts.iter().sum();
            if counts[keystone] >= 1 && total >= 2 && total <= max_bodies {
                out.push(counts.clone());
            }
            return;
        }
        for q in 0..=cap {
            counts[k] = q;
            walk(k + 1, n, cap, counts, keystone, max_bodies, out);
        }
        counts[k] = 0;
    }
    walk(
        0,
        n_creatures,
        cap,
        &mut counts,
        keystone,
        max_bodies,
        &mut out,
    );

    // Smallest first: fewest bodies, then fewest *kinds* (a cleaner warband reads better), then most keystone.
    out.sort_by_key(|c| {
        (
            c.iter().sum::<u32>(),
            c.iter().filter(|&&q| q > 0).count(),
            std::cmp::Reverse(c[keystone]),
        )
    });
    out
}

fn main() {
    println!("regions_tune_corners - the smallest warband that leans on exactly one hand\n");
    println!("Creature STATS are locked by the 4/4 solos (a solo is one creature), so the only");
    println!("lever is composition. Compositions are walked SMALLEST FIRST, so whatever comes");
    println!("back is the fewest bodies that teach the lesson.\n");

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

        println!(
            "{} - keystone {} -> must need {want}",
            e.location, e.keystone
        );

        let want_i = names.iter().position(|n| *n == want).expect("counter kit");
        let mut found = None;
        let mut tried = 0;
        for counts in warbands(keystone, creatures.len(), 6) {
            let foes: Vec<Combatant> = creatures
                .iter()
                .zip(&counts)
                .flat_map(|(c, &q)| std::iter::repeat_n(beast(c), q as usize))
                .collect();
            tried += 1;
            // Fast pass: one party line. It can only over-report necessity, never under-report it.
            if !uniquely_needs(&kits, want_i, &foes, false) {
                continue;
            }
            // Candidate. Now believe it only if it survives the party's FULL freedom to split its formation.
            if uniquely_needs(&kits, want_i, &foes, true) {
                found = Some((counts, foes.len()));
                break;
            }
            println!("      (a candidate at {tried} did not survive the full formation check)");
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
                println!("      FOUND after {tried} tries, {bodies} bodies:");
                println!("      foes: &[{}],", spec.join(", "));
            }
            None => println!("      NOTHING WORKS in {tried} warbands of up to 6 bodies."),
        }
        println!();
    }

    println!("----------------------------------------------------------------");
    println!(
        "{solved}/4 corners solved   ({} ms)",
        t0.elapsed().as_millis()
    );
    if solved < 4 {
        println!();
        println!(
            "A corner that no composition can fix is not a tuning problem: it means the party"
        );
        println!(
            "has no hand that is UNIQUELY needed against that lock, however much of it we pile"
        );
        println!("up. That would be a roster problem, and worth knowing.");
    }
}
