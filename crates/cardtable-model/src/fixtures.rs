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

/// Add a **Kit** card: a Small card (name + type) that grows to show its five-stat line and
/// ability. `stats` is `[Might, Vitality, Toughness, Cadence, Finesse]` — the suitless-roster order
/// from `data/balance/generic-classes.ron`; `ability` is the derived strike card (Jab / Shot / …).
fn starter(tree: &mut Tableau, pile: PileId, name: &str, stats: [u8; 5], ability: &str) -> CardId {
    let id = typed(tree, pile, name, "Kit");
    let [might, vitality, toughness, cadence, finesse] = stats;
    tree.set_card_detail(
        id,
        vec![
            format!("Might {might} · Vitality {vitality} · Toughness {toughness}"),
            format!("Cadence {cadence} · Finesse {finesse}"),
            format!("Abilities: {ability}"),
        ],
    )
    .expect("starter card just added");
    // The kit's **recipe** — the ordered cards a character receives when equipped with it: each stat as
    // a name card then a value card, then the ability.
    tree.set_card_recipe(
        id,
        vec![
            "Might".into(),
            might.to_string(),
            "Vitality".into(),
            vitality.to_string(),
            "Toughness".into(),
            toughness.to_string(),
            "Cadence".into(),
            cadence.to_string(),
            "Finesse".into(),
            finesse.to_string(),
            ability.into(),
        ],
    )
    .expect("starter card just added");
    id
}

/// The authored location names from the Deckbound Name Bank
/// (`docs/games/deckbound/name-bank.md` § Locations). Ordered so "Ashfen Crossing" falls in the
/// centre cell (index 4, row-major) of the 3×3 grid.
const LOCATIONS: [&str; 9] = [
    "The Hollow Rampart",
    "Cinderwatch Keep",
    "Greywater Ford",
    "The Sundered Vault",
    "Ashfen Crossing",
    "Thornmarch Gate",
    "Emberfall Hollow",
    "The Salt Barrows",
    "Ninefold Deep",
];

/// The authored adventurer names from the Deckbound Name Bank
/// (`docs/games/deckbound/name-bank.md` § Adventurers) — the heroes stationed at Ashfen Crossing.
const HEROES: [&str; 9] = [
    "Vael Thornbrand",
    "Sera of the Ninth Watch",
    "Bram Cutter",
    "Isolde Greymantle",
    "Kord the Sentinel",
    "Nyx Ashwell",
    "Dallen Rook",
    "Mira Tempestborne",
    "Osric Vane",
];

/// The abilities currently in play — the derived strike cards (one per range × area cell; see
/// `deckbound::engagement`) that the Kit starters carry — each with a one-line description.
const ABILITIES: [(&str, &str); 4] = [
    ("Jab", "Melee · single target"),
    ("Shot", "Ranged · single target"),
    ("Sweep", "Melee · area"),
    ("Salvo", "Ranged · area"),
];

/// A small, representative table for the card-table game: an **Identity** deck of unrecruited heroes, a
/// **Kit** deck, an **Abilities** deck, and a **Locations** grid whose centre, **Ashfen
/// Crossing**, is the *inn* — a projection of the Identity and Kit decks where you drag a hero
/// onto a kit (or vice versa) to recruit them into a character deck. Every card is a physical,
/// single-homed card; a projection only *shows* other decks' cards, it doesn't move them.
pub fn sample_table() -> Tableau {
    let mut tree = Tableau::new();
    let root = tree.root_id();

    // The "Identity" deck: the unrecruited heroes — the canonical home of their identity cards. The inn
    // projects this deck; recruiting a hero (see `Tableau::combine`) removes it from here.
    let identity = tree.add_pile(root, "Identity").expect("root exists");
    for hero in HEROES {
        typed(&mut tree, identity, hero, "hero");
    }
    let identity_zone = typed(&mut tree, identity, "Identity", "Label");
    tree.set_card_kind(identity_zone, CardKind::Zone)
        .expect("identity zone card");
    tree.set_layout(
        identity,
        Layout {
            arrangement: Arrangement::Free,
            editable: true,
        },
    )
    .expect("identity exists");

    // A "Kit" deck: one card per generic starter (the suitless roster from
    // `data/balance/generic-classes.ron`). Each is a Small card that grows to its five-stat line and
    // ability, and carries a **recipe** — the cards a character gains when equipped with it.
    let starting_kit = tree.add_pile(root, "Kit").expect("root exists");
    starter(
        &mut tree,
        starting_kit,
        "Skirmisher",
        [2, 2, 1, 2, 1],
        "Jab",
    );
    starter(&mut tree, starting_kit, "Sentinel", [1, 2, 2, 1, 2], "Shot");
    starter(&mut tree, starting_kit, "Tempest", [1, 1, 1, 1, 2], "Salvo");
    starter(&mut tree, starting_kit, "Cleaver", [1, 1, 2, 1, 1], "Sweep");
    let kit_zone = typed(&mut tree, starting_kit, "Kit", "Label");
    tree.set_card_kind(kit_zone, CardKind::Zone)
        .expect("kit zone card");
    tree.set_layout(
        starting_kit,
        Layout {
            arrangement: Arrangement::Free,
            editable: true,
        },
    )
    .expect("starting kit exists");

    // An "Abilities" deck: one card per ability currently in use (the strike cards the starters carry).
    // Each is a Small card that grows to its one-line description.
    let abilities = tree.add_pile(root, "Abilities").expect("root exists");
    for (name, description) in ABILITIES {
        let id = typed(&mut tree, abilities, name, "ability");
        tree.set_card_detail(id, vec![description.to_string()])
            .expect("ability card just added");
    }
    let abilities_zone = typed(&mut tree, abilities, "Abilities", "Label");
    tree.set_card_kind(abilities_zone, CardKind::Zone)
        .expect("abilities zone card");
    tree.set_layout(
        abilities,
        Layout {
            arrangement: Arrangement::Free,
            editable: true,
        },
    )
    .expect("abilities exists");

    // The "Locations" deck: a fixed 3×3 grid (2-D, non-editable) of place-piles from the Name Bank,
    // each labelled by its Location-typed Zone card. **Ashfen Crossing** (the centre) is the *inn*: a
    // projection of the Identity and Kit decks — drill in to see the heroes and the kits
    // together and drag one onto the other to recruit (see `on_drop` / `Tableau::combine`).
    let locations = tree.add_pile(root, "Locations").expect("root exists");
    for place in LOCATIONS {
        let place_pile = tree.add_pile(locations, place).expect("locations exists");
        let name = typed(&mut tree, place_pile, place, "Location");
        tree.set_card_kind(name, CardKind::Zone)
            .expect("place name card");
        if place == "Ashfen Crossing" {
            // Ashfen holds one card, the **Inn** — drill into it to reach the assignment view: a
            // `Rows` pile whose Hero / Kit rows project the Identity and Kit decks, and whose Active
            // row (its own cards) holds the recruited hero-kit pairs (empty at first).
            let inn = tree.add_pile(place_pile, "Inn").expect("ashfen exists");
            for header in ["Hero", "Kit", "Active"] {
                let h = tree
                    .add_card(
                        inn,
                        Face::Up {
                            title: header.into(),
                        },
                        None,
                    )
                    .expect("inn exists");
                tree.set_card_kind(h, CardKind::Header)
                    .expect("header card");
                // Row headers are organizational, not playable — they print the "Label" type.
                tree.set_card_type(h, "Label").expect("header card");
            }
            tree.set_projection(inn, vec![identity, starting_kit])
                .expect("inn exists");
            tree.set_layout(
                inn,
                Layout {
                    arrangement: Arrangement::Rows,
                    editable: false,
                },
            )
            .expect("inn exists");
        }
    }
    let loc_zone = typed(&mut tree, locations, "Location", "Label");
    tree.set_card_kind(loc_zone, CardKind::Zone)
        .expect("zone card exists");
    tree.set_layout(
        locations,
        Layout {
            arrangement: Arrangement::Grid { columns: 3 },
            editable: false,
        },
    )
    .expect("locations exists");

    // Spread the piles across the table so they start un-stacked; drag repositions them.
    tree.set_pile_pos(identity, 40.0, 40.0)
        .expect("identity exists");
    tree.set_pile_pos(starting_kit, 220.0, 40.0)
        .expect("starting kit exists");
    tree.set_pile_pos(abilities, 400.0, 40.0)
        .expect("abilities exists");
    tree.set_pile_pos(locations, 580.0, 40.0)
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
        assert_eq!(root.subpiles().len(), 4); // Identity, Kit, Abilities, Locations
        // Identity: 9 heroes + a Zone card. Kit: 4 starters + a Zone card. Abilities: 4 + a Zone card.
        // Locations: a "Location" Zone card + 9 place name cards + the inn's 3 row-header cards
        // (Hero / Kit / Active) under Ashfen Crossing.
        assert_eq!(t.card_count(), (9 + 1) + (4 + 1) + (4 + 1) + (1 + 9 + 3));
    }

    #[test]
    fn abilities_deck_has_one_card_per_ability() {
        let t = sample_table();
        let root = t.pile(t.root_id()).unwrap();
        let id = *root
            .subpiles()
            .iter()
            .find(|&&id| t.pile(id).unwrap().label == "Abilities")
            .unwrap();
        let abilities = t.pile(id).unwrap();

        let cards = t.content_cards(abilities.id);
        assert_eq!(cards.len(), ABILITIES.len());
        for (&cid, (name, _)) in cards.iter().zip(ABILITIES) {
            let card = t.card(cid).unwrap();
            assert_eq!(card.name(), name);
            assert_eq!(card.card_type(), "ability");
        }
        let top = t.card(*abilities.cards().last().unwrap()).unwrap();
        assert_eq!(top.name(), "Abilities");
        assert_eq!(top.kind(), CardKind::Zone);
    }

    #[test]
    fn starting_kit_holds_the_four_starters() {
        let t = sample_table();
        let root = t.pile(t.root_id()).unwrap();
        let kit_id = *root
            .subpiles()
            .iter()
            .find(|&&id| t.pile(id).unwrap().label == "Kit")
            .unwrap();
        let kit = t.pile(kit_id).unwrap();

        // Four starter cards under a "Kit" Zone card.
        let starters = t.content_cards(kit.id);
        assert_eq!(starters.len(), 4);
        let names: Vec<&str> = starters
            .iter()
            .map(|&c| t.card(c).unwrap().name())
            .collect();
        assert_eq!(names, ["Skirmisher", "Sentinel", "Tempest", "Cleaver"]);
        for &c in starters {
            assert_eq!(t.card(c).unwrap().card_type(), "Kit");
        }

        // Skirmisher grows to its stat line + ability.
        let skirmisher = t.card(starters[0]).unwrap();
        assert!(skirmisher.detail().iter().any(|l| l.contains("Might 2")));
        assert!(
            skirmisher
                .detail()
                .iter()
                .any(|l| l.contains("Abilities: Jab"))
        );

        let top = t.card(*kit.cards().last().unwrap()).unwrap();
        assert_eq!(top.name(), "Kit");
        assert_eq!(top.kind(), CardKind::Zone);
    }

    #[test]
    fn heroes_live_in_identity_and_ashfen_is_the_inn_projection() {
        let t = sample_table();
        let root = t.pile(t.root_id()).unwrap();
        let find = |label: &str| {
            *root
                .subpiles()
                .iter()
                .find(|&&id| t.pile(id).unwrap().label == label)
                .unwrap()
        };

        // The nine heroes' canonical home is the Identity deck.
        let identity = find("Identity");
        let heroes = t.content_cards(identity);
        assert_eq!(heroes.len(), HEROES.len());
        for (&cid, name) in heroes.iter().zip(HEROES) {
            let hero = t.card(cid).unwrap();
            assert_eq!(hero.name(), name);
            assert_eq!(hero.card_type(), "hero");
        }

        // The Locations grid: 9 place-piles labelled by Location Zone cards; Ashfen (centre, index 4)
        // holds one card — the Inn.
        let locations = t.pile(find("Locations")).unwrap();
        assert_eq!(locations.subpiles().len(), LOCATIONS.len());
        let ashfen = t.pile(locations.subpiles()[4]).unwrap();
        assert_eq!(
            t.card(*ashfen.cards().last().unwrap()).unwrap().name(),
            "Ashfen Crossing"
        );
        let inn_id = ashfen.subpiles()[0];
        assert_eq!(t.pile(inn_id).unwrap().label, "Inn");

        // The Inn is a Rows pile projecting Identity + Kit; its rows are Hero, Kit, Active.
        let inn = t.pile(inn_id).unwrap();
        assert_eq!(inn.layout().arrangement, Arrangement::Rows);
        assert_eq!(inn.projection(), &[identity, find("Kit")]);
        let rows = t.row_groups(inn_id);
        assert_eq!(rows.len(), 3);
        let header = |i: usize| t.card(rows[i].0).unwrap().name();
        assert_eq!((header(0), header(1), header(2)), ("Hero", "Kit", "Active"));
        assert_eq!(rows[0].1.len(), HEROES.len()); // Hero row ← Identity deck
        assert_eq!(rows[1].1.len(), 4); // Kit row ← Kit deck
        assert!(rows[2].1.is_empty()); // Active row starts empty
    }

    #[test]
    fn active_pairs_reflect_as_character_decks_on_the_table() {
        let mut t = sample_table();
        let inn = {
            let root = t.pile(t.root_id()).unwrap();
            let locations = *root
                .subpiles()
                .iter()
                .find(|&&id| t.pile(id).unwrap().label == "Locations")
                .unwrap();
            let ashfen = t.pile(t.pile(locations).unwrap().subpiles()[4]).unwrap();
            ashfen.subpiles()[0]
        };

        // No pairs yet → no reflection decks.
        t.sync_character_decks(inn).unwrap();
        let reflections = |t: &Tableau| -> Vec<PileId> {
            t.pile(t.root_id())
                .unwrap()
                .subpiles()
                .iter()
                .copied()
                .filter(|&s| t.pile(s).unwrap().reflects().is_some())
                .collect()
        };
        assert!(reflections(&t).is_empty());

        // Form one active pair in the inn: a hero moved in + a kit copy with a recipe.
        let identity = *t
            .pile(t.root_id())
            .unwrap()
            .subpiles()
            .iter()
            .find(|&&id| t.pile(id).unwrap().label == "Identity")
            .unwrap();
        let hero = t.content_cards(identity)[0];
        let hero_name = t.card(hero).unwrap().name().to_string();
        let at = t.pile(inn).unwrap().cards().len();
        t.move_card(hero, inn, at).unwrap();
        let kit = t
            .add_card(
                inn,
                Face::Up {
                    title: "Cleaver".into(),
                },
                None,
            )
            .unwrap();
        t.set_card_recipe(kit, vec!["Might".into(), "1".into()])
            .unwrap();

        // Reflecting yields exactly one deck for that hero: the kit's cards under the hero's Zone label.
        t.sync_character_decks(inn).unwrap();
        let decks = reflections(&t);
        assert_eq!(decks.len(), 1);
        let deck = t.pile(decks[0]).unwrap();
        assert_eq!(deck.reflects(), Some(hero));
        assert_eq!(deck.label, hero_name);
        let names: Vec<&str> = t
            .content_cards(deck.id)
            .iter()
            .map(|&c| t.card(c).unwrap().name())
            .collect();
        assert_eq!(names, ["Might", "1"]); // the kit's recipe, copied
        assert_eq!(
            t.card(*deck.cards().last().unwrap()).unwrap().name(),
            hero_name // topped by the hero's Zone label
        );

        // Idempotent: reflecting again does not duplicate the deck.
        t.sync_character_decks(inn).unwrap();
        assert_eq!(reflections(&t).len(), 1);

        // Put the pair back (un-recruit) → the reflection deck disappears.
        t.remove_card(kit).unwrap();
        t.remove_card(hero).unwrap();
        t.sync_character_decks(inn).unwrap();
        assert!(reflections(&t).is_empty());
    }
}
