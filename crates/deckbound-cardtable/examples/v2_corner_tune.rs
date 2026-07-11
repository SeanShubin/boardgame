//! **Retune the four corners.** For each corner (leaning on kit `i`, keystone = `CREATURES[i]`), search small
//! foe compositions (keystone x1-2, each other creature x0-1) for one that:
//!   - the full four-kit party WINS,
//!   - NO single kit solos (it is a party fight),
//!   - and, ideally, the party WITHOUT kit `i` LOSES (it visibly leans on `i`).
//! Prefer a real lean, then fewer foes. Reports the best composition per corner (to wire into the catalog).
//!
//! Run: `cargo run --release -p deckbound-cardtable --example v2_corner_tune`

use deckbound::actor::Intention as Rank;
use deckbound::catalog::{self, Creature};
use deckbound_cardtable::combat::{Combatant, Side};
use deckbound_cardtable::solver::winnable;

fn rank_of(word: &str) -> Rank {
    match word {
        "Outrider" => Rank::Outrider,
        "Rearguard" => Rank::Rearguard,
        _ => Rank::Vanguard,
    }
}
fn kit_unit(i: usize) -> Combatant {
    let (name, stats, ability) = catalog::ROSTER[i];
    let (melee, ranged) = catalog::ability_reach(ability);
    let (_r, aoe) = catalog::ability_shape(ability);
    Combatant::from_stats(name, Side::Party, Rank::Vanguard, stats, 0, melee, ranged).with_aoe(aoe)
}
fn creature_unit(c: &Creature) -> Combatant {
    let rank = rank_of(catalog::creature_intention(c));
    Combatant::from_stats(c.name, Side::Foe, rank, c.stats, 0, c.melee, c.ranged)
        .with_aoe(c.aoe)
        .as_horde(c.horde)
}
fn party(mask: &[usize]) -> Vec<Combatant> {
    mask.iter().map(|&i| kit_unit(i)).collect()
}
fn foes(qty: [u32; 4]) -> Vec<Combatant> {
    let mut v = Vec::new();
    for (j, &q) in qty.iter().enumerate() {
        for _ in 0..q {
            v.push(creature_unit(&catalog::CREATURES[j]));
        }
    }
    v
}

fn main() {
    let all = [0usize, 1, 2, 3];
    for i in 0..4 {
        let without_i: Vec<usize> = all.iter().copied().filter(|&k| k != i).collect();
        // Candidate compositions: keystone i at 1 or 2, each other creature at 0 or 1.
        let mut best: Option<([u32; 4], bool, u32)> = None; // (qty, leans, total)
        for kq in 1..=2u32 {
            for a in 0..=1u32 {
                for b in 0..=1u32 {
                    for c in 0..=1u32 {
                        let mut qty = [0u32; 4];
                        qty[i] = kq;
                        let others: Vec<usize> = without_i.clone();
                        qty[others[0]] = a;
                        qty[others[1]] = b;
                        qty[others[2]] = c;
                        let total: u32 = qty.iter().sum();
                        if total < 2 {
                            continue; // a lone threat is an adjacent, not a corner
                        }
                        let e = foes(qty);
                        // Required: full party wins, no single kit solos.
                        if !winnable(&party(&all), &e) {
                            continue;
                        }
                        if (0..4).any(|k| winnable(std::slice::from_ref(&kit_unit(k)), &e)) {
                            continue;
                        }
                        let leans = !winnable(&party(&without_i), &e); // needs kit i
                        // Prefer: leans, then fewer foes. Keep the first best under that order.
                        let better = match best {
                            None => true,
                            Some((_, bl, bt)) => {
                                (leans, std::cmp::Reverse(total)) > (bl, std::cmp::Reverse(bt))
                            }
                        };
                        if better {
                            best = Some((qty, leans, total));
                        }
                    }
                }
            }
        }
        report(i, best);
    }
}

fn report(i: usize, best: Option<([u32; 4], bool, u32)>) {
    let kit = catalog::ROSTER[i].0;
    match best {
        None => println!(
            "corner leaning on {kit}: NO winnable+non-solo composition found (keystone x1-2)"
        ),
        Some((qty, leans, total)) => {
            let list: Vec<String> = qty
                .iter()
                .enumerate()
                .filter(|&(_, &q)| q > 0)
                .map(|(j, &q)| format!("{} x{q}", catalog::CREATURES[j].name))
                .collect();
            println!(
                "corner leaning on {kit:<8}: [{}]  ({total} foes)  {}",
                list.join(", "),
                if leans {
                    "LEANS on it (party without it loses)"
                } else {
                    "winnable+non-solo, but does not strictly need it"
                }
            );
        }
    }
}
