//! **Exact v2 winnability solver.** Answers "can the party *force* a win against the scripted greedy foe?"
//! by searching every party line (formation + per-mini-phase allocations) and the deterministic greedy foe
//! response through the [`crate::combat`] resolvers. The foe is a fixed policy, so this is a single-agent
//! reachability search: the party is winnable iff *some* sequence of its choices reaches a party-win state.
//!
//! Tractability rests on the v2 structure (memory `combat-frozen-revisit-after-tooling`): resolution is
//! order-free and deterministic, targets are SCHEDULE-gated, and bids are **threshold contests** — so the
//! only strike bids worth trying are `min-to-land` and `min-to-deny-evade`, not the whole `0..tempo` range.
//! States are memoized at sub-phase boundaries. Exponential in party size in the worst case; trivial for the
//! solo duel-locks matchups, which is what the diagonal balance property needs.

use std::collections::HashMap;

use deckbound_content::rank::Intention as Rank;
use deckbound_content::schedule::SCHEDULE;

use crate::battle::{Greedy, MAX_ROUNDS, Policy};
use crate::combat::{self, Blows, Combatant, Contact, Dodge, Engage, Side};

/// The three ranks a party unit may be assigned (the formation search space).
const RANKS: [Rank; 3] = [Rank::Vanguard, Rank::Outrider, Rank::Rearguard];

/// Whether the party can **force a win** vs the scripted greedy foe. `party` come rank-less (each formation
/// overwrites their `rank`); `foes` keep their scripted ranks. The party picks one formation for the battle
/// (round-0 ranks, held — a sufficient condition: if some fixed formation wins, the party wins).
pub fn winnable(party: &[Combatant], foes: &[Combatant]) -> bool {
    winnable_traced(party, foes, false).0
}

/// Whether the party can force a win **when it may re-Marshal every round** — the rule the *game* actually
/// plays, as against [`winnable`]'s fixed-formation assumption.
///
/// The game unranks the foes at each Reset and lets you move your heroes, so a position a fixed-formation
/// search calls lost may be winnable by re-ranking next round. If the two ever disagree, the fixed search is
/// **wrong about the game** - and a "doomed" verdict built on it would tell you to abandon a fight you could
/// still win, which is the one failure mode a certainty indicator may never have.
pub fn winnable_remarshal(party: &[Combatant], foes: &[Combatant]) -> bool {
    winnable_traced(party, foes, true).0
}

/// [`winnable`] / [`winnable_remarshal`], reporting the size of the memo table it built — the honest measure
/// of what an in-app solver would have to hold.
pub fn winnable_traced(party: &[Combatant], foes: &[Combatant], remarshal: bool) -> (bool, usize) {
    // Re-marshalling branches every formation at the top of each round anyway, so the opening ranks are
    // whatever: round 0 will overwrite them. A fixed search must enumerate them itself.
    if remarshal {
        let mut units: Vec<Combatant> = party.to_vec();
        units.extend(foes.iter().cloned());
        let mut memo = HashMap::new();
        let win = forces_win_with(&units, 0, 0, &mut memo, true);
        return (win, memo.len());
    }
    let n = party.len();
    let mut states = 0;
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
        let win = forces_win(&units, 0, 0, &mut memo);
        states += memo.len();
        if win {
            return (true, states);
        }
    }
    (false, states)
}

/// Every way to rank the party's **living** heroes (the fallen have no formation). One entry per hero index.
fn formations(units: &[Combatant]) -> Vec<Vec<(usize, Rank)>> {
    let living: Vec<usize> = (0..units.len())
        .filter(|&i| units[i].side == Side::Party && !units[i].fallen)
        .collect();
    let mut out = Vec::new();
    for f in 0..3usize.pow(living.len() as u32) {
        out.push(
            living
                .iter()
                .enumerate()
                .map(|(k, &i)| (i, RANKS[(f / 3usize.pow(k as u32)) % 3]))
                .collect(),
        );
    }
    out
}

/// A memo key: the mutable state (per unit health/tempo/fallen/**pile**) plus the walk position.
///
/// The pile has to be in here. It used to be safe to omit it only because the pile was wiped at every
/// sub-phase boundary - i.e. it was *always zero* at exactly the points we memoize. Now that wounds carry
/// across a round's sub-phases, two positions with identical health and tempo but different accumulated
/// damage are genuinely different positions, and conflating them would make the solver return confidently
/// wrong answers rather than fail. It costs state space (a wound counter in `[0, grit)` per unit), which
/// is the price of the rule.
/// The **rank is in the key too**, and it has to be: once the party may re-Marshal, the formation is part of
/// the mutable state, not a constant of the battle. Leaving it out would conflate two genuinely different
/// positions and hand back a confidently wrong answer. (For a fixed-formation search it is a constant, so it
/// costs nothing but a few bytes.)
type Key = (Vec<(u32, u32, bool, u32, u8)>, usize, usize);

fn key_of(units: &[Combatant], round: usize, sub: usize) -> Key {
    let rank = |r: Rank| match r {
        Rank::Vanguard => 0u8,
        Rank::Outrider => 1,
        Rank::Rearguard => 2,
    };
    (
        units
            .iter()
            .map(|u| (u.health, u.tempo, u.fallen, u.pending, rank(u.rank)))
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
    forces_win_with(units, round, sub, memo, false)
}

fn forces_win_with(
    units: &[Combatant],
    round: usize,
    sub: usize,
    memo: &mut HashMap<Key, bool>,
    remarshal: bool,
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
    // **Marshal.** With re-marshalling the party re-declares its formation at the top of every round - which is
    // what the game actually lets you do. Without it, the ranks it walked in with are the ranks it fights with.
    let win = if remarshal && sub == 0 {
        formations(&units).into_iter().any(|f| {
            let mut u = units.clone();
            for (i, r) in f {
                u[i].rank = r;
            }
            search_engage_with(&u, round, sub, memo, remarshal)
        })
    } else {
        search_engage_with(&units, round, sub, memo, remarshal)
    };
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

/// Engage step: try every party engagement plan (joint over attackers), fold in the greedy foe, resolve,
/// recurse into Evade.
fn search_engage_with(
    units: &[Combatant],
    round: usize,
    sub: usize,
    memo: &mut HashMap<Key, bool>,
    rm: bool,
) -> bool {
    let foe_engagements = Greedy.engagements(units, Side::Foe, sub);
    let options = party_engage_options(units, sub);
    any_combo(&options, &mut |chosen| {
        let mut u = units.to_vec();
        let mut all: Vec<Engage> = chosen.iter().flatten().copied().collect();
        all.extend(foe_engagements.iter().copied());
        let reaching = combat::resolve_engage(&mut u, &all);
        search_evade(&u, &reaching, round, sub, memo, rm)
    })
}

/// Evade step: try every party dodge plan (Slip or Stand, per reached party unit), fold in the greedy foe,
/// resolve, recurse into Strike.
///
/// This is where the "no partial slip" rule pays for itself in the search: the branch is **binary** per unit,
/// not `0..tempo` wide. The dominated option was not merely bad play, it was a whole dimension of the tree.
fn search_evade(
    units: &[Combatant],
    reaching: &[Contact],
    round: usize,
    sub: usize,
    memo: &mut HashMap<Key, bool>,
    rm: bool,
) -> bool {
    // Party units that something is reaching for, and that can actually afford to escape it.
    let reached: Vec<usize> = (0..units.len())
        .filter(|&i| {
            units[i].side == Side::Party
                && !units[i].fallen
                && combat::slip_cost(units, reaching, i).is_some_and(|c| c <= units[i].tempo)
        })
        .collect();
    let options: Vec<Vec<Dodge>> = reached
        .iter()
        .map(|_| vec![Dodge::Stand, Dodge::Slip])
        .collect();
    any_combo(&options, &mut |chosen| {
        let dodges: Vec<Dodge> = (0..units.len())
            .map(|i| match reached.iter().position(|&r| r == i) {
                Some(pos) => chosen[pos],
                None if units[i].side == Side::Foe => Greedy.dodge(units, reaching, i),
                None => Dodge::Stand, // cannot afford to slip: standing is the only thing on offer
            })
            .collect();
        let mut u = units.to_vec();
        let contacts = combat::resolve_evade(&mut u, reaching, &dodges);
        search_strike(&u, &contacts, round, sub, memo, rm)
    })
}

/// Strike step: try every party blow plan (how many cards each contacted party unit pours in), fold in the
/// greedy foe, resolve, close the sub-phase, recurse to the next.
fn search_strike(
    units: &[Combatant],
    contacts: &[Contact],
    round: usize,
    sub: usize,
    memo: &mut HashMap<Key, bool>,
    rm: bool,
) -> bool {
    let foe_blows = Greedy.blows(units, Side::Foe, contacts);
    // Every party unit on an edge it may swing along - as the engager, or answering along a melee edge.
    let party_edges: Vec<(usize, usize)> = (0..units.len())
        .filter(|&i| units[i].side == Side::Party && !units[i].fallen && units[i].tempo > 0)
        .filter_map(|i| combat::strike_target(units, contacts, i).map(|t| (i, t)))
        .collect();
    let options: Vec<Vec<Blows>> = party_edges
        .iter()
        .map(|&(i, target)| {
            (0..=units[i].tempo)
                .map(|cards| Blows {
                    unit: i,
                    target,
                    cards,
                })
                .collect()
        })
        .collect();
    let (nr, ns) = next(round, sub);
    any_combo(&options, &mut |chosen| {
        let mut u = units.to_vec();
        let mut blows: Vec<Blows> = chosen.iter().filter(|b| b.cards > 0).copied().collect();
        blows.extend(foe_blows.iter().copied());
        combat::resolve_strike(&mut u, contacts, &blows);
        combat::end_sub_phase(&mut u);
        forces_win_with(&u, nr, ns, memo, rm)
    })
}

// ---- the party's pruned option sets -------------------------------------------------------------------

/// Each party attacker's engagement options this sub-phase: `None` (do not reach), plus, for each legal and
/// reachable foe, the two canonical commitments — **one card** (cheapest reach, most tempo kept back for
/// blows) and **the fewest cards they cannot afford to slip** (landing guaranteed). Everything in between is
/// strictly worse than one of those two: it neither saves tempo nor denies the escape.
fn party_engage_options(units: &[Combatant], sub: usize) -> Vec<Vec<Option<Engage>>> {
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
                    || !combat::legal_strike(sub, u.rank, v.rank)
                    || !combat::back_access_ok(units, u.rank, j)
                {
                    continue;
                }
                if u.aoe {
                    // An area strike cannot be slipped - no commitment to tune, one card, no follow-up.
                    opts.push(Some(Engage {
                        attacker: i,
                        target: j,
                        cards: 1,
                    }));
                    continue;
                }
                // Deny the slip: the fewest cards whose value they cannot out-spend at their Finesse.
                let deny = (1..=u.tempo)
                    .find(|&c| (c * u.finesse.max(1)) / v.finesse.max(1) + 1 > v.tempo);
                for cards in [Some(1), deny].into_iter().flatten() {
                    opts.push(Some(Engage {
                        attacker: i,
                        target: j,
                        cards,
                    }));
                }
            }
            dedup(opts)
        })
        .collect()
}

fn dedup(mut v: Vec<Option<Engage>>) -> Vec<Option<Engage>> {
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
        grit: u32,
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
            grit,
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
