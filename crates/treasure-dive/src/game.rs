//! The rules of Treasure Dive, as an [`engine::Game`].

use engine::{
    CardView, Game, GameError, Layout, Outcome, PlayerId, Rng, TableView, Zone, ZoneView,
};

use crate::cards::full_deck;
use crate::state::{PlayerState, State};

/// How many face-down deck cards to depict in the view. The deck label always
/// carries the true count; this just keeps the stack from sprawling.
const DECK_PREVIEW: usize = 5;

/// The one decision a player makes on their turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Flip the top card of the deck onto the dive pile.
    Dive,
    /// Stop and bank the dive pile.
    Surface,
}

/// The Treasure Dive ruleset. It holds no state of its own.
#[derive(Clone, Copy, Debug, Default)]
pub struct TreasureDive;

impl Game for TreasureDive {
    type State = State;
    type Action = Action;

    fn new_game(&self, seed: u64, players: usize) -> State {
        let mut cards = full_deck();
        let mut rng = Rng::new(seed);
        rng.shuffle(&mut cards);
        State {
            deck: Zone::from_cards(cards),
            discard: Zone::new(),
            dive: Vec::new(),
            players: (0..players.max(1))
                .map(|_| PlayerState { score: 0 })
                .collect(),
            current: 0,
            over: false,
        }
    }

    fn current_player(&self, state: &State) -> Option<PlayerId> {
        if state.over {
            None
        } else {
            Some(PlayerId(state.current))
        }
    }

    fn legal_actions(&self, state: &State) -> Vec<Action> {
        if state.over || state.deck.is_empty() {
            Vec::new()
        } else {
            vec![Action::Dive, Action::Surface]
        }
    }

    fn action_label(&self, state: &State, action: &Action) -> String {
        match action {
            Action::Dive => format!("Dive — flip the top card ({} left)", state.deck.len()),
            Action::Surface => format!("Surface — bank {} point(s)", state.dive_value()),
        }
    }

    fn apply(&self, state: &mut State, action: &Action) -> Result<(), GameError> {
        if state.over {
            return Err(GameError::new("the game is already over"));
        }
        match action {
            Action::Dive => {
                let card = state
                    .deck
                    .draw()
                    .ok_or_else(|| GameError::new("the deck is empty"))?;
                let bust = state.dive.iter().any(|held| held.suit == card.suit);
                if bust {
                    // Lose the whole pile and the card that busted it.
                    state.discard.push(card);
                    let lost = std::mem::take(&mut state.dive);
                    state.discard.extend(lost);
                    state.advance();
                } else {
                    state.dive.push(card);
                }
            }
            Action::Surface => {
                state.bank_current();
                state.advance();
            }
        }
        // Once the deck is spent, bank whatever the now-active player holds and
        // end the game.
        if state.deck.is_empty() {
            state.bank_current();
            state.over = true;
        }
        Ok(())
    }

    fn outcome(&self, state: &State) -> Option<Outcome> {
        if !state.over {
            return None;
        }
        let best = state.players.iter().map(|p| p.score).max().unwrap_or(0);
        let winners: Vec<PlayerId> = state
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.score == best)
            .map(|(seat, _)| PlayerId(seat))
            .collect();
        Some(if winners.len() == 1 {
            Outcome::Win(winners[0])
        } else {
            Outcome::Tie(winners)
        })
    }

    fn view(&self, state: &State, _perspective: Option<PlayerId>) -> TableView {
        let mut zones = Vec::new();

        let shown = state.deck.len().min(DECK_PREVIEW);
        zones.push(ZoneView {
            label: format!("Deck ({})", state.deck.len()),
            layout: Layout::Stack,
            owner: None,
            cards: (0..shown).map(|_| CardView::down()).collect(),
        });

        zones.push(ZoneView {
            label: format!("Dive pile ({} pts)", state.dive_value()),
            layout: Layout::Row,
            owner: None,
            cards: state
                .dive
                .iter()
                .map(|card| CardView::up_valued(card.suit.name(), card.value as i32))
                .collect(),
        });

        zones.push(ZoneView {
            label: format!("Discard ({})", state.discard.len()),
            layout: Layout::Stack,
            owner: None,
            cards: (0..state.discard.len().min(DECK_PREVIEW))
                .map(|_| CardView::down())
                .collect(),
        });

        for (seat, player) in state.players.iter().enumerate() {
            let active = if !state.over && seat == state.current {
                "  (turn)"
            } else {
                ""
            };
            zones.push(ZoneView {
                label: format!("Player {seat}: {} pts{active}", player.score),
                layout: Layout::Row,
                owner: Some(PlayerId(seat)),
                cards: Vec::new(),
            });
        }

        let status = match self.outcome(state) {
            Some(Outcome::Win(p)) => format!("Game over — {p} wins!"),
            Some(Outcome::Tie(players)) => {
                let names: Vec<String> = players.iter().map(|p| p.to_string()).collect();
                format!("Game over — tie between {}", names.join(", "))
            }
            None => format!("Player {}'s turn — Dive or Surface?", state.current),
        };

        TableView {
            status,
            zones,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::{Card, Suit};

    #[test]
    fn new_game_has_full_shuffled_deck() {
        let state = TreasureDive.new_game(1, 2);
        assert_eq!(state.deck.len(), 36);
        assert_eq!(state.players.len(), 2);
        assert!(state.dive.is_empty());
        assert!(!state.over);
    }

    #[test]
    fn same_seed_is_reproducible() {
        let a = TreasureDive.new_game(7, 3);
        let b = TreasureDive.new_game(7, 3);
        assert_eq!(a.deck.cards(), b.deck.cards());
    }

    #[test]
    fn surfacing_banks_the_dive_pile_and_passes_the_turn() {
        let game = TreasureDive;
        let mut state = game.new_game(2, 2);
        // Dive until we are about to bust, but at least once.
        game.apply(&mut state, &Action::Dive).unwrap();
        let banked = state.dive_value();
        let diver = state.current;
        game.apply(&mut state, &Action::Surface).unwrap();
        assert_eq!(state.players[diver].score, banked);
        assert!(state.dive.is_empty());
        assert_ne!(state.current, diver);
    }

    #[test]
    fn busting_discards_the_dive_pile_and_scores_nothing() {
        let game = TreasureDive;
        // A deck rigged so the first two cards share a suit: an instant bust.
        let mut state = game.new_game(0, 2);
        state.deck = Zone::from_cards(vec![
            Card {
                suit: Suit::Fish,
                value: 3,
            },
            Card {
                suit: Suit::Fish,
                value: 5,
            },
        ]);
        state.dive.clear();
        state.over = false;

        game.apply(&mut state, &Action::Dive).unwrap(); // Fish 5 onto the pile
        assert_eq!(state.dive.len(), 1);
        game.apply(&mut state, &Action::Dive).unwrap(); // Fish 3 busts it
        assert!(state.dive.is_empty());
        assert_eq!(state.players[0].score, 0);
        // Deck is now empty, so the game ends.
        assert!(state.over);
    }

    #[test]
    fn game_ends_when_deck_empties_and_reports_a_winner() {
        let game = TreasureDive;
        let mut state = game.new_game(99, 2);
        let mut guard = 0;
        while game.current_player(&state).is_some() {
            // A relentless diver: always dive. The seed decides the rest.
            game.apply(&mut state, &Action::Dive).unwrap();
            guard += 1;
            assert!(guard < 1000, "game failed to terminate");
        }
        assert!(state.over);
        assert!(game.legal_actions(&state).is_empty());
        assert!(game.outcome(&state).is_some());
    }
}
