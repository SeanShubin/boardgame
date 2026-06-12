//! An ordered collection of cards.
//!
//! A [`Zone`] models any pile of cards on the table: a draw deck, a discard
//! pile, a player's hand, a play area. It is generic over the card type, so
//! each game supplies its own. The "top" of the zone is the end of the
//! underlying vector, which makes drawing and placing cards an O(1) operation.

use crate::rng::Rng;

/// An ordered pile of cards of type `C`.
#[derive(Clone, Debug)]
pub struct Zone<C> {
    cards: Vec<C>,
}

impl<C> Default for Zone<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C> Zone<C> {
    /// Creates an empty zone.
    pub fn new() -> Self {
        Self { cards: Vec::new() }
    }

    /// Creates a zone holding the given cards, bottom to top.
    pub fn from_cards(cards: Vec<C>) -> Self {
        Self { cards }
    }

    /// The number of cards in the zone.
    pub fn len(&self) -> usize {
        self.cards.len()
    }

    /// Whether the zone holds no cards.
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// The cards in the zone, bottom to top.
    pub fn cards(&self) -> &[C] {
        &self.cards
    }

    /// Places a card on top of the zone.
    pub fn push(&mut self, card: C) {
        self.cards.push(card);
    }

    /// Removes and returns the top card, or `None` when the zone is empty.
    pub fn draw(&mut self) -> Option<C> {
        self.cards.pop()
    }

    /// Removes and returns the card at `index` (0 is the bottom), shifting the
    /// rest down. Returns `None` when the index is out of range.
    pub fn remove(&mut self, index: usize) -> Option<C> {
        if index < self.cards.len() {
            Some(self.cards.remove(index))
        } else {
            None
        }
    }

    /// Removes every card and returns them, leaving the zone empty.
    pub fn take_all(&mut self) -> Vec<C> {
        std::mem::take(&mut self.cards)
    }

    /// Randomizes the order of the cards using the supplied generator.
    pub fn shuffle(&mut self, rng: &mut Rng) {
        rng.shuffle(&mut self.cards);
    }
}

impl<C> Extend<C> for Zone<C> {
    fn extend<T: IntoIterator<Item = C>>(&mut self, iter: T) {
        self.cards.extend(iter);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_then_draw_is_last_in_first_out() {
        let mut zone = Zone::new();
        zone.push(1);
        zone.push(2);
        zone.push(3);
        assert_eq!(zone.len(), 3);
        assert_eq!(zone.draw(), Some(3));
        assert_eq!(zone.draw(), Some(2));
        assert_eq!(zone.draw(), Some(1));
        assert_eq!(zone.draw(), None);
        assert!(zone.is_empty());
    }

    #[test]
    fn shuffle_preserves_contents() {
        let mut rng = Rng::new(123);
        let mut zone = Zone::from_cards((0..30).collect());
        zone.shuffle(&mut rng);
        let mut remaining = zone.take_all();
        remaining.sort();
        assert_eq!(remaining, (0..30).collect::<Vec<_>>());
    }

    #[test]
    fn remove_out_of_range_is_none() {
        let mut zone = Zone::from_cards(vec![10, 20]);
        assert_eq!(zone.remove(5), None);
        assert_eq!(zone.remove(0), Some(10));
    }
}
