//! A tiny **generic demo table** for the renderer's dev tools (the `sandbox` example and the card
//! gallery). It carries no game content - just a handful of cards with varied text so the renderer can be
//! driven and its text handling audited at every render size. The product supplies its own real table (the
//! game's opening board); the renderer itself ships no world.

use cardtable_model::{CardKind, Face, Tableau};

/// A small game-free table: one deck of cards with short and long titles, detail lines, and a multi-line
/// body panel - enough to drive and audit the renderer without any game wired in.
pub fn demo_table() -> Tableau {
    let mut tree = Tableau::new();
    let root = tree.root_id();

    let deck = tree.add_pile(root, "Samples").expect("root exists");
    let cards: [(&str, &[&str]); 4] = [
        ("Knight", &["A stalwart front-liner."]),
        ("Mage", &["Slings spells from afar.", "Fragile."]),
        (
            "A Very Long Card Title That Should Wrap Or Clip",
            &["Tests overflow of the title at each render size."],
        ),
        (
            "Note",
            &[
                "A longer body to exercise the Large render size:",
                "line two",
                "line three",
                "line four",
            ],
        ),
    ];
    for (title, detail) in cards {
        let id = tree
            .add_card(
                deck,
                Face::Up {
                    title: title.into(),
                },
                None,
            )
            .expect("deck exists");
        tree.set_card_detail(id, detail.iter().map(|s| s.to_string()).collect())
            .expect("card just added");
    }
    // A zone label card so the deck reads as a named zone, like a product deck.
    let zone = tree
        .add_card(
            deck,
            Face::Up {
                title: "Samples".into(),
            },
            None,
        )
        .expect("deck exists");
    tree.set_card_kind(zone, CardKind::Zone).expect("zone card");

    tree
}
