//! The only bridge to the game side: turn a renderer-agnostic [`contract::TableView`] into a
//! [`DeckTree`]. Each [`ZoneView`](contract::ZoneView) becomes a deck under the root; each
//! [`CardView`](contract::CardView) becomes a card, carrying its actionable index. This is the sole
//! module that depends on `contract`; everything in [`model`](crate::model) stays game-agnostic.

use contract::{CardFace, TableView};

use crate::model::{DeckTree, Face};

/// Builds a fresh [`DeckTree`] from a table snapshot: a root deck holding one sub-deck per zone, in
/// presentation order, each filled with the zone's cards.
pub fn from_table_view(view: &TableView) -> DeckTree {
    let mut tree = DeckTree::new();
    let root = tree.root_id();
    for (index, zone) in view.zones.iter().enumerate() {
        let deck = tree
            .add_deck(root, zone.label.clone())
            .expect("root deck exists");
        // Lay the decks out in a starting row; the renderer lets the player drag them anywhere.
        tree.set_deck_pos(deck, 24.0 + index as f32 * 180.0, 24.0)
            .expect("just-created deck exists");
        for card in &zone.cards {
            let face = match &card.face {
                CardFace::Up { title, .. } => Face::Up {
                    title: title.clone(),
                },
                CardFace::Down => Face::Down,
            };
            tree.add_card(deck, face, card.action)
                .expect("just-created deck exists");
        }
    }
    tree
}

#[cfg(test)]
mod tests {
    use super::*;
    use contract::{CardView, Layout, TableView, ZoneView};

    fn zone(label: &str, cards: Vec<CardView>) -> ZoneView {
        ZoneView {
            label: label.to_string(),
            layout: Layout::Stack,
            owner: None,
            cards,
        }
    }

    #[test]
    fn maps_zones_to_decks_and_faces_without_a_game() {
        let view = TableView {
            status: "test".to_string(),
            zones: vec![
                zone(
                    "Hand",
                    vec![CardView::up("Knight").action(0), CardView::down()],
                ),
                zone("Deck", vec![CardView::up("Mage")]),
            ],
            ..Default::default()
        };

        let tree = from_table_view(&view);
        let root = tree.deck(tree.root_id()).unwrap();
        assert_eq!(root.subdecks().len(), 2);

        let hand = tree.deck(root.subdecks()[0]).unwrap();
        assert_eq!(hand.label, "Hand");
        assert_eq!(hand.cards().len(), 2);

        let knight = tree.card(hand.cards()[0]).unwrap();
        assert_eq!(
            knight.face,
            Face::Up {
                title: "Knight".to_string()
            }
        );
        assert_eq!(knight.actionable, Some(0));

        let back = tree.card(hand.cards()[1]).unwrap();
        assert_eq!(back.face, Face::Down);
        assert!(!back.is_actionable());

        assert_eq!(tree.card_count(), 3);
    }
}
