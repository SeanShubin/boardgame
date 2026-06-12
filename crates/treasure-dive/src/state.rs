//! The state of a Treasure Dive game in progress.

use engine::Zone;

use crate::cards::Card;

/// Per-player bookkeeping. For this small game that is just a banked score.
#[derive(Clone, Debug)]
pub struct PlayerState {
    pub score: u32,
}

/// The full state of a game.
///
/// On a turn the active player repeatedly *dives* (flips the top of the deck
/// onto their dive pile) or *surfaces* (banks the dive pile into their score).
/// Flipping a second card of a suit already in the dive pile busts the dive:
/// the whole pile is lost to the discard and the turn ends.
#[derive(Clone, Debug)]
pub struct State {
    /// The face-down draw pile.
    pub deck: Zone<Card>,
    /// Busted and banked cards, out of play.
    pub discard: Zone<Card>,
    /// The active player's dive pile this turn, oldest first.
    pub dive: Vec<Card>,
    /// One entry per seat.
    pub players: Vec<PlayerState>,
    /// The seat whose turn it is.
    pub current: usize,
    /// Set once the deck is exhausted and the final pile has been banked.
    pub over: bool,
}

impl State {
    /// The total value currently sitting in the dive pile.
    pub fn dive_value(&self) -> u32 {
        self.dive.iter().map(|card| card.value as u32).sum()
    }

    /// Banks the active player's dive pile into their score and clears it to
    /// the discard.
    pub fn bank_current(&mut self) {
        let gained = self.dive_value();
        self.players[self.current].score += gained;
        for card in self.dive.drain(..) {
            self.discard.push(card);
        }
    }

    /// Passes the turn to the next seat.
    pub fn advance(&mut self) {
        self.current = (self.current + 1) % self.players.len();
    }
}
