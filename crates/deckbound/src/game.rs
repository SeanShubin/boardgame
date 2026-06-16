//! Deckbound as an [`engine::Game`].
//!
//! Combat is a sequence of **rounds**. In the player phase you **engage** foes
//! (spending tempo) through interactive duels and **play actions** (AoE/support);
//! at **round end** the creatures act and foes you couldn't cover (focus) free-hit.
//! A 1v1 tutorial is just the smallest case. All numbers live in `data/booklet.ron`.

use engine::{
    Accent, CardView, Game, GameError, Layout, Outcome, PlayerId, Rng, TableView, ZoneView,
};

use crate::combat::{self, base_strike};
use crate::duel::{self, Move, Side};
use crate::scenarios::{self, Scenario};
use crate::state::{Dive, Duel, Menu, Phase, State, Versus};

/// Break off a Clash after this many consecutive beats with no Body lost (a termination
/// backstop — see §1.6, reworded for Body-attrition).
const STALL_CAP: u32 = 12;
/// Flat tempo cost to play an action card (tunable).
const ACTION_COST: u32 = 2;

/// One step the player can take.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    OpenScenarios,
    OpenGod,
    OpenTutorial,
    OpenVersus,
    PickScenario(usize),
    Exit,
    ToMenu,
    Back,
    Replay,
    /// This hero engages this foe (begins a Clash; spends Tempo = the foe's Speed).
    Engage(usize, usize),
    /// This hero dives the enemy gauntlet toward a back-line foe (§4 formation).
    Dive(usize, usize),
    /// This hero shifts line (front <-> back) — free, between rounds (§4).
    Reposition(usize),
    /// HeroDive: push through the gauntlet (pay the guards' combined Speed, eat their hits).
    PushThrough,
    /// HeroDive: halt the dive and pull back (no cost, no engage).
    Halt,
    /// FoeDive: this front-line hero intercepts the runner (pays Tempo = the runner's Speed).
    Intercept(usize),
    /// FoeDive: stop adding interceptors; let the runner through to the back line.
    LetThrough,
    /// This hero plays this action card.
    PlayAction(usize, usize),
    /// End the player phase; the foe phase begins.
    EndRound,
    /// Play one move in the active Clash.
    Play(Move),
    /// Foe phase: defend the current incoming attack (Focus → survive, foe reset).
    Defend,
    /// Foe phase: counterattack the current incoming attack (Tempo → mutual, can kill).
    Counter,
    /// Foe phase: take the incoming attack as a free hit (base damage).
    TakeHit,
}

/// The ruleset. Holds no state of its own.
#[derive(Clone, Copy, Debug, Default)]
pub struct Deckbound;

fn menu_state(seed: u64) -> State {
    State {
        round: 0,
        heroes: Vec::new(),
        creatures: Vec::new(),
        phase: Phase::Menu(Menu::Top),
        duel: None,
        scenario: None,
        exiting: false,
        log: vec!["Deckbound - choose a scenario set.".into()],
        rng: Rng::new(seed),
        seed,
        outcome: None,
        engaged: Vec::new(),
        queued_cards: Vec::new(),
        foe_queue: Vec::new(),
        dive: None,
        versus: None,
    }
}

fn list_for(menu: Menu) -> Vec<Scenario> {
    match menu {
        Menu::Scenarios => scenarios::campaign(),
        Menu::God => scenarios::god(),
        Menu::Tutorial => scenarios::tutorials(),
        Menu::Versus => scenarios::versus(),
        Menu::Top => Vec::new(),
    }
}

fn load_scenario(state: &mut State, scenario: Scenario) {
    let (heroes, creatures) = scenario.roster();
    state.heroes = heroes;
    state.creatures = creatures;
    state.duel = None;
    state.round = 1;
    state.outcome = None;
    state.reset_round_plan();
    if scenario.pvp {
        // A hotseat PvP duel: both sides human, lockstep beats (§3.4).
        state.versus = Some(Versus::new());
        state.phase = Phase::Versus;
        state.log = vec![scenario.blurb.clone(), "-- the duel begins --".into()];
    } else {
        state.versus = None;
        state.phase = Phase::Choosing;
        state.log = vec![scenario.blurb.clone(), "-- Round 1 --".into()];
    }
    state.scenario = Some(scenario);
}

pub(crate) fn check_outcome(state: &mut State) {
    if state.living_creatures() == 0 {
        state.duel = None;
        state.outcome = Some(Outcome::Win(PlayerId(0)));
        state.log.push("Every foe is down - victory!".into());
    } else if state.living_heroes() == 0 {
        state.duel = None;
        state.outcome = Some(Outcome::Win(PlayerId(1)));
        state.log.push("The party has fallen.".into());
    }
}

fn form_duel(state: &mut State, hero: usize, foe: usize) {
    // Engaging spends Tempo (cost = foe's Speed), pay-after (§3.1): the engage happens; if it
    // drives Tempo negative it is simply your last action. The engaged foe spends its one
    // action defending, so it does not also attack this round. The Clash is **mutual** —
    // results stick both ways (the hero can kill, the foe can hit back). Force starts at 0.
    let speed = state.creatures[foe].offense.speed.max(1);
    state.heroes[hero].tempo -= speed as i32;
    state.engaged[foe] = true;
    state.duel = Some(Duel {
        hero,
        foe,
        hero_force: 0,
        foe_force: 0,
        beat: 0,
        stall: 0,
        defending: false,
        from_foe_phase: false,
    });
    state.phase = Phase::Combat;
    state.log.push(format!(
        "{} engages the {} (tempo {} left).",
        state.heroes[hero].name, state.creatures[foe].name, state.heroes[hero].tempo
    ));
}

impl Deckbound {
    /// Resolve one beat of the active Clash: the hero plays `hero_move`, the creature draws
    /// from its deck, [`duel::resolve`] settles it, and Force/Body update. **Ends-on-strike**:
    /// a connecting blow ends the duel (then the Body persists into the next).
    fn clash_beat(&self, state: &mut State, hero_move: Move) {
        let Some(duel) = state.duel else { return };

        let creature_move = {
            // Per-duel, per-beat keyed RNG so the order the player resolves duels in cannot
            // change any duel's draws (§1.9 order-independence).
            let key = state.seed
                ^ (state.round as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
                ^ (duel.hero as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
                ^ (duel.foe as u64).wrapping_mul(0x1656_67B1_9E37_79F9)
                ^ (duel.beat as u64).wrapping_mul(0x27D4_EB2F_1656_67C5);
            let mut drng = Rng::new(key);
            state.creatures[duel.foe]
                .behavior()
                .expect("a creature drives the duel")
                .pick(duel.foe_force, &mut drng)
        };

        let (h_pow, h_dt, h_pre) = base_strike(&state.heroes[duel.hero]);
        let (c_pow, c_dt, c_pre) = base_strike(&state.creatures[duel.foe]);
        let h_name = state.heroes[duel.hero].name.clone();
        let c_name = state.creatures[duel.foe].name.clone();
        let a = Side {
            power: h_pow,
            dtype: h_dt,
            precision: h_pre,
            force: duel.hero_force,
            name: &h_name,
        };
        let b = Side {
            power: c_pow,
            dtype: c_dt,
            precision: c_pre,
            force: duel.foe_force,
            name: &c_name,
        };
        let result = duel::resolve(&a, hero_move, &b, creature_move);
        state.log.push(result.note);

        if let Some(s) = result.on_a {
            combat::apply_strike(&mut state.heroes[duel.hero], s, &c_name, &mut state.log);
        }
        // In a defensive Clash the foe is reset (survival only) — the hero's hit is voided.
        if let Some(s) = result.on_b {
            if duel.defending {
                state
                    .log
                    .push(format!("  ({} breaks off, unharmed)", c_name));
            } else {
                combat::apply_strike(&mut state.creatures[duel.foe], s, &h_name, &mut state.log);
            }
        }

        let hero_down = state.heroes[duel.hero].is_down();
        let foe_down = state.creatures[duel.foe].is_down();
        // Ends-on-strike: a connecting blow (or a down) ends the duel.
        if result.ends || hero_down || foe_down {
            let from_foe = duel.from_foe_phase;
            state.duel = None;
            // Duels resolve immediately (no simultaneous tier), so finalize a death the
            // instant it lands — a decisive Win/Defeat shows at once. (A trade kills both
            // this beat; check_outcome scores the Win first, so a mutual kill is a win.)
            if hero_down {
                state.heroes[duel.hero].fallen = true;
            }
            if foe_down {
                state.creatures[duel.foe].fallen = true;
            }
            check_outcome(state);
            if state.outcome.is_some() {
                return;
            }
            if from_foe {
                // This incoming attack is resolved — drop it; continue the foe phase.
                self.next_foe(state);
            } else {
                state.phase = Phase::Choosing;
            }
        } else {
            let stall = duel.stall + 1; // no connect this beat
            if stall >= STALL_CAP {
                let from_foe = duel.from_foe_phase;
                state.duel = None;
                state
                    .log
                    .push("(neither can land a blow — they break off)".into());
                if from_foe {
                    // The incoming attack is spent (broken off); drain it and continue.
                    self.next_foe(state);
                } else {
                    state.phase = Phase::Choosing;
                }
            } else {
                state.duel = Some(Duel {
                    hero: duel.hero,
                    foe: duel.foe,
                    hero_force: result.a_force,
                    foe_force: result.b_force,
                    beat: duel.beat + 1,
                    stall,
                    defending: duel.defending,
                    from_foe_phase: duel.from_foe_phase,
                });
            }
        }
    }

    fn end_round(&self, state: &mut State) {
        // Tier 1: queued attack cards (instant).
        combat::resolve_attack_cards(state);
        if state.outcome.is_some() {
            return;
        }
        // Tier 2: the foe phase — each un-engaged foe attacks, resolved interactively
        // (Defend / Counter / Eat, or the gauntlet for a diving runner). If none attacks,
        // close the round now.
        state.foe_queue = combat::foe_attacks(state);
        if state.foe_queue.is_empty() {
            self.finish_round(state);
        } else {
            state.log.push("-- the foes strike --".into());
            self.advance_foe(state);
        }
    }

    /// Present the foe-phase attack at the front of the queue: a melee runner with guards
    /// still up opens the interactive gauntlet ([`Phase::FoeDive`]); anything else is a
    /// straight incoming attack ([`Phase::FoePhase`]). An empty queue closes the round.
    fn advance_foe(&self, state: &mut State) {
        let Some(&(foe, target)) = state.foe_queue.first() else {
            self.finish_round(state);
            return;
        };
        if combat::is_dive(state, foe, target) {
            let guards = combat::front_guards(&state.heroes);
            state.dive = Some(Dive {
                runner: foe,
                target,
                guards,
                chosen: Vec::new(),
            });
            state.phase = Phase::FoeDive;
        } else {
            state.phase = Phase::FoePhase;
        }
    }

    /// The current foe-phase attack is resolved: drop it and present the next one.
    fn next_foe(&self, state: &mut State) {
        state.dive = None;
        if !state.foe_queue.is_empty() {
            state.foe_queue.remove(0);
        }
        self.advance_foe(state);
    }

    /// Tier 3 + round close: self/ally buff cards, the death tally (foe-phase downs finalize
    /// here, §1.9), then refresh.
    fn finish_round(&self, state: &mut State) {
        combat::resolve_buff_cards(state);
        for a in state.heroes.iter_mut().chain(state.creatures.iter_mut()) {
            if a.is_down() {
                a.fallen = true;
            }
        }
        check_outcome(state);
        if state.outcome.is_some() {
            return;
        }
        for a in state.heroes.iter_mut().chain(state.creatures.iter_mut()) {
            if !a.is_down() {
                a.refresh_round();
            }
        }
        state.round += 1;
        state.reset_round_plan();
        state.phase = Phase::Choosing;
        state.log.push(format!("-- Round {} --", state.round));
    }

    /// Open a duel for an incoming foe attack (the foe initiates; the hero defends or
    /// counters). `defending` resets the foe afterward (survive only); else it's mutual.
    fn begin_foe_duel(&self, state: &mut State, hero: usize, foe: usize, defending: bool) {
        state.duel = Some(Duel {
            hero,
            foe,
            hero_force: 0,
            foe_force: 0,
            beat: 0,
            stall: 0,
            defending,
            from_foe_phase: true,
        });
        state.phase = Phase::Combat;
    }

    /// HeroDive: push through the gauntlet (§4). Pay the guards' combined Speed as Tempo,
    /// eat each guard's base hit, and — if still standing — engage the back-line foe.
    fn hero_push_through(&self, state: &mut State) -> Result<(), GameError> {
        let dive = state
            .dive
            .take()
            .ok_or_else(|| GameError::new("no dive in progress"))?;
        let h = dive.runner;
        let combined: u32 = dive
            .guards
            .iter()
            .map(|&g| state.creatures[g].offense.speed.max(1))
            .sum();
        state.heroes[h].tempo -= combined as i32; // pay-after (§3.1)
        state.log.push(format!(
            "{} pushes through (drag {combined}).",
            state.heroes[h].name
        ));
        // Each guard gets a swing as the runner crosses.
        for &g in &dive.guards {
            if state.creatures[g].is_down() {
                continue;
            }
            let (raw, dtype, precision) = base_strike(&state.creatures[g]);
            let gname = state.creatures[g].name.clone();
            combat::apply_strike(
                &mut state.heroes[h],
                duel::Strike {
                    raw,
                    dtype,
                    precision,
                },
                &gname,
                &mut state.log,
            );
            if state.heroes[h].is_down() {
                break;
            }
        }
        if state.heroes[h].is_down() {
            state.heroes[h].fallen = true;
            check_outcome(state);
            if state.outcome.is_none() {
                state.phase = Phase::Choosing;
            }
            return Ok(());
        }
        // Reached the back line — engage the foe (a normal mutual Clash).
        if state
            .creatures
            .get(dive.target)
            .is_some_and(|c| !c.is_down())
        {
            form_duel(state, h, dive.target);
        } else {
            state.phase = Phase::Choosing;
        }
        Ok(())
    }

    /// FoeDive: a front-line hero intercepts the diving runner (§4). The hero pays Tempo =
    /// the runner's Speed and strikes it; once the interceptors' combined Speed walls the
    /// runner (≥ its Speed) it is stopped and the attack is spent.
    fn foe_dive_intercept(&self, state: &mut State, g: usize) -> Result<(), GameError> {
        let mut dive = state
            .dive
            .clone()
            .ok_or_else(|| GameError::new("no dive in progress"))?;
        if !dive.guards.contains(&g) || dive.chosen.contains(&g) {
            return Err(GameError::new("that hero cannot intercept"));
        }
        let runner = dive.runner;
        let runner_speed = state.creatures[runner].offense.speed.max(1);
        state.heroes[g].tempo -= runner_speed as i32; // pay-after (§3.1)
        let (raw, dtype, precision) = base_strike(&state.heroes[g]);
        let hname = state.heroes[g].name.clone();
        state.log.push(format!(
            "{hname} cuts in to block the {}.",
            state.creatures[runner].name
        ));
        combat::apply_strike(
            &mut state.creatures[runner],
            duel::Strike {
                raw,
                dtype,
                precision,
            },
            &hname,
            &mut state.log,
        );
        dive.chosen.push(g);
        let drag: u32 = dive
            .chosen
            .iter()
            .map(|&i| state.heroes[i].offense.speed.max(1))
            .sum();
        if state.creatures[runner].is_down() {
            state.creatures[runner].fallen = true;
            check_outcome(state);
            if state.outcome.is_none() {
                self.next_foe(state);
            }
        } else if drag >= runner_speed {
            state.log.push(format!(
                "the line walls off the {} (drag {drag} ≥ {runner_speed}).",
                state.creatures[runner].name
            ));
            self.next_foe(state);
        } else {
            state.dive = Some(dive);
        }
        Ok(())
    }

    /// FoeDive: stop intercepting — the runner slips the line and free-hits its back-line
    /// target (§4).
    fn foe_dive_let_through(&self, state: &mut State) {
        let Some(dive) = state.dive.clone() else {
            return;
        };
        state.log.push(format!(
            "the {} slips the line!",
            state.creatures[dive.runner].name
        ));
        combat::free_hit(state, dive.runner, dive.target);
        if state.heroes[dive.target].is_down() {
            state.heroes[dive.target].fallen = true;
        }
        check_outcome(state);
        if state.outcome.is_none() {
            self.next_foe(state);
        }
    }

    /// Resolve one lockstep beat of a hotseat PvP duel (§3.4): both committed moves are now
    /// revealed and settled. The duel is a fight to the fall — Body persists, no ends-on-strike
    /// — until one side drops (or both, a draw). A stalemate backstop caps a stall as a tie.
    fn versus_beat(&self, state: &mut State, a_move: Move, b_move: Move) {
        let Some(v) = state.versus else { return };
        let (a_pow, a_dt, a_pre) = base_strike(&state.heroes[0]);
        let (b_pow, b_dt, b_pre) = base_strike(&state.creatures[0]);
        let a_name = state.heroes[0].name.clone();
        let b_name = state.creatures[0].name.clone();
        let a = Side {
            power: a_pow,
            dtype: a_dt,
            precision: a_pre,
            force: v.a_force,
            name: &a_name,
        };
        let b = Side {
            power: b_pow,
            dtype: b_dt,
            precision: b_pre,
            force: v.b_force,
            name: &b_name,
        };
        let result = duel::resolve(&a, a_move, &b, b_move);
        state.log.push(format!(
            "Reveal: {a_name} {} / {b_name} {}.",
            a_move.name(),
            b_move.name()
        ));
        state.log.push(result.note.clone());
        if let Some(s) = result.on_a {
            combat::apply_strike(&mut state.heroes[0], s, &b_name, &mut state.log);
        }
        if let Some(s) = result.on_b {
            combat::apply_strike(&mut state.creatures[0], s, &a_name, &mut state.log);
        }

        let a_down = state.heroes[0].is_down();
        let b_down = state.creatures[0].is_down();
        if a_down || b_down {
            let outcome = if a_down && b_down {
                state.log.push("Both fighters fall — a draw!".into());
                Outcome::Tie(vec![PlayerId(0), PlayerId(1)])
            } else if b_down {
                state.log.push(format!("{b_name} falls — Player 1 wins!"));
                Outcome::Win(PlayerId(0))
            } else {
                state.log.push(format!("{a_name} falls — Player 2 wins!"));
                Outcome::Win(PlayerId(1))
            };
            state.outcome = Some(outcome);
            return;
        }

        let stall = if result.ends { 0 } else { v.stall + 1 };
        if stall >= STALL_CAP {
            state
                .log
                .push("Neither can land the killing blow — a draw.".into());
            state.outcome = Some(Outcome::Tie(vec![PlayerId(0), PlayerId(1)]));
            return;
        }
        state.versus = Some(Versus {
            a_force: result.a_force,
            b_force: result.b_force,
            beat: v.beat + 1,
            stall,
            committed: None,
        });
    }

    fn status(&self, state: &State) -> String {
        let log = state
            .log
            .iter()
            .rev()
            .take(12)
            .rev()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        let prompt = match (&state.outcome, &state.phase) {
            (Some(Outcome::Win(PlayerId(0))), _) => "Victory! Replay, or Main menu.".to_string(),
            (Some(_), _) => "Defeat. Replay, or Main menu.".to_string(),
            (None, Phase::Menu(Menu::Top)) => "Deckbound - pick a scenario set.".to_string(),
            (None, Phase::Menu(Menu::Scenarios)) => {
                "Cooperation - pick one. (Esc: back)".to_string()
            }
            (None, Phase::Menu(Menu::God)) => "God-tier - pick one. (Esc: back)".to_string(),
            (None, Phase::Menu(Menu::Tutorial)) => {
                "Duels - learn the Clash. (Esc: back)".to_string()
            }
            (None, Phase::Menu(Menu::Versus)) => {
                "Versus - pick a hotseat duel. (Esc: back)".to_string()
            }
            (None, Phase::Choosing) => format!(
                "Round {} - engage a foe, dive, reposition, queue a card, or end the round. (Esc: menu)",
                state.round
            ),
            (None, Phase::Combat) => match state.duel {
                Some(d) => format!(
                    "Clash: {} vs the {} - Strike/Anticipate, Gather/Evade. A strike ends it. (Esc: menu)",
                    state.heroes[d.hero].name, state.creatures[d.foe].name
                ),
                None => "...".to_string(),
            },
            (None, Phase::FoePhase) => match state.foe_queue.first() {
                Some(&(f, t)) => format!(
                    "The {} attacks {} - Defend (Focus), Counter (Tempo), or Take the hit. (Esc: menu)",
                    state
                        .creatures
                        .get(f)
                        .map(|x| x.name.as_str())
                        .unwrap_or("foe"),
                    state
                        .heroes
                        .get(t)
                        .map(|x| x.name.as_str())
                        .unwrap_or("you"),
                ),
                None => "...".to_string(),
            },
            (None, Phase::HeroDive) => match &state.dive {
                Some(d) => {
                    let drag: u32 = d
                        .guards
                        .iter()
                        .map(|&g| state.creatures[g].offense.speed.max(1))
                        .sum();
                    format!(
                        "{} faces the gauntlet ({} guards, drag {drag}) - Push through or Halt. (Esc: halt)",
                        state.heroes[d.runner].name,
                        d.guards.len()
                    )
                }
                None => "...".to_string(),
            },
            (None, Phase::FoeDive) => match &state.dive {
                Some(d) => format!(
                    "The {} dives for {} - pick interceptors, or Let it through. (Esc: menu)",
                    state.creatures[d.runner].name,
                    state
                        .heroes
                        .get(d.target)
                        .map(|x| x.name.as_str())
                        .unwrap_or("the back line"),
                ),
                None => "...".to_string(),
            },
            (None, Phase::Versus) => {
                let a = state
                    .heroes
                    .first()
                    .map(|x| x.name.as_str())
                    .unwrap_or("P1");
                let b = state
                    .creatures
                    .first()
                    .map(|x| x.name.as_str())
                    .unwrap_or("P2");
                match state.versus.and_then(|v| v.committed) {
                    // Side A has committed (hidden) — side B chooses without seeing it.
                    Some(_) => format!(
                        "Player 2 ({b}): commit your move. Player 1 has chosen — don't peek! (Esc: menu)"
                    ),
                    None => format!(
                        "Player 1 ({a}): commit your move, then pass to Player 2 ({b}). (Esc: menu)"
                    ),
                }
            }
        };
        format!("{prompt}\n\n{log}")
    }
}

impl Game for Deckbound {
    type State = State;
    type Action = Action;

    fn new_game(&self, seed: u64, _players: usize) -> State {
        menu_state(seed)
    }

    fn current_player(&self, state: &State) -> Option<PlayerId> {
        if state.outcome.is_some() {
            return None;
        }
        // In a hotseat duel, side A commits first (hidden), then side B replies.
        if state.phase == Phase::Versus {
            let b_to_move = state
                .versus
                .as_ref()
                .map(|v| v.committed.is_some())
                .unwrap_or(false);
            return Some(PlayerId(if b_to_move { 1 } else { 0 }));
        }
        Some(PlayerId(0))
    }

    fn legal_actions(&self, state: &State) -> Vec<Action> {
        if state.outcome.is_some() {
            return vec![Action::Replay, Action::ToMenu];
        }
        match &state.phase {
            Phase::Menu(Menu::Top) => vec![
                Action::OpenTutorial,
                Action::OpenScenarios,
                Action::OpenGod,
                Action::OpenVersus,
                Action::Exit,
            ],
            Phase::Menu(m) => {
                let mut a: Vec<Action> =
                    (0..list_for(*m).len()).map(Action::PickScenario).collect();
                a.push(Action::Back);
                a
            }
            Phase::Choosing => {
                let mut a = Vec::new();
                // Repositioning is free and available between rounds (§4) — even with no Tempo.
                for (h, hero) in state.heroes.iter().enumerate() {
                    if !hero.fallen {
                        a.push(Action::Reposition(h));
                    }
                }
                for (h, hero) in state.heroes.iter().enumerate() {
                    if !state.hero_can_act(h) {
                        continue;
                    }
                    for (f, foe) in state.creatures.iter().enumerate() {
                        if foe.is_down() {
                            continue;
                        }
                        // Reach (§4): hit it directly, or dive the gauntlet for a guarded back-liner.
                        if combat::reaches_directly(hero, &state.creatures, f) {
                            a.push(Action::Engage(h, f));
                        } else {
                            a.push(Action::Dive(h, f));
                        }
                    }
                    for idx in 0..hero.actions.len() {
                        a.push(Action::PlayAction(h, idx));
                    }
                }
                a.push(Action::EndRound);
                a.push(Action::ToMenu);
                a
            }
            Phase::Combat => {
                // The kit is always complete — all four moves available every beat.
                vec![
                    Action::Play(Move::Strike),
                    Action::Play(Move::Anticipate),
                    Action::Play(Move::Gather),
                    Action::Play(Move::Evade),
                ]
            }
            Phase::FoePhase => {
                let mut a = Vec::new();
                if let Some(&(foe, target)) = state.foe_queue.first() {
                    let cost = state.creatures[foe].offense.speed.max(1);
                    if state.heroes[target].focus >= cost {
                        a.push(Action::Defend);
                    }
                    a.push(Action::Counter);
                    a.push(Action::TakeHit);
                }
                a.push(Action::ToMenu);
                a
            }
            Phase::Versus => {
                // Both sides have the full kit every beat (the same hidden-commit choice).
                vec![
                    Action::Play(Move::Strike),
                    Action::Play(Move::Anticipate),
                    Action::Play(Move::Gather),
                    Action::Play(Move::Evade),
                    Action::ToMenu,
                ]
            }
            Phase::HeroDive => {
                vec![Action::PushThrough, Action::Halt, Action::ToMenu]
            }
            Phase::FoeDive => {
                let mut a = Vec::new();
                if let Some(d) = &state.dive {
                    for &g in &d.guards {
                        if !state.heroes[g].fallen && !d.chosen.contains(&g) {
                            a.push(Action::Intercept(g));
                        }
                    }
                }
                a.push(Action::LetThrough);
                a.push(Action::ToMenu);
                a
            }
        }
    }

    fn action_label(&self, state: &State, action: &Action) -> String {
        match action {
            Action::OpenScenarios => "Cooperation".into(),
            Action::OpenGod => "God-tier".into(),
            Action::OpenTutorial => "Duels".into(),
            Action::OpenVersus => "Versus (hotseat)".into(),
            Action::Exit => "Exit".into(),
            Action::ToMenu => "Main menu".into(),
            Action::Back => "< Back".into(),
            Action::Replay => "Replay this scenario".into(),
            Action::EndRound => "End round (foes act)".into(),
            Action::PickScenario(i) => match &state.phase {
                Phase::Menu(m) => list_for(*m)
                    .get(*i)
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| "?".into()),
                _ => "?".into(),
            },
            Action::Engage(h, f) => {
                let hero = state.heroes.get(*h).map(|x| x.name.as_str()).unwrap_or("?");
                let foe = state
                    .creatures
                    .get(*f)
                    .map(|x| x.name.as_str())
                    .unwrap_or("?");
                format!("{hero} engages the {foe}")
            }
            Action::Dive(h, f) => {
                let hero = state.heroes.get(*h).map(|x| x.name.as_str()).unwrap_or("?");
                let foe = state
                    .creatures
                    .get(*f)
                    .map(|x| x.name.as_str())
                    .unwrap_or("?");
                format!("{hero} dives the gauntlet for the {foe} (back line)")
            }
            Action::Reposition(h) => {
                let hero = state.heroes.get(*h);
                let name = hero.map(|x| x.name.as_str()).unwrap_or("?");
                let to = match hero.map(|x| x.line) {
                    Some(crate::actor::Line::Front) => "back",
                    _ => "front",
                };
                format!("{name}: shift to the {to} line (free)")
            }
            Action::PushThrough => {
                let drag: u32 = state
                    .dive
                    .as_ref()
                    .map(|d| {
                        d.guards
                            .iter()
                            .map(|&g| state.creatures[g].offense.speed.max(1))
                            .sum()
                    })
                    .unwrap_or(0);
                format!("Push through (pay {drag} Tempo, eat the guards' hits)")
            }
            Action::Halt => "Halt — pull back (no cost)".into(),
            Action::Intercept(g) => {
                let hero = state.heroes.get(*g).map(|x| x.name.as_str()).unwrap_or("?");
                let speed = state
                    .dive
                    .as_ref()
                    .map(|d| state.creatures[d.runner].offense.speed.max(1))
                    .unwrap_or(0);
                format!("{hero} intercepts (pay {speed} Tempo, strike the runner)")
            }
            Action::LetThrough => "Let it through (it free-hits the back line)".into(),
            Action::PlayAction(h, idx) => {
                let hero = state.heroes.get(*h);
                let name = hero.map(|x| x.name.as_str()).unwrap_or("?");
                let card = hero.and_then(|x| x.actions.get(*idx));
                match card {
                    Some(c) => format!("{name}: {} ({})", c.name, c.summary()),
                    None => format!("{name}: ?"),
                }
            }
            Action::Play(Move::Strike) => "Strike - hit where they are (beats Gather)".into(),
            Action::Play(Move::Anticipate) => "Anticipate - lead them (beats Evade)".into(),
            Action::Play(Move::Gather) => "Gather - hold & build Force (beats Anticipate)".into(),
            Action::Play(Move::Evade) => "Evade - dodge & steal (beats Strike)".into(),
            Action::Defend => {
                let foe = state
                    .foe_queue
                    .first()
                    .and_then(|&(f, _)| state.creatures.get(f))
                    .map(|x| x.name.as_str())
                    .unwrap_or("foe");
                format!("Defend (Focus) - survive the {foe}, can't kill it")
            }
            Action::Counter => {
                let foe = state
                    .foe_queue
                    .first()
                    .and_then(|&(f, _)| state.creatures.get(f))
                    .map(|x| x.name.as_str())
                    .unwrap_or("foe");
                format!("Counter (Tempo) - duel the {foe}, can kill but risk a hit")
            }
            Action::TakeHit => "Take the hit (free; base damage)".into(),
        }
    }

    fn apply(&self, state: &mut State, action: &Action) -> Result<(), GameError> {
        match action {
            Action::Exit => {
                state.exiting = true;
                return Ok(());
            }
            Action::ToMenu => {
                *state = menu_state(state.seed);
                return Ok(());
            }
            Action::Replay => {
                if state.outcome.is_none() {
                    return Err(GameError::new("the fight is not over yet"));
                }
                let seed = state.seed.wrapping_add(1);
                state.seed = seed;
                state.rng = Rng::new(seed);
                if let Some(scenario) = state.scenario.clone() {
                    load_scenario(state, scenario);
                }
                return Ok(());
            }
            _ => {}
        }
        if state.outcome.is_some() {
            return Err(GameError::new("the fight is over"));
        }
        match (&state.phase, action) {
            (Phase::Menu(Menu::Top), Action::OpenScenarios) => {
                state.phase = Phase::Menu(Menu::Scenarios)
            }
            (Phase::Menu(Menu::Top), Action::OpenGod) => state.phase = Phase::Menu(Menu::God),
            (Phase::Menu(Menu::Top), Action::OpenTutorial) => {
                state.phase = Phase::Menu(Menu::Tutorial)
            }
            (Phase::Menu(Menu::Top), Action::OpenVersus) => state.phase = Phase::Menu(Menu::Versus),
            (Phase::Menu(m), Action::PickScenario(i)) if *m != Menu::Top => {
                let s = list_for(*m)
                    .into_iter()
                    .nth(*i)
                    .ok_or_else(|| GameError::new("no such scenario"))?;
                load_scenario(state, s);
            }
            (Phase::Menu(_), Action::Back) => state.phase = Phase::Menu(Menu::Top),
            (Phase::Choosing, Action::Engage(h, f)) => {
                if state.hero_can_act(*h) && state.creatures.get(*f).is_some_and(|x| !x.is_down()) {
                    form_duel(state, *h, *f);
                } else {
                    return Err(GameError::new("that engagement is not available"));
                }
            }
            (Phase::Choosing, Action::PlayAction(h, idx)) => {
                let valid = state.hero_can_act(*h)
                    && state.heroes.get(*h).is_some_and(|x| *idx < x.actions.len());
                if !valid {
                    return Err(GameError::new("that action is not available"));
                }
                // Queue the card; its effects resolve in tiers at round end (§1.9 —
                // attacks before buffs). Tempo is paid now (pay-after, §3.1).
                state.queued_cards.push((*h, *idx));
                state.heroes[*h].tempo -= ACTION_COST as i32;
            }
            (Phase::Choosing, Action::Reposition(h)) => {
                let hero = state
                    .heroes
                    .get_mut(*h)
                    .filter(|x| !x.fallen)
                    .ok_or_else(|| GameError::new("no such hero"))?;
                hero.line = match hero.line {
                    crate::actor::Line::Front => crate::actor::Line::Back,
                    crate::actor::Line::Back => crate::actor::Line::Front,
                };
                let line = if hero.line == crate::actor::Line::Front {
                    "front"
                } else {
                    "back"
                };
                let name = hero.name.clone();
                state.log.push(format!("{name} shifts to the {line} line."));
            }
            (Phase::Choosing, Action::Dive(h, f)) => {
                if !self.legal_actions(state).contains(action) {
                    return Err(GameError::new("that dive is not available"));
                }
                let guards = combat::front_guards(&state.creatures);
                state.dive = Some(Dive {
                    runner: *h,
                    target: *f,
                    guards,
                    chosen: Vec::new(),
                });
                state.phase = Phase::HeroDive;
                state.log.push(format!(
                    "{} charges the gauntlet toward the {}.",
                    state.heroes[*h].name, state.creatures[*f].name
                ));
            }
            (Phase::HeroDive, Action::PushThrough) => self.hero_push_through(state)?,
            (Phase::HeroDive, Action::Halt) => {
                if let Some(d) = state.dive.take() {
                    state
                        .log
                        .push(format!("{} pulls back.", state.heroes[d.runner].name));
                }
                state.phase = Phase::Choosing;
            }
            (Phase::FoeDive, Action::Intercept(g)) => self.foe_dive_intercept(state, *g)?,
            (Phase::FoeDive, Action::LetThrough) => self.foe_dive_let_through(state),
            (Phase::Versus, Action::Play(m)) => {
                let committed = state
                    .versus
                    .as_ref()
                    .ok_or_else(|| GameError::new("no duel in progress"))?
                    .committed;
                match committed {
                    None => {
                        // Side A commits in secret; pass the device to side B.
                        if let Some(v) = state.versus.as_mut() {
                            v.committed = Some(*m);
                        }
                        state
                            .log
                            .push("Player 1 has committed — pass to Player 2.".into());
                    }
                    Some(am) => {
                        if let Some(v) = state.versus.as_mut() {
                            v.committed = None;
                        }
                        self.versus_beat(state, am, *m);
                    }
                }
            }
            (Phase::Choosing, Action::EndRound) => self.end_round(state),
            (Phase::Combat, Action::Play(m)) => {
                if state.duel.is_none() {
                    return Err(GameError::new("no active duel"));
                }
                if !self.legal_actions(state).contains(action) {
                    return Err(GameError::new("that move is not available"));
                }
                self.clash_beat(state, *m);
            }
            (Phase::FoePhase, Action::TakeHit) => {
                let (foe, target) = *state
                    .foe_queue
                    .first()
                    .ok_or_else(|| GameError::new("no incoming attack"))?;
                combat::free_hit(state, foe, target);
                if state.heroes[target].is_down() {
                    state.heroes[target].fallen = true;
                }
                check_outcome(state);
                if state.outcome.is_none() {
                    self.next_foe(state);
                }
            }
            (Phase::FoePhase, Action::Defend) => {
                let (foe, target) = *state
                    .foe_queue
                    .first()
                    .ok_or_else(|| GameError::new("no incoming attack"))?;
                let cost = state.creatures[foe].offense.speed.max(1);
                if state.heroes[target].focus < cost {
                    return Err(GameError::new("not enough focus to defend"));
                }
                state.heroes[target].focus -= cost;
                self.begin_foe_duel(state, target, foe, true);
            }
            (Phase::FoePhase, Action::Counter) => {
                let (foe, target) = *state
                    .foe_queue
                    .first()
                    .ok_or_else(|| GameError::new("no incoming attack"))?;
                let cost = state.creatures[foe].offense.speed.max(1);
                state.heroes[target].tempo -= cost as i32; // pay-after
                self.begin_foe_duel(state, target, foe, false);
            }
            _ => return Err(GameError::new("that action is not legal right now")),
        }
        Ok(())
    }

    fn outcome(&self, state: &State) -> Option<Outcome> {
        state.outcome.clone()
    }

    fn cancel_action(&self, state: &State) -> Option<Action> {
        if state.outcome.is_some() {
            return None;
        }
        match &state.phase {
            Phase::Menu(Menu::Top) => None,
            Phase::Menu(_) => Some(Action::Back),
            Phase::HeroDive => Some(Action::Halt),
            Phase::Choosing | Phase::Combat | Phase::FoePhase | Phase::FoeDive | Phase::Versus => {
                Some(Action::ToMenu)
            }
        }
    }

    fn exit_requested(&self, state: &State) -> bool {
        state.exiting
    }

    fn is_exit_action(&self, _state: &State, action: &Action) -> bool {
        matches!(action, Action::Exit)
    }

    fn view(&self, state: &State, _perspective: Option<PlayerId>) -> TableView {
        let mut zones = Vec::new();
        match &state.phase {
            Phase::Menu(Menu::Top) => zones.push(menu_options_zone()),
            Phase::Menu(m) => zones.push(scenario_list_zone(*m)),
            Phase::Choosing => {
                zones.push(creature_zone(state, None));
                zones.push(hero_zone(state, None));
            }
            Phase::Combat => {
                let (ah, af) = match state.duel {
                    Some(d) => (Some(d.hero), Some(d.foe)),
                    None => (None, None),
                };
                zones.push(creature_zone(state, af));
                if let Some(d) = state.duel {
                    let foe_name = state.creatures[d.foe].name.clone();
                    zones.push(force_zone(
                        &format!("The {foe_name}'s Force"),
                        d.foe_force,
                        Accent::Foe,
                    ));
                    let hero_name = state.heroes[d.hero].name.clone();
                    zones.push(force_zone(
                        &format!("{hero_name}'s Force"),
                        d.hero_force,
                        Accent::Good,
                    ));
                }
                zones.push(hero_zone(state, ah));
            }
            Phase::FoePhase => {
                let (af, ah) = match state.foe_queue.first() {
                    Some(&(f, t)) => (Some(f), Some(t)),
                    None => (None, None),
                };
                zones.push(creature_zone(state, af));
                zones.push(hero_zone(state, ah));
            }
            Phase::HeroDive | Phase::FoeDive => {
                let (af, ah) = match &state.dive {
                    Some(d) if state.phase == Phase::HeroDive => (Some(d.target), Some(d.runner)),
                    Some(d) => (Some(d.runner), Some(d.target)),
                    None => (None, None),
                };
                zones.push(creature_zone(state, af));
                zones.push(hero_zone(state, ah));
            }
            Phase::Versus => {
                // Both sides on the table; the committed move is never rendered (hidden commit).
                zones.push(creature_zone(state, Some(0)));
                if let Some(v) = state.versus {
                    zones.push(force_zone(
                        &format!("Player 2 ({})'s Force", state.creatures[0].name),
                        v.b_force,
                        Accent::Foe,
                    ));
                    zones.push(force_zone(
                        &format!("Player 1 ({})'s Force", state.heroes[0].name),
                        v.a_force,
                        Accent::Good,
                    ));
                }
                zones.push(hero_zone(state, Some(0)));
            }
        }
        TableView {
            status: self.status(state),
            zones,
        }
    }
}

// ---- view helpers -------------------------------------------------------

fn pips(remaining: u32, max: u32) -> String {
    let lost = max.saturating_sub(remaining) as usize;
    format!("{}{}", "#".repeat(remaining as usize), ".".repeat(lost))
}

fn actor_card(a: &crate::actor::Actor, show_budgets: bool, accent: Accent) -> CardView {
    let d = &a.defense;
    let mut body = vec![
        format!("HP [{}]", pips(d.body.remaining, d.body.max)),
        format!("Spd {} Pow {}", a.offense.speed, a.offense.power),
    ];
    if show_budgets {
        body.push(format!("tempo {} focus {}", a.tempo, a.focus));
    } else {
        body.push(format!("R{} M{}", d.resolve, d.mind));
    }
    CardView::up(a.name.clone())
        .typed(a.role.clone())
        .body(body)
        .corner(format!("{}/{}", d.body.remaining, d.body.max))
        .accent(accent)
}

fn creature_zone(state: &State, active: Option<usize>) -> ZoneView {
    ZoneView {
        label: "The warband".into(),
        layout: Layout::Row,
        owner: None,
        cards: state
            .creatures
            .iter()
            .enumerate()
            .filter(|(_, c)| !c.is_down())
            .map(|(i, c)| {
                actor_card(
                    c,
                    false,
                    if active == Some(i) {
                        Accent::Selected
                    } else {
                        Accent::Foe
                    },
                )
            })
            .collect(),
    }
}

fn hero_zone(state: &State, active: Option<usize>) -> ZoneView {
    ZoneView {
        label: "Your party".into(),
        layout: Layout::Row,
        owner: Some(PlayerId(0)),
        cards: state
            .heroes
            .iter()
            .enumerate()
            .filter(|(_, h)| !h.fallen)
            .map(|(i, h)| {
                actor_card(
                    h,
                    true,
                    if active == Some(i) {
                        Accent::Selected
                    } else {
                        Accent::Ally
                    },
                )
            })
            .collect(),
    }
}

/// Render a side's Force as a stack — each unit doubles the next hit (`×2^force`).
fn force_zone(label: &str, force: u32, accent: Accent) -> ZoneView {
    let mult = if force >= 16 { u32::MAX } else { 1u32 << force };
    ZoneView {
        label: format!("{label} (×{mult})"),
        layout: Layout::Stack,
        owner: None,
        cards: (0..force.min(16))
            .map(|_| CardView::up("Force").accent(accent))
            .collect(),
    }
}

fn menu_options_zone() -> ZoneView {
    ZoneView {
        label: "Deckbound".into(),
        layout: Layout::Row,
        owner: None,
        cards: vec![
            CardView::up("Duels")
                .typed("set")
                .body(vec!["Start here - learn the Clash.".into()])
                .accent(Accent::Good),
            CardView::up("Cooperation")
                .typed("set")
                .body(vec!["Roles & teamwork.".into()])
                .accent(Accent::Ally),
            CardView::up("God-tier")
                .typed("set")
                .body(vec!["One vs many; be strategic.".into()])
                .accent(Accent::Good),
            CardView::up("Exit")
                .typed("menu")
                .body(vec!["Quit.".into()]),
        ],
    }
}

fn scenario_list_zone(menu: Menu) -> ZoneView {
    let label = match menu {
        Menu::Scenarios => "Cooperation",
        Menu::God => "God-tier",
        Menu::Tutorial => "Duels",
        Menu::Versus => "Versus (hotseat)",
        Menu::Top => "",
    };
    ZoneView {
        label: label.into(),
        layout: Layout::Row,
        owner: None,
        cards: list_for(menu)
            .iter()
            .map(|s| {
                CardView::up(s.name.clone())
                    .typed("scenario")
                    .body(vec![s.blurb.clone()])
                    .accent(Accent::Good)
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn launch(game: &Deckbound, state: &mut State, open: Action, index: usize) {
        game.apply(state, &open).unwrap();
        game.apply(state, &Action::PickScenario(index)).unwrap();
    }

    /// Drive a scenario to an outcome with a rough auto-strategy. Returns the outcome.
    fn autoplay(game: &Deckbound, s: &mut State) -> Outcome {
        let mut guard = 0;
        while game.current_player(s).is_some() {
            let action = match s.phase {
                Phase::Combat => {
                    // Alternate Strike/Anticipate by beat: Strike connects vs anything but a
                    // dodge, Anticipate connects vs a dodge — so a connect (ends-on-strike)
                    // comes fast against any deck, terminating the duel.
                    let beat = s.duel.map(|d| d.beat).unwrap_or(0);
                    if beat % 2 == 0 {
                        Action::Play(Move::Strike)
                    } else {
                        Action::Play(Move::Anticipate)
                    }
                }
                Phase::Choosing => {
                    // Engage the first foe if a hero can; else end the round.
                    game.legal_actions(s)
                        .into_iter()
                        .find(|a| matches!(a, Action::Engage(_, _)))
                        .unwrap_or(Action::EndRound)
                }
                Phase::FoePhase => {
                    // Defend if we can afford the Focus; else eat the hit.
                    let acts = game.legal_actions(s);
                    if acts.contains(&Action::Defend) {
                        Action::Defend
                    } else {
                        Action::TakeHit
                    }
                }
                // The gauntlet: a diving hero pulls back; a diving foe is let through.
                Phase::HeroDive => Action::Halt,
                Phase::FoeDive => Action::LetThrough,
                // Hotseat duel: both sides just trade Strikes — Body attrition terminates it.
                Phase::Versus => Action::Play(Move::Strike),
                _ => break,
            };
            game.apply(s, &action).unwrap();
            guard += 1;
            assert!(guard < 20_000, "scenario should terminate");
        }
        game.outcome(s).unwrap()
    }

    #[test]
    fn new_game_starts_in_menu() {
        let s = Deckbound.new_game(1, 1);
        assert_eq!(s.phase, Phase::Menu(Menu::Top));
    }

    #[test]
    fn a_tutorial_duel_runs_to_an_outcome() {
        let game = Deckbound;
        let mut s = game.new_game(1, 1);
        launch(&game, &mut s, Action::OpenTutorial, 0);
        assert_eq!(s.phase, Phase::Choosing);
        let _ = autoplay(&game, &mut s);
    }

    #[test]
    fn every_scenario_terminates() {
        let game = Deckbound;
        for open in [
            Action::OpenScenarios,
            Action::OpenGod,
            Action::OpenTutorial,
            Action::OpenVersus,
        ] {
            let count = match open {
                Action::OpenScenarios => scenarios::campaign().len(),
                Action::OpenGod => scenarios::god().len(),
                Action::OpenVersus => scenarios::versus().len(),
                _ => scenarios::tutorials().len(),
            };
            for i in 0..count {
                let mut s = game.new_game(7 + i as u64, 1);
                launch(&game, &mut s, open, i);
                let _ = autoplay(&game, &mut s);
            }
        }
    }

    fn god_state(seed: u64, name_contains: &str) -> State {
        let scen = scenarios::god()
            .into_iter()
            .find(|s| s.name.contains(name_contains))
            .unwrap();
        let mut s = menu_state(seed);
        load_scenario(&mut s, scen);
        s
    }

    fn campaign_state(seed: u64, name_contains: &str) -> State {
        let scen = scenarios::campaign()
            .into_iter()
            .find(|s| s.name.contains(name_contains))
            .unwrap();
        let mut s = menu_state(seed);
        load_scenario(&mut s, scen);
        s
    }

    /// Engage a foe and play the Clash to its conclusion (alternate Strike/Anticipate).
    fn drive_duel(game: &Deckbound, s: &mut State, hero: usize, foe: usize) {
        game.apply(s, &Action::Engage(hero, foe)).unwrap();
        let mut guard = 0;
        while s.phase == Phase::Combat {
            let beat = s.duel.map(|d| d.beat).unwrap_or(0);
            let m = if beat % 2 == 0 {
                Move::Strike
            } else {
                Move::Anticipate
            };
            game.apply(s, &Action::Play(m)).unwrap();
            guard += 1;
            assert!(guard < 1000, "a duel should terminate");
        }
    }

    /// §1.9: the order the player resolves duels in must not change the end-state
    /// (guaranteed by the per-duel keyed RNG).
    #[test]
    fn duel_order_does_not_change_outcome() {
        let game = Deckbound;
        // Kael (Mind 6) can afford to read two Speed-3 husks.
        let mut a = god_state(7, "Goliath");
        drive_duel(&game, &mut a, 0, 0);
        drive_duel(&game, &mut a, 0, 1);

        let mut b = god_state(7, "Goliath");
        drive_duel(&game, &mut b, 0, 1);
        drive_duel(&game, &mut b, 0, 0);

        let bodies = |s: &State| {
            s.creatures
                .iter()
                .map(|c| c.defense.body.remaining)
                .collect::<Vec<_>>()
        };
        assert_eq!(bodies(&a), bodies(&b), "duel order changed foe outcomes");
        assert_eq!(
            a.heroes[0].defense.body.remaining,
            b.heroes[0].defense.body.remaining
        );
        assert_eq!(a.heroes[0].tempo, b.heroes[0].tempo);
        assert_eq!(a.heroes[0].focus, b.heroes[0].focus);
    }

    /// §1 The Clash: the four standing moves are always available (this is what makes
    /// "avoid" and "land" hold for the whole duel — defense never depletes).
    #[test]
    fn combat_always_offers_the_standing_moves() {
        let game = Deckbound;
        let mut s = god_state(3, "Goliath");
        game.apply(&mut s, &Action::Engage(0, 0)).unwrap();
        assert_eq!(s.phase, Phase::Combat);
        assert!(s.engaged[0], "the engaged foe won't also free-hit");
        let acts = game.legal_actions(&s);
        for m in [Move::Strike, Move::Anticipate, Move::Gather, Move::Evade] {
            assert!(acts.contains(&Action::Play(m)), "{m:?} is always available");
        }
    }

    /// §1.9: action cards are queued and resolve in tiers at round end (attacks before
    /// buffs), not the instant they are played.
    #[test]
    fn action_cards_resolve_at_round_end_not_immediately() {
        let game = Deckbound;
        let mut s = campaign_state(5, "Warband");
        let sefa = s.heroes.iter().position(|h| h.name == "Sefa").unwrap();
        let fs = s.heroes[sefa]
            .actions
            .iter()
            .position(|c| c.name == "Firestorm")
            .unwrap();
        let before = s.creatures.iter().filter(|c| !c.is_down()).count();
        game.apply(&mut s, &Action::PlayAction(sefa, fs)).unwrap();
        let mid = s.creatures.iter().filter(|c| !c.is_down()).count();
        assert_eq!(before, mid, "the card is queued, not applied immediately");
        game.apply(&mut s, &Action::EndRound).unwrap();
        let after = s.creatures.iter().filter(|c| !c.is_down()).count();
        assert!(after < before, "the queued Firestorm resolves at round end");
    }

    /// §3.1: pay-after — the action that drives Tempo negative still happens (it's just your
    /// last). No Exposed penalty (§3.3 removed).
    #[test]
    fn pay_after_grants_the_action() {
        let game = Deckbound;
        let mut s = god_state(1, "Goliath");
        s.heroes[0].tempo = 1;
        game.apply(&mut s, &Action::Engage(0, 0)).unwrap();
        assert!(
            s.heroes[0].tempo < 0,
            "pay-after: the engage still happened"
        );
        assert_eq!(s.phase, Phase::Combat, "the duel formed");
        assert!(s.duel.is_some());
        // With Tempo negative, the hero can take no further action this round.
        assert!(!s.hero_can_act(0));
    }

    /// §4 reach: a ranged hero reaches a guarded back-line foe directly; a melee hero can
    /// only get there by diving the gauntlet.
    #[test]
    fn ranged_reaches_the_back_line_but_melee_must_dive() {
        let game = Deckbound;
        let s = campaign_state(2, "Pierce the Line");
        let aldric = s.heroes.iter().position(|h| h.name == "Aldric").unwrap();
        let tamsin = s.heroes.iter().position(|h| h.name == "Tamsin").unwrap();
        let seer = s.creatures.iter().position(|c| c.name == "Seer").unwrap();
        assert_eq!(s.creatures[seer].line, crate::actor::Line::Back);
        let acts = game.legal_actions(&s);
        assert!(
            acts.contains(&Action::Engage(tamsin, seer)),
            "the archer shoots over the wall"
        );
        assert!(
            !acts.contains(&Action::Engage(aldric, seer)),
            "the knight can't reach the back line directly"
        );
        assert!(
            acts.contains(&Action::Dive(aldric, seer)),
            "the knight must dive the gauntlet"
        );
    }

    /// §4 reposition: shifting line is free and flips front <-> back.
    #[test]
    fn reposition_is_free_and_flips_the_line() {
        let game = Deckbound;
        let mut s = campaign_state(2, "Pierce the Line");
        let aldric = s.heroes.iter().position(|h| h.name == "Aldric").unwrap();
        assert_eq!(s.heroes[aldric].line, crate::actor::Line::Front);
        let tempo = s.heroes[aldric].tempo;
        game.apply(&mut s, &Action::Reposition(aldric)).unwrap();
        assert_eq!(s.heroes[aldric].line, crate::actor::Line::Back);
        assert_eq!(s.heroes[aldric].tempo, tempo, "repositioning is free");
    }

    /// §4 gauntlet: a runner foe dives the hero back line; the player can intercept with a
    /// front-line hero, who strikes the runner as it crosses.
    #[test]
    fn a_runner_foe_dives_and_can_be_intercepted() {
        let game = Deckbound;
        let mut s = campaign_state(2, "Pierce the Line");
        game.apply(&mut s, &Action::EndRound).unwrap();
        let mut saw_dive = false;
        let mut guard = 0;
        while matches!(s.phase, Phase::FoePhase | Phase::FoeDive) && s.outcome.is_none() {
            if s.phase == Phase::FoeDive {
                saw_dive = true;
                let intercept = game
                    .legal_actions(&s)
                    .into_iter()
                    .find(|a| matches!(a, Action::Intercept(_)));
                game.apply(&mut s, &intercept.unwrap_or(Action::LetThrough))
                    .unwrap();
            } else {
                game.apply(&mut s, &Action::TakeHit).unwrap();
            }
            guard += 1;
            assert!(guard < 100, "the foe phase should drain");
        }
        assert!(saw_dive, "the Stalker dove the hero gauntlet");
    }

    /// §3.4 PvP: a hotseat duel collects side A's commit hidden, then side B's reply, then
    /// resolves the beat — and runs to a decisive outcome.
    #[test]
    fn hotseat_duel_hides_the_commit_then_resolves() {
        let game = Deckbound;
        let mut s = game.new_game(1, 1);
        game.apply(&mut s, &Action::OpenVersus).unwrap();
        game.apply(&mut s, &Action::PickScenario(0)).unwrap();
        assert_eq!(s.phase, Phase::Versus);
        assert_eq!(game.current_player(&s), Some(PlayerId(0)));

        // Player 1 commits in secret — the turn passes to Player 2, who has not seen it.
        game.apply(&mut s, &Action::Play(Move::Strike)).unwrap();
        assert!(s.versus.unwrap().committed.is_some(), "the commit is held");
        assert_eq!(game.current_player(&s), Some(PlayerId(1)));

        // Player 2 replies — the beat resolves and the commit is cleared.
        game.apply(&mut s, &Action::Play(Move::Strike)).unwrap();
        assert!(
            s.versus.map(|v| v.committed.is_none()).unwrap_or(true),
            "the beat resolved"
        );

        // Trading Strikes is pure Body attrition — the duel reaches a verdict.
        let mut guard = 0;
        while game.current_player(&s).is_some() {
            game.apply(&mut s, &Action::Play(Move::Strike)).unwrap();
            guard += 1;
            assert!(guard < 1000, "a duel should terminate");
        }
        assert!(game.outcome(&s).is_some(), "someone won (or a draw)");
    }

    /// Item 2: ending a round with un-engaged foes opens the interactive foe phase — each
    /// incoming attack offers Defend / Counter / Eat, and resolving the whole queue returns
    /// to the player's phase (or settles the outcome).
    #[test]
    fn foe_phase_resolves_incoming_attacks() {
        let game = Deckbound;
        let mut s = god_state(3, "Goliath");
        // Don't engage anyone — every living foe will attack in the foe phase.
        game.apply(&mut s, &Action::EndRound).unwrap();
        assert_eq!(
            s.phase,
            Phase::FoePhase,
            "un-engaged foes open the foe phase"
        );
        assert!(!s.foe_queue.is_empty(), "there are incoming attacks");
        let acts = game.legal_actions(&s);
        assert!(acts.contains(&Action::TakeHit), "Eat is always available");
        assert!(acts.contains(&Action::Counter), "Counter is available");

        // Eat every incoming hit until the queue drains.
        let mut guard = 0;
        while s.phase == Phase::FoePhase {
            game.apply(&mut s, &Action::TakeHit).unwrap();
            guard += 1;
            assert!(guard < 100, "the foe phase should drain");
        }
        assert!(
            s.phase == Phase::Choosing || s.outcome.is_some(),
            "draining the queue returns to play or settles the fight"
        );
    }
}
