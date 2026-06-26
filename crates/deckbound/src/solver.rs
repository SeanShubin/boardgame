//! Headless deterministic auto-resolution — the **par-solver substrate** (§8). With the optional
//! Clash module **off**, a §4 battle is a pure function of both sides' choices + seed (creatures are
//! deterministic), so a **greedy hero policy** plays it to an `Outcome`. Only the hero side needs a
//! policy; the foe side is the game's own creature AI. See §4.2 (deterministic base mode).

use engine::{Game, Outcome, PlayerId};

use crate::actor::Actor;
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
        // §4.6 #1 Standoff: positions default from the attack profile (melee fronts, ranged/support
        // holds back), which is what the greedy wants — so it only casts any beneficial `Standing`
        // buffs, then advances to the Fray.
        Phase::Standoff => best_play(state, actions).unwrap_or(Deploy),
        // §4.6 #2 Fray: cast a **setup** ability first (a foe stat-drop / amp — e.g. the Controller's
        // Sunder lowers the wall *before* allies strike this phase, the whole point of the role); else
        // play the **best `Strike` card** (a damage AoE / DoT — a unit's once-per-round role card is its
        // strongest blow, well above a plain weapon poke); else strike a reachable foe with the weapon;
        // else pass. A debuff is read at strike time, so the setup leads (resolution is order-independent
        // within the phase, but the token must be on the target before the blow snapshots it).
        Phase::Fray => setup_play(state, actions)
            .or_else(|| best_play(state, actions))
            .or_else(|| actions.iter().copied().find(|a| matches!(a, Target(..))))
            .unwrap_or_else(|| first_attack_or_pass(actions)),
        // §4.6 #3 Volley: a free Vanguard charges the enemy rear (or flanks); a Rearguard fires again;
        // else cast; else pass. Prefer a charge (reach the back) over a flank.
        Phase::Volley => actions
            .iter()
            .copied()
            .find(|a| matches!(a, Charge(..)))
            .or_else(|| actions.iter().copied().find(|a| matches!(a, Target(..))))
            .or_else(|| best_play(state, actions))
            .unwrap_or_else(|| first_attack_or_pass(actions)),
        // Breach & Reckoning resolve automatically; the greedy never has a choice there.
        Phase::Breach | Phase::Reckoning => first_attack_or_pass(actions),
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

/// A **setup** play to fire before striking this phase: the highest-scoring playable card that is a
/// foe **stat-drop** (Sunder / Mark / Mire / Defang — the Controller's amp/soften) or an own-side
/// **amp** (Empower / Haste). These shape the phase's strikes (a Sunder lowers the wall the allies are
/// about to hit), so the greedy casts one *before* it attacks. Returns `None` if the best play is not a
/// setup effect (then the greedy attacks, then falls back to any other play).
fn setup_play(state: &State, actions: &[Action]) -> Option<Action> {
    use crate::cards::Effect::*;
    let side = state.plan.committing;
    let is_setup = |c: &crate::cards::Card| {
        c.effects.iter().any(|e| {
            matches!(
                e,
                Sunder { .. }
                    | Mark { .. }
                    | Mire { .. }
                    | Defang { .. }
                    | Empower { .. }
                    | Haste { .. }
            )
        })
    };
    actions
        .iter()
        .copied()
        .filter_map(|a| match a {
            Action::PlayCard(i, idx) => state
                .s_pool(side)
                .get(i)
                .and_then(|act| act.actions.get(idx))
                .filter(|c| is_setup(c))
                .map(|c| (a, play_score(c))),
            _ => None,
        })
        .max_by_key(|&(_, score)| score)
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
            Damage { power } => 100 + *power as i32,
            Haste { tempo } => 50 + *tempo as i32,
            Empower { might } => 50 + *might as i32,
            Slow { .. } | Confuse { .. } | Suppress { .. } | Stagger | Shove | Disarm | Rout => 40,
            // §10 token effects. Burn (DoT damage) and Charge (a damage setup) rank near offense;
            // proactive debuff tokens (Mark/Mire) with the other debuffs; Smoke/Silence as enablers.
            Burn { stacks, power } => 80 + (*stacks * *power) as i32,
            Charge { amount } => 60 + *amount as i32,
            Mark { .. } | Mire { .. } | Silence | Smoke | Pin => 40,
            // Sunder/Defang (Controller stat-drops). Sunder lowers the foe's per-phase wall — it is the
            // amp that lets the party crack a foe it can't out-burst, so rank it above the other debuffs
            // (a Sunder this Fray makes this round's strikes land). Defang softens incoming blows.
            Sunder { toughness } => 70 + *toughness as i32,
            Defang { might } => 45 + *might as i32,
            Guard { .. }
            | BankCadence { .. }
            | Ward
            | Lifeline
            | Brace { .. }
            | Cover
            | Thorns { .. } => 20,
            // Reactive: only worth it once someone is hurt — at Muster (full health) it is a
            // wasted play, so the greedy ranks it below acting.
            Mend { .. } | Recover => 5,
        })
        .sum()
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
    fn tempo_refreshes_to_cadence() {
        // §3 tripwire: the Tempo pool's *count* is Cadence. A freshly built/refreshed actor holds
        // exactly Cadence-many Tempo cards. If this drifts, the Cadence·Finesse·Tempo identity is broken.
        use crate::scenarios::build_character;
        let a = build_character("Novice", &[]);
        assert_eq!(
            a.tempo, a.offense.cadence as i32,
            "a refreshed actor must hold Cadence-many Tempo cards"
        );
    }

    // (Removed `higher_finesse_crosses_an_equal_one_card_tie_is_held`: the static-ranks **crossing
    // contest** it tested was retired with the old charge-gauntlet model; the §4.6 Volley charge / flank
    // replaces it, and the evade contest is covered by `combat::evade_contest_strictly_exceeds_the_volley`.)

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
