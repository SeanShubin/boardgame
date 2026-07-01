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

/// Add a **Starter Kit** card: a Small card (name + type) that grows to show its five-stat line and
/// ability. `stats` is `[Might, Vitality, Toughness, Cadence, Finesse]` — the suitless-roster order
/// from `data/balance/generic-classes.ron`; `ability` is the derived strike card (Jab / Shot / …).
fn starter(tree: &mut Tableau, pile: PileId, name: &str, stats: [u8; 5], ability: &str) -> CardId {
    let id = typed(tree, pile, name, "Starter Kit");
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
/// `deckbound::engagement`) that the Starting Kit starters carry — each with a one-line description.
const ABILITIES: [(&str, &str); 4] = [
    ("Jab", "Melee · single target"),
    ("Shot", "Ranged · single target"),
    ("Sweep", "Melee · area"),
    ("Salvo", "Ranged · area"),
];

/// A small, representative table: a `Hand` of face-up cards (two actionable), a face-down `Deck`, and
/// a `Discard`. Enough to exercise focus/zoom, collapsed-vs-fanned piles, actionable highlighting, and
/// moving cards between piles. Every card carries a type, shown as a badge and as the deck's top label.
pub fn sample_table() -> Tableau {
    let mut tree = Tableau::new();
    let root = tree.root_id();

    // The "Locations" deck: a fixed 3×3 grid (2-D, non-editable) of place-piles from the Name Bank.
    // Each place is a small deck labelled by its Location-typed Zone card; clicking one drills into it.
    // Ashfen Crossing (the centre) has the heroes stationed under its name card, so drilling into it
    // retitles to "Ashfen Crossing" and reveals each hero.
    let locations = tree.add_pile(root, "Locations").expect("root exists");
    for place in LOCATIONS {
        let place_pile = tree.add_pile(locations, place).expect("locations exists");
        // Heroes slide UNDER the name card — added first, so the name card ends up on top.
        if place == "Ashfen Crossing" {
            for hero in HEROES {
                typed(&mut tree, place_pile, hero, "hero");
            }
        }
        // The name card caps the place: a Location-typed Zone card that labels it and titles its zone.
        let name = typed(&mut tree, place_pile, place, "Location");
        tree.set_card_kind(name, CardKind::Zone)
            .expect("place name card");
    }
    // The deck itself is labelled by a "Location" Zone card and reads as a fixed 3×3 grid.
    let loc_zone = typed(&mut tree, locations, "Location", "Zone");
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

    // A "Starting Kit" deck: one card per generic starter (the suitless roster from
    // `data/balance/generic-classes.ron`). Each is a Small card that grows to its five-stat line and
    // ability; a Zone card caps the deck so it reads "Starting Kit" collapsed and as the drill-in title.
    let starting_kit = tree.add_pile(root, "Starting Kit").expect("root exists");
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
    let kit_zone = typed(&mut tree, starting_kit, "Starting Kit", "Zone");
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
    let abilities_zone = typed(&mut tree, abilities, "Abilities", "Zone");
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

    // Spread the piles across the table so they start un-stacked; drag repositions them.
    tree.set_pile_pos(locations, 40.0, 40.0)
        .expect("locations exists");
    tree.set_pile_pos(starting_kit, 220.0, 40.0)
        .expect("starting kit exists");
    tree.set_pile_pos(abilities, 400.0, 40.0)
        .expect("abilities exists");

    tree
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_table_is_well_formed() {
        let t = sample_table();
        let root = t.pile(t.root_id()).unwrap();
        assert_eq!(root.subpiles().len(), 3); // Locations, Starting Kit, Abilities
        // Locations: a "Location" Zone card + 9 place name cards + 9 heroes under Ashfen Crossing.
        // Starting Kit: 4 starters + a Zone card. Abilities: 4 abilities + a Zone card.
        assert_eq!(t.card_count(), (1 + 9 + 9) + (4 + 1) + (4 + 1));
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
            .find(|&&id| t.pile(id).unwrap().label == "Starting Kit")
            .unwrap();
        let kit = t.pile(kit_id).unwrap();

        // Four starter cards under a "Starting Kit" Zone card.
        let starters = t.content_cards(kit.id);
        assert_eq!(starters.len(), 4);
        let names: Vec<&str> = starters
            .iter()
            .map(|&c| t.card(c).unwrap().name())
            .collect();
        assert_eq!(names, ["Skirmisher", "Sentinel", "Tempest", "Cleaver"]);
        for &c in starters {
            assert_eq!(t.card(c).unwrap().card_type(), "Starter Kit");
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
        assert_eq!(top.name(), "Starting Kit");
        assert_eq!(top.kind(), CardKind::Zone);
    }

    #[test]
    fn locations_are_places_with_heroes_under_ashfen() {
        let t = sample_table();
        let root = t.pile(t.root_id()).unwrap();
        let loc_id = *root
            .subpiles()
            .iter()
            .find(|&&id| t.pile(id).unwrap().label == "Locations")
            .unwrap();
        let locations = t.pile(loc_id).unwrap();

        // Nine place-piles, one per Name-Bank location, each labelled by a Location-typed Zone card.
        assert_eq!(locations.subpiles().len(), LOCATIONS.len());
        for (&pid, name) in locations.subpiles().iter().zip(LOCATIONS) {
            let place = t.pile(pid).unwrap();
            let label = t.card(*place.cards().last().unwrap()).unwrap();
            assert_eq!(label.name(), name);
            assert_eq!(label.card_type(), "Location");
            assert_eq!(label.kind(), CardKind::Zone);
        }

        // Ashfen Crossing sits in the centre cell (subpile index 4) with the heroes under its label.
        let ashfen = t.pile(locations.subpiles()[4]).unwrap();
        assert_eq!(
            t.card(*ashfen.cards().last().unwrap()).unwrap().name(),
            "Ashfen Crossing"
        );
        let heroes = t.content_cards(ashfen.id);
        assert_eq!(heroes.len(), HEROES.len());
        for (&cid, name) in heroes.iter().zip(HEROES) {
            let hero = t.card(cid).unwrap();
            assert_eq!(hero.name(), name);
            assert_eq!(hero.card_type(), "hero");
        }

        // The other places have nothing under their label yet.
        assert!(t.content_cards(locations.subpiles()[0]).is_empty());

        // The deck itself is still labelled by a "Location" Zone card.
        let loc_zone = t.card(*locations.cards().last().unwrap()).unwrap();
        assert_eq!(loc_zone.name(), "Location");
        assert_eq!(loc_zone.kind(), CardKind::Zone);
    }
}
