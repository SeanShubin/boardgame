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
use crate::state::{Duel, Menu, Phase, State};

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
    PickScenario(usize),
    Exit,
    ToMenu,
    Back,
    Replay,
    /// This hero engages this foe (begins a Clash; spends Tempo = the foe's Speed).
    Engage(usize, usize),
    /// This hero plays this action card.
    PlayAction(usize, usize),
    /// End the player phase; creatures act, then the round refreshes.
    EndRound,
    /// Play one move in the active Clash.
    Play(Move),
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
    }
}

fn list_for(menu: Menu) -> Vec<Scenario> {
    match menu {
        Menu::Scenarios => scenarios::campaign(),
        Menu::God => scenarios::god(),
        Menu::Tutorial => scenarios::tutorials(),
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
    state.log = vec![scenario.blurb.clone(), "-- Round 1 --".into()];
    state.scenario = Some(scenario);
    state.phase = Phase::Choosing;
    state.reset_round_plan();
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
    // Engaging spends Tempo (cost = foe's Speed), pay-after: the engage happens and, if
    // it drives Tempo negative, you go all-in (§3.3). The foe is now engaged and will not
    // also free-hit at round end. The Clash itself starts at zero Charges on both sides.
    let speed = state.creatures[foe].offense.speed.max(1);
    let h = &mut state.heroes[hero];
    h.tempo -= speed as i32;
    if h.tempo < 0 {
        h.expose();
    }
    state.engaged[foe] = true;
    state.duel = Some(Duel {
        hero,
        foe,
        hero_up: 0,
        hero_down: 0,
        foe_up: 0,
        foe_down: 0,
        beat: 0,
        stall: 0,
    });
    state.phase = Phase::Combat;
    state.log.push(format!(
        "{} engages the {} (tempo {} left).",
        state.heroes[hero].name, state.creatures[foe].name, state.heroes[hero].tempo
    ));
}

impl Deckbound {
    /// Resolve one beat of the active Clash: the hero plays `hero_move`, the creature
    /// answers with its instinct, [`duel::resolve`] settles it, and Body/Charges update.
    fn clash_beat(&self, state: &mut State, hero_move: Move) {
        let Some(duel) = state.duel else { return };
        let max_h = state.heroes[duel.hero].charges_max;
        let max_f = state.creatures[duel.foe].charges_max;

        let creature_move = {
            let policy = state.creatures[duel.foe]
                .behavior()
                .map(|b| b.policy)
                .expect("a creature drives the duel");
            // Per-duel, per-beat keyed RNG so the order the player resolves duels in
            // cannot change any duel's rolls (§1.9 order-independence).
            let key = state.seed
                ^ (state.round as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
                ^ (duel.hero as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
                ^ (duel.foe as u64).wrapping_mul(0x1656_67B1_9E37_79F9)
                ^ (duel.beat as u64).wrapping_mul(0x27D4_EB2F_1656_67C5);
            let mut drng = Rng::new(key);
            policy.pick_move(duel.foe_up, duel.foe_down, max_f, &mut drng)
        };

        let (h_pow, h_dt, h_pre) = base_strike(&state.heroes[duel.hero]);
        let (c_pow, c_dt, c_pre) = base_strike(&state.creatures[duel.foe]);
        let h_name = state.heroes[duel.hero].name.clone();
        let c_name = state.creatures[duel.foe].name.clone();
        let a = Side {
            power: h_pow,
            dtype: h_dt,
            precision: h_pre,
            up: duel.hero_up,
            down: duel.hero_down,
            max: max_h,
            name: &h_name,
        };
        let b = Side {
            power: c_pow,
            dtype: c_dt,
            precision: c_pre,
            up: duel.foe_up,
            down: duel.foe_down,
            max: max_f,
            name: &c_name,
        };
        let result = duel::resolve(&a, hero_move, &b, creature_move);
        state.log.push(result.note);

        let h_body = state.heroes[duel.hero].defense.body.remaining;
        let f_body = state.creatures[duel.foe].defense.body.remaining;
        if let Some(s) = result.on_a {
            combat::apply_strike(&mut state.heroes[duel.hero], s, &c_name, &mut state.log);
        }
        if let Some(s) = result.on_b {
            combat::apply_strike(&mut state.creatures[duel.foe], s, &h_name, &mut state.log);
        }
        // Progress = Body actually lost this beat (a connecting hit can be fully absorbed
        // by armor); the backstop counts beats with no progress.
        let progressed = state.heroes[duel.hero].defense.body.remaining < h_body
            || state.creatures[duel.foe].defense.body.remaining < f_body;
        let stall = if progressed { 0 } else { duel.stall + 1 };

        let hero_down = state.heroes[duel.hero].is_down();
        let foe_down = state.creatures[duel.foe].is_down();
        if hero_down || foe_down {
            state.duel = None;
            state.phase = Phase::Choosing;
            check_outcome(state);
        } else if stall >= STALL_CAP {
            state.duel = None;
            state.phase = Phase::Choosing;
            state
                .log
                .push("(neither can land a wound — they break off)".into());
        } else {
            state.duel = Some(Duel {
                hero: duel.hero,
                foe: duel.foe,
                hero_up: result.a.up,
                hero_down: result.a.down,
                foe_up: result.b.up,
                foe_down: result.b.down,
                beat: duel.beat + 1,
                stall,
            });
        }
    }

    fn end_round(&self, state: &mut State) {
        combat::resolve_round(state);
        if state.outcome.is_some() {
            return;
        }
        // Death tally (§1.9 — downs finalize at the round boundary, not mid-stream): a
        // hero mortally wounded this round fought on and landed its committed blows; now
        // finalize who fell, then check for defeat.
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
        state.log.push(format!("-- Round {} --", state.round));
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
            (None, Phase::Choosing) => format!(
                "Round {} - engage a foe, queue a card, or end the round. (Esc: menu)",
                state.round
            ),
            (None, Phase::Combat) => match state.duel {
                Some(d) => format!(
                    "Clash: {} vs the {} - Strike/Throw, Parry/Evade, Charge/Recover. (Esc: menu)",
                    state.heroes[d.hero].name, state.creatures[d.foe].name
                ),
                None => "...".to_string(),
            },
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
            None
        } else {
            Some(PlayerId(0))
        }
    }

    fn legal_actions(&self, state: &State) -> Vec<Action> {
        if state.outcome.is_some() {
            return vec![Action::Replay, Action::ToMenu];
        }
        match &state.phase {
            Phase::Menu(Menu::Top) => vec![
                Action::OpenScenarios,
                Action::OpenGod,
                Action::OpenTutorial,
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
                for (h, hero) in state.heroes.iter().enumerate() {
                    if !state.hero_can_act(h) {
                        continue;
                    }
                    for (f, foe) in state.creatures.iter().enumerate() {
                        if !foe.is_down() {
                            a.push(Action::Engage(h, f));
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
                // The standing moves are always available (this is what makes "avoid" and
                // "land" hold for the whole Clash); the setups gate on Charge state.
                let mut a = vec![
                    Action::Play(Move::Strike),
                    Action::Play(Move::Throw),
                    Action::Play(Move::Parry),
                    Action::Play(Move::Evade),
                ];
                if let Some(d) = state.duel {
                    let max = state.heroes[d.hero].charges_max;
                    if d.hero_up + d.hero_down < max {
                        a.push(Action::Play(Move::Charge));
                    }
                    if d.hero_down > 0 {
                        a.push(Action::Play(Move::Recover));
                    }
                }
                a
            }
        }
    }

    fn action_label(&self, state: &State, action: &Action) -> String {
        match action {
            Action::OpenScenarios => "Cooperation".into(),
            Action::OpenGod => "God-tier".into(),
            Action::OpenTutorial => "Duels".into(),
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
            Action::PlayAction(h, idx) => {
                let hero = state.heroes.get(*h);
                let name = hero.map(|x| x.name.as_str()).unwrap_or("?");
                let card = hero.and_then(|x| x.actions.get(*idx));
                match card {
                    Some(c) => format!("{name}: {} ({})", c.name, c.summary()),
                    None => format!("{name}: ?"),
                }
            }
            Action::Play(Move::Strike) => "Strike - beats Evade; trades with Strike".into(),
            Action::Play(Move::Throw) => "Throw - beats Parry; loses to Strike/Evade".into(),
            Action::Play(Move::Parry) => "Parry - negate a Strike".into(),
            Action::Play(Move::Evade) => "Evade - negate a Throw".into(),
            Action::Play(Move::Charge) => "Charge - x2 your hits (durable)".into(),
            Action::Play(Move::Recover) => "Recover - flip your charges back up".into(),
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
                // attacks before buffs). Tempo is paid now (pay-after).
                state.queued_cards.push((*h, *idx));
                let hero = &mut state.heroes[*h];
                hero.tempo -= ACTION_COST as i32;
                if hero.tempo < 0 {
                    hero.expose();
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
            Phase::Choosing | Phase::Combat => Some(Action::ToMenu),
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
                    zones.push(charge_zone(
                        &format!("The {foe_name}'s Charges"),
                        d.foe_up,
                        d.foe_down,
                        Accent::Foe,
                    ));
                    let hero_name = state.heroes[d.hero].name.clone();
                    zones.push(charge_zone(
                        &format!("{hero_name}'s Charges"),
                        d.hero_up,
                        d.hero_down,
                        Accent::Good,
                    ));
                }
                zones.push(hero_zone(state, ah));
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
    if a.exposed {
        body.push("EXPOSED".into());
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

/// Render a side's Charges: `up` active (face-up, doubling) and `down` flipped (disabled
/// by a successful defense, restored by Recover).
fn charge_zone(label: &str, up: u32, down: u32, accent: Accent) -> ZoneView {
    let mut cards: Vec<CardView> = (0..up)
        .map(|_| CardView::up("Charge").accent(accent))
        .collect();
    cards.extend((0..down).map(|_| CardView::down()));
    ZoneView {
        label: format!("{label} (^{up} v{down})"),
        layout: Layout::Stack,
        owner: None,
        cards,
    }
}

fn menu_options_zone() -> ZoneView {
    ZoneView {
        label: "Deckbound".into(),
        layout: Layout::Row,
        owner: None,
        cards: vec![
            CardView::up("Cooperation")
                .typed("set")
                .body(vec!["Roles & teamwork.".into()])
                .accent(Accent::Ally),
            CardView::up("God-tier")
                .typed("set")
                .body(vec!["One vs many; be strategic.".into()])
                .accent(Accent::Good),
            CardView::up("Duels")
                .typed("set")
                .body(vec!["Learn one stance at a time.".into()])
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
                    // Alternate Strike/Throw by beat so we beat both guards over time
                    // (a pure turtle parries Strikes but folds to Throws) — keeps duels
                    // making progress and terminating.
                    let beat = s.duel.map(|d| d.beat).unwrap_or(0);
                    if beat % 2 == 0 {
                        Action::Play(Move::Strike)
                    } else {
                        Action::Play(Move::Throw)
                    }
                }
                Phase::Choosing => {
                    // Engage the first foe if a hero can; else end the round.
                    game.legal_actions(s)
                        .into_iter()
                        .find(|a| matches!(a, Action::Engage(_, _)))
                        .unwrap_or(Action::EndRound)
                }
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
        for open in [Action::OpenScenarios, Action::OpenGod, Action::OpenTutorial] {
            let count = match open {
                Action::OpenScenarios => scenarios::campaign().len(),
                Action::OpenGod => scenarios::god().len(),
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

    /// Engage a foe and play the Clash to its conclusion (wind up one Charge, then Strike).
    fn drive_duel(game: &Deckbound, s: &mut State, hero: usize, foe: usize) {
        game.apply(s, &Action::Engage(hero, foe)).unwrap();
        let mut guard = 0;
        while s.phase == Phase::Combat {
            // Alternate Strike/Throw by beat (beats both guards over time).
            let beat = s.duel.map(|d| d.beat).unwrap_or(0);
            let m = if beat % 2 == 0 {
                Move::Strike
            } else {
                Move::Throw
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
        for m in [Move::Strike, Move::Throw, Move::Parry, Move::Evade] {
            assert!(acts.contains(&Action::Play(m)), "{m:?} is always available");
        }
    }

    /// §1 The Clash: Charge is offered until capacity; Recover only once a Charge has been
    /// flipped face-down.
    #[test]
    fn charge_and_recover_gate_on_charge_state() {
        let game = Deckbound;
        let mut s = god_state(3, "Goliath");
        game.apply(&mut s, &Action::Engage(0, 0)).unwrap();
        let acts = game.legal_actions(&s);
        assert!(
            acts.contains(&Action::Play(Move::Charge)),
            "fresh duel → can Charge"
        );
        assert!(
            !acts.contains(&Action::Play(Move::Recover)),
            "no down charges → no Recover"
        );
        if let Some(d) = s.duel.as_mut() {
            d.hero_down = 1; // a defended attack flipped a charge down
        }
        assert!(
            game.legal_actions(&s)
                .contains(&Action::Play(Move::Recover)),
            "a face-down charge unlocks Recover"
        );
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

    /// §3.1/§3.3: pay-after — the action that drives Tempo negative still happens, and
    /// leaves you Exposed (all-in).
    #[test]
    fn pay_after_grants_the_action_then_exposes() {
        let game = Deckbound;
        let mut s = god_state(1, "Goliath");
        s.heroes[0].tempo = 1;
        s.heroes[0].focus = 10;
        game.apply(&mut s, &Action::Engage(0, 0)).unwrap();
        assert!(s.heroes[0].exposed, "overdrawing Tempo goes all-in");
        assert!(
            s.heroes[0].tempo < 0,
            "pay-after: the engage still happened"
        );
        assert_eq!(s.phase, Phase::Combat, "the duel formed");
        assert!(s.duel.is_some());
    }
}
