//! Headless deterministic auto-resolution — the **par-solver substrate** (§8). With the optional
//! Clash module **off**, a §4 battle is a pure function of both sides' choices + seed (creatures are
//! deterministic), so a **greedy hero policy** plays it to an `Outcome`. Only the hero side needs a
//! policy; the foe side is the game's own creature AI. See §4.2 (deterministic base mode).

use engine::{Game, Outcome, PlayerId};

use crate::actor::{Actor, Range};
use crate::duel::Move;
use crate::game::{Action, Deckbound, battle_state_with};
use crate::ruleset::Ruleset;
use crate::state::{Phase, State};

/// Hard cap on decision steps, so a degenerate scenario (no one can damage anyone) returns rather
/// than spinning forever.
const MAX_STEPS: usize = 100_000;

/// Auto-resolve a PvE battle headlessly (Clash off → deterministic): the party (`heroes`) vs
/// `foes`. `Some(true)` = heroes win, `Some(false)` = heroes fall **or draw** (a draw is no different
/// from a loss in PvE), `None` = it never resolved (a degenerate stalemate — a balance/AI bug rather
/// than a silent result). Runs under the **analysis** [`Ruleset`] (bounded round horizon) so the
/// combat is finite, matching how the balance tooling sets up games (§0).
pub fn auto_resolve(heroes: Vec<Actor>, foes: Vec<Actor>, seed: u64) -> Option<bool> {
    auto_resolve_with(heroes, foes, seed, Ruleset::analysis())
}

/// As [`auto_resolve`], but with an explicit [`Ruleset`] (round/roster bounds).
pub fn auto_resolve_with(
    heroes: Vec<Actor>,
    foes: Vec<Actor>,
    seed: u64,
    ruleset: Ruleset,
) -> Option<bool> {
    let game = Deckbound;
    let mut state = battle_state_with(heroes, foes, false, seed, ruleset);
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
        // Charge selection: melee fighters (Wall / Infiltrator / plain) run the gauntlet; keep
        // back-line casters and shooters (Artillery / Controller / Support kits) in the Reserve so
        // they fire / cast from the rear (§4); then Deploy to resolve the gauntlet.
        Phase::Assemble => {
            // 1. Send melee front-liners to charge (casters/shooters hold back to fire/cast).
            for a in actions {
                if let SetVanguard(i) = a
                    && state.heroes[*i].can_contest(Range::Melee)
                    && !wants_backline(&state.heroes[*i])
                {
                    return *a;
                }
            }
            // 2. Muster: play standing defenses / debuffs / buffs before the gauntlet so they bite it.
            // 3. Then Deploy.
            best_play(state, actions).unwrap_or(Deploy)
        }
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

/// The best `PlayCard` for the committing side — the highest-**scoring** playable card, so a member
/// spends its one-per-role play on its strongest option (and deeper cards get used), not the first it
/// happens to find. Scoring ranks **damage** (wins the race) over **amplification** (Empower/Haste —
/// indirect offense, race-positive) over proactive **debuffs**, with reactive heals last (a Mend at
/// Muster heals nobody — the solver shouldn't burn its play on it). Returns `None` if no card is
/// playable.
fn best_play(state: &State, actions: &[Action]) -> Option<Action> {
    let side = state.plan.committing;
    actions
        .iter()
        .copied()
        .filter_map(|a| match a {
            Action::PlayCard(i, idx) => state
                .s_pool(side)
                .get(i)
                .and_then(|act| act.actions.get(idx))
                .map(|c| (a, play_score(c))),
            _ => None,
        })
        .max_by_key(|&(_, score)| score)
        .filter(|&(_, score)| score > 0)
        .map(|(a, _)| a)
}

/// A heuristic value for playing `card` now (greedy policy). Damage ≫ amplification ≫ proactive debuff
/// ≫ minor buff ≫ reactive heal. The magnitude terms give a mild preference for the deeper (stronger)
/// card of a track. Used only by the greedy resolver — not a rule.
fn play_score(card: &crate::cards::Card) -> i32 {
    use crate::cards::Effect::*;
    card.effects
        .iter()
        .map(|e| match e {
            Damage { power, .. } => 100 + *power as i32,
            Haste { tempo } => 50 + *tempo as i32,
            Empower { power } => 50 + *power as i32,
            Slow { .. }
            | Confuse { .. }
            | Suppress { .. }
            | Stagger
            | Shove
            | Disarm
            | Sunder { .. } => 40,
            Rally { .. } | Guard { .. } | BankSpeed { .. } | Ward | Lifeline => 20,
            // Reactive: only worth it once someone is hurt/feared — at Muster (full health) it is a
            // wasted play, so the greedy ranks it below acting.
            Mend { .. } | Steel | Recover => 5,
        })
        .sum()
}

/// A hero whose strength is **ranged fire from the Reserve**: it carries a non-passive Artillery /
/// Controller card (Brass / Bone) — cards that *attack the enemy* from range, so it holds back to cast
/// rather than trading weakly up front. **Support (Salt) does *not* want the back line**: its cards are
/// ally **buffs** (Empower / Haste / Mend) that work from anywhere, played at Muster — so a Salt member
/// should **charge and fight in melee** (a Reserve full of buff-only melee actors deals no damage and
/// is simply raided).
fn wants_backline(a: &Actor) -> bool {
    use crate::currency::Currency::{Bone, Brass};
    a.actions
        .iter()
        .any(|c| !c.passive && matches!(c.role, Some(Brass) | Some(Bone)))
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

    #[test]
    fn tempo_refreshes_to_speed() {
        // §3 tripwire: the Tempo pool's *count* is Speed. A freshly built/refreshed actor holds
        // exactly Speed-many Tempo cards. If this drifts, the Speed·Drive·Tempo identity is broken.
        use crate::scenarios::build_character;
        let a = build_character("Novice", &[]);
        assert_eq!(
            a.tempo, a.offense.speed as i32,
            "a refreshed actor must hold Speed-many Tempo cards"
        );
    }

    #[test]
    fn higher_drive_slips_the_gauntlet_a_tie_stops_both() {
        // §3 tripwire: a gauntlet crossing is decided by **Drive**, not Speed/Power. The
        // higher-Drive charger slips past (Skirmisher); equal Drive stops both (Vanguard).
        use crate::currency::Currency;
        use crate::scenarios::{build_character, rewards_for};

        // Silver (Infiltrator) rewards seed Drive; a bare Novice floors at Drive 1.
        let runner = build_character("Novice", &rewards_for(Currency::Silver));
        let blocker = build_character("Novice", &[]);
        assert!(
            runner.offense.drive > blocker.offense.drive.max(1),
            "test premise: the Silver-kitted runner must out-Drive the bare blocker"
        );

        let mut heroes = vec![runner];
        let mut foes = vec![blocker];
        let mut log = Vec::new();
        let (h_skirm, _f_skirm) =
            crate::combat::gauntlet(&mut heroes, &[true], &mut foes, &[true], &mut log);
        assert!(
            h_skirm[0],
            "the higher-Drive charger must break through as a Skirmisher"
        );

        // Equal Drive → neither slips (both held as Vanguard).
        let mut a = vec![build_character("Novice", &[])];
        let mut b = vec![build_character("Novice", &[])];
        let mut log = Vec::new();
        let (a_sk, b_sk) = crate::combat::gauntlet(&mut a, &[true], &mut b, &[true], &mut log);
        assert!(
            !a_sk[0] && !b_sk[0],
            "equal Drive must stop both chargers (tie to the catcher)"
        );
    }

    // (Removed `a_holding_wall_plays_its_role_cards`: the gauntlet auto-resolves the Vanguard, so
    // there is no interactive Wall play window in v1 — a known limitation, see role-card-redesign.)

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
