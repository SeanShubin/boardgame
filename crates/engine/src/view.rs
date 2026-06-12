//! A renderer-agnostic snapshot of the table.
//!
//! A [`Game`](crate::Game) turns its private state into a [`TableView`]: a plain
//! description of what is on the table and whose turn it is. A presentation
//! layer draws a `TableView` without knowing the rules of any particular game,
//! and a game produces one without knowing how it will be drawn. This is the
//! seam that lets one renderer display every game.

use crate::player::PlayerId;

/// Everything a presentation layer needs to draw the table once.
#[derive(Clone, Debug)]
pub struct TableView {
    /// A short, human-readable description of the current situation, e.g.
    /// "Player 0's turn" or "Game over — Player 1 wins!".
    pub status: String,
    /// The piles on the table, in the order they should be presented.
    pub zones: Vec<ZoneView>,
}

/// A single pile of cards as it should appear to the viewer.
#[derive(Clone, Debug)]
pub struct ZoneView {
    /// A label for the pile, e.g. "Deck (24)".
    pub label: String,
    /// A hint for how the cards should be arranged.
    pub layout: Layout,
    /// The player this pile belongs to, if any.
    pub owner: Option<PlayerId>,
    /// The cards in the pile, in presentation order.
    pub cards: Vec<CardView>,
}

/// A hint for how a renderer should arrange the cards in a zone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// Overlapping pile (a deck or discard).
    Stack,
    /// A straight, evenly spaced row.
    Row,
    /// An overlapping fan, as a hand is held.
    Fan,
}

/// A single card as it should appear to the viewer.
#[derive(Clone, Debug)]
pub struct CardView {
    /// Whether the card's face is visible, and what it shows.
    pub face: CardFace,
}

/// The visible side of a card.
#[derive(Clone, Debug)]
pub enum CardFace {
    /// Face up: the title (and optional numeric value) are shown.
    Up { title: String, value: Option<i32> },
    /// Face down: only the card back is shown.
    Down,
}

impl CardView {
    /// A face-up card showing only a title.
    pub fn up(title: impl Into<String>) -> Self {
        Self {
            face: CardFace::Up {
                title: title.into(),
                value: None,
            },
        }
    }

    /// A face-up card showing a title and a numeric value.
    pub fn up_valued(title: impl Into<String>, value: i32) -> Self {
        Self {
            face: CardFace::Up {
                title: title.into(),
                value: Some(value),
            },
        }
    }

    /// A face-down card.
    pub fn down() -> Self {
        Self {
            face: CardFace::Down,
        }
    }
}
