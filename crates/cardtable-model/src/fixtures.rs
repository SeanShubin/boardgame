//! Sample [`Tableau`]s for prototyping and tests — a shared source of truth so feature prototypes
//! (the `cardtable` examples) and dev harnesses don't each hand-roll table data. Pure: no game, no
//! Bevy.

use crate::model::{Arrangement, CardId, CardKind, Face, Layout, PileId, Tableau};

/// Add a face-up card with a name and a [`type`](crate::model::Card::card_type) to `pile`, returning
/// its id. The type is what the card-table shows as its type badge and the deck's top-card label.
fn typed(tree: &mut Tableau, pile: PileId, title: &str, card_type: &str) -> CardId {
    let id = tree
        .add_card(
            pile,
            Face::Up {
                title: title.into(),
            },
            None,
        )
        .expect("pile exists");
    tree.set_card_type(id, card_type).expect("card just added");
    id
}

/// The authored location names from the Deckbound Name Bank
/// (`docs/games/deckbound/name-bank.md` § Locations).
const LOCATIONS: [&str; 9] = [
    "Ashfen Crossing",
    "The Hollow Rampart",
    "Cinderwatch Keep",
    "Greywater Ford",
    "The Sundered Vault",
    "Thornmarch Gate",
    "Emberfall Hollow",
    "The Salt Barrows",
    "Ninefold Deep",
];

/// A small, representative table: a `Hand` of face-up cards (two actionable), a face-down `Deck`, and
/// a `Discard`. Enough to exercise focus/zoom, collapsed-vs-fanned piles, actionable highlighting, and
/// moving cards between piles. Every card carries a type, shown as a badge and as the deck's top label.
pub fn sample_table() -> Tableau {
    let mut tree = Tableau::new();
    let root = tree.root_id();

    // Hand: face-up cards. Knight and Mage carry detail (click to grow to the Card size); Healer is
    // name-only (clicking it does nothing — a good contrast).
    let hand = tree.add_pile(root, "Hand").expect("root exists");
    let knight = typed(&mut tree, hand, "Knight", "Adventurer");
    tree.set_card_detail(
        knight,
        vec![
            "Might 4 · Vitality 6".into(),
            "Toughness 3 · Finesse 2".into(),
            "A stalwart fighter who holds the line.".into(),
        ],
    )
    .expect("knight exists");
    let mage = typed(&mut tree, hand, "Mage", "Adventurer");
    tree.set_card_detail(
        mage,
        vec![
            "Might 1 · Vitality 4".into(),
            "Cadence 5 · Finesse 4".into(),
            "Hurls a bolt of fire at a distant foe.".into(),
        ],
    )
    .expect("mage exists");
    typed(&mut tree, hand, "Healer", "Adventurer");

    let pile = tree.add_pile(root, "Deck").expect("root exists");
    for _ in 0..6 {
        tree.add_card(pile, Face::Down, None).expect("pile exists");
    }

    let discard = tree.add_pile(root, "Discard").expect("root exists");
    typed(&mut tree, discard, "Spent Bolt", "Item");

    // A "Combat Log" — a utility card with no physical counterpart: only a Full panel, no card detail,
    // so clicking it cycles Name -> Full -> Name.
    let utility = tree.add_pile(root, "Utility").expect("root exists");
    let log = typed(&mut tree, utility, "Combat Log", "Log");
    tree.set_card_panel(
        log,
        vec![
            "— Round 1 —".into(),
            "Knight strikes Goblin for 3.".into(),
            "Goblin falls.".into(),
            "Mage scorches the Ogre for 5.".into(),
            "— Round 2 —".into(),
            "Ogre swings at Knight — turned aside by armor.".into(),
            "Healer mends Knight for 2.".into(),
        ],
    )
    .expect("log card");

    // A "Quiver" of 5 identical Arrows — drilling in shows them grouped as "Arrow ×5".
    let quiver = tree.add_pile(root, "Quiver").expect("root exists");
    for _ in 0..5 {
        typed(&mut tree, quiver, "Arrow", "Item");
    }

    // A "Locations" pile drawn from the authored Name Bank — each a small Location card; the deck chip
    // shows its top card's name and type over the count.
    let locations = tree.add_pile(root, "Locations").expect("root exists");
    for place in LOCATIONS {
        typed(&mut tree, locations, place, "Location");
    }
    // A zone-naming card on top of the deck: because it is a `Zone` card, drilling into the pile shows
    // its name ("Location") as the zone header, and its type badge reads "Zone".
    let loc_zone = typed(&mut tree, locations, "Location", "Zone");
    tree.set_card_kind(loc_zone, CardKind::Zone)
        .expect("zone card exists");
    // The nine locations read as a fixed 3×3 grid that can't be reordered — a 2-D, non-editable deck.
    tree.set_layout(
        locations,
        Layout {
            arrangement: Arrangement::Grid { columns: 3 },
            editable: false,
        },
    )
    .expect("locations exists");

    // Spread the piles across the table so they start un-stacked; drag repositions them.
    tree.set_pile_pos(hand, 40.0, 40.0).expect("hand exists");
    tree.set_pile_pos(pile, 220.0, 40.0).expect("pile exists");
    tree.set_pile_pos(discard, 400.0, 40.0)
        .expect("discard exists");
    tree.set_pile_pos(locations, 40.0, 240.0)
        .expect("locations exists");
    tree.set_pile_pos(utility, 220.0, 240.0)
        .expect("utility exists");
    tree.set_pile_pos(quiver, 400.0, 240.0)
        .expect("quiver exists");

    tree
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_table_is_well_formed() {
        let t = sample_table();
        let root = t.pile(t.root_id()).unwrap();
        assert_eq!(root.subpiles().len(), 6); // Hand, Deck, Discard, Utility, Quiver, Locations
        assert_eq!(t.card_count(), 3 + 6 + 1 + 1 + 5 + (9 + 1)); // Locations: 9 places + a Zone card
    }

    #[test]
    fn locations_are_typed_from_the_name_bank() {
        let t = sample_table();
        let root = t.pile(t.root_id()).unwrap();
        // Locations is the last sub-pile added.
        let locations = t.pile(*root.subpiles().last().unwrap()).unwrap();
        // The 9 authored places sit at the bottom; a Zone card ("Location") caps the deck.
        assert_eq!(locations.cards().len(), LOCATIONS.len() + 1);
        for (&cid, name) in locations.cards().iter().zip(LOCATIONS) {
            let card = t.card(cid).unwrap();
            assert_eq!(card.name(), name);
            assert_eq!(card.card_type(), "Location");
        }
        let top = t.card(*locations.cards().last().unwrap()).unwrap();
        assert_eq!(top.name(), "Location");
        assert_eq!(top.card_type(), "Zone");
        assert_eq!(top.kind(), CardKind::Zone);
    }
}
