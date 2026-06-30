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

    // Hand: face-up cards. Knight and Mage carry detail (click to grow to the Card size); Healer is
    // name-only (clicking it does nothing — a good contrast).
    let hand = tree.add_pile(root, "Hand").expect("root exists");
    let knight = tree
        .add_card(
            hand,
            Face::Up {
                title: "Knight".into(),
            },
            None,
        )
        .expect("hand exists");
    tree.set_card_detail(
        knight,
        vec![
            "Might 4 · Vitality 6".into(),
            "Toughness 3 · Finesse 2".into(),
            "A stalwart fighter who holds the line.".into(),
        ],
    )
    .expect("knight exists");
    let mage = tree
        .add_card(
            hand,
            Face::Up {
                title: "Mage".into(),
            },
            None,
        )
        .expect("hand exists");
    tree.set_card_detail(
        mage,
        vec![
            "Might 1 · Vitality 4".into(),
            "Cadence 5 · Finesse 4".into(),
            "Hurls a bolt of fire at a distant foe.".into(),
        ],
    )
    .expect("mage exists");
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

    // A "Combat Log" — a utility card with no physical counterpart: only a Full panel, no card detail,
    // so clicking it cycles Name -> Full -> Name.
    let utility = tree.add_pile(root, "Utility").expect("root exists");
    let log = tree
        .add_card(
            utility,
            Face::Up {
                title: "Combat Log".into(),
            },
            None,
        )
        .expect("utility exists");
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
        tree.add_card(
            quiver,
            Face::Up {
                title: "Arrow".into(),
            },
            None,
        )
        .expect("quiver exists");
    }

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
        assert_eq!(t.card_count(), 3 + 6 + 1 + 1 + 5 + 25);
    }
}
