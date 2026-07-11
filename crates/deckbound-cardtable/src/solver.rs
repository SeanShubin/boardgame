//! **Exact v2 winnability solver.** Answers "can the party *force* a win against the scripted greedy foe?"
//! by searching every party line (formation + per-mini-phase allocations) and the deterministic greedy foe
//! response through the [`crate::combat`] resolvers. The foe is a fixed policy, so this is a single-agent
//! reachability search: the party is winnable iff *some* sequence of its choices reaches a party-win state.
//!
//! Tractability rests on the v2 structure (memory `combat-frozen-revisit-after-tooling`): resolution is
//! order-free and deterministic, targets are SCHEDULE-gated, and bids are **threshold contests** — so the
//! only catch bids worth trying are `min-to-land` and `min-to-deny-evade`, not the whole `0..tempo` range.
//! States are memoized at sub-phase boundaries. Exponential in party size in the worst case; trivial for the
//! solo duel-locks matchups, which is what the diagonal balance property needs.

use std::collections::HashMap;

use deckbound::actor::Intention as Rank;
use deckbound::combat::SCHEDULE;

use crate::battle::{Greedy, MAX_ROUNDS, Policy};
use crate::combat::{self, Catch, Combatant, Contact, ExtraStrike, React, Side};

/// The three ranks a party unit may be assigned (the formation search space).
const RANKS: [Rank; 3] = [Rank::Vanguard, Rank::Outrider, Rank::Rearguard];

/// Whether the party can **force a win** vs the scripted greedy foe. `party` come rank-less (each formation
/// overwrites their `rank`); `foes` keep their scripted ranks. The party picks one formation for the battle
/// (round-0 ranks, held — a sufficient condition: if some fixed formation wins, the party wins).
pub fn winnable(party: &[Combatant], foes: &[Combatant]) -> bool {
    let n = party.len();
    for f in 0..3usize.pow(n as u32) {
        let mut units: Vec<Combatant> = party
            .iter()
            .enumerate()
            .map(|(k, p)| {
                let mut u = p.clone();
                u.rank = RANKS[(f / 3usize.pow(k as u32)) % 3];
                u
            })
            .collect();
        units.extend(foes.iter().cloned());
        let mut memo = HashMap::new();
        if forces_win(&units, 0, 0, &mut memo) {
            return true;
        }
    }
    false
}

/// A memo key: the mutable state (per unit health/tempo/fallen) plus the walk position.
type Key = (Vec<(u32, u32, bool)>, usize, usize);

fn key_of(units: &[Combatant], round: usize, sub: usize) -> Key {
    (
        units
            .iter()
            .map(|u| (u.health, u.tempo, u.fallen))
            .collect(),
        round,
        sub,
    )
}

fn party_won(units: &[Combatant]) -> Option<bool> {
    let party = units.iter().any(|u| u.side == Side::Party && !u.fallen);
    let foes = units.iter().any(|u| u.side == Side::Foe && !u.fallen);
    match (party, foes) {
        (true, true) => None,
        (won, _) => Some(won),
    }
}

/// Can the party force a win from the start of sub-phase `sub` in `round`?
fn forces_win(
    units: &[Combatant],
    round: usize,
    sub: usize,
    memo: &mut HashMap<Key, bool>,
) -> bool {
    if let Some(done) = party_won(units) {
        return done;
    }
    if round >= MAX_ROUNDS {
        return false; // hit the round cap undecided = a draw, not a win
    }
    let key = key_of(units, round, sub);
    if let Some(&r) = memo.get(&key) {
        return r;
    }
    let mut units = units.to_vec();
    if sub == 0 {
        combat::refresh_round(&mut units); // Tempo refreshes to Cadence each round
    }
    let win = search_catch(&units, round, sub, memo);
    memo.insert(key, win);
    win
}

/// The **next** walk position after resolving sub-phase `sub`.
fn next(round: usize, sub: usize) -> (usize, usize) {
    if sub + 1 < SCHEDULE.len() {
        (round, sub + 1)
    } else {
        (round + 1, 0)
    }
}

/// Catch step: try every party catch plan (joint over attackers), fold in the greedy foe, resolve, recurse
/// into React.
fn search_catch(
    units: &[Combatant],
    round: usize,
    sub: usize,
    memo: &mut HashMap<Key, bool>,
) -> bool {
    let foe_catches = Greedy.catches(units, Side::Foe, sub);
    let options = party_catch_options(units, sub);
    any_combo(&options, &mut |chosen| {
        let mut u = units.to_vec();
        let mut all: Vec<Catch> = chosen.iter().flatten().copied().collect();
        all.extend(foe_catches.iter().copied());
        let contacts = combat::resolve_catch(&mut u, &all);
        search_react(&u, &contacts, round, sub, memo)
    })
}

/// React step: try every party reaction plan (one per party-targeted contact), fold in the greedy foe
/// reactions, resolve, recurse into Extra.
fn search_react(
    units: &[Combatant],
    contacts: &[Contact],
    round: usize,
    sub: usize,
    memo: &mut HashMap<Key, bool>,
) -> bool {
    // Which contacts hit a party unit (the party chooses their reactions); foe-targeted ones are greedy.
    let party_hits: Vec<usize> = (0..contacts.len())
        .filter(|&i| units[contacts[i].target].side == Side::Party)
        .collect();
    let options: Vec<Vec<React>> = party_hits
        .iter()
        .map(|&i| react_options(units, &contacts[i]))
        .collect();
    any_combo(&options, &mut |chosen| {
        let reactions: Vec<React> = contacts
            .iter()
            .enumerate()
            .map(|(i, c)| {
                if let Some(pos) = party_hits.iter().position(|&h| h == i) {
                    chosen[pos]
                } else {
                    Greedy.react(units, c)
                }
            })
            .collect();
        let mut u = units.to_vec();
        let surviving = combat::resolve_react(&mut u, contacts, &reactions);
        search_extra(&u, &surviving, round, sub, memo)
    })
}

/// Extra step: try every party extra-strike plan (how many cards each still-contacted party unit flips),
/// fold in the greedy foe, resolve, close the sub-phase, recurse to the next sub-phase.
fn search_extra(
    units: &[Combatant],
    surviving: &[Contact],
    round: usize,
    sub: usize,
    memo: &mut HashMap<Key, bool>,
) -> bool {
    let foe_extras = Greedy.extras(units, Side::Foe, surviving);
    // Party units still on a surviving contact may flip 0..tempo cards each.
    let party_edges: Vec<&Contact> = surviving
        .iter()
        .filter(|c| units[c.attacker].side == Side::Party && units[c.attacker].tempo > 0)
        .collect();
    let options: Vec<Vec<ExtraStrike>> = party_edges
        .iter()
        .map(|c| {
            (0..=units[c.attacker].tempo)
                .map(|cards| ExtraStrike {
                    attacker: c.attacker,
                    target: c.target,
                    cards,
                })
                .collect()
        })
        .collect();
    let (nr, ns) = next(round, sub);
    any_combo(&options, &mut |chosen| {
        let mut u = units.to_vec();
        let mut extras: Vec<ExtraStrike> = chosen.iter().filter(|e| e.cards > 0).copied().collect();
        extras.extend(foe_extras.iter().copied());
        combat::resolve_extra(&mut u, &extras);
        combat::end_sub_phase(&mut u);
        forces_win(&u, nr, ns, memo)
    })
}

// ---- the party's pruned option sets -------------------------------------------------------------------

/// Each party attacker's catch options this sub-phase: `None` (don't catch), plus for each legal + reachable
/// + affordable foe the two canonical bids — **min-to-land** and **min-to-deny-evade** (enough that the
/// defender can't out-bid it). Intermediate bids only waste Tempo, so they are pruned.
fn party_catch_options(units: &[Combatant], sub: usize) -> Vec<Vec<Option<Catch>>> {
    units
        .iter()
        .enumerate()
        .filter(|(_, u)| {
            u.side == Side::Party
                && !u.fallen
                && u.tempo > 0
                && combat::effective_in_rank(u.rank, u.melee, u.ranged)
        })
        .map(|(i, u)| {
            let mut opts = vec![None];
            for (j, v) in units.iter().enumerate() {
                if v.fallen
                    || v.side == Side::Party
                    || !combat::legal_catch(sub, u.rank, v.rank)
                    || !combat::back_access_ok(units, u.rank, j)
                {
                    continue;
                }
                if u.aoe {
                    // An area strike is one unevadable sweep of the target's rank — no bid to tune, one card.
                    if u.tempo > 0 {
                        opts.push(Some(Catch {
                            attacker: i,
                            target: j,
                            cards: 1,
                        }));
                    }
                    continue;
                }
                let min_land = v.finesse.div_ceil(u.finesse.max(1)).max(1);
                if min_land > u.tempo {
                    continue; // can't even land
                }
                // Deny-evade: bid value must reach the defender's whole Tempo-at-Finesse, so cards such that
                // cards * F_att >= tempo_def * F_def.
                let deny = (v.tempo * v.finesse)
                    .div_ceil(u.finesse.max(1))
                    .max(min_land);
                for cards in [min_land, deny] {
                    if cards <= u.tempo {
                        opts.push(Some(Catch {
                            attacker: i,
                            target: j,
                            cards,
                        }));
                    }
                }
            }
            dedup(opts)
        })
        .collect()
}

/// A party defender's reactions to one incoming `contact`: Eat (free); Evade (min cards to strictly beat the
/// bid, if affordable); Strike Back (if the blow is melee and the defender carries a melee answer + Tempo).
fn react_options(units: &[Combatant], contact: &Contact) -> Vec<React> {
    let d = &units[contact.target];
    let mut opts = vec![React::Eat];
    let need = contact.bid / d.finesse.max(1) + 1;
    if need > 0 && need <= d.tempo {
        opts.push(React::Evade { cards: need });
    }
    let incoming_melee = !combat::rank_is_ranged(units[contact.attacker].rank);
    if incoming_melee && d.melee && d.tempo > 0 {
        opts.push(React::StrikeBack);
    }
    opts
}

fn dedup(mut v: Vec<Option<Catch>>) -> Vec<Option<Catch>> {
    v.sort_by_key(|o| o.map(|c| (c.target, c.cards)));
    v.dedup();
    v
}

/// Try every combination of one option per slot; return `true` as soon as `f` accepts one (short-circuit).
fn any_combo<T: Clone>(options: &[Vec<T>], f: &mut dyn FnMut(&[T]) -> bool) -> bool {
    fn go<T: Clone>(
        options: &[Vec<T>],
        i: usize,
        acc: &mut Vec<T>,
        f: &mut dyn FnMut(&[T]) -> bool,
    ) -> bool {
        if i == options.len() {
            return f(acc);
        }
        for opt in &options[i] {
            acc.push(opt.clone());
            if go(options, i + 1, acc, f) {
                return true;
            }
            acc.pop();
        }
        false
    }
    go(options, 0, &mut Vec::new(), f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::play_battle;

    fn unit(
        name: &str,
        side: Side,
        might: u32,
        finesse: u32,
        cadence: u32,
        toughness: u32,
        vitality: u32,
        melee: bool,
        ranged: bool,
    ) -> Combatant {
        Combatant {
            name: name.into(),
            side,
            rank: Rank::Vanguard, // overwritten per formation for party; scripted for foes
            might,
            finesse,
            cadence,
            toughness,
            armor: 0,
            melee,
            ranged,
            aoe: false,
            horde: false,
            tempo: cadence,
            health: vitality,
            pending: 0,
            fallen: false,
        }
    }

    /// A clearly-dominant hero (out-hits, out-lasts) can force a win; a clearly-outmatched one cannot.
    #[test]
    fn dominant_hero_is_winnable_weak_one_is_not() {
        let strong = unit("Strong", Side::Party, 4, 2, 2, 1, 5, true, false);
        let weak = unit("Weak", Side::Party, 1, 1, 1, 1, 2, true, false);
        let foe = unit("Foe", Side::Foe, 2, 1, 1, 1, 3, true, false);

        assert!(winnable(&[strong], &[foe.clone()]));
        assert!(!winnable(&[weak], &[foe]));
    }

    /// Soundness: if the *greedy* party already wins the fight, the solver (which searches at least as well)
    /// must report winnable. Checked across a spread of solo matchups.
    #[test]
    fn winnable_dominates_greedy_play() {
        for might in 1..=4 {
            for vit in 2..=4 {
                let hero = unit("Hero", Side::Party, might, 2, 2, 1, vit, true, false);
                let mut foe = unit("Foe", Side::Foe, 2, 1, 1, 1, 3, true, false);
                foe.rank = Rank::Vanguard;
                let mut party = hero.clone();
                party.rank = Rank::Vanguard;
                if play_battle(vec![party, foe.clone()], &Greedy, &Greedy) == Some(true) {
                    assert!(
                        winnable(&[hero], &[foe]),
                        "greedy won but solver said not winnable (might {might}, vit {vit})"
                    );
                }
            }
        }
    }

    /// A ranged hero has a winnable formation (Rearguard, where it is effective) against a melee foe it
    /// out-values; the solver must find it even though the default Vanguard placement is dead weight.
    #[test]
    fn solver_finds_the_effective_formation_for_a_ranged_hero() {
        let archer = unit("Archer", Side::Party, 3, 2, 2, 1, 5, false, true);
        let mut foe = unit("Brute", Side::Foe, 1, 1, 1, 1, 3, true, false);
        foe.rank = Rank::Vanguard;
        assert!(winnable(&[archer], &[foe]));
    }
}
