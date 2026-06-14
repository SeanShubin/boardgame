//! Deckbound as an [`engine::Game`].
//!
//! Combat is a sequence of **rounds**. In the player phase you **engage** foes
//! (spending tempo) through interactive duels and **play actions** (AoE/support);
//! at **round end** the creatures act and foes you couldn't cover (focus) free-hit.
//! A 1v1 tutorial is just the smallest case. All numbers live in `data/booklet.ron`.

use engine::{Accent, CardView, Game, GameError, Layout, Outcome, PlayerId, Rng, TableView, ZoneView};

use crate::combat::{self, base_strike};
use crate::duel::{self, Side, Stance};
use crate::scenarios::{self, Scenario};
use crate::state::{Duel, Menu, Phase, State};

/// Force an exchange after this many mutual-Marshals (implementation backstop).
const STALL_CAP: u32 = 10;
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
    /// This hero engages this foe (a duel).
    Engage(usize, usize),
    /// This hero plays this action card.
    PlayAction(usize, usize),
    /// End the player phase; creatures act, then the round refreshes.
    EndRound,
    /// A duel stance.
    Stance(Stance),
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
}

fn check_outcome(state: &mut State) {
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
    // Engaging costs the foe's Speed from the hero's tempo; overspending exposes.
    let cost = state.creatures[foe].offense.speed.max(1);
    let h = &mut state.heroes[hero];
    if cost > h.tempo {
        h.exposed = true;
        h.tempo = 0;
    } else {
        h.tempo -= cost;
    }
    state.duel = Some(Duel {
        hero,
        foe,
        hero_edge: 0,
        foe_edge: 0,
        beat: 0,
        double_marshals: 0,
    });
    state.phase = Phase::Combat;
    state.log.push(format!(
        "{} engages the {} (tempo {} left).",
        state.heroes[hero].name, state.creatures[foe].name, state.heroes[hero].tempo
    ));
}

impl Deckbound {
    /// Resolve one beat of the active duel against `hero_stance`.
    fn beat(&self, state: &mut State, hero_stance: Stance) {
        let Some(duel) = state.duel else { return };

        let creature_stance = {
            let b = state.creatures[duel.foe]
                .behavior()
                .map(|b| b.policy)
                .expect("a creature drives the duel");
            b.stance(duel.foe_edge, duel.hero_edge, &mut state.rng)
        };

        // Stall backstop: after STALL_CAP mutual-Marshals, force the exchange.
        let mut hero_stance = hero_stance;
        let mut creature_stance = creature_stance;
        let mut double_marshals = 0;
        if hero_stance == Stance::Marshal && creature_stance == Stance::Marshal {
            double_marshals = duel.double_marshals + 1;
            if double_marshals >= STALL_CAP {
                hero_stance = Stance::Unleash;
                creature_stance = Stance::Unleash;
                state
                    .log
                    .push("(stalemate held too long - forced exchange)".into());
                double_marshals = 0;
            }
        }

        let (h_pow, h_dt, h_pre) = base_strike(&state.heroes[duel.hero]);
        let (c_pow, c_dt, c_pre) = base_strike(&state.creatures[duel.foe]);
        let h_name = state.heroes[duel.hero].name.clone();
        let c_name = state.creatures[duel.foe].name.clone();
        let a = Side {
            edge: duel.hero_edge,
            power: h_pow,
            dtype: h_dt,
            precision: h_pre,
            name: &h_name,
        };
        let b = Side {
            edge: duel.foe_edge,
            power: c_pow,
            dtype: c_dt,
            precision: c_pre,
            name: &c_name,
        };
        let result = duel::resolve(&a, hero_stance, &b, creature_stance);
        state.log.push(result.note);

        if let Some(s) = result.on_a {
            combat::apply_strike(&mut state.heroes[duel.hero], s, &c_name, &mut state.log);
        }
        if let Some(s) = result.on_b {
            combat::apply_strike(&mut state.creatures[duel.foe], s, &h_name, &mut state.log);
        }

        let hero_down = state.heroes[duel.hero].is_down();
        let foe_down = state.creatures[duel.foe].is_down();
        if result.ends || hero_down || foe_down {
            state.duel = None;
            state.phase = Phase::Choosing;
            check_outcome(state);
        } else {
            state.duel = Some(Duel {
                hero: duel.hero,
                foe: duel.foe,
                hero_edge: result.a_edge,
                foe_edge: result.b_edge,
                beat: duel.beat + 1,
                double_marshals,
            });
        }
    }

    fn end_round(&self, state: &mut State) {
        state.log.push("-- creatures act --".into());
        combat::creature_phase(state);
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
            (None, Phase::Menu(Menu::Scenarios)) => "Cooperation - pick one. (Esc: back)".to_string(),
            (None, Phase::Menu(Menu::God)) => "God-tier - pick one. (Esc: back)".to_string(),
            (None, Phase::Menu(Menu::Tutorial)) => "Duels - one stance each. (Esc: back)".to_string(),
            (None, Phase::Choosing) => format!(
                "Round {} - engage a foe, play an action, or end the round. (Esc: menu)",
                state.round
            ),
            (None, Phase::Combat) => match state.duel {
                Some(d) => format!(
                    "Duel: {} vs the {} - your stance? (Esc: menu)",
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
                let mut a: Vec<Action> = (0..list_for(*m).len()).map(Action::PickScenario).collect();
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
            Phase::Combat => Stance::ALL.iter().map(|&s| Action::Stance(s)).collect(),
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
                Phase::Menu(m) => list_for(*m).get(*i).map(|s| s.name.clone()).unwrap_or_else(|| "?".into()),
                _ => "?".into(),
            },
            Action::Engage(h, f) => {
                let hero = state.heroes.get(*h).map(|x| x.name.as_str()).unwrap_or("?");
                let foe = state.creatures.get(*f).map(|x| x.name.as_str()).unwrap_or("?");
                format!("{hero} -> the {foe}")
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
            Action::Stance(Stance::Marshal) => "Marshal - gather Edge".into(),
            Action::Stance(Stance::Unleash) => "Unleash - strike with all Edge".into(),
            Action::Stance(Stance::Overwhelm) => "Overwhelm - break a guard".into(),
            Action::Stance(Stance::Parry) => "Parry - read the strike & steal".into(),
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
            (Phase::Menu(Menu::Top), Action::OpenScenarios) => state.phase = Phase::Menu(Menu::Scenarios),
            (Phase::Menu(Menu::Top), Action::OpenGod) => state.phase = Phase::Menu(Menu::God),
            (Phase::Menu(Menu::Top), Action::OpenTutorial) => state.phase = Phase::Menu(Menu::Tutorial),
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
                if !state.hero_can_act(*h) {
                    return Err(GameError::new("that hero cannot act"));
                }
                let cost = combat::play_action(state, *h, *idx).max(ACTION_COST);
                let hero = &mut state.heroes[*h];
                if cost > hero.tempo {
                    hero.exposed = true;
                    hero.tempo = 0;
                } else {
                    hero.tempo -= cost;
                }
                check_outcome(state);
            }
            (Phase::Choosing, Action::EndRound) => self.end_round(state),
            (Phase::Combat, Action::Stance(s)) => {
                if state.duel.is_none() {
                    return Err(GameError::new("no active duel"));
                }
                self.beat(state, *s);
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
                let (ah, af, he, fe) = match state.duel {
                    Some(d) => (Some(d.hero), Some(d.foe), d.hero_edge, d.foe_edge),
                    None => (None, None, 0, 0),
                };
                zones.push(creature_zone(state, af));
                let foe_name = af.map(|i| state.creatures[i].name.as_str()).unwrap_or("foe");
                zones.push(edge_zone(&format!("The {foe_name}'s Edge"), fe, Accent::Foe));
                let hero_name = ah.map(|i| state.heroes[i].name.as_str()).unwrap_or("you");
                zones.push(edge_zone(&format!("{hero_name}'s Edge"), he, Accent::Good));
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

fn actor_card(
    a: &crate::actor::Actor,
    show_budgets: bool,
    accent: Accent,
) -> CardView {
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
                    if active == Some(i) { Accent::Selected } else { Accent::Foe },
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
            .filter(|(_, h)| !h.is_down())
            .map(|(i, h)| {
                actor_card(
                    h,
                    true,
                    if active == Some(i) { Accent::Selected } else { Accent::Ally },
                )
            })
            .collect(),
    }
}

fn edge_zone(label: &str, n: u32, accent: Accent) -> ZoneView {
    ZoneView {
        label: format!("{label} ({n})"),
        layout: Layout::Stack,
        owner: None,
        cards: (0..n).map(|_| CardView::up("Edge").accent(accent)).collect(),
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
            CardView::up("Exit").typed("menu").body(vec!["Quit.".into()]),
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
                    let e = s.duel.map(|d| d.hero_edge).unwrap_or(0);
                    if e >= 2 {
                        Action::Stance(Stance::Unleash)
                    } else {
                        Action::Stance(Stance::Marshal)
                    }
                }
                Phase::Choosing => {
                    // engage the first foe if any hero can; else end the round.
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
}
