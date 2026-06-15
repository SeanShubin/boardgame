//! Deckbound as an [`engine::Game`].
//!
//! Combat is a sequence of **rounds**. In the player phase you **engage** foes
//! (spending tempo) through interactive duels and **play actions** (AoE/support);
//! at **round end** the creatures act and foes you couldn't cover (focus) free-hit.
//! A 1v1 tutorial is just the smallest case. All numbers live in `data/booklet.ron`.

use engine::{Accent, CardView, Game, GameError, Layout, Outcome, PlayerId, Rng, TableView, ZoneView};

use crate::combat::{self, base_strike};
use crate::duel::{self, Side, Stance, Strike};
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
    /// This hero reads + attacks this foe (a duel: spends Focus and Tempo).
    Engage(usize, usize),
    /// This hero attacks this foe without reading it (a magnitude trade: Tempo only).
    Trade(usize, usize),
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
    // A duel = read (Focus) + engage (Tempo), each costing the foe's Speed (§1.7/§1.8).
    // Tempo is pay-after: the engage happens and, if it drives Tempo negative, you go
    // all-in (§3.3). The foe is now engaged and will not also free-hit at round end.
    let speed = state.creatures[foe].offense.speed.max(1);
    let h = &mut state.heroes[hero];
    h.focus = h.focus.saturating_sub(speed);
    h.tempo -= speed as i32;
    if h.tempo < 0 {
        h.expose();
    }
    state.engaged[foe] = true;
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

/// A magnitude trade (§1.8): the hero attacks a foe without reading it — both take one
/// base strike, no stance, no Edge. Costs Tempo only (pay-after). The foe is engaged
/// (it will not also free-hit) and its return blow lands unmitigated.
fn trade(state: &mut State, hero: usize, foe: usize) {
    let (h_pow, h_dt, h_pre) = base_strike(&state.heroes[hero]);
    let (c_pow, c_dt, c_pre) = base_strike(&state.creatures[foe]);
    let h_name = state.heroes[hero].name.clone();
    let c_name = state.creatures[foe].name.clone();
    let speed = state.creatures[foe].offense.speed.max(1);
    state.log.push(format!("{h_name} trades blows with the {c_name}."));
    combat::apply_strike(
        &mut state.creatures[foe],
        Strike { raw: h_pow, dtype: h_dt, precision: h_pre },
        &h_name,
        &mut state.log,
    );
    combat::apply_strike(
        &mut state.heroes[hero],
        Strike { raw: c_pow, dtype: c_dt, precision: c_pre },
        &c_name,
        &mut state.log,
    );
    state.engaged[foe] = true;
    let h = &mut state.heroes[hero];
    h.tempo -= speed as i32;
    if h.tempo < 0 {
        h.expose();
    }
    check_outcome(state);
}

impl Deckbound {
    /// Resolve one beat of the active duel against `hero_stance`.
    fn beat(&self, state: &mut State, hero_stance: Stance) {
        let Some(duel) = state.duel else { return };

        let creature_stance = {
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
            policy.stance(duel.foe_edge, duel.hero_edge, &mut drng)
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
            (None, Phase::Menu(Menu::Scenarios)) => "Cooperation - pick one. (Esc: back)".to_string(),
            (None, Phase::Menu(Menu::God)) => "God-tier - pick one. (Esc: back)".to_string(),
            (None, Phase::Menu(Menu::Tutorial)) => "Duels - one stance each. (Esc: back)".to_string(),
            (None, Phase::Choosing) => format!(
                "Round {} - duel (read+strike) or trade a foe, queue a card, or end the round. (Esc: menu)",
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
                        if foe.is_down() {
                            continue;
                        }
                        // Duel (read + attack) only if Focus can afford the read; you can
                        // always Trade (attack without reading).
                        if hero.focus >= foe.offense.speed.max(1) {
                            a.push(Action::Engage(h, f));
                        }
                        a.push(Action::Trade(h, f));
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
                format!("{hero} duels the {foe} (read + strike)")
            }
            Action::Trade(h, f) => {
                let hero = state.heroes.get(*h).map(|x| x.name.as_str()).unwrap_or("?");
                let foe = state.creatures.get(*f).map(|x| x.name.as_str()).unwrap_or("?");
                format!("{hero} trades with the {foe} (strike, no read)")
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
                let can = state.hero_can_act(*h)
                    && state.creatures.get(*f).is_some_and(|x| !x.is_down())
                    && state.heroes[*h].focus >= state.creatures[*f].offense.speed.max(1);
                if can {
                    form_duel(state, *h, *f);
                } else {
                    return Err(GameError::new("that engagement is not available"));
                }
            }
            (Phase::Choosing, Action::Trade(h, f)) => {
                if state.hero_can_act(*h) && state.creatures.get(*f).is_some_and(|x| !x.is_down()) {
                    trade(state, *h, *f);
                } else {
                    return Err(GameError::new("that trade is not available"));
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
            .filter(|(_, h)| !h.fallen)
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
                    // Duel the first affordable foe; else trade; else end the round.
                    let acts = game.legal_actions(s);
                    acts.iter()
                        .find(|a| matches!(a, Action::Engage(_, _)))
                        .or_else(|| acts.iter().find(|a| matches!(a, Action::Trade(_, _))))
                        .cloned()
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

    /// Engage a foe and play the duel to its conclusion (Marshal up, then Unleash).
    fn drive_duel(game: &Deckbound, s: &mut State, hero: usize, foe: usize) {
        game.apply(s, &Action::Engage(hero, foe)).unwrap();
        let mut guard = 0;
        while s.phase == Phase::Combat {
            let e = s.duel.map(|d| d.hero_edge).unwrap_or(0);
            let stance = if e >= 2 { Stance::Unleash } else { Stance::Marshal };
            game.apply(s, &Action::Stance(stance)).unwrap();
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

    /// §1.8: a trade is a single base exchange — no duel, no Edge — and engages the foe.
    #[test]
    fn trade_exchanges_base_blows_without_a_duel() {
        let game = Deckbound;
        let mut s = god_state(3, "Goliath");
        game.apply(&mut s, &Action::Trade(0, 0)).unwrap();
        assert!(s.creatures[0].is_down(), "Kael's base strike fells the husk");
        assert_eq!(s.phase, Phase::Choosing, "a trade starts no duel");
        assert!(s.duel.is_none());
        assert!(s.engaged[0], "the traded foe is engaged — it won't also free-hit");
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
        assert!(s.heroes[0].tempo < 0, "pay-after: the engage still happened");
        assert_eq!(s.phase, Phase::Combat, "the duel formed");
        assert!(s.duel.is_some());
    }
}
