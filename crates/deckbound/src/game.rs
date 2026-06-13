//! Deckbound's sample combat as an [`engine::Game`].
//!
//! One human (seat 0) declares for the whole party; the warband is resolved
//! deterministically from the seed. Each living hero's secret declaration is
//! collected as its own turn; once the last is in, the round resolves at once
//! (see [`crate::resolve`]).

use engine::{
    CardView, Game, GameError, Layout, Outcome, PlayerId, Rng, TableView, ZoneView,
};

use crate::actors::{Line, Play, roster};
use crate::read::Read;
use crate::resolve::resolve_round;
use crate::state::{Decl, State};

/// One declaration the player commits for a hero this round.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Action {
    /// Which hero this declaration is for (must be the one on the clock).
    pub hero: usize,
    pub play: Play,
    /// The chosen target creature, for plays that need one.
    pub target: Option<usize>,
}

/// The Deckbound sample-combat ruleset. Holds no state of its own.
#[derive(Clone, Copy, Debug, Default)]
pub struct Deckbound;

impl Game for Deckbound {
    type State = State;
    type Action = Action;

    fn new_game(&self, seed: u64, _players: usize) -> State {
        let (heroes, creatures) = roster();
        let declarations = vec![None; heroes.len()];
        State {
            round: 1,
            heroes,
            creatures,
            declarations,
            log: vec!["The warband presses in. Declare your party's round.".into()],
            rng: Rng::new(seed),
            outcome: None,
        }
    }

    fn current_player(&self, state: &State) -> Option<PlayerId> {
        if state.outcome.is_some() {
            None
        } else {
            Some(PlayerId(0))
        }
    }

    fn legal_actions(&self, state: &State) -> Vec<Action> {
        let Some(hero) = state.current_hero() else {
            return Vec::new();
        };
        let targets: Vec<usize> = state
            .creatures
            .iter()
            .enumerate()
            .filter(|(_, c)| c.alive())
            .map(|(i, _)| i)
            .collect();

        let mut actions = Vec::new();
        for &play in &state.heroes[hero].plays {
            if play.needs_target() {
                for &ci in &targets {
                    actions.push(Action {
                        hero,
                        play,
                        target: Some(ci),
                    });
                }
            } else {
                actions.push(Action {
                    hero,
                    play,
                    target: None,
                });
            }
        }
        actions
    }

    fn action_label(&self, state: &State, action: &Action) -> String {
        let who = state.heroes[action.hero].name;
        let target_name = action
            .target
            .and_then(|ci| state.creatures.get(ci))
            .map(|c| c.name)
            .unwrap_or("?");
        match action.play {
            Play::Read(Read::Strike) => format!("{who}: Strike the {target_name}"),
            Play::Read(r) => format!("{who}: Hold — {}", r.name()),
            Play::Bash => format!("{who}: Bash the {target_name}"),
            Play::Riposte => format!("{who}: Hold — Riposte"),
            Play::Firestorm => format!("{who}: Firestorm (heat, up to 5 foes)"),
            Play::Frostbite => format!("{who}: Frostbite the {target_name}"),
            Play::Rally => format!("{who}: Rally (party Resolve)"),
            Play::Dread => format!("{who}: Dread the {target_name}"),
            Play::Steel => format!("{who}: Steel (recover nerve)"),
        }
    }

    fn apply(&self, state: &mut State, action: &Action) -> Result<(), GameError> {
        if state.outcome.is_some() {
            return Err(GameError::new("the fight is already over"));
        }
        let current = state
            .current_hero()
            .ok_or_else(|| GameError::new("no hero is awaiting a declaration"))?;
        if action.hero != current {
            return Err(GameError::new("it is not that hero's turn to declare"));
        }
        if action.play.needs_target()
            && !action
                .target
                .and_then(|ci| state.creatures.get(ci))
                .is_some_and(|c| c.alive())
        {
            return Err(GameError::new("that play needs a living target"));
        }

        state.declarations[action.hero] = Some(Decl {
            play: action.play,
            target: action.target,
        });

        if state.all_alive_declared() {
            resolve_round(state);
            state.clear_declarations();
            state.round += 1;
        }
        Ok(())
    }

    fn outcome(&self, state: &State) -> Option<Outcome> {
        state.outcome.clone()
    }

    fn view(&self, state: &State, _perspective: Option<PlayerId>) -> TableView {
        let mut zones = Vec::new();

        // The warband.
        zones.push(ZoneView {
            label: "Warband".into(),
            layout: Layout::Row,
            owner: None,
            cards: state
                .creatures
                .iter()
                .filter(|c| c.alive())
                .map(|c| {
                    let title = if c.is_swarm() {
                        format!("{} ×{}\nSpd{} Pow{}", c.name, c.count, c.speed, c.power)
                    } else {
                        let armor = if c.armor.is_some() { " plate" } else { "" };
                        format!("{}\nSpd{} Pow{}{}", c.name, c.speed, c.power, armor)
                    };
                    let value = if c.is_swarm() {
                        c.count as i32
                    } else {
                        c.body.remaining as i32
                    };
                    CardView::up_valued(title, value)
                })
                .collect(),
        });

        // The two lines.
        for (label, line) in [("Front line", Line::Front), ("Back line", Line::Back)] {
            zones.push(ZoneView {
                label: label.into(),
                layout: Layout::Row,
                owner: Some(PlayerId(0)),
                cards: state
                    .heroes
                    .iter()
                    .enumerate()
                    .filter(|(_, h)| h.line == line)
                    .map(|(i, h)| hero_card(state, i, h))
                    .collect(),
            });
        }

        let status = self.status_line(state);
        TableView { status, zones }
    }
}

impl Deckbound {
    fn status_line(&self, state: &State) -> String {
        let recent: Vec<String> = state.log.iter().rev().take(10).rev().cloned().collect();
        let log = recent.join("\n");
        match &state.outcome {
            Some(Outcome::Win(PlayerId(0))) => format!("Victory — the warband is broken.\n\n{log}"),
            Some(Outcome::Win(_)) => format!("Defeat — the party has fallen.\n\n{log}"),
            Some(Outcome::Tie(_)) => format!("The fight ends in a stalemate.\n\n{log}"),
            None => {
                let hero = state.current_hero().map(|i| state.heroes[i].name).unwrap_or("?");
                format!(
                    "Round {} — declare for {hero}. (Front holds the wall; the back acts over it.)\n\n{log}",
                    state.round
                )
            }
        }
    }
}

fn hero_card(state: &State, index: usize, h: &crate::actors::Hero) -> CardView {
    if h.is_down() {
        return CardView::up(format!("{} DOWN", h.name));
    }
    let declared = state.declarations[index]
        .as_ref()
        .map(|d| format!(" ✓{}", d.play.name()))
        .unwrap_or_default();
    let aspect = if h.magic > 0 {
        format!("Mag{}", h.magic)
    } else if h.spirit > 0 {
        format!("Spr{}", h.spirit)
    } else {
        format!("Pow{}", h.power)
    };
    let title = format!(
        "{}\nSpd{} {} R{}{}",
        h.name,
        h.speed,
        aspect,
        h.resolve(),
        declared
    );
    CardView::up_valued(title, h.body.remaining as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::Behavior;

    /// Play the documented winning line: front holds (drag stops the Stalker),
    /// Bram Rallies (Sefa holds vs the Howler), Sefa Firestorms the front.
    /// Repeat until the warband breaks; the party must win in finite rounds.
    fn idx(state: &State, name: &str) -> Option<usize> {
        state
            .creatures
            .iter()
            .position(|c| c.name == name && c.alive())
    }

    /// Declare exactly one round of the winning line (the round counter ticks
    /// once the last hero commits, ending the loop).
    fn declare_winning_round(game: &Deckbound, state: &mut State) {
        let round = state.round;
        while state.round == round {
            let Some(hero) = state.current_hero() else {
                break;
            };
            let name = state.heroes[hero].name;
            let action = match name {
                "Aldric" => Action {
                    hero,
                    play: Play::Read(Read::Block),
                    target: None,
                },
                "Vera" => Action {
                    hero,
                    play: Play::Read(Read::Evade),
                    target: None,
                },
                "Bram" => Action {
                    hero,
                    play: Play::Rally,
                    target: None,
                },
                "Sefa" => Action {
                    hero,
                    play: Play::Firestorm,
                    target: None,
                },
                _ => unreachable!(),
            };
            game.apply(state, &action).unwrap();
        }
    }

    #[test]
    fn new_game_sets_up_the_sample_roster() {
        let state = Deckbound.new_game(1, 1);
        assert_eq!(state.heroes.len(), 4);
        assert_eq!(state.creatures.len(), 4);
        assert!(state.outcome.is_none());
        assert_eq!(state.current_hero(), Some(0)); // Aldric first
    }

    #[test]
    fn first_round_stops_the_stalker_and_clears_husks() {
        let game = Deckbound;
        let mut state = game.new_game(1, 1);
        declare_winning_round(&game, &mut state);
        // Stalker dragged to Body 1 (6 − Aldric's Pow 5), Howler burned down,
        // husks thinned. Sefa unharmed (held her nerve via Rally).
        let stalker = state.creatures.iter().find(|c| c.name == "Stalker").unwrap();
        assert_eq!(stalker.body.remaining, 1);
        assert!(idx(&state, "Howler").is_none(), "Firestorm should burn the Howler");
        let sefa = state.heroes.iter().find(|h| h.name == "Sefa").unwrap();
        assert!(!sefa.is_down(), "Rally must keep Sefa standing");
    }

    #[test]
    fn the_winning_line_wins_in_finite_rounds() {
        let game = Deckbound;
        let mut state = game.new_game(1, 1);
        let mut guard = 0;
        while game.current_player(&state).is_some() {
            declare_winning_round(&game, &mut state);
            guard += 1;
            assert!(guard < 100, "the fight should resolve");
        }
        assert_eq!(game.outcome(&state), Some(Outcome::Win(PlayerId(0))));
    }

    #[test]
    fn dropping_the_wall_lets_the_stalker_kill_sefa() {
        // Aldric attacks instead of holding -> drag 5 < 6 -> Stalker reaches Sefa.
        let game = Deckbound;
        let mut state = game.new_game(1, 1);
        let husk = state.creatures.iter().position(|c| c.behavior == Behavior::Swarm).unwrap();
        let round = state.round;
        while state.round == round {
            let Some(hero) = state.current_hero() else { break };
            let name = state.heroes[hero].name;
            let action = match name {
                "Aldric" => Action { hero, play: Play::Bash, target: Some(husk) },
                "Vera" => Action { hero, play: Play::Read(Read::Evade), target: None },
                "Bram" => Action { hero, play: Play::Rally, target: None },
                "Sefa" => Action { hero, play: Play::Firestorm, target: None },
                _ => unreachable!(),
            };
            game.apply(&mut state, &action).unwrap();
        }
        // Vera alone holds: drag 5 < Stalker 6, so it gets through to Sefa.
        let sefa = state.heroes.iter().find(|h| h.name == "Sefa").unwrap();
        assert!(sefa.is_down(), "an unguarded Stalker should run Sefa down");
    }
}
