//! Sample [`DeckTree`]s for prototyping and tests — a shared source of truth so feature prototypes
//! (the `cardtable` examples) and dev harnesses don't each hand-roll table data. Pure: no game, no
//! Bevy.

use crate::model::{DeckTree, Face};

/// A small, representative table: a `Hand` of face-up cards (two actionable), a face-down `Deck`, and
/// a `Discard`. Enough to exercise focus/zoom, collapsed-vs-fanned decks, actionable highlighting, and
/// moving cards between decks.
pub fn sample_table() -> DeckTree {
    let mut tree = DeckTree::new();
    let root = tree.root_id();

    let hand = tree.add_deck(root, "Hand").expect("root exists");
    tree.add_card(
        hand,
        Face::Up {
            title: "Knight".into(),
        },
        Some(0),
    )
    .expect("hand exists");
    tree.add_card(
        hand,
        Face::Up {
            title: "Mage".into(),
        },
        Some(1),
    )
    .expect("hand exists");
    tree.add_card(
        hand,
        Face::Up {
            title: "Healer".into(),
        },
        None,
    )
    .expect("hand exists");

    let deck = tree.add_deck(root, "Deck").expect("root exists");
    for _ in 0..6 {
        tree.add_card(deck, Face::Down, None).expect("deck exists");
    }

    let discard = tree.add_deck(root, "Discard").expect("root exists");
    tree.add_card(
        discard,
        Face::Up {
            title: "Spent Bolt".into(),
        },
        None,
    )
    .expect("discard exists");

    tree
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_table_is_well_formed() {
        let t = sample_table();
        let root = t.deck(t.root_id()).unwrap();
        assert_eq!(root.subdecks().len(), 3); // Hand, Deck, Discard
        assert_eq!(t.card_count(), 3 + 6 + 1);
    }
}
