//! A tiny **generic demo table** for the renderer's dev tools (the `sandbox` example and the card
//! gallery). It carries no game content - just a handful of cards with varied text so the renderer can be
//! driven and its text handling audited at every render size. The product supplies its own real table (the
//! game's opening board); the renderer itself ships no world.

use cardtable_model::{Board, CardKind, Face};

/// A small game-free table: one deck of cards with short and long titles, detail lines, and a multi-line
/// body panel - enough to drive and audit the renderer without any game wired in.
pub fn demo_table() -> Board {
    let mut tree = Board::new();
    let root = tree.root_id();

    let deck = tree.add_pile(root, "Samples").expect("root exists");
    // Each row carries a `type` too, so the audit exercises the **type badge** the Medium footprint budgets
    // for - the extra row that pushes a real hero card over if the height formula is a line short. The
    // "Vanguard" row mirrors a game hero: a type badge plus a three-line stat block.
    let cards: [(&str, &str, &[&str]); 5] = [
        ("Knight", "hero", &["A stalwart front-liner."]),
        ("Mage", "hero", &["Slings spells from afar.", "Fragile."]),
        (
            "Vanguard",
            "hero",
            &[
                "Might 3 | Vitality 2 | Toughness 4",
                "Cadence 2 | Finesse 1",
                "Abilities: Jab (reach 1)",
            ],
        ),
        (
            "A Very Long Card Title That Should Wrap Or Clip",
            "Kit",
            &["Tests overflow of the title at each render size."],
        ),
        (
            "Note",
            "event",
            &[
                "A longer body to exercise the Large render size:",
                "line two",
                "line three",
                "line four",
            ],
        ),
    ];
    for (title, card_type, detail) in cards {
        let id = tree
            .add_card(
                deck,
                Face::Up {
                    title: title.into(),
                },
                None,
            )
            .expect("deck exists");
        tree.set_card_type(id, card_type).expect("card just added");
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
