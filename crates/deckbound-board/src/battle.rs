//! **Headless v2 battle simulator** — a pure driver that plays a whole fight over [`Combatant`]s (no board,
//! no renderer), the foundation the exact solver and the balance harness build on. It walks the same
//! SCHEDULE the arena does (up to five rounds, each five sub-phases of Engage -> Evade -> Strike), folding two
//! [`Policy`]s (party + foe) through the [`crate::combat`] resolvers.
//!
//! Deterministic: given the starting units + both policies, the outcome is fixed (no RNG in v2 resolution).
//! The formation (each unit's rank) is an *input* — Marshal is not simulated here; a solver enumerates
//! formations and calls [`play_battle`] for each.
//!
//! (The [`Greedy`] policy reimplements the arena's greedy plan so this module stays board-free; the two
//! converge when the v2 combat brain is extracted to a shared crate — memory `combat-frozen-revisit-after-tooling`.)

use deckbound_content::schedule::SCHEDULE;

use crate::combat::{self, Blows, Combatant, Contact, Dodge, Engage, Side};

/// The most rounds a battle runs before it is called a draw (Spec §0.4 — an unresolved fight is a draw).
pub const MAX_ROUNDS: usize = 5;

/// A side's play: what it commits in each mini-phase. The driver ([`play_battle`]) calls the party's policy
/// for party units and the foe's for foe units, so a solver can swap in an optimal party policy while the
/// foe stays scripted.
pub trait Policy {
    /// The side's engagements (attacker -> target, tempo committed to reach) in sub-phase `sub`.
    fn engagements(&self, units: &[Combatant], side: Side, sub: usize) -> Vec<Engage>;
    /// Whether unit `i` of this side pays [`combat::slip_cost`] to break everything reaching it, or stands.
    fn dodge(&self, units: &[Combatant], contacts: &[Contact], i: usize) -> Dodge;
    /// The side's strikes along its established contacts (beyond each engager's free opening blow).
    fn blows(&self, units: &[Combatant], side: Side, contacts: &[Contact]) -> Vec<Blows>;
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
            // Engage: both sides commit tempo to reach; each reach is a (not yet established) contact.
            let mut engagements = party.engagements(&units, Side::Party, sub);
            engagements.extend(foe.engagements(&units, Side::Foe, sub));
            let reaching = combat::resolve_engage(&mut units, &engagements);

            // Evade: each target - now seeing exactly what was committed - pays to slip, or stands.
            let dodges: Vec<Dodge> = (0..units.len())
                .map(|i| {
                    let pol: &dyn Policy = if units[i].side == Side::Party {
                        party
                    } else {
                        foe
                    };
                    pol.dodge(&units, &reaching, i)
                })
                .collect();
            let contacts = combat::resolve_evade(&mut units, &reaching, &dodges);

            // Strike: each engager's free opening blow, plus whatever tempo either end pours in after it.
            let mut blows = party.blows(&units, Side::Party, &contacts);
            blows.extend(foe.blows(&units, Side::Foe, &contacts));
            combat::resolve_strike(&mut units, &contacts, &blows);
            combat::end_sub_phase(&mut units);

            if let Some(done) = outcome(&units) {
                return Some(done);
            }
        }
    }
    None
}

/// The **greedy** policy (the scripted default for both sides, and the foe's fixed strategy the solver plays
/// against). It plays the attack tension the honest way: commit the **fewest** cards that the target cannot
/// afford to slip — landing guaranteed, and every card saved becomes a blow. If nothing prices them out, reach
/// with one card and take the chance. A target stands whenever it can answer (an edge it can swing along is
/// worth more than an escape), else slips if it can afford it and the incoming blow actually threatens it.
/// Everyone then dumps their remaining tempo into strikes.
pub struct Greedy;

/// The tempo `defender` would need to slip a reach worth `bid` — the same arithmetic as [`combat::slip_cost`],
/// but for a *hypothetical* bid the attacker has not committed yet.
fn slip_price(bid: u32, f_def: u32) -> u32 {
    bid / f_def.max(1) + 1
}

impl Policy for Greedy {
    fn engagements(&self, units: &[Combatant], side: Side, sub: usize) -> Vec<Engage> {
        let mut out = Vec::new();
        for (i, u) in units.iter().enumerate() {
            if u.fallen
                || u.side != side
                || u.tempo == 0
                || !combat::effective_in_rank(u.rank, u.melee, u.ranged)
            {
                continue;
            }
            let Some(t) = units.iter().enumerate().position(|(j, v)| {
                !v.fallen
                    && v.side != side
                    && combat::legal_strike(sub, u.rank, v.rank)
                    && combat::back_access_ok(units, u.rank, j)
            }) else {
                continue;
            };
            // An area strike cannot be slipped, so reaching costs exactly one card and nothing is gained by
            // committing more. Otherwise: the cheapest commitment they cannot afford to escape, else one card.
            let cards = if u.aoe {
                1
            } else {
                (1..=u.tempo)
                    .find(|&c| slip_price(c * u.finesse.max(1), units[t].finesse) > units[t].tempo)
                    .unwrap_or(1)
            };
            out.push(Engage {
                attacker: i,
                target: t,
                cards,
            });
        }
        out
    }

    fn dodge(&self, units: &[Combatant], contacts: &[Contact], i: usize) -> Dodge {
        let u = &units[i];
        let Some(cost) = combat::slip_cost(units, contacts, i) else {
            return Dodge::Stand; // nothing is reaching you
        };
        if u.fallen || cost > u.tempo {
            return Dodge::Stand; // cannot afford it - so it is not on offer at all
        }
        // If there is an edge you can swing along, standing is worth more than escaping: let them come, and
        // spend the tempo hitting back.
        if combat::strike_target(units, contacts, i).is_some() {
            return Dodge::Stand;
        }
        // Nothing to answer with (a shot from the back line, or no melee of your own). Escape if it threatens.
        let worst = contacts
            .iter()
            .filter(|c| c.target == i)
            .map(|c| units[c.attacker].might)
            .max()
            .unwrap_or(0);
        if worst >= u.toughness.max(1) {
            Dodge::Slip
        } else {
            Dodge::Stand
        }
    }

    fn blows(&self, units: &[Combatant], side: Side, contacts: &[Contact]) -> Vec<Blows> {
        (0..units.len())
            .filter(|&i| units[i].side == side && !units[i].fallen && units[i].tempo > 0)
            .filter_map(|i| {
                combat::strike_target(units, contacts, i).map(|target| Blows {
                    unit: i,
                    target,
                    cards: units[i].tempo,
                })
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
