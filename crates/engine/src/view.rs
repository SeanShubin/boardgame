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

/// A colour hint for a card, so a renderer can tell allies from foes, flag a
/// warning, or highlight a selection — without knowing any game's rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Accent {
    /// No special meaning.
    #[default]
    Neutral,
    /// Belongs to the viewing player (an ally).
    Ally,
    /// An opponent or hazard.
    Foe,
    /// Something to watch — exposed, panicked, low.
    Warn,
    /// A benefit — a buff, a banked gain.
    Good,
    /// The current focus of a decision.
    Selected,
}

/// A single card as it should appear to the viewer.
#[derive(Clone, Debug)]
pub struct CardView {
    /// Whether the card's face is visible, and what it shows.
    pub face: CardFace,
}

/// The visible side of a card.
///
/// A face-up card is laid out like a collectible-card-game card — a title bar,
/// an optional type line, a body of stat / rules lines, and a corner badge
/// (the "power/toughness" spot) — minus art and flavour, so the space goes to
/// information.
#[derive(Clone, Debug)]
pub enum CardFace {
    /// Face up.
    Up {
        /// The card's name, shown in the title bar.
        title: String,
        /// An optional type line beneath the title (e.g. "Knight · Front").
        type_line: Option<String>,
        /// Stat / rules lines in the card body, top to bottom.
        body: Vec<String>,
        /// An optional corner badge (e.g. a health total "5/8").
        corner: Option<String>,
        /// A colour hint for the card.
        accent: Accent,
    },
    /// Face down: only the card back is shown.
    Down,
}

impl CardView {
    /// A face-up card showing only a title.
    pub fn up(title: impl Into<String>) -> Self {
        Self {
            face: CardFace::Up {
                title: title.into(),
                type_line: None,
                body: Vec::new(),
                corner: None,
                accent: Accent::Neutral,
            },
        }
    }

    /// A face-up card showing a title and a numeric corner badge.
    pub fn up_valued(title: impl Into<String>, value: i32) -> Self {
        Self::up(title).corner(value.to_string())
    }

    /// A face-down card.
    pub fn down() -> Self {
        Self {
            face: CardFace::Down,
        }
    }

    /// Set the type line (no-op on a face-down card).
    pub fn typed(mut self, type_line: impl Into<String>) -> Self {
        if let CardFace::Up { type_line: t, .. } = &mut self.face {
            *t = Some(type_line.into());
        }
        self
    }

    /// Set the body lines (no-op on a face-down card).
    pub fn body(mut self, lines: Vec<String>) -> Self {
        if let CardFace::Up { body, .. } = &mut self.face {
            *body = lines;
        }
        self
    }

    /// Set the corner badge (no-op on a face-down card).
    pub fn corner(mut self, text: impl Into<String>) -> Self {
        if let CardFace::Up { corner, .. } = &mut self.face {
            *corner = Some(text.into());
        }
        self
    }

    /// Set the colour accent (no-op on a face-down card).
    pub fn accent(mut self, a: Accent) -> Self {
        if let CardFace::Up { accent, .. } = &mut self.face {
            *accent = a;
        }
        self
    }
}
