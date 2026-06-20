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
/// or play a power if there's nothing to hit. Picks one action; called repeatedly. Public so the
/// campaign can suggest a combat move to the player.
pub fn greedy(state: &State, actions: &[Action]) -> Action {
    use Action::*;
    match state.phase {
        // Put melee fighters (Wall / Infiltrator / plain) in the Vanguard; keep back-line casters
        // and shooters (Artillery / Controller / Support kits) in the Reserve so their role cards
        // fire from where they act (§4.4); then Deploy.
        Phase::Muster => {
            for a in actions {
                if let SetVanguard(i) = a
                    && state.heroes[*i].can_contest(Range::Melee)
                    && !wants_backline(&state.heroes[*i])
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
        // A holding Vanguard plays its role cards (Brace / Rally / Last Stand) before the front
        // resolves — buffs land first; the per-role cap ends it, then resolve. Default hold/slip.
        Phase::Slip => actions
            .iter()
            .copied()
            .find(|a| matches!(a, PlayCard(..)))
            .unwrap_or(ResolveFront),
        // Strike a reachable foe; else play a role card (a damaging one first); else pass.
        Phase::Skirmish => actions
            .iter()
            .copied()
            .find(|a| matches!(a, Target(..)))
            .or_else(|| best_play(state, actions))
            .unwrap_or_else(|| first_attack_or_pass(actions)),
        // Reserve: fire on the front, else play a power (a damaging one first), else pass.
        Phase::Reserve => actions
            .iter()
            .copied()
            .find(|a| matches!(a, Target(..)))
            .or_else(|| best_play(state, actions))
            .unwrap_or_else(|| first_attack_or_pass(actions)),
        // The Clash is off in the solver; if somehow reached, just strike.
        Phase::Clash => Play(Move::Strike),
        Phase::Menu(_) => ToMenu,
    }
}

/// The best `PlayCard` for the committing side: a card that deals damage (kills the gate) is
/// preferred over a pure buff/debuff; otherwise the first playable card.
fn best_play(state: &State, actions: &[Action]) -> Option<Action> {
    use crate::cards::Effect;
    let side = state.plan.committing;
    let deals_damage = |i: usize, idx: usize| {
        state
            .s_pool(side)
            .get(i)
            .and_then(|a| a.actions.get(idx))
            .is_some_and(|c| c.effects.iter().any(|e| matches!(e, Effect::Damage { .. })))
    };
    let mut fallback = None;
    for a in actions {
        if let Action::PlayCard(i, idx) = a {
            if deals_damage(*i, *idx) {
                return Some(*a);
            }
            fallback.get_or_insert(*a);
        }
    }
    fallback
}

/// A hero whose strength is played from the back line: it carries a non-passive Artillery /
/// Controller / Support role card (Brass / Bone / Salt). Such heroes stay in the Reserve so they
/// can cast (the Reserve is the phase that plays cards), rather than trading weakly in a lane.
fn wants_backline(a: &Actor) -> bool {
    use crate::currency::Currency::{Bone, Brass, Salt};
    a.actions
        .iter()
        .any(|c| !c.passive && matches!(c.role, Some(Brass) | Some(Bone) | Some(Salt)))
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

    /// Diagnostic (run on demand): print win/lose for a clean-slate vs upgraded character against
    /// scaling foe counts, to calibrate encounter difficulty. `cargo test probe_power -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_power() {
        use crate::currency::Currency;
        use crate::encounter::{EncounterCard, RosterEntry};
        use crate::form::StatCard;
        use crate::scenarios::{build_character, build_encounter_foes, rewards_for};

        let enc = |creature: &str, count: u32| EncounterCard {
            name: "probe".into(),
            currency: Currency::Iron,
            strategy: "aggressor".into(),
            foes: vec![RosterEntry {
                creature: creature.into(),
                from_level: 1,
                base: count,
                growth: 0,
            }],
            scaling: StatCard::default(),
        };
        for k in 1..=8 {
            let foes = build_encounter_foes(&enc("Husk", k), 1);
            let bare = vec![build_character("Novice", &[])];
            let wall = vec![build_character("Novice", &rewards_for(Currency::Iron))];
            let b = auto_resolve(bare, foes.clone(), 1);
            let u = auto_resolve(wall, foes, 1);
            println!("Husk x{k}: bare={b:?}  Wall-kit={u:?}");
        }
    }

    /// A Wall that holds a lane now plays its role cards (Brace / Rally / Last Stand) during the
    /// Slip phase, before the front resolves (§4.4 Vanguard play). Drives a solo Wall specialist vs
    /// a husk with the guide and checks a Wall card actually fired.
    #[test]
    fn a_holding_wall_plays_its_role_cards() {
        use crate::currency::Currency;
        use crate::encounter::{EncounterCard, RosterEntry};
        use crate::form::StatCard;
        use crate::game::{Deckbound, battle_state};
        use crate::scenarios::{build_character, build_encounter_foes, rewards_for};
        use engine::Game;

        let wall = build_character("Novice", &rewards_for(Currency::Iron));
        let enc = EncounterCard {
            name: "probe".into(),
            currency: Currency::Iron,
            strategy: "aggressor".into(),
            foes: vec![RosterEntry {
                creature: "Husk".into(),
                from_level: 1,
                base: 1,
                growth: 0,
            }],
            scaling: StatCard::default(),
        };
        let game = Deckbound;
        let mut s = battle_state(vec![wall], build_encounter_foes(&enc, 1), false, 1);
        let mut played = false;
        for _ in 0..2_000 {
            if game.outcome(&s).is_some() {
                break;
            }
            let action = greedy(&s, &game.legal_actions(&s));
            game.apply(&mut s, &action).expect("guided move is legal");
            if s.log.iter().any(|l| {
                l.contains("plays Brace")
                    || l.contains("plays Rally")
                    || l.contains("plays Last Stand")
            }) {
                played = true;
                break;
            }
        }
        assert!(
            played,
            "a holding Wall never played a role card; log tail: {:?}",
            &s.log[s.log.len().saturating_sub(12)..]
        );
    }

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
