//! **Headless v2 battle simulator** — a pure driver that plays a whole fight over [`Combatant`]s (no board,
//! no renderer), the foundation the exact solver and the balance harness build on. It walks the same
//! SCHEDULE the arena does (up to five rounds, each five sub-phases of Strike -> React -> Extra), folding two
//! [`Policy`]s (party + foe) through the [`crate::combat`] resolvers.
//!
//! Deterministic: given the starting units + both policies, the outcome is fixed (no RNG in v2 resolution).
//! The formation (each unit's rank) is an *input* — Marshal is not simulated here; a solver enumerates
//! formations and calls [`play_battle`] for each.
//!
//! (The [`Greedy`] policy reimplements the arena's greedy plan so this module stays board-free; the two
//! converge when the v2 combat brain is extracted to a shared crate — memory `combat-frozen-revisit-after-tooling`.)

use deckbound_content::schedule::SCHEDULE;

use crate::combat::{self, Combatant, Contact, ExtraStrike, React, Side, Strike};

/// The most rounds a battle runs before it is called a draw (Spec §0.4 — an unresolved fight is a draw).
pub const MAX_ROUNDS: usize = 5;

/// A side's play: what it commits in each mini-phase. The driver ([`play_battle`]) calls the party's policy
/// for party units and the foe's for foe units, so a solver can swap in an optimal party policy while the
/// foe stays scripted.
pub trait Policy {
    /// The side's strikes (attacker -> target bids) in sub-phase `sub`.
    fn strikes(&self, units: &[Combatant], side: Side, sub: usize) -> Vec<Strike>;
    /// How a unit of this side reacts to one incoming `contact` (its target is on this side).
    fn react(&self, units: &[Combatant], contact: &Contact) -> React;
    /// The side's extra strikes along its still-surviving contacts.
    fn extras(&self, units: &[Combatant], side: Side, surviving: &[Contact]) -> Vec<ExtraStrike>;
}

/// Whether the fight is over: `Some(true)` = party won, `Some(false)` = foe won, `None` = still going. A side
/// loses when all its units are fallen.
fn outcome(units: &[Combatant]) -> Option<bool> {
    let party_alive = units.iter().any(|u| u.side == Side::Party && !u.fallen);
    let foes_alive = units.iter().any(|u| u.side == Side::Foe && !u.fallen);
    match (party_alive, foes_alive) {
        (true, true) => None,
        (won, _) => Some(won),
    }
}

/// Play a whole battle from a fixed formation. Returns `Some(true)` if the party wins, `Some(false)` if the
/// foes win, `None` for a draw at the round cap. Each round refreshes Tempo to Cadence, then walks the
/// SCHEDULE; a mini-phase boundary finalizes deaths, and the fight ends the moment one side is wiped.
pub fn play_battle(
    mut units: Vec<Combatant>,
    party: &dyn Policy,
    foe: &dyn Policy,
) -> Option<bool> {
    for _round in 0..MAX_ROUNDS {
        combat::refresh_round(&mut units);
        for sub in 0..SCHEDULE.len() {
            // Strike: both sides bid; landed strikes become contacts.
            let mut strikes = party.strikes(&units, Side::Party, sub);
            strikes.extend(foe.strikes(&units, Side::Foe, sub));
            let contacts = combat::resolve_strike(&mut units, &strikes);

            // React: each incoming contact is answered by its target's side.
            let reactions: Vec<React> = contacts
                .iter()
                .map(|c| {
                    let pol: &dyn Policy = if units[c.target].side == Side::Party {
                        party
                    } else {
                        foe
                    };
                    pol.react(&units, c)
                })
                .collect();
            let surviving = combat::resolve_react(&mut units, &contacts, &reactions);

            // Extra: still-contacted units flip remaining Tempo.
            let mut extras = party.extras(&units, Side::Party, &surviving);
            extras.extend(foe.extras(&units, Side::Foe, &surviving));
            combat::resolve_extra(&mut units, &extras);
            combat::end_sub_phase(&mut units);

            if let Some(done) = outcome(&units) {
                return Some(done);
            }
        }
    }
    None
}

/// The **greedy** policy (the scripted default for both sides, and the foe's fixed strategy the solver plays
/// against): each effective unit strikes the first enemy it can legally reach and afford, at the minimum
/// landing bid; a struck unit evades when the blow threatens a flip and it can afford to beat the bid, else
/// eats; every still-contacted unit dumps its remaining Tempo as extra strikes.
pub struct Greedy;

impl Policy for Greedy {
    fn strikes(&self, units: &[Combatant], side: Side, sub: usize) -> Vec<Strike> {
        let mut strikes = Vec::new();
        for (i, u) in units.iter().enumerate() {
            if u.fallen
                || u.side != side
                || u.tempo == 0
                || !combat::effective_in_rank(u.rank, u.melee, u.ranged)
            {
                continue;
            }
            if let Some((t, cards)) = units.iter().enumerate().find_map(|(j, v)| {
                if v.fallen
                    || v.side == side
                    || !combat::legal_strike(sub, u.rank, v.rank)
                    || !combat::back_access_ok(units, u.rank, j)
                {
                    return None;
                }
                // An area strike is unevadable and costs one card; a single strike bids the minimum to land.
                let need = if u.aoe {
                    1
                } else {
                    v.finesse.div_ceil(u.finesse.max(1)).max(1)
                };
                (need <= u.tempo).then_some((j, need))
            }) {
                strikes.push(Strike {
                    attacker: i,
                    target: t,
                    cards,
                });
            }
        }
        strikes
    }

    fn react(&self, units: &[Combatant], contact: &Contact) -> React {
        let d = &units[contact.target];
        let threatens = units[contact.attacker].might >= d.toughness.max(1);
        // Cards needed to strictly exceed the attacker's spent bid, at the defender's Finesse.
        let need = contact.bid / d.finesse.max(1) + 1;
        if threatens && need > 0 && need <= d.tempo {
            React::Evade { cards: need }
        } else {
            React::Eat
        }
    }

    fn extras(&self, units: &[Combatant], side: Side, surviving: &[Contact]) -> Vec<ExtraStrike> {
        surviving
            .iter()
            .filter(|c| units[c.attacker].side == side && units[c.attacker].tempo > 0)
            .map(|c| ExtraStrike {
                attacker: c.attacker,
                target: c.target,
                cards: units[c.attacker].tempo,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deckbound_content::rank::Intention as Rank;

    fn unit(
        name: &str,
        side: Side,
        rank: Rank,
        might: u32,
        finesse: u32,
        cadence: u32,
        toughness: u32,
        health: u32,
    ) -> Combatant {
        Combatant {
            name: name.into(),
            side,
            rank,
            might,
            finesse,
            cadence,
            toughness,
            armor: 0,
            melee: true,
            ranged: false,
            aoe: false,
            horde: false,
            tempo: cadence,
            health,
            pending: 0,
            fallen: false,
        }
    }

    /// A lone Vanguard vs a lone Vanguard (Clash, V->V): the harder-hitting one wins, and the sim terminates.
    #[test]
    fn stronger_vanguard_wins_the_clash() {
        let units = vec![
            unit("Hero", Side::Party, Rank::Vanguard, 3, 2, 3, 1, 4),
            unit("Foe", Side::Foe, Rank::Vanguard, 1, 1, 1, 1, 4),
        ];
        assert_eq!(play_battle(units, &Greedy, &Greedy), Some(true));
    }

    /// Deterministic: the same formation replays to the same result.
    #[test]
    fn play_is_deterministic() {
        let build = || {
            vec![
                unit("Hero", Side::Party, Rank::Vanguard, 2, 2, 2, 1, 3),
                unit("Foe", Side::Foe, Rank::Vanguard, 2, 2, 2, 1, 3),
            ]
        };
        let a = play_battle(build(), &Greedy, &Greedy);
        let b = play_battle(build(), &Greedy, &Greedy);
        assert_eq!(a, b);
    }

    /// A fight neither side can finish inside the round cap is a draw (two untouchable Rearguards with no
    /// legal melee, so nothing lands).
    #[test]
    fn unresolvable_fight_is_a_draw() {
        // Ranged-less Rearguards: rank_is_ranged wants `ranged`, which these lack, so neither is effective and
        // no strike ever forms.
        let units = vec![
            unit("Hero", Side::Party, Rank::Rearguard, 3, 2, 3, 1, 3),
            unit("Foe", Side::Foe, Rank::Rearguard, 3, 2, 3, 1, 3),
        ];
        assert_eq!(play_battle(units, &Greedy, &Greedy), None);
    }
}
