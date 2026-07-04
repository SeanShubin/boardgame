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

/// The round's phases in order, each with a one-line mechanical summary (condensed from
/// `docs/games/deckbound/reference/combat-phases.md`, the canonical text). The Rules deck renders one
/// card per phase; in combat we will surface these and cycle which one is active.
const PHASES: [(&str, &str); 10] = [
    (
        "Marshal",
        "Secretly assign each unit an intention — Vanguard, Outrider or Rearguard — and maybe bind a group. Re-declared each round.",
    ),
    (
        "Reveal",
        "Intentions and groups are revealed together and positions lock. Nobody moves; everything after resolves in the open.",
    ),
    (
        "Ready",
        "Standing abilities cast now (a Wall's brace, a Support's buff): ally-targeted, auto-land, last the round.",
    ),
    (
        "Intercept",
        "The front screens the flankers: each Vanguard strikes an enemy Outrider as it crosses, before it can raid.",
    ),
    (
        "Volley",
        "The back fires on the flankers: each Rearguard shoots an enemy Outrider — the pre-empt, before it arrives.",
    ),
    (
        "Raid",
        "Surviving Outriders strike the enemy Rearguard they crossed for — the breaker lands on the exposed back.",
    ),
    (
        "Clash",
        "The lines meet: each Rearguard fires an enemy Vanguard, and each engaging Vanguard strikes an enemy Vanguard.",
    ),
    (
        "Breach",
        "The deep blows land last: a Vanguard crosses to an exposed enemy Rearguard; stranded Outriders fall on the front.",
    ),
    (
        "Wipe pile",
        "At each engagement boundary the per-phase damage pile clears — sub-threshold damage does not carry; only Health persists.",
    ),
    (
        "Refresh",
        "Round end (the Lull): spent Tempo resets, Health carries over, the round advances. Five undecided rounds is a draw.",
    ),
];

/// The abilities currently in play — the derived strike cards (one per range × area cell; see
/// `deckbound::engagement`) that the Kit starters carry — each with a one-line description.
const ABILITIES: [(&str, &str); 4] = [
    ("Jab", "Melee · single target"),
    ("Shot", "Ranged · single target"),
    ("Sweep", "Melee · area"),
    ("Salvo", "Ranged · area"),
];

/// The one-line mechanical summary for a phase name (from [`PHASES`]), or `""` if unknown.
fn phase_detail(name: &str) -> &'static str {
    PHASES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|&(_, detail)| detail)
        .unwrap_or_default()
}

/// Lay a **Free** deck's content out in a tidy grid *below the top band* — the strip where the floating
/// Back and title overlays sit — so the very first render is clean (the shove is then only needed when a
/// card is actually dragged). Positions the deck's content cards, then any sub-piles, row-major across
/// `cols`. Saved tables restore their own positions, so this only shapes a fresh table.
fn grid_below_band(tree: &mut Tableau, deck: PileId, cols: usize) {
    const BAND: f32 = 52.0; // clears the Back / title overlay row
    const CW: f32 = 150.0; // cell width  (a Small card plus margin)
    const CH: f32 = 100.0; // cell height
    let spot = |i: usize| {
        let (col, row) = (i % cols, i / cols);
        (20.0 + col as f32 * CW, BAND + row as f32 * CH)
    };
    let cards: Vec<CardId> = tree.content_cards(deck).to_vec();
    let subs: Vec<PileId> = tree
        .pile(deck)
        .map(|p| p.subpiles().to_vec())
        .unwrap_or_default();
    for (i, c) in cards.iter().enumerate() {
        let (x, y) = spot(i);
        let _ = tree.set_card_pos(*c, x, y);
    }
    for (k, s) in subs.iter().enumerate() {
        let (x, y) = spot(cards.len() + k);
        let _ = tree.set_pile_pos(*s, x, y);
    }
}

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

    // A "Rules" deck: the round's phases as a two-level **hierarchy** (organizational only — nothing
    // mechanical changes). The five damage-dealing phases are children of a parent **Engage** sub-deck;
    // the rest are leaf cards. Every phase title carries an `(x/y)` sibling position, and the Engage card
    // lists its children. A Free deck (expanding a card shoves neighbours clear), seeded tidy.
    const TOP: [&str; 6] = [
        "Marshal",
        "Reveal",
        "Ready",
        "Engage",
        "Wipe pile",
        "Refresh",
    ];
    const ENGAGE_CHILDREN: [&str; 5] = ["Intercept", "Volley", "Raid", "Clash", "Breach"];
    let rules = tree.add_pile(root, "Rules").expect("root exists");
    for (i, &name) in TOP.iter().enumerate() {
        let pos = format!("({}/{})", i + 1, TOP.len());
        if name == "Engage" {
            // The parent deck of the damage-dealing phases; drill in to see its children.
            let engage = tree.add_pile(rules, "Engage").expect("rules exists");
            for (j, &child) in ENGAGE_CHILDREN.iter().enumerate() {
                let title = format!("{child} ({}/{})", j + 1, ENGAGE_CHILDREN.len());
                let id = typed(&mut tree, engage, &title, "phase");
                tree.set_card_detail(id, vec![phase_detail(child).to_string()])
                    .expect("child phase card");
            }
            // Engage's label *is* the parent card: its title lists the children (and its own `(x/y)`),
            // its detail is the damage-order summary.
            let label = format!("Engage {pos}: {}", ENGAGE_CHILDREN.join(", "));
            let engage_zone = typed(&mut tree, engage, &label, "phase");
            tree.set_card_kind(engage_zone, CardKind::Zone)
                .expect("engage label");
            tree.set_card_detail(
                engage_zone,
                vec![
                    "Intercept — Vanguard -> Outrider".into(),
                    "Volley — Rearguard -> Outrider".into(),
                    "Raid — Outrider -> Rearguard".into(),
                    "Clash — Rearguard / Vanguard -> Vanguard".into(),
                    "Breach — the trailing blows land".into(),
                ],
            )
            .expect("engage detail");
            grid_below_band(&mut tree, engage, 3);
            tree.set_layout(
                engage,
                Layout {
                    arrangement: Arrangement::Free,
                    editable: true,
                },
            )
            .expect("engage exists");
        } else {
            let title = format!("{name} {pos}");
            let id = typed(&mut tree, rules, &title, "phase");
            tree.set_card_detail(id, vec![phase_detail(name).to_string()])
                .expect("leaf phase card");
        }
    }
    let rules_zone = typed(&mut tree, rules, "Rules", "Label");
    tree.set_card_kind(rules_zone, CardKind::Zone)
        .expect("rules zone card");
    grid_below_band(&mut tree, rules, 3);
    tree.set_layout(
        rules,
        Layout {
            arrangement: Arrangement::Free,
            editable: true,
        },
    )
    .expect("rules exists");

    // Lay each Free deck's cards out tidily below the overlay band, so the first render of a zone is
    // clean — the Back card sits in its own row up top with the cards beneath it, no shove required yet.
    grid_below_band(&mut tree, identity, 4);
    grid_below_band(&mut tree, starting_kit, 4);
    grid_below_band(&mut tree, abilities, 4);

    // Spread the piles across the table so they start un-stacked; drag repositions them.
    tree.set_pile_pos(identity, 40.0, 40.0)
        .expect("identity exists");
    tree.set_pile_pos(starting_kit, 220.0, 40.0)
        .expect("starting kit exists");
    tree.set_pile_pos(abilities, 400.0, 40.0)
        .expect("abilities exists");
    tree.set_pile_pos(locations, 580.0, 40.0)
        .expect("locations exists");
    tree.set_pile_pos(rules, 760.0, 40.0).expect("rules exists");

    tree
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_table_is_well_formed() {
        let t = sample_table();
        let root = t.pile(t.root_id()).unwrap();
        assert_eq!(root.subpiles().len(), 5); // Identity, Kit, Abilities, Locations, Rules
        // Identity: 9 heroes + a Zone card. Kit: 4 starters + a Zone card. Abilities: 4 + a Zone card.
        // Locations: a "Location" Zone card + 9 place name cards + the inn's 3 row-header cards
        // (Hero / Kit / Active) under Ashfen Crossing. Rules: 5 leaf phase cards + a Zone label; the
        // Engage sub-deck: 5 child phases + a Zone label.
        assert_eq!(
            t.card_count(),
            (9 + 1) + (4 + 1) + (4 + 1) + (1 + 9 + 3) + ((5 + 1) + (5 + 1))
        );
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
    fn sample_table_round_trips_through_ron() {
        let t = sample_table();
        let text = ron::to_string(&t).expect("serialize to RON");
        let back: Tableau = ron::from_str(&text).expect("deserialize from RON");
        // Structure survives the round-trip (HashMaps keyed by integer ids and all card/pile data).
        assert_eq!(back.card_count(), t.card_count());
        let subs = |t: &Tableau| t.pile(t.root_id()).unwrap().subpiles().len();
        assert_eq!(subs(&back), subs(&t));
        // A known card comes back intact.
        let root = t.pile(t.root_id()).unwrap();
        let rules_id = *root
            .subpiles()
            .iter()
            .find(|&&id| t.pile(id).unwrap().label == "Rules")
            .unwrap();
        let first = t.content_cards(rules_id)[0];
        assert_eq!(back.card(first).unwrap().name(), "Marshal (1/6)");
    }

    #[test]
    fn rules_phases_form_a_hierarchy_with_engage_parenting_the_damage_phases() {
        let t = sample_table();
        let root = t.pile(t.root_id()).unwrap();
        let rules = t
            .pile(
                *root
                    .subpiles()
                    .iter()
                    .find(|&&id| t.pile(id).unwrap().label == "Rules")
                    .unwrap(),
            )
            .unwrap();

        // Five leaf phases as content cards, each with an (x/6) sibling position.
        let leaves: Vec<&str> = t
            .content_cards(rules.id)
            .iter()
            .map(|&c| t.card(c).unwrap().name())
            .collect();
        assert_eq!(
            leaves,
            [
                "Marshal (1/6)",
                "Reveal (2/6)",
                "Ready (3/6)",
                "Wipe pile (5/6)",
                "Refresh (6/6)"
            ]
        );

        // Engage is the parent sub-deck of the damage phases; its label lists the children and its (x/6).
        assert_eq!(rules.subpiles().len(), 1);
        let engage = t.pile(rules.subpiles()[0]).unwrap();
        assert_eq!(engage.label, "Engage");
        assert_eq!(
            t.card(*engage.cards().last().unwrap()).unwrap().name(),
            "Engage (4/6): Intercept, Volley, Raid, Clash, Breach"
        );

        // Five child phases, each with an (x/5) sibling position.
        let children: Vec<&str> = t
            .content_cards(engage.id)
            .iter()
            .map(|&c| t.card(c).unwrap().name())
            .collect();
        assert_eq!(
            children,
            [
                "Intercept (1/5)",
                "Volley (2/5)",
                "Raid (3/5)",
                "Clash (4/5)",
                "Breach (5/5)"
            ]
        );

        // Topped by a "Rules" Zone label.
        let top = t.card(*rules.cards().last().unwrap()).unwrap();
        assert_eq!(top.name(), "Rules");
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

        // No pairs yet -> no reflection decks.
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

        // Put the pair back (un-recruit) -> the reflection deck disappears.
        t.remove_card(kit).unwrap();
        t.remove_card(hero).unwrap();
        t.sync_character_decks(inn).unwrap();
        assert!(reflections(&t).is_empty());
    }
}
