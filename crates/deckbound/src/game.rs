//! Deckbound's sample combat as an [`engine::Game`].
//!
//! One human (seat 0) commands the whole party. The fight walks a small state
//! machine: set the formation, then each round pick a hero, a play, and (if
//! needed) a target — one staged choice per action, with `Back` to rewind. Once
//! the last living hero commits, the round resolves at once (see
//! [`crate::resolve`]). The warband is resolved deterministically from the seed.

use engine::{
    Accent, CardView, Game, GameError, Layout, Outcome, PlayerId, Rng, TableView, ZoneView,
};

use crate::actors::{Behavior, Hero, Line, Play, roster};
use crate::read::Read;
use crate::resolve::resolve_round;
use crate::state::{Decl, Phase, State};

/// One step of the party's decision flow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Formation: assign a hero to a line.
    Place(usize, Line),
    /// Formation: lines are set — start the fight.
    Begin,
    /// Once the battle is over: start a fresh fight.
    Replay,
    /// Declaring (round start): re-open the formation to rearrange lines.
    Reform,
    /// Declaring: choose which hero to declare for.
    PickHero(usize),
    /// Declaring: choose that hero's play (commits if it needs no target).
    PickPlay(Play),
    /// Declaring: choose the target and commit.
    PickTarget(usize),
    /// Rewind one step.
    Back,
}

/// The Deckbound sample-combat ruleset. Holds no state of its own.
#[derive(Clone, Copy, Debug, Default)]
pub struct Deckbound;

/// A fresh battle seeded by `seed`: full rosters, at the formation step.
fn initial_state(seed: u64) -> State {
    let (heroes, creatures) = roster();
    let n = heroes.len();
    State {
        round: 1,
        heroes,
        creatures,
        declarations: vec![None; n],
        declared_order: Vec::new(),
        formation: vec![None; n],
        formation_order: Vec::new(),
        phase: Phase::Formation,
        log: vec!["Set your formation: assign each hero to a line.".into()],
        rng: Rng::new(seed),
        seed,
        outcome: None,
    }
}

impl Deckbound {
    /// Lock in a hero's declaration; resolve the round once the party is done.
    fn commit(&self, state: &mut State, hero: usize, play: Play, target: Option<usize>) {
        state.declarations[hero] = Some(Decl { play, target });
        state.declared_order.push(hero);
        state.phase = Phase::Declaring {
            hero: None,
            play: None,
        };
        if state.all_alive_declared() {
            resolve_round(state);
            state.clear_declarations();
            state.round += 1;
            state.phase = Phase::Declaring {
                hero: None,
                play: None,
            };
        }
    }

    fn status(&self, state: &State) -> String {
        let log = state
            .log
            .iter()
            .rev()
            .take(8)
            .rev()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        let prompt = match &state.outcome {
            Some(Outcome::Win(PlayerId(0))) => {
                "Victory - the warband is broken. Replay to fight again.".to_string()
            }
            Some(Outcome::Win(_)) => {
                "Defeat - the party has fallen. Replay to fight again.".to_string()
            }
            Some(Outcome::Tie(_)) => {
                "The fight ends in a stalemate. Replay to fight again.".to_string()
            }
            None => match &state.phase {
                Phase::Formation => {
                    let placed = state.formation.iter().filter(|x| x.is_some()).count();
                    if state.formation_complete() {
                        "Formation set. Begin the battle (or Esc to rearrange).".to_string()
                    } else {
                        format!(
                            "Formation - place each hero. Front = the wall (holds, drags Runners); Back acts over it. ({}/{} placed)",
                            placed,
                            state.heroes.len()
                        )
                    }
                }
                Phase::Declaring {
                    hero: None,
                    ..
                } => format!(
                    "Round {}: choose a hero to declare for. ({}/{} locked in)",
                    state.round,
                    state.declared_order.len(),
                    state.living_heroes()
                ),
                Phase::Declaring {
                    hero: Some(h),
                    play: None,
                } => format!(
                    "Round {}: choose {}'s play. (Esc to go back)",
                    state.round, state.heroes[*h].name
                ),
                Phase::Declaring {
                    hero: Some(h),
                    play: Some(p),
                } => format!(
                    "Round {}: choose a target for {}'s {}. (Esc to go back)",
                    state.round,
                    state.heroes[*h].name,
                    p.name()
                ),
            },
        };
        format!("{prompt}\n\n{log}")
    }
}

impl Game for Deckbound {
    type State = State;
    type Action = Action;

    fn new_game(&self, seed: u64, _players: usize) -> State {
        initial_state(seed)
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
            return vec![Action::Replay];
        }
        let mut acts = Vec::new();
        match &state.phase {
            Phase::Formation => {
                if state.formation_complete() {
                    acts.push(Action::Begin);
                } else {
                    for (h, slot) in state.formation.iter().enumerate() {
                        if slot.is_none() {
                            acts.push(Action::Place(h, Line::Front));
                            acts.push(Action::Place(h, Line::Back));
                        }
                    }
                }
                if !state.formation_order.is_empty() {
                    acts.push(Action::Back);
                }
            }
            Phase::Declaring { hero: None, .. } => {
                for h in state.undeclared_living() {
                    acts.push(Action::PickHero(h));
                }
                if state.declared_order.is_empty() {
                    // Clean round start — offer to rearrange before committing.
                    acts.push(Action::Reform);
                } else {
                    acts.push(Action::Back);
                }
            }
            Phase::Declaring {
                hero: Some(h),
                play: None,
            } => {
                for &p in &state.heroes[*h].plays {
                    acts.push(Action::PickPlay(p));
                }
                acts.push(Action::Back);
            }
            Phase::Declaring {
                hero: Some(_),
                play: Some(_),
            } => {
                for (ci, c) in state.creatures.iter().enumerate() {
                    if c.alive() {
                        acts.push(Action::PickTarget(ci));
                    }
                }
                acts.push(Action::Back);
            }
        }
        acts
    }

    fn action_label(&self, state: &State, action: &Action) -> String {
        match action {
            Action::Place(h, line) => {
                let line = match line {
                    Line::Front => "Front",
                    Line::Back => "Back",
                };
                format!("{} -> {} line", state.heroes[*h].name, line)
            }
            Action::Begin => "Begin the battle".to_string(),
            Action::Replay => "Replay the battle".to_string(),
            Action::Reform => "Change formation".to_string(),
            Action::PickHero(h) => format!("Declare for {}", state.heroes[*h].name),
            Action::PickPlay(p) => play_label(*p),
            Action::PickTarget(ci) => {
                let name = state.creatures.get(*ci).map(|c| c.name).unwrap_or("?");
                format!("vs the {name}")
            }
            Action::Back => "< Back".to_string(),
        }
    }

    fn apply(&self, state: &mut State, action: &Action) -> Result<(), GameError> {
        if let Action::Replay = action {
            if state.outcome.is_none() {
                return Err(GameError::new("the battle is not over yet"));
            }
            // A fresh attempt: same scenario, re-rolled warband bluffs.
            *state = initial_state(state.seed.wrapping_add(1));
            return Ok(());
        }
        if state.outcome.is_some() {
            return Err(GameError::new("the fight is already over"));
        }
        let phase = state.phase.clone();
        match (phase, action) {
            (Phase::Formation, Action::Place(h, line)) => {
                if state.formation[*h].is_some() {
                    return Err(GameError::new("that hero is already placed"));
                }
                state.formation[*h] = Some(*line);
                state.heroes[*h].line = *line;
                state.formation_order.push(*h);
            }
            (Phase::Formation, Action::Begin) => {
                if !state.formation_complete() {
                    return Err(GameError::new("place every hero first"));
                }
                state.phase = Phase::Declaring {
                    hero: None,
                    play: None,
                };
                state.log = vec!["The lines are set. Declare your round.".into()];
            }
            (Phase::Formation, Action::Back) => {
                let h = state
                    .formation_order
                    .pop()
                    .ok_or_else(|| GameError::new("nothing to undo"))?;
                state.formation[h] = None;
            }
            (Phase::Declaring { hero: None, .. }, Action::PickHero(h)) => {
                if state.heroes[*h].is_down() || state.declarations[*h].is_some() {
                    return Err(GameError::new("that hero cannot declare now"));
                }
                state.phase = Phase::Declaring {
                    hero: Some(*h),
                    play: None,
                };
            }
            (Phase::Declaring { hero: None, .. }, Action::Back) => {
                let h = state
                    .declared_order
                    .pop()
                    .ok_or_else(|| GameError::new("nothing to undo"))?;
                state.declarations[h] = None;
            }
            (Phase::Declaring { hero: None, .. }, Action::Reform) => {
                if !state.declared_order.is_empty() {
                    return Err(GameError::new("finish or undo this round before reforming"));
                }
                state.phase = Phase::Formation;
                state.log = vec!["Re-forming the lines - Begin to confirm.".into()];
            }
            (Phase::Declaring { hero: Some(h), play: None }, Action::PickPlay(p)) => {
                if !state.heroes[h].plays.contains(p) {
                    return Err(GameError::new("that hero has no such play"));
                }
                if p.needs_target() {
                    state.phase = Phase::Declaring {
                        hero: Some(h),
                        play: Some(*p),
                    };
                } else {
                    self.commit(state, h, *p, None);
                }
            }
            (Phase::Declaring { hero: Some(_), play: None }, Action::Back) => {
                state.phase = Phase::Declaring {
                    hero: None,
                    play: None,
                };
            }
            (Phase::Declaring { hero: Some(h), play: Some(p) }, Action::PickTarget(ci)) => {
                if !state.creatures.get(*ci).is_some_and(|c| c.alive()) {
                    return Err(GameError::new("that play needs a living target"));
                }
                self.commit(state, h, p, Some(*ci));
            }
            (Phase::Declaring { hero: Some(h), play: Some(_) }, Action::Back) => {
                state.phase = Phase::Declaring {
                    hero: Some(h),
                    play: None,
                };
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
            Phase::Formation => (!state.formation_order.is_empty()).then_some(Action::Back),
            Phase::Declaring { hero: None, .. } => {
                (!state.declared_order.is_empty()).then_some(Action::Back)
            }
            Phase::Declaring { hero: Some(_), .. } => Some(Action::Back),
        }
    }

    fn view(&self, state: &State, _perspective: Option<PlayerId>) -> TableView {
        let mut zones = Vec::new();

        match &state.phase {
            Phase::Formation => {
                zones.push(warband_zone(state, "The warband (incoming)"));
                zones.push(ZoneView {
                    label: "Your heroes - place each one".into(),
                    layout: Layout::Row,
                    owner: Some(PlayerId(0)),
                    cards: (0..state.heroes.len())
                        .map(|i| hero_card(state, i, formation_label(state, i)))
                        .collect(),
                });
            }
            Phase::Declaring { .. } => {
                zones.push(warband_zone(state, "Warband"));
                for (label, line) in [("Your front line", Line::Front), ("Your back line", Line::Back)]
                {
                    zones.push(ZoneView {
                        label: label.into(),
                        layout: Layout::Row,
                        owner: Some(PlayerId(0)),
                        cards: (0..state.heroes.len())
                            .filter(|&i| state.heroes[i].line == line)
                            .map(|i| hero_card(state, i, line_label(line)))
                            .collect(),
                    });
                }
                if !state.declared_order.is_empty() {
                    zones.push(ZoneView {
                        label: "Locked in this round".into(),
                        layout: Layout::Row,
                        owner: Some(PlayerId(0)),
                        cards: state
                            .declared_order
                            .iter()
                            .filter_map(|&h| {
                                state.declarations[h].as_ref().map(|d| {
                                    CardView::up(state.heroes[h].name)
                                        .typed("locked")
                                        .body(vec![decl_label(state, d)])
                                        .accent(Accent::Good)
                                })
                            })
                            .collect(),
                    });
                }
            }
        }

        TableView {
            status: self.status(state),
            zones,
        }
    }
}

// ---- card builders ------------------------------------------------------

fn play_label(p: Play) -> String {
    match p {
        Play::Read(Read::Strike) => "Strike (pick a foe)".into(),
        Play::Read(r) => format!("Hold: {}", r.name()),
        Play::Bash => "Bash (pick a foe)".into(),
        Play::Riposte => "Hold: Riposte".into(),
        Play::Firestorm => "Firestorm (heat, 5 foes)".into(),
        Play::Frostbite => "Frostbite (pick a foe)".into(),
        Play::Rally => "Rally (whole party)".into(),
        Play::Dread => "Dread (pick a foe)".into(),
        Play::Steel => "Steel (self)".into(),
    }
}

fn line_label(line: Line) -> String {
    match line {
        Line::Front => "Front".into(),
        Line::Back => "Back".into(),
    }
}

fn formation_label(state: &State, i: usize) -> String {
    match state.formation[i] {
        Some(Line::Front) => "Front".into(),
        Some(Line::Back) => "Back".into(),
        None => "unplaced".into(),
    }
}

fn pips(remaining: u32, max: u32) -> String {
    let lost = max.saturating_sub(remaining) as usize;
    format!("{}{}", "#".repeat(remaining as usize), ".".repeat(lost))
}

fn decl_label(state: &State, d: &Decl) -> String {
    let p = d.play.name();
    match d.target.and_then(|ci| state.creatures.get(ci)) {
        Some(c) => format!("{p} -> {}", c.name),
        None => p.to_string(),
    }
}

fn hero_card(state: &State, i: usize, line_label: String) -> CardView {
    let h: &Hero = &state.heroes[i];
    if h.is_down() {
        return CardView::up(h.name)
            .typed(format!("{} - down", h.role))
            .body(vec!["(out of the fight)".into()])
            .corner(format!("0/{}", h.body.max))
            .accent(Accent::Warn);
    }
    let aspect = if h.magic > 0 {
        format!("Pow {} - Mag {}", h.power, h.magic)
    } else if h.spirit > 0 {
        format!("Pow {} - Spr {}", h.power, h.spirit)
    } else {
        format!("Pow {}", h.power)
    };
    let mut body = vec![
        format!("Spd {} - {}", h.speed, aspect),
        format!("Resolve {}", h.resolve()),
        format!("[{}] T{}", pips(h.body.remaining, h.body.max), h.body.toughness),
    ];
    if let Some(decl) = &state.declarations[i] {
        body.push(format!("> {}", decl_label(state, decl)));
    }
    CardView::up(h.name)
        .typed(format!("{} - {}", h.role, line_label))
        .body(body)
        .corner(format!("{}/{}", h.body.remaining, h.body.max))
        .accent(Accent::Ally)
}

fn creature_card(state: &State, i: usize) -> CardView {
    let c = &state.creatures[i];
    let behavior = match c.behavior {
        Behavior::Bluff => "Bluffer",
        Behavior::Runner => "Runner",
        Behavior::Howl => "Fear-caster",
        Behavior::Swarm => "Swarm",
    };
    let place = if c.runner {
        "runs the gauntlet".to_string()
    } else {
        line_label(c.line)
    };
    let stat = if c.fear > 0 {
        format!("Spd {} - Fear {}", c.speed, c.fear)
    } else {
        format!("Spd {} - Pow {}", c.speed, c.power)
    };
    let armor = if c.armor.is_some() {
        "Plate (heat passes)".to_string()
    } else {
        "unarmored".to_string()
    };
    let mut body = vec![stat, armor];
    let corner = if c.is_swarm() {
        body.push(format!("[{}] each T{}", pips(c.count, 6), c.body.toughness));
        format!("x{}", c.count)
    } else {
        body.push(format!(
            "[{}] T{}",
            pips(c.body.remaining, c.body.max),
            c.body.toughness
        ));
        format!("{}/{}", c.body.remaining, c.body.max)
    };
    CardView::up(c.name)
        .typed(format!("{behavior} - {place}"))
        .body(body)
        .corner(corner)
        .accent(Accent::Foe)
}

fn warband_zone(state: &State, label: &str) -> ZoneView {
    ZoneView {
        label: label.into(),
        layout: Layout::Row,
        owner: None,
        cards: (0..state.creatures.len())
            .filter(|&i| state.creatures[i].alive())
            .map(|i| creature_card(state, i))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive the formation step: place the named heroes in front, the rest in
    /// back, then Begin.
    fn set_formation(game: &Deckbound, state: &mut State, fronts: &[&str]) {
        while matches!(state.phase, Phase::Formation) {
            if state.formation_complete() {
                game.apply(state, &Action::Begin).unwrap();
                break;
            }
            let h = (0..state.heroes.len())
                .find(|&i| state.formation[i].is_none())
                .unwrap();
            let line = if fronts.contains(&state.heroes[h].name) {
                Line::Front
            } else {
                Line::Back
            };
            game.apply(state, &Action::Place(h, line)).unwrap();
        }
    }

    fn creature_index(state: &State, name: &str) -> usize {
        state
            .creatures
            .iter()
            .position(|c| c.name == name && c.alive())
            .expect("living creature")
    }

    /// Drive one round, choosing each hero's play (and target) from `pick`.
    fn declare_round(
        game: &Deckbound,
        state: &mut State,
        pick: impl Fn(&str) -> (Play, Option<&'static str>),
    ) {
        let round = state.round;
        while state.round == round && state.outcome.is_none() {
            match state.phase.clone() {
                Phase::Declaring { hero: None, .. } => {
                    let h = state.undeclared_living()[0];
                    game.apply(state, &Action::PickHero(h)).unwrap();
                }
                Phase::Declaring { hero: Some(h), play: None } => {
                    let (play, _) = pick(state.heroes[h].name);
                    game.apply(state, &Action::PickPlay(play)).unwrap();
                }
                Phase::Declaring { hero: Some(h), play: Some(_) } => {
                    let (_, target) = pick(state.heroes[h].name);
                    let ci = creature_index(state, target.expect("a target name"));
                    game.apply(state, &Action::PickTarget(ci)).unwrap();
                }
                Phase::Formation => break,
            }
        }
    }

    fn winning_pick(name: &str) -> (Play, Option<&'static str>) {
        match name {
            "Aldric" => (Play::Read(Read::Block), None),
            "Vera" => (Play::Read(Read::Evade), None),
            "Bram" => (Play::Rally, None),
            "Sefa" => (Play::Firestorm, None),
            _ => unreachable!(),
        }
    }

    #[test]
    fn new_game_starts_in_formation() {
        let state = Deckbound.new_game(1, 1);
        assert_eq!(state.phase, Phase::Formation);
        assert!(!state.formation_complete());
        assert_eq!(state.heroes.len(), 4);
    }

    #[test]
    fn begin_requires_a_full_formation() {
        let game = Deckbound;
        let mut state = game.new_game(1, 1);
        assert!(game.apply(&mut state, &Action::Begin).is_err());
        set_formation(&game, &mut state, &["Aldric", "Vera"]);
        assert!(matches!(state.phase, Phase::Declaring { .. }));
    }

    #[test]
    fn back_rewinds_a_partial_declaration() {
        let game = Deckbound;
        let mut state = game.new_game(1, 1);
        set_formation(&game, &mut state, &["Aldric", "Vera"]);
        // Pick a hero, then a target-needing play, then rewind twice.
        let h = state.undeclared_living()[0];
        game.apply(&mut state, &Action::PickHero(h)).unwrap();
        game.apply(&mut state, &Action::PickPlay(Play::Read(Read::Strike))).unwrap();
        assert!(matches!(state.phase, Phase::Declaring { hero: Some(_), play: Some(_) }));
        assert_eq!(game.cancel_action(&state), Some(Action::Back));
        game.apply(&mut state, &Action::Back).unwrap(); // back to play choice
        assert!(matches!(state.phase, Phase::Declaring { hero: Some(_), play: None }));
        game.apply(&mut state, &Action::Back).unwrap(); // back to hero choice
        assert!(matches!(state.phase, Phase::Declaring { hero: None, play: None }));
        assert!(state.declared_order.is_empty());
    }

    #[test]
    fn first_round_stops_the_stalker_and_burns_the_howler() {
        let game = Deckbound;
        let mut state = game.new_game(1, 1);
        set_formation(&game, &mut state, &["Aldric", "Vera"]);
        declare_round(&game, &mut state, winning_pick);
        let stalker = state.creatures.iter().find(|c| c.name == "Stalker").unwrap();
        assert_eq!(stalker.body.remaining, 1);
        assert!(!state.creatures.iter().any(|c| c.name == "Howler" && c.alive()));
        assert!(!state.heroes.iter().find(|h| h.name == "Sefa").unwrap().is_down());
    }

    #[test]
    fn change_formation_reopens_placement_without_advancing_the_round() {
        let game = Deckbound;
        let mut state = game.new_game(1, 1);
        set_formation(&game, &mut state, &["Aldric", "Vera"]);
        let round = state.round;
        game.apply(&mut state, &Action::Reform).unwrap();
        assert_eq!(state.phase, Phase::Formation);
        // Default is to keep as-is: just begin again.
        game.apply(&mut state, &Action::Begin).unwrap();
        assert!(matches!(state.phase, Phase::Declaring { hero: None, .. }));
        assert_eq!(state.round, round);
    }

    #[test]
    fn the_winning_line_wins_in_finite_rounds() {
        let game = Deckbound;
        let mut state = game.new_game(1, 1);
        set_formation(&game, &mut state, &["Aldric", "Vera"]);
        let mut guard = 0;
        while game.current_player(&state).is_some() {
            declare_round(&game, &mut state, winning_pick);
            guard += 1;
            assert!(guard < 100, "the fight should resolve");
        }
        assert_eq!(game.outcome(&state), Some(Outcome::Win(PlayerId(0))));
    }

    #[test]
    fn replay_resets_a_finished_battle() {
        let game = Deckbound;
        let mut state = game.new_game(1, 1);
        set_formation(&game, &mut state, &["Aldric", "Vera"]);
        while game.current_player(&state).is_some() {
            declare_round(&game, &mut state, winning_pick);
        }
        assert!(state.outcome.is_some());
        assert_eq!(game.legal_actions(&state), vec![Action::Replay]);
        game.apply(&mut state, &Action::Replay).unwrap();
        assert!(state.outcome.is_none());
        assert_eq!(state.phase, Phase::Formation);
        assert_eq!(state.round, 1);
        assert!(state.creatures.iter().all(|c| c.alive()));
        assert!(state.heroes.iter().all(|h| !h.is_down()));
    }

    #[test]
    fn dropping_the_wall_lets_the_stalker_kill_sefa() {
        let game = Deckbound;
        let mut state = game.new_game(1, 1);
        set_formation(&game, &mut state, &["Aldric", "Vera"]);
        // Aldric leaves the wall to Bash a husk -> drag 5 < Stalker's 6.
        declare_round(&game, &mut state, |name| match name {
            "Aldric" => (Play::Bash, Some("Husks")),
            "Vera" => (Play::Read(Read::Evade), None),
            "Bram" => (Play::Rally, None),
            "Sefa" => (Play::Firestorm, None),
            _ => unreachable!(),
        });
        assert!(state.heroes.iter().find(|h| h.name == "Sefa").unwrap().is_down());
    }
}
