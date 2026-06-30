//! Sample [`Tableau`]s for prototyping and tests — a shared source of truth so feature prototypes
//! (the `cardtable` examples) and dev harnesses don't each hand-roll table data. Pure: no game, no
//! Bevy.

use crate::model::{Face, Tableau};

/// A small, representative table: a `Hand` of face-up cards (two actionable), a face-down `Deck`, and
/// a `Discard`. Enough to exercise focus/zoom, collapsed-vs-fanned piles, actionable highlighting, and
/// moving cards between piles.
pub fn sample_table() -> Tableau {
    let mut tree = Tableau::new();
    let root = tree.root_id();

    let hand = tree.add_pile(root, "Hand").expect("root exists");
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

    let pile = tree.add_pile(root, "Deck").expect("root exists");
    for _ in 0..6 {
        tree.add_card(pile, Face::Down, None).expect("pile exists");
    }

    let discard = tree.add_pile(root, "Discard").expect("root exists");
    tree.add_card(
        discard,
        Face::Up {
            title: "Spent Bolt".into(),
        },
        None,
    )
    .expect("discard exists");

    // A "Locations" pile of 25 fantasy places — exercises the stacked-depth visual and a high count.
    let locations = tree.add_pile(root, "Locations").expect("root exists");
    const PLACES: [&str; 25] = [
        "Emberfall",
        "Thornwood",
        "Greymarsh",
        "Duskhollow",
        "Frostspire",
        "Ravenmoor",
        "Ashen Reach",
        "Mistveil",
        "Stonewatch",
        "Wyrmrest",
        "Shadowfen",
        "Goldvale",
        "Ironhold",
        "Stormhaven",
        "Witchlight Bog",
        "Hollow Crown",
        "Sablewood",
        "Brackenmere",
        "Sunken Spire",
        "Cinderhall",
        "Briargate",
        "Moonwell",
        "Drakes Hollow",
        "Veiled Pass",
        "Thunderstep",
    ];
    for place in PLACES {
        tree.add_card(
            locations,
            Face::Up {
                title: place.to_string(),
            },
            None,
        )
        .expect("locations exists");
    }

    // Spread the piles across the table so they start un-stacked; drag repositions them.
    tree.set_pile_pos(hand, 40.0, 40.0).expect("hand exists");
    tree.set_pile_pos(pile, 220.0, 40.0).expect("pile exists");
    tree.set_pile_pos(discard, 400.0, 40.0)
        .expect("discard exists");
    tree.set_pile_pos(locations, 40.0, 240.0)
        .expect("locations exists");

    tree
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_table_is_well_formed() {
        let t = sample_table();
        let root = t.pile(t.root_id()).unwrap();
        assert_eq!(root.subpiles().len(), 4); // Hand, Deck, Discard, Locations
        assert_eq!(t.card_count(), 3 + 6 + 1 + 25);
    }
}
