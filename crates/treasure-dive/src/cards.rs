//! The deck of Treasure Dive.

/// The six suits of treasure. A dive busts when a second card of a suit already
/// in the dive is flipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Suit {
    Anchor,
    Fish,
    Chest,
    Map,
    Sword,
    Pearl,
}

impl Suit {
    /// Every suit, in a stable order.
    pub const ALL: [Suit; 6] = [
        Suit::Anchor,
        Suit::Fish,
        Suit::Chest,
        Suit::Map,
        Suit::Sword,
        Suit::Pearl,
    ];

    /// The highest value printed on any card of a suit; each suit has one card
    /// of every value from 1 to this.
    pub const MAX_VALUE: u8 = 6;

    /// A short display name.
    pub fn name(self) -> &'static str {
        match self {
            Suit::Anchor => "Anchor",
            Suit::Fish => "Fish",
            Suit::Chest => "Chest",
            Suit::Map => "Map",
            Suit::Sword => "Sword",
            Suit::Pearl => "Pearl",
        }
    }
}

/// A single card: one suit, one value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Card {
    pub suit: Suit,
    pub value: u8,
}

impl Card {
    /// A short label such as "Pearl 5".
    pub fn label(self) -> String {
        format!("{} {}", self.suit.name(), self.value)
    }
}

/// Builds one fresh, unshuffled deck: every suit at every value from 1 to
/// [`Suit::MAX_VALUE`].
pub fn full_deck() -> Vec<Card> {
    let mut cards = Vec::with_capacity(Suit::ALL.len() * Suit::MAX_VALUE as usize);
    for suit in Suit::ALL {
        for value in 1..=Suit::MAX_VALUE {
            cards.push(Card { suit, value });
        }
    }
    cards
}
