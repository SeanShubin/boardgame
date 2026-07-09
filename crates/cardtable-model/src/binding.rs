//! The only bridge to the game side: turn a renderer-agnostic [`contract::TableView`] into a
//! [`Tableau`]. Each [`ZoneView`](contract::ZoneView) becomes a pile under the root; each
//! [`CardView`](contract::CardView) becomes a card, carrying its actionable index. This is the sole
//! module that depends on `contract`; everything in [`model`](crate::model) stays game-agnostic.

use contract::{CardFace, TableView, ZoneView};

use crate::model::{Face, PileId, Tableau};

/// Builds a fresh [`Tableau`] from a table snapshot: a root pile holding one sub-pile per zone, in
/// presentation order, each filled with the zone's cards — and, recursively, any nested sub-zones as
/// piles inside their parent (so a card-table can drill into them).
pub fn from_table_view(view: &TableView) -> Tableau {
    let mut tree = Tableau::new();
    let root = tree.root_id();
    for (index, zone) in view.zones.iter().enumerate() {
        add_zone(&mut tree, root, zone, index);
    }
    tree
}

/// Add one [`ZoneView`] as a pile under `parent`, fill it with the zone's cards, then recurse into its
/// nested sub-zones. `index` positions the pile in a starting row (the renderer lets the player drag it
/// anywhere afterwards).
fn add_zone(tree: &mut Tableau, parent: PileId, zone: &ZoneView, index: usize) -> PileId {
    let pile = tree
        .add_pile(parent, zone.label.clone())
        .expect("parent pile exists");
    tree.set_pile_pos(pile, 24.0 + index as f32 * 180.0, 24.0)
        .expect("just-created pile exists");
    for card in &zone.cards {
        let (face, card_type, detail, panel) = match &card.face {
            CardFace::Up {
                title,
                type_line,
                body,
                panel,
                ..
            } => (
                Face::Up {
                    title: title.clone(),
                },
                type_line.clone(),
                body.clone(),
                panel.clone(),
            ),
            // A contract view carries no front for a face-down card, so it comes across anonymous
            // (empty remembered front). The model keeps the field (PC.2) for cards it flips itself.
            CardFace::Down => (
                Face::Down {
                    title: String::new(),
                },
                None,
                Vec::new(),
                Vec::new(),
            ),
        };
        let id = tree
            .add_card(pile, face, card.action)
            .expect("just-created pile exists");
        if let Some(card_type) = card_type {
            tree.set_card_type(id, card_type)
                .expect("just-created card exists");
        }
        if !detail.is_empty() {
            tree.set_card_detail(id, detail)
                .expect("just-created card exists");
        }
        if !panel.is_empty() {
            tree.set_card_panel(id, panel)
                .expect("just-created card exists");
        }
        if card.quantity != 1 {
            tree.set_card_quantity(id, card.quantity)
                .expect("just-created card exists");
        }
    }
    // Nested sub-zones become piles inside this one — the recursion that makes the seam card-table-native.
    for (i, sub) in zone.zones.iter().enumerate() {
        add_zone(tree, pile, sub, i);
    }
    pile
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
            zones: Vec::new(),
        }
    }

    fn nested(label: &str, cards: Vec<CardView>, zones: Vec<ZoneView>) -> ZoneView {
        ZoneView {
            zones,
            ..zone(label, cards)
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
        let root = tree.pile(tree.root_id()).unwrap();
        assert_eq!(root.subpiles().len(), 2);

        let hand = tree.pile(root.subpiles()[0]).unwrap();
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
        assert!(back.is_face_down());
        assert!(!back.is_actionable());

        assert_eq!(tree.card_count(), 3);
    }

    #[test]
    fn nested_zones_become_piles_inside_their_parent() {
        // A "Locations" zone with one card, containing a "Keep" sub-zone that itself holds a card.
        let view = TableView {
            zones: vec![nested(
                "Locations",
                vec![CardView::up("Map")],
                vec![zone("Keep", vec![CardView::up("Warden")])],
            )],
            ..Default::default()
        };

        let tree = from_table_view(&view);
        let root = tree.pile(tree.root_id()).unwrap();
        assert_eq!(root.subpiles().len(), 1, "one top-level zone");

        let locations = tree.pile(root.subpiles()[0]).unwrap();
        assert_eq!(locations.label, "Locations");
        assert_eq!(locations.cards().len(), 1, "its own card stays");
        assert_eq!(
            locations.subpiles().len(),
            1,
            "the nested zone became a pile inside it"
        );

        let keep = tree.pile(locations.subpiles()[0]).unwrap();
        assert_eq!(keep.label, "Keep");
        assert_eq!(tree.card(keep.cards()[0]).unwrap().front_title(), "Warden");
    }

    #[test]
    fn carries_card_type_body_panel_and_quantity() {
        let view = TableView {
            zones: vec![zone(
                "Stats",
                vec![
                    CardView::up("Might")
                        .typed("stat")
                        .body(vec!["force behind a strike".into()])
                        .panel(vec!["a longer explanation".into()])
                        .times(5),
                ],
            )],
            ..Default::default()
        };

        let tree = from_table_view(&view);
        let stats = tree
            .pile(tree.pile(tree.root_id()).unwrap().subpiles()[0])
            .unwrap();
        let c = tree.card(stats.cards()[0]).unwrap();
        assert_eq!(c.card_type(), "stat");
        assert_eq!(c.detail(), &["force behind a strike".to_string()]);
        assert_eq!(c.panel(), &["a longer explanation".to_string()]);
        assert_eq!(c.quantity(), 5);
    }
}
