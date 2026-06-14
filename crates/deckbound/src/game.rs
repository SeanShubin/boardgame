//! Deckbound's duel sandbox as an [`engine::Game`].
//!
//! Menus pick a scenario; combat is a sequence of one-on-one duels. With more
//! than one fighter a side, you **choose the matchup** (which hero duels which
//! foe); then the duel runs beat-by-beat (see [`crate::duel`]) until a strike
//! ends it. While you duel, foes you can't watch (beyond your **bandwidth**)
//! free-hit you each beat — the swarm. Edge is public and resets every duel.

use engine::{Accent, CardView, Game, GameError, Layout, Outcome, PlayerId, Rng, TableView, ZoneView};

use crate::duel::{self, Side, Stance};
use crate::scenarios::{self, Scenario};
use crate::state::{Duel, Menu, Phase, State};

/// Force an exchange after this many mutual-Marshals (implementation backstop).
const STALL_CAP: u32 = 10;
/// Damage each unwatched foe deals per beat (the swarm tax).
const CHIP: u32 = 1;

/// One step the player can take.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    OpenScenarios,
    OpenTutorial,
    PickScenario(usize),
    Exit,
    ToMenu,
    Back,
    Replay,
    /// Matchmaking: this hero duels this foe.
    PickPair(usize, usize),
    /// A duel stance.
    Stance(Stance),
}

/// The ruleset. Holds no state of its own.
#[derive(Clone, Copy, Debug, Default)]
pub struct Deckbound;

fn menu_state(seed: u64) -> State {
    State {
        duel_no: 0,
        heroes: Vec::new(),
        creatures: Vec::new(),
        phase: Phase::Menu(Menu::Top),
        duel: None,
        scenario: None,
        exiting: false,
        log: vec!["Deckbound - choose Scenarios, Tutorial, or Exit.".into()],
        rng: Rng::new(seed),
        seed,
        outcome: None,
    }
}

fn load_scenario(state: &mut State, scenario: Scenario) {
    let (heroes, creatures) = scenario.roster();
    state.heroes = heroes;
    state.creatures = creatures;
    state.duel = None;
    state.duel_no = 0;
    state.outcome = None;
    state.log = vec![scenario.blurb.clone()];
    state.scenario = Some(scenario);
    advance(state);
}

/// Decide what happens next: end the fight, ask for a matchup, or auto-form the
/// only possible duel.
fn advance(state: &mut State) {
    if state.living_creatures() == 0 {
        state.duel = None;
        state.outcome = Some(Outcome::Win(PlayerId(0)));
        state.log.push("Every foe is down - victory!".into());
        return;
    }
    if state.living_heroes() == 0 {
        state.duel = None;
        state.outcome = Some(Outcome::Win(PlayerId(1)));
        state.log.push("The party has fallen.".into());
        return;
    }
    if state.living_heroes() == 1 && state.living_creatures() == 1 {
        let h = state.first_living_hero().unwrap();
        let f = state.first_living_creature().unwrap();
        form_duel(state, h, f);
    } else {
        state.duel = None;
        state.phase = Phase::Choosing;
    }
}

fn form_duel(state: &mut State, hero: usize, foe: usize) {
    state.duel_no += 1;
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
        "-- Duel {}: {} vs the {} --",
        state.duel_no, state.heroes[hero].name, state.creatures[foe].name
    ));
}

fn menu_catalog(phase: &Phase) -> Vec<Scenario> {
    match phase {
        Phase::Menu(Menu::Scenarios) => scenarios::campaign(),
        Phase::Menu(Menu::Tutorial) => scenarios::tutorials(),
        _ => Vec::new(),
    }
}

impl Deckbound {
    /// Resolve one beat of the active duel against `hero_stance`.
    fn beat(&self, state: &mut State, hero_stance: Stance) {
        let Some(duel) = state.duel else { return };

        // Living counts at the start of the beat drive the swarm overflow.
        let lc = state.living_creatures();
        let lh = state.living_heroes();

        // The creature reads back off the public Edge banks (not the human's
        // hidden pick this beat), so the read stays blind.
        let creature_stance = {
            let c = &state.creatures[duel.foe];
            c.stance(duel.foe_edge, duel.hero_edge, &mut state.rng)
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

        let (h_base, h_name, h_bandwidth) = {
            let h = &state.heroes[duel.hero];
            (h.base, h.name.clone(), h.bandwidth as usize)
        };
        let (c_base, c_name, c_bandwidth) = {
            let c = &state.creatures[duel.foe];
            (c.base, c.name.clone(), c.bandwidth as usize)
        };
        let a = Side {
            edge: duel.hero_edge,
            base: h_base,
            name: &h_name,
        };
        let b = Side {
            edge: duel.foe_edge,
            base: c_base,
            name: &c_name,
        };
        let result = duel::resolve(&a, hero_stance, &b, creature_stance);

        if result.a_dmg > 0 {
            state.heroes[duel.hero].body.take(result.a_dmg);
        }
        if result.b_dmg > 0 {
            state.creatures[duel.foe].body.take(result.b_dmg);
        }
        state.log.push(result.note);

        // The swarm: foes/allies you can't watch (beyond bandwidth) free-hit.
        let hero_overflow = lc.saturating_sub(h_bandwidth) as u32;
        let foe_overflow = lh.saturating_sub(c_bandwidth) as u32;
        if hero_overflow > 0 {
            let dmg = hero_overflow * CHIP;
            state.heroes[duel.hero].body.take(dmg);
            state
                .log
                .push(format!("  {hero_overflow} unwatched foe(s) harry {h_name}: -{dmg}"));
        }
        if foe_overflow > 0 {
            let dmg = foe_overflow * CHIP;
            state.creatures[duel.foe].body.take(dmg);
            state
                .log
                .push(format!("  {foe_overflow} of your number harry the {c_name}: -{dmg}"));
        }

        let hero_down = state.heroes[duel.hero].is_down();
        let foe_down = state.creatures[duel.foe].is_down();
        if result.ends || hero_down || foe_down {
            if hero_down {
                state.log.push(format!("{h_name} falls!"));
            }
            if foe_down {
                state.log.push(format!("The {c_name} falls!"));
            }
            advance(state);
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

    fn status(&self, state: &State) -> String {
        let log = state
            .log
            .iter()
            .rev()
            .take(10)
            .rev()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        let prompt = match (&state.outcome, &state.phase) {
            (Some(Outcome::Win(PlayerId(0))), _) => "Victory! Replay, or Main menu.".to_string(),
            (Some(_), _) => "Defeat. Replay, or Main menu.".to_string(),
            (None, Phase::Menu(Menu::Top)) => {
                "Deckbound - choose Scenarios, Tutorial, or Exit.".to_string()
            }
            (None, Phase::Menu(Menu::Scenarios)) => "Scenarios - pick one. (Esc: back)".to_string(),
            (None, Phase::Menu(Menu::Tutorial)) => {
                "Tutorials - each isolates one stance. (Esc: back)".to_string()
            }
            (None, Phase::Choosing) => "Choose a matchup - who duels whom? (Esc: menu)".to_string(),
            (None, Phase::Combat) => match state.duel {
                Some(d) => format!(
                    "Duel {}: {} vs the {} - your stance? (Esc: menu)",
                    state.duel_no, state.heroes[d.hero].name, state.creatures[d.foe].name
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
            Phase::Menu(Menu::Top) => {
                vec![Action::OpenScenarios, Action::OpenTutorial, Action::Exit]
            }
            Phase::Menu(Menu::Scenarios) => {
                let mut a: Vec<Action> = (0..scenarios::campaign().len())
                    .map(Action::PickScenario)
                    .collect();
                a.push(Action::Back);
                a
            }
            Phase::Menu(Menu::Tutorial) => {
                let mut a: Vec<Action> = (0..scenarios::tutorials().len())
                    .map(Action::PickScenario)
                    .collect();
                a.push(Action::Back);
                a
            }
            Phase::Choosing => {
                let mut a = Vec::new();
                for (h, hero) in state.heroes.iter().enumerate() {
                    if hero.is_down() {
                        continue;
                    }
                    for (f, foe) in state.creatures.iter().enumerate() {
                        if !foe.is_down() {
                            a.push(Action::PickPair(h, f));
                        }
                    }
                }
                a.push(Action::ToMenu);
                a
            }
            Phase::Combat => Stance::ALL.iter().map(|&s| Action::Stance(s)).collect(),
        }
    }

    fn action_label(&self, state: &State, action: &Action) -> String {
        match action {
            Action::OpenScenarios => "Scenarios".into(),
            Action::OpenTutorial => "Tutorial".into(),
            Action::Exit => "Exit".into(),
            Action::ToMenu => "Main menu".into(),
            Action::Back => "< Back".into(),
            Action::Replay => "Replay this scenario".into(),
            Action::PickScenario(i) => menu_catalog(&state.phase)
                .get(*i)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "?".into()),
            Action::PickPair(h, f) => {
                let hero = state.heroes.get(*h).map(|x| x.name.as_str()).unwrap_or("?");
                let foe = state.creatures.get(*f).map(|x| x.name.as_str()).unwrap_or("?");
                format!("{hero} vs the {foe}")
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
            (Phase::Menu(Menu::Top), Action::OpenScenarios) => {
                state.phase = Phase::Menu(Menu::Scenarios);
            }
            (Phase::Menu(Menu::Top), Action::OpenTutorial) => {
                state.phase = Phase::Menu(Menu::Tutorial);
            }
            (Phase::Menu(Menu::Scenarios), Action::PickScenario(i)) => {
                let s = scenarios::campaign()
                    .into_iter()
                    .nth(*i)
                    .ok_or_else(|| GameError::new("no such scenario"))?;
                load_scenario(state, s);
            }
            (Phase::Menu(Menu::Tutorial), Action::PickScenario(i)) => {
                let s = scenarios::tutorials()
                    .into_iter()
                    .nth(*i)
                    .ok_or_else(|| GameError::new("no such tutorial"))?;
                load_scenario(state, s);
            }
            (Phase::Menu(Menu::Scenarios | Menu::Tutorial), Action::Back) => {
                state.phase = Phase::Menu(Menu::Top);
            }
            (Phase::Choosing, Action::PickPair(h, f)) => {
                if state.heroes.get(*h).is_some_and(|x| !x.is_down())
                    && state.creatures.get(*f).is_some_and(|x| !x.is_down())
                {
                    form_duel(state, *h, *f);
                } else {
                    return Err(GameError::new("that matchup is not available"));
                }
            }
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
            Phase::Menu(Menu::Scenarios) => {
                zones.push(scenario_list_zone("Scenarios", &scenarios::campaign()))
            }
            Phase::Menu(Menu::Tutorial) => {
                zones.push(scenario_list_zone("Tutorials", &scenarios::tutorials()))
            }
            Phase::Choosing => {
                push_rosters(&mut zones, state, None, None);
            }
            Phase::Combat => {
                let (ah, af, he, fe) = match state.duel {
                    Some(d) => (Some(d.hero), Some(d.foe), d.hero_edge, d.foe_edge),
                    None => (None, None, 0, 0),
                };
                // Warband, then both Edge banks, then party.
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

fn fighter_card(name: &str, role: &str, remaining: u32, max: u32, base: u32, bw: u32, accent: Accent) -> CardView {
    let mut body = vec![format!("HP [{}]", pips(remaining, max)), format!("base {base}")];
    if bw > 1 {
        body.push(format!("bandwidth {bw}"));
    }
    CardView::up(name.to_string())
        .typed(role.to_string())
        .body(body)
        .corner(format!("{remaining}/{max}"))
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
                fighter_card(
                    &c.name,
                    &c.role,
                    c.body.remaining,
                    c.body.max,
                    c.base,
                    c.bandwidth,
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
                fighter_card(
                    &h.name,
                    &h.role,
                    h.body.remaining,
                    h.body.max,
                    h.base,
                    h.bandwidth,
                    if active == Some(i) { Accent::Selected } else { Accent::Ally },
                )
            })
            .collect(),
    }
}

fn push_rosters(zones: &mut Vec<ZoneView>, state: &State, ah: Option<usize>, af: Option<usize>) {
    zones.push(creature_zone(state, af));
    zones.push(hero_zone(state, ah));
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
            CardView::up("Scenarios")
                .typed("menu")
                .body(vec!["Team & god-tier fights.".into()])
                .accent(Accent::Ally),
            CardView::up("Tutorial")
                .typed("menu")
                .body(vec!["Learn one stance at a time.".into()])
                .accent(Accent::Good),
            CardView::up("Exit").typed("menu").body(vec!["Quit.".into()]),
        ],
    }
}

fn scenario_list_zone(label: &str, scenarios: &[Scenario]) -> ZoneView {
    ZoneView {
        label: label.into(),
        layout: Layout::Row,
        owner: None,
        cards: scenarios
            .iter()
            .map(|s| {
                CardView::up(s.name.clone())
                    .typed("lesson")
                    .body(vec![s.blurb.clone()])
                    .accent(Accent::Good)
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn launch_tutorial(game: &Deckbound, state: &mut State, index: usize) {
        game.apply(state, &Action::OpenTutorial).unwrap();
        game.apply(state, &Action::PickScenario(index)).unwrap();
    }

    fn launch_scenario(game: &Deckbound, state: &mut State, index: usize) {
        game.apply(state, &Action::OpenScenarios).unwrap();
        game.apply(state, &Action::PickScenario(index)).unwrap();
    }

    #[test]
    fn new_game_starts_in_menu() {
        let s = Deckbound.new_game(1, 1);
        assert_eq!(s.phase, Phase::Menu(Menu::Top));
        assert!(s.heroes.is_empty());
    }

    #[test]
    fn a_tutorial_is_one_on_one_and_skips_matchmaking() {
        let game = Deckbound;
        let mut s = game.new_game(1, 1);
        launch_tutorial(&game, &mut s, 0);
        assert_eq!(s.phase, Phase::Combat);
        assert!(s.duel.is_some());
    }

    #[test]
    fn marshalling_builds_edge() {
        let game = Deckbound;
        let mut s = game.new_game(1, 1);
        launch_tutorial(&game, &mut s, 0); // Pell always Marshals
        game.apply(&mut s, &Action::Stance(Stance::Marshal)).unwrap();
        let d = s.duel.unwrap();
        assert_eq!(d.hero_edge, 1);
        assert_eq!(d.foe_edge, 1);
    }

    #[test]
    fn build_then_unleash_beats_the_sandbag() {
        let game = Deckbound;
        let mut s = game.new_game(1, 1);
        launch_tutorial(&game, &mut s, 0);
        let mut guard = 0;
        while game.current_player(&s).is_some() {
            let edge = s.duel.map(|d| d.hero_edge).unwrap_or(0);
            let stance = if edge >= 3 { Stance::Unleash } else { Stance::Marshal };
            game.apply(&mut s, &Action::Stance(stance)).unwrap();
            guard += 1;
            assert!(guard < 500, "should resolve");
        }
        assert_eq!(game.outcome(&s), Some(Outcome::Win(PlayerId(0))));
    }

    #[test]
    fn a_team_scenario_asks_for_a_matchup() {
        let game = Deckbound;
        let mut s = game.new_game(1, 1);
        launch_scenario(&game, &mut s, 1); // Even Skirmish, 2 vs 2
        assert_eq!(s.phase, Phase::Choosing);
        // Pick the first available matchup -> a duel begins.
        game.apply(&mut s, &Action::PickPair(0, 0)).unwrap();
        assert_eq!(s.phase, Phase::Combat);
        assert!(s.duel.is_some());
    }

    #[test]
    fn a_swarm_chips_the_outnumbered_duelist() {
        // Goliath's Stand: Kael (bandwidth 4) vs 8 husks. After one beat, the
        // unwatched 4 husks should have chipped Kael (8 - 4 = 4 overflow).
        let game = Deckbound;
        let mut s = game.new_game(3, 1);
        launch_scenario(&game, &mut s, 3); // Goliath's Stand
        assert_eq!(s.phase, Phase::Choosing);
        game.apply(&mut s, &Action::PickPair(0, 0)).unwrap();
        let kael_before = s.heroes[0].body.remaining;
        game.apply(&mut s, &Action::Stance(Stance::Marshal)).unwrap();
        // Kael lost at least the 4 overflow chip (possibly more from the stance).
        assert!(s.heroes[0].body.remaining <= kael_before.saturating_sub(4));
    }

    #[test]
    fn every_scenario_and_tutorial_runs_to_an_outcome() {
        let game = Deckbound;
        for menu_open in [Action::OpenScenarios, Action::OpenTutorial] {
            let count = if matches!(menu_open, Action::OpenScenarios) {
                scenarios::campaign().len()
            } else {
                scenarios::tutorials().len()
            };
            for i in 0..count {
                let mut s = game.new_game(11 + i as u64, 1);
                game.apply(&mut s, &menu_open).unwrap();
                game.apply(&mut s, &Action::PickScenario(i)).unwrap();
                let mut guard = 0;
                while game.current_player(&s).is_some() {
                    let action = match s.phase {
                        Phase::Choosing => {
                            // first legal matchup
                            game.legal_actions(&s)
                                .into_iter()
                                .find(|a| matches!(a, Action::PickPair(_, _)))
                                .unwrap()
                        }
                        Phase::Combat => {
                            // a rough strategy: build a little, then unleash/overwhelm
                            let e = s.duel.map(|d| d.hero_edge).unwrap_or(0);
                            if e >= 2 {
                                Action::Stance(Stance::Unleash)
                            } else {
                                Action::Stance(Stance::Marshal)
                            }
                        }
                        _ => break,
                    };
                    game.apply(&mut s, &action).unwrap();
                    guard += 1;
                    assert!(guard < 5000, "scenario {i} should terminate");
                }
                assert!(game.outcome(&s).is_some());
            }
        }
    }
}
