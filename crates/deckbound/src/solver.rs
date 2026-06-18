//! Headless deterministic auto-resolution — the **par-solver substrate** (§8). With the optional
//! Clash module **off**, a §4 battle is a pure function of both sides' choices + seed (creatures are
//! deterministic), so a **greedy hero policy** plays it to an `Outcome`. Only the hero side needs a
//! policy; the foe side is the game's own creature AI. See §4.2 (deterministic base mode).

use engine::{Game, Outcome, PlayerId};

use crate::actor::{Actor, Range};
use crate::duel::Move;
use crate::game::{Action, Deckbound, battle_state};
use crate::state::{Phase, State};

/// Hard cap on decision steps, so a degenerate scenario (no one can damage anyone) returns rather
/// than spinning forever.
const MAX_STEPS: usize = 100_000;

/// Auto-resolve a PvE battle headlessly (Clash off → deterministic): the party (`heroes`) vs
/// `foes`. `Some(true)` = heroes win, `Some(false)` = heroes fall, `None` = it never resolved
/// (a degenerate stalemate — surfaces as a balance/AI bug rather than a silent result).
pub fn auto_resolve(heroes: Vec<Actor>, foes: Vec<Actor>, seed: u64) -> Option<bool> {
    let game = Deckbound;
    let mut state = battle_state(heroes, foes, false, seed);
    for _ in 0..MAX_STEPS {
        if let Some(outcome) = game.outcome(&state) {
            return Some(matches!(outcome, Outcome::Win(PlayerId(0))));
        }
        let actions = game.legal_actions(&state);
        let action = greedy(&state, &actions);
        if game.apply(&mut state, &action).is_err() {
            return None;
        }
    }
    None
}

/// A moderately-greedy hero policy: commit melee to the Vanguard, hold and fight, strike the front,
/// or play a power if there's nothing to hit. Picks one action; called repeatedly.
fn greedy(state: &State, actions: &[Action]) -> Action {
    use Action::*;
    match state.phase {
        // Put melee-capable heroes in the Vanguard; ranged/support stay Reserve; then Deploy.
        Phase::Muster => {
            for a in actions {
                if let SetVanguard(i) = a
                    && state.heroes[*i].can_contest(Range::Melee)
                {
                    return *a;
                }
            }
            Deploy
        }
        // Assign the queued Vanguard to a lane (first offered = lane 0; stacking is fine).
        Phase::Assign => actions
            .iter()
            .copied()
            .find(|a| matches!(a, AssignLane(..)))
            .unwrap_or(Deploy),
        // Use the default hold/slip (the wall fights its lanes), then resolve the front.
        Phase::Slip => ResolveFront,
        // Strike a reachable foe; else pass.
        Phase::Skirmish => first_attack_or_pass(actions),
        // Reserve: fire on the front, else play a power (heal/buff/debuff), else pass.
        Phase::Reserve => actions
            .iter()
            .copied()
            .find(|a| matches!(a, Target(..)))
            .or_else(|| actions.iter().copied().find(|a| matches!(a, PlayCard(..))))
            .unwrap_or_else(|| first_attack_or_pass(actions)),
        // The Clash is off in the solver; if somehow reached, just strike.
        Phase::Clash => Play(Move::Strike),
        Phase::Menu(_) => ToMenu,
    }
}

/// First `Target` (attack), else `Pass`, else the first non-`ToMenu` action.
fn first_attack_or_pass(actions: &[Action]) -> Action {
    use Action::*;
    actions
        .iter()
        .copied()
        .find(|a| matches!(a, Target(..)))
        .or_else(|| actions.iter().copied().find(|a| matches!(a, Pass(..))))
        .or_else(|| actions.iter().copied().find(|a| !matches!(a, ToMenu)))
        .unwrap_or(ToMenu)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenarios::campaign;

    #[test]
    fn auto_resolve_terminates_on_every_campaign_scenario() {
        // The greedy policy, Clash off, must drive every real scenario to a decisive result —
        // no stalemate, no error. (Win or loss is fine; *non-termination* is the bug we catch.)
        for s in campaign() {
            let (heroes, foes) = s.roster();
            assert!(
                auto_resolve(heroes, foes, 1).is_some(),
                "scenario {:?} did not resolve under the greedy policy",
                s.name
            );
        }
    }
}
