//! Sample [`Board`]s for prototyping and tests — a shared source of truth so feature prototypes
//! (the `cardtable` examples) and dev harnesses don't each hand-roll table data. Pure: no game, no
//! Bevy.

use cardtable_model::{Arrangement, Board, CardId, CardKind, Face, Layout, Node, PileId, Recipe};
use deckbound_content::catalog;

/// Add a face-up card with a name and a [`type`](cardtable_model::Card::card_type) to `pile`, returning
/// its id. The type is what the card-table shows as its type badge and the deck's top-card label.
fn typed(tree: &mut Board, pile: PileId, title: &str, card_type: &str) -> CardId {
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

/// Add a **hero** card — the kit *is* the hero (they were merged; see the Inn removal). Named for its kit
/// ("Raider", "Marksman", ...), typed `hero`, it grows to show the five-stat line and ability that kit
/// gives, and carries that kit's **recipe**. `stats` is `[Might, Vitality, Grit, Cadence, Finesse]` —
/// the suitless-roster order from `data/balance/generic-classes.ron`; `ability` is the derived strike card
/// (Jab / Shot / ...).
///
/// The recipe is a reusable *spec* (never consumed): the five stat values + the ability that
/// [`Board::equip_character`] assembles by **moving** a stat-name card, a number card, and an ability card
/// out of the banks into the character deck (PC.2, no mint).
fn hero(tree: &mut Board, pile: PileId, name: &str, stats: [u8; 5], ability: &str) -> CardId {
    let id = typed(tree, pile, name, "hero");
    let [might, vitality, grit, cadence, finesse] = stats;
    // The card carries its **computed** stats, not just its raw ones: the two **pools** its stats become in a
    // fight — Vitality is its Health cards, Cadence its Tempo cards. That is what connects a stat printed on
    // the card to the cards you actually flip in combat. Derived, never stored.
    //
    // It carries **no rank/intention indicator**: which position a hero is for is already obvious from
    // context — its kit *name* says it (a Raider raids, a Bastion holds the line), and in a fight its tile
    // sits in its rank's lane. The reach and spread likewise need no line of their own; the ability line
    // below already reads "Jab: Melee | single target". (The derivation still exists — `catalog::kit_intention`
    // — it just isn't printed.)
    tree.set_card_detail(
        id,
        vec![
            format!("Might {might} | Vitality {vitality} | Grit {grit}"),
            format!("Cadence {cadence} | Finesse {finesse}"),
            format!("Health {vitality} cards | Tempo {cadence} cards"),
            format!("{ability}: {}", catalog::ability_description(ability)),
        ],
    )
    .expect("hero card just added");
    tree.set_card_recipe(
        id,
        Recipe {
            stats,
            ability: ability.into(),
        },
    )
    .expect("hero card just added");
    id
}

/// Author a **foe** card for creature `c` (typed `foe`): a Small card (name + type) that grows to show
/// its derived intention and posture, its five-stat line, and its ability. Both the intention and the
/// posture are *derived* from the stats + ability (`catalog::creature_intention` / `creature_posture`),
/// never stored — the card reads back what the numbers already say. Mirrors [`hero`] for the party.
fn creature_card(tree: &mut Board, pile: PileId, c: &catalog::Creature) -> CardId {
    let id = typed(tree, pile, c.name, "foe");
    let [might, vitality, grit, cadence, finesse] = c.stats;
    tree.set_card_detail(
        id,
        vec![
            format!(
                "{} | {}",
                catalog::creature_intention(c),
                catalog::creature_posture(c)
            ),
            format!("Might {might} | Vitality {vitality} | Grit {grit}"),
            format!("Cadence {cadence} | Finesse {finesse}"),
            format!(
                "{}: {}",
                c.ability,
                catalog::creature_ability_description(c.ability)
            ),
        ],
    )
    .expect("foe card just added");
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

// The authored adventurer names (Name Bank § Adventurers) are gone: a hero *is* its kit now, so the party is
// named for the four kits in `catalog::ROSTER` (Raider / Marksman / Bastion / Bombardier). See the Inn removal.

/// The round's phases in order, each with a one-line mechanical summary (condensed from
/// `docs/games/deckbound/reference/combat-phases.md`, the canonical text). The Rules deck renders one
/// card per phase; in combat we will surface these and cycle which one is active.
const PHASES: [(&str, &str); 10] = [
    (
        "Marshal",
        "Secretly assign each unit an intention - Vanguard, Outrider or Rearguard - and maybe bind a group. Re-declared each round.",
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
        "The back fires on the flankers: each Rearguard shoots an enemy Outrider - the pre-empt, before it arrives.",
    ),
    (
        "Raid",
        "Surviving Outriders strike the enemy Rearguard they crossed for - the breaker lands on the exposed back.",
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
        "The boundary rule of every combat phase above, not a step of its own: as each phase ends its damage pile clears - sub-Grit damage that didn't flip a Health card does not carry to the next phase. Only Health persists; there is no separate end-of-round wipe.",
    ),
    (
        "Refresh",
        "Round end (the Reset): spent Tempo resets, Health carries over, the round advances. Five undecided rounds is a draw.",
    ),
];

/// The one-line mechanical summary for a phase name (from [`PHASES`]), or `""` if unknown.
fn phase_detail(name: &str) -> &'static str {
    PHASES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|&(_, detail)| detail)
        .unwrap_or_default()
}

/// Lay a **Free** deck's content out in a tidy grid so the very first render is clean. A freely-placed
/// zone shares the felt with the floating overlays (title / Back), so the seed **leaves the top row
/// empty** — content starts one row down, clear of them on first render — while that row stays felt: the
/// shove only keeps cards off the fixtures, not the whole row, so you can still place cards up there.
/// Children are laid out row-major across `cols` in **child order** — leaf cards and sub-piles
/// interleaved — so a sub-deck keeps its slot (e.g. the Rules deck's Engage `(4/6)` stays fourth instead
/// of being pushed past the leaf cards). Saved tables restore their own positions, so this only shapes a
/// fresh table.
fn grid_layout(tree: &mut Board, deck: PileId, cols: usize) {
    // Kept in step with the renderer's spacing (cardtable `GAP` / `CARD_W` / `CARD_H` / `OVERLAY_BAND`) so
    // a freshly-seeded Free deck already sits at the exact constant-gap spacing the renderer would compute
    // — the cards start non-overlapping, so the overlap-shove never fires and never distorts them.
    const GAP: f32 = 12.0;
    const CARD_W: f32 = 124.0; // rendered Small card width  (SMALL_W + 2px border each side)
    const CARD_H: f32 = 100.0; // rendered Small card height (SMALL_H + 2px border each side)
    const OVERLAY_BAND: f32 = 52.0;
    const LEFT: f32 = GAP; // one standard gap in from the left edge
    const TOP: f32 = OVERLAY_BAND + GAP; // clears the overlay band (title / Back) by one standard gap
    // One cell = a rendered card plus one gap, so horizontal and vertical spacing between cards match.
    let spot = |i: usize| {
        let (col, row) = (i % cols, i / cols);
        (
            LEFT + col as f32 * (CARD_W + GAP),
            TOP + row as f32 * (CARD_H + GAP),
        )
    };
    for (i, node) in tree.movable_children(deck).into_iter().enumerate() {
        let (x, y) = spot(i);
        match node {
            Node::Card(c) => {
                let _ = tree.set_card_pos(c, x as i32, y as i32);
            }
            Node::Pile(p) => {
                let _ = tree.set_pile_pos(p, x as i32, y as i32);
            }
        }
    }
}

/// A small, representative table for the card-table game: a **Heroes** deck (the reserve the party's cards
/// are dealt from), the **Abilities** / **Stats** / **Numbers** banks, and a **Locations** grid. The party
/// of four — one hero per kit — starts **already assembled** and stationed at **Ashfen Crossing** (the
/// centre). Every card is a physical, single-homed card.
///
/// There is no Inn and no recruiting: a hero *is* its kit. See the Inn-removal commit if that concept is
/// wanted back — the Inn was a `Rows` **projection** of the Heroes and Kit decks at Ashfen Crossing, where
/// you dragged a hero identity onto a kit card to assemble a character deck from the banks.
pub fn sample_table() -> Board {
    let mut tree = Board::new();
    let root = tree.root_id();
    // The table itself is a **Free** layout: its top-level decks are placed by position (auto-tidied into a
    // row by the renderer's `settle_table_piles`, draggable in between), not a structured grid. Without
    // this the root would default to `List` and the renderer would try to flow its decks — sweeping the
    // System deck out of its parked corner.
    tree.set_layout(
        root,
        Layout {
            arrangement: Arrangement::Free,
            editable: true,
        },
    )
    .expect("root exists");

    // The "Heroes" deck: the party's **reserve**, one `hero` card per kit (`catalog::ROSTER`) **stacked ×4** —
    // the four physical copies a hero needs at once when active (character-deck label, rank marker, map
    // position, move marker). The kit *is* the hero, so each card carries that kit's stat line, ability, and
    // recipe. The party is assembled from this deck at the bottom of this function, which deals all four
    // copies out and empties every stack — so the Heroes deck starts the game empty, and is the home the
    // cards return to if a character is un-equipped.
    const HERO_COPIES: u32 = 4;
    let heroes = tree.add_pile(root, "Heroes").expect("root exists");
    for &(name, stats, ability) in &catalog::ROSTER {
        let h = hero(&mut tree, heroes, name, stats, ability);
        tree.set_card_quantity(h, HERO_COPIES).expect("hero stack");
    }
    let heroes_zone = typed(&mut tree, heroes, "Heroes", "Label");
    tree.set_card_kind(heroes_zone, CardKind::Zone)
        .expect("heroes zone card");
    tree.set_layout(
        heroes,
        Layout {
            arrangement: Arrangement::Free,
            editable: true,
        },
    )
    .expect("heroes exists");

    // Provisioned bank sizes (spec `canon/2-spec/physical-cards.md` PC.5 — sufficient, bounded, tunable):
    // one of each stat name / ability per party member, and enough number cards for their stat values.
    const STAT_COPIES: usize = 5; // per stat name (one per party member)
    const ABILITY_COPIES: usize = 5; // per ability (one per party member)
    const NUMBER_COPIES: usize = 12; // per digit 1-9 (a character spends ~5 number cards)

    // The "Abilities" **bank**: one `×N` stack per ability (PC.2 — a run of duplicates is one card with a
    // quantity), split one off into a character deck on equip and merged back on un-equip.
    let abilities = tree.add_pile(root, "Abilities").expect("root exists");
    for (name, description) in catalog::ABILITIES {
        let id = typed(&mut tree, abilities, name, "ability");
        tree.set_card_detail(id, vec![description.to_string()])
            .expect("ability card just added");
        tree.set_card_quantity(id, ABILITY_COPIES as u32)
            .expect("ability stack");
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

    // The "Stats" **bank**: one `×N` stack per stat *name*. A character spells a stat as a **name card
    // then a number card** (e.g. `Might` then `6`), one of each split from the banks on equip (PC.2/PC.5).
    let stats = tree.add_pile(root, "Stats").expect("root exists");
    for (name, description) in catalog::STATS {
        let id = typed(&mut tree, stats, name, "stat");
        tree.set_card_detail(id, vec![description.to_string()])
            .expect("stat card just added");
        tree.set_card_quantity(id, STAT_COPIES as u32)
            .expect("stat stack");
    }
    let stats_zone = typed(&mut tree, stats, "Stats", "Label");
    tree.set_card_kind(stats_zone, CardKind::Zone)
        .expect("stats zone card");
    tree.set_layout(
        stats,
        Layout {
            arrangement: Arrangement::Free,
            editable: true,
        },
    )
    .expect("stats exists");

    // The "Numbers" **bank**: one `×N` stack per digit 1-9 (PC.5 — numbers are 0-9). A character's stat
    // value is a number card placed **after** its stat-name card; a `6` is interchangeable across stats.
    let numbers = tree.add_pile(root, "Numbers").expect("root exists");
    for d in 1..=9 {
        let id = typed(&mut tree, numbers, &d.to_string(), "number");
        tree.set_card_quantity(id, NUMBER_COPIES as u32)
            .expect("number stack");
    }
    let numbers_zone = typed(&mut tree, numbers, "Numbers", "Label");
    tree.set_card_kind(numbers_zone, CardKind::Zone)
        .expect("numbers zone card");
    tree.set_layout(
        numbers,
        Layout {
            arrangement: Arrangement::Free,
            editable: true,
        },
    )
    .expect("numbers exists");

    // The "Locations" deck: a fixed 3×3 grid (2-D, non-editable) of place-piles from the Name Bank,
    // each labelled by its Location-typed Zone card. **Ashfen Crossing** (the centre) is the party's home:
    // the four heroes are stationed there at the bottom of this function - AND it stations the capstone
    // (The Last Stand at Ashfen), like every other cell stations its encounter: the graduation exam is
    // simply there, at your doorstep, fought when you choose to. (Ashfen once held the **Inn**, a
    // projection you recruited in; that concept was removed, see the Inn-removal commit.)
    let locations = tree.add_pile(root, "Locations").expect("root exists");
    for place in LOCATIONS {
        let place_pile = tree.add_pile(locations, place).expect("locations exists");
        let name = typed(&mut tree, place_pile, place, "Location");
        tree.set_card_kind(name, CardKind::Zone)
            .expect("place name card");
        if let Some(enc) = catalog::encounter_for(place) {
            // Every location stations its **encounter** card. Its foes are **virtual** — the
            // card *lists* them (name ×qty); the real foe cards live in the Bestiary and are only instantiated
            // into the battle arena when a fight starts. A solo (a home-adjacent cell) fields its one
            // keystone creature; a corner fields all four with the keystone doubled.
            let header = typed(&mut tree, place_pile, enc.title, "encounter");
            let mut detail = vec![enc.flavor.to_string()];
            let foes: Vec<String> = catalog::encounter_foes(enc)
                .iter()
                .map(|(c, q)| {
                    if *q > 1 {
                        format!("{} x{q}", c.name)
                    } else {
                        c.name.to_string()
                    }
                })
                .collect();
            detail.push(format!("Foes: {}", foes.join(", ")));
            tree.set_card_detail(header, detail)
                .expect("encounter detail");
            // A Rumors card next to the header — app-only (a `Virtual` readout, not counted in the physical
            // tally) — spelling out how to beat this encounter, derived from its foes so it stays in step.
            let rumor = typed(&mut tree, place_pile, "Rumors", "rumors");
            tree.set_card_kind(rumor, CardKind::Virtual)
                .expect("rumor card exists");
            tree.set_card_detail(rumor, catalog::encounter_rumor(enc))
                .expect("rumor detail");
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
            // Engage's label is the parent card: name + its `(x/y)`. Its children are one drill-in away
            // and its damage-order summary is in the detail, so the title stays short.
            let label = format!("Engage {pos}");
            let engage_zone = typed(&mut tree, engage, &label, "phase");
            tree.set_card_kind(engage_zone, CardKind::Zone)
                .expect("engage label");
            tree.set_card_detail(
                engage_zone,
                vec![
                    "Intercept - Vanguard -> Outrider".into(),
                    "Volley - Rearguard -> Outrider".into(),
                    "Raid - Outrider -> Rearguard".into(),
                    "Clash - Rearguard / Vanguard -> Vanguard".into(),
                    "Breach - the trailing blows land".into(),
                    "Each combat phase banks its own damage pile and wipes it at that boundary: sub-Grit damage does not carry to the next.".into(),
                ],
            )
            .expect("engage detail");
            grid_layout(&mut tree, engage, 3);
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
    grid_layout(&mut tree, rules, 3);
    tree.set_layout(
        rules,
        Layout {
            arrangement: Arrangement::Free,
            editable: true,
        },
    )
    .expect("rules exists");

    // The physical **day clock** (spec `canon/2-spec/physical-cards.md` PC.5), consolidated onto **two**
    // decks (the reserve is now the Heroes deck, PC.2):
    // - **Progress** — the day clock proper. It starts **empty** (Day 0); each `advance_day` lays one
    //   `Day Passed` count card here, so its `event`-card count *is* the current day (no number cap). Once
    //   heroes are recruited it also holds one face-up `hero` **move marker** per active character (face-up
    //   = hasn't moved today; a move flips it down, `Board::mark_moved`). Count (type `event`) and markers
    //   (type `hero`) are told apart by type.
    // - **Events** — the bounded reserve of `Day Passed` cards `advance_day` draws from each time every
    //   marker has flipped down. Its size is the provisioned max game length (raise as needed).
    const DAYS_PROVISIONED: usize = 12;
    let free = |tree: &mut Board, pile: PileId| {
        tree.set_layout(
            pile,
            Layout {
                arrangement: Arrangement::Free,
                editable: true,
            },
        )
        .expect("pile exists");
    };

    let progress = tree.add_pile(root, "Progress").expect("root exists");
    let progress_zone = typed(&mut tree, progress, "Progress", "Label");
    tree.set_card_kind(progress_zone, CardKind::Zone)
        .expect("progress zone card");
    // A **structured** (non-editable Grid) day clock: the count card + move markers tile below the overlay
    // band automatically, so a card dealt at runtime (advance_day's Day Passed, a recruit's move marker)
    // never lands at the default (0,0) under the Back button. Status cards, so not draggable.
    tree.set_layout(
        progress,
        Layout {
            arrangement: Arrangement::Grid { columns: 5 },
            editable: false,
        },
    )
    .expect("progress exists");

    let events = tree.add_pile(root, "Events").expect("root exists");
    let events_stack = typed(&mut tree, events, "Day Passed", "event"); // one `Day Passed xN` stack (PC.2)
    tree.set_card_quantity(events_stack, DAYS_PROVISIONED as u32)
        .expect("events stack");
    let events_zone = typed(&mut tree, events, "Events", "Label");
    tree.set_card_kind(events_zone, CardKind::Zone)
        .expect("events zone card");
    free(&mut tree, events);

    // The **Bestiary**: the foes' home deck — one `foe` card per creature type, stacked `×N` (the
    // provisioned supply of instances a battle can field; an encounter *lists* which and how many, and the
    // arena deals them from here). A location holds only its encounter card, not physical foes.
    const FOE_COPIES: u32 = 4;
    let bestiary = tree.add_pile(root, "Bestiary").expect("root exists");
    for c in &catalog::CREATURES {
        let f = creature_card(&mut tree, bestiary, c);
        tree.set_card_quantity(f, FOE_COPIES).expect("foe stack");
    }
    let bestiary_zone = typed(&mut tree, bestiary, "Bestiary", "Label");
    tree.set_card_kind(bestiary_zone, CardKind::Zone)
        .expect("bestiary zone card");
    free(&mut tree, bestiary);

    // Lay each Free deck's cards out tidily below the overlay band, so the first render of a zone is
    // clean — the Back card sits in its own row up top with the cards beneath it, no shove required yet.
    grid_layout(&mut tree, heroes, 4);
    grid_layout(&mut tree, abilities, 4);
    grid_layout(&mut tree, stats, 4);
    grid_layout(&mut tree, numbers, 4);
    grid_layout(&mut tree, events, 4);
    grid_layout(&mut tree, bestiary, 4);

    // Seed the top-level piles un-stacked so the very first frame is sane. Their real positions are an
    // exact constant-gap row computed by `Board::arrange_row` once the chips are sized (see the
    // renderer's `settle_table_piles`); these seeds only need to be non-overlapping until then.
    tree.set_pile_pos(heroes, 40, 40).expect("heroes exists");
    tree.set_pile_pos(abilities, 320, 40)
        .expect("abilities exists");
    tree.set_pile_pos(stats, 460, 40).expect("stats exists");
    tree.set_pile_pos(numbers, 460, 200)
        .expect("numbers exists");
    tree.set_pile_pos(locations, 600, 40)
        .expect("locations exists");
    tree.set_pile_pos(rules, 740, 40).expect("rules exists");
    tree.set_pile_pos(progress, 1020, 40)
        .expect("progress exists");
    tree.set_pile_pos(events, 1160, 40).expect("events exists");
    tree.set_pile_pos(bestiary, 1440, 40)
        .expect("bestiary exists");

    // **The starting party.** A hero *is* its kit, so the four kits in `catalog::ROSTER` are the four heroes —
    // and they begin already assembled, rather than being recruited at an inn. Each is dealt out of the Heroes
    // reserve into its own character deck (a stat-name card + a number card per stat, then the ability, all
    // *moved* from the banks — conservation-clean, PC.2) and stationed at Ashfen Crossing, the home cell.
    // Dealing all four copies of each hero empties the Heroes deck, which is then just the home those cards
    // return to on un-equip.
    let ashfen = tree
        .pile(locations)
        .expect("locations exists")
        .subpiles()
        .into_iter()
        .find(|&p| tree.pile(p).map(|d| d.label.as_str()) == Some("Ashfen Crossing"))
        .expect("Ashfen Crossing is one of the LOCATIONS");
    for &(name, stat_values, ability) in &catalog::ROSTER {
        let recipe = Recipe {
            stats: stat_values,
            ability: ability.into(),
        };
        tree.equip_character(
            name,
            &recipe,
            &catalog::stat_names(),
            heroes,
            stats,
            numbers,
            abilities,
            ashfen,
            progress,
        )
        .expect("the banks provision the whole starting party");
    }

    tree
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_table_is_well_formed() {
        let t = sample_table();
        let root = t.pile(t.root_id()).unwrap();
        // The nine decks — Heroes, the banks (Abilities, Stats, Numbers), Locations, Rules, the day clock
        // (Progress, Events), the Bestiary — plus the four character decks the starting party is assembled
        // into (one per kit). There is no Kit deck: a hero *is* its kit.
        assert_eq!(root.subpiles().len(), 9 + catalog::ROSTER.len());
        // The grand total is the **provisioned** total: assembling the party only *moves* cards out of the
        // Heroes reserve and the banks into the character decks (PC.2, no mint), so it doesn't change.
        //   Heroes    4 kits x4 copies + a Zone label   (all 16 copies get dealt into the party)
        //   Abilities 4 abilities x5 copies + a label
        //   Stats     5 names x5 copies + a label
        //   Numbers   9 digits x12 copies + a label
        //   Locations a Zone card + 9 place names + 9 encounter headers + 9 Rumors (app-only readouts).
        //             The party's 4 map positions come out of Heroes, so they're already counted there.
        //   Rules     5 leaf phases + a label, and the Engage sub-deck's 5 + a label
        //   Progress  a Zone label (Day 0; the 4 move markers also come out of Heroes)
        //   Events    the Day Passed x12 reserve + a label
        //   Bestiary  6 creature `foe` stacks x4 + a label
        assert_eq!(
            t.card_count(),
            (4 * 4 + 1)
                + (4 * 5 + 1)
                + (5 * 5 + 1)
                + (9 * 12 + 1)
                + (1 + 9 + 9 + 9)
                + ((5 + 1) + (5 + 1))
                + 1
                + (12 + 1)
                + (6 * 4 + 1)
        );
    }

    /// The consolidated day clock lives on **Progress** (PC.5): a `Day Passed` count reading Day 1 plus
    /// per-hero move markers, with an Events reserve to draw from. Driving it — add a marker, spend its
    /// move, advance — grows the day and conserves the total card count (PC.2).
    #[test]
    fn day_clock_is_provisioned_and_advances_on_the_table() {
        let mut t = sample_table();
        let (progress, events) = (deck(&t, "Progress"), deck(&t, "Events"));

        // Day 0 (Progress starts empty of count cards), the full Events reserve of 12, no markers yet.
        let events_qty = |t: &Board| -> u32 {
            t.content_cards(events)
                .iter()
                .map(|&c| t.card(c).unwrap().quantity())
                .sum()
        };
        assert_eq!(t.current_day(progress), 0);
        assert_eq!(events_qty(&t), 12);
        // The starting party laid a face-up move marker each — nobody has moved, so the day isn't over.
        assert!(!t.day_is_over(progress), "every marker is still standing");

        // Spend every hero's move (each marker flips face-down); only then is the day over.
        let total = t.card_count();
        let markers: Vec<String> = catalog::ROSTER
            .iter()
            .map(|&(n, _, _)| n.to_string())
            .collect();
        for (i, name) in markers.iter().enumerate() {
            t.mark_moved(progress, name).unwrap();
            let last = i + 1 == markers.len();
            assert_eq!(
                t.day_is_over(progress),
                last,
                "the day is over only once the last hero has moved ({name})"
            );
        }

        // Advance: the markers stand back up, one `Day Passed` moves Events -> Progress, day ticks to 1.
        t.advance_day(progress, events).unwrap();
        assert_eq!(t.current_day(progress), 1);
        assert_eq!(events_qty(&t), 11, "one drawn from the reserve stack");
        assert!(!t.day_is_over(progress), "every marker stood back up");
        assert_eq!(
            t.card_count(),
            total,
            "conservation: play (move + advance) minted nothing"
        );
    }

    /// The zone-title tally (`physical_card_count`) counts every card physically there — including each
    /// deck's own title card — exactly once, so the whole table sums to the game's true total.
    #[test]
    fn physical_card_count_sums_to_the_game_total() {
        let t = sample_table();
        // The headline invariant: the recursive tally of the whole table equals `card_count` minus the
        // software-only cards (the 9 app-only Rumors readouts, one per encounter), so adding up the deck
        // chips on the table screen gives the real number of physical cards.
        assert_eq!(t.physical_card_count(t.root_id()), t.card_count() - 9);
        // Inclusive of each deck's own title card, and stacks count by quantity. The party was dealt out of
        // Heroes, emptying it — all that is left is the "Heroes" label.
        assert_eq!(t.physical_card_count(deck(&t, "Heroes")), 1);
        // Events is a `Day Passed ×12` stack + the "Events" label.
        assert_eq!(t.physical_card_count(deck(&t, "Events")), 12 + 1);
        // Numbers was provisioned 9 digits ×12, less the 5 each hero spent on its stat values.
        assert_eq!(
            t.physical_card_count(deck(&t, "Numbers")),
            (9 * 12 + 1) - 5 * catalog::ROSTER.len()
        );
        // A place counts its "Location" title + its encounter header. Foes are virtual (listed on the
        // header, not dealt as cards), so a corner and a solo count the same. Index 0 (The Hollow Rampart)
        // is a corner: 1 title + 1 header = 2.
        let locations = deck(&t, "Locations");
        let a_corner = t.pile(locations).unwrap().subpiles()[0];
        assert_eq!(t.physical_card_count(a_corner), 1 + 1);
        // Index 1 (Cinderwatch Keep) is a solo: 1 title + 1 header = 2.
        let a_solo = t.pile(locations).unwrap().subpiles()[1];
        assert_eq!(t.physical_card_count(a_solo), 1 + 1);
        // The home cell (Ashfen, index 4) stations the capstone like any other cell - its title, the
        // encounter header, and the party standing there, one map-position card each.
        let ashfen = t.pile(locations).unwrap().subpiles()[4];
        assert_eq!(
            t.physical_card_count(ashfen),
            1 + 1 + catalog::ROSTER.len(),
            "the Location title, the Last Stand header, and the four heroes at home"
        );
    }

    /// A software-only deck — one holding [`Utility`] action cards, like the renderer's System deck —
    /// counts as zero physical cards, its label included: those are app controls, not tabletop cards.
    #[test]
    fn physical_card_count_skips_software_only_decks() {
        let mut t = Board::new();
        let root = t.root_id();
        let system = t.add_pile(root, "System").unwrap();
        let start = t
            .add_card(
                system,
                Face::Up {
                    title: "Start Over".into(),
                },
                None,
            )
            .unwrap();
        t.set_card_kind(
            start,
            CardKind::Utility(cardtable_model::Utility::StartOver),
        )
        .unwrap();
        let label = typed(&mut t, system, "System", "Label");
        t.set_card_kind(label, CardKind::Zone).unwrap();
        assert_eq!(t.physical_card_count(system), 0);
        // And a physical deck sitting beside it still counts its own label + content.
        let day = t.add_pile(root, "Day").unwrap();
        let zone = typed(&mut t, day, "Day", "Label");
        t.set_card_kind(zone, CardKind::Zone).unwrap();
        assert_eq!(
            t.physical_card_count(day),
            1,
            "empty physical deck = its label"
        );
    }

    /// A [`CardKind::Virtual`] card (a combat log) is a software readout — it sits in a physical pile but
    /// is not counted, so a location's tally reflects only its real tabletop cards.
    #[test]
    fn physical_card_count_skips_virtual_cards() {
        let mut t = Board::new();
        let root = t.root_id();
        let place = t.add_pile(root, "Place").unwrap();
        let zone = typed(&mut t, place, "Place", "Location");
        t.set_card_kind(zone, CardKind::Zone).unwrap();
        typed(&mut t, place, "A Foe", "foe"); // one physical card
        let log = typed(&mut t, place, "Victory", "log");
        t.set_card_kind(log, CardKind::Virtual).unwrap();
        // The label + the foe count; the virtual log does not.
        assert_eq!(t.physical_card_count(place), 1 + 1);
    }

    /// The creature read-outs are *derived*, and the derivation reproduces each creature's intended
    /// position (the `default_intentions` rule) exactly — so editing a stat re-derives the stance.
    #[test]
    fn creature_intention_reproduces_the_roster_positions() {
        let intent = |name: &str| catalog::creature_intention(catalog::creature(name).unwrap());
        assert_eq!(intent("The Wall"), "Vanguard"); // M1 < G6
        assert_eq!(intent("The Duelist"), "Vanguard"); // authored pos override
        assert_eq!(intent("The Swarm"), "Vanguard"); // authored pos override (melee front horde)
        assert_eq!(intent("The Brood"), "Rearguard"); // ranged
        assert_eq!(intent("The Sniper"), "Rearguard"); // ranged
        assert_eq!(intent("The Reaver"), "Vanguard"); // authored pos override
        // Each keystone's posture points at exactly one answering kit — the clean diagonal. (The Sniper and
        // the Reaver are corner threats, not solos: no lone counter kit.)
        let counter = |name: &str| catalog::creature_counter(catalog::creature(name).unwrap());
        assert_eq!(counter("The Wall"), "Raider");
        assert_eq!(counter("The Duelist"), "Marksman");
        assert_eq!(counter("The Swarm"), "Bombardier");
        assert_eq!(counter("The Brood"), "Bastion");
        assert_eq!(counter("The Sniper"), "");
        assert_eq!(counter("The Reaver"), "");
    }

    /// Each non-inn place stations its encounter as a single **header** card — no physical foe cards. The
    /// foes are *virtual*: the header lists them (name ×qty), and the real supply lives, stacked, in the
    /// Bestiary. A solo header lists its one keystone; a corner header lists all four, keystone doubled.
    #[test]
    fn encounters_are_virtual_headers_backed_by_the_bestiary() {
        let t = sample_table();
        let locations = deck(&t, "Locations");
        let place = |name: &str| {
            *t.pile(locations)
                .unwrap()
                .subpiles()
                .iter()
                .find(|&&p| t.pile(p).unwrap().label == name)
                .unwrap()
        };
        // No physical foe cards anywhere on the table — the foes are virtual.
        let foe_cards = |p: PileId| -> usize {
            t.content_cards(p)
                .iter()
                .filter(|&&c| t.card(c).unwrap().card_type() == "foe")
                .count()
        };
        assert_eq!(foe_cards(place("The Sundered Vault")), 0);
        assert_eq!(foe_cards(place("Emberfall Hollow")), 0);
        // A place holds exactly one `encounter` header, listing its foes in its detail lines.
        let header = |p: PileId| {
            t.content_cards(p)
                .into_iter()
                .find(|&c| t.card(c).unwrap().card_type() == "encounter")
                .expect("an encounter header")
        };
        let solo = header(place("The Sundered Vault"));
        let solo_detail = t.card(solo).unwrap().detail().join(" ");
        assert!(
            solo_detail.contains("The Wall"),
            "the solo header lists its keystone: {solo_detail}"
        );
        let corner = header(place("Emberfall Hollow"));
        let corner_detail = t.card(corner).unwrap().detail().join(" ");
        assert!(
            corner_detail.contains("The Wall x2"),
            "the corner header lists the doubled keystone: {corner_detail}"
        );
        // The Bestiary backs them with a `×4` stack per creature type (+ its Zone label).
        assert_eq!(
            t.physical_card_count(deck(&t, "Bestiary")),
            catalog::CREATURES.len() * 4 + 1
        );
    }

    /// Each encounter location stations an **app-only** Rumors card (a `Virtual` readout, not counted in the
    /// physical tally) that spells out the strategy - a solo names its one answering kit, a corner tells you
    /// to bring the full party.
    #[test]
    fn every_encounter_stations_an_app_only_rumors_card() {
        let t = sample_table();
        let locations = deck(&t, "Locations");
        let place = |name: &str| {
            *t.pile(locations)
                .unwrap()
                .subpiles()
                .iter()
                .find(|&&p| t.pile(p).unwrap().label == name)
                .unwrap()
        };
        let rumor = |loc: &str| {
            t.content_cards(place(loc))
                .into_iter()
                .find(|&c| t.card(c).unwrap().card_type() == "rumors")
                .expect("a Rumors card")
        };
        // A solo (The Sundered Vault -> The Wall) names its counter kit, and is app-only (not physical).
        let solo = rumor("The Sundered Vault");
        assert!(!t.card(solo).unwrap().is_physical(), "Rumors is app-only");
        let solo_text = t.card(solo).unwrap().detail().join(" ");
        assert!(
            solo_text.contains("Raider"),
            "the Wall's rumor names its counter: {solo_text}"
        );
        // A corner (Emberfall Hollow) tells you to bring the full party.
        let corner_text = t
            .card(rumor("Emberfall Hollow"))
            .unwrap()
            .detail()
            .join(" ");
        assert!(
            corner_text.contains("full party"),
            "the corner's rumor calls for the party: {corner_text}"
        );
        // The home cell stations the capstone, so it carries a Rumors card like every other encounter.
        assert!(
            t.content_cards(place("Ashfen Crossing"))
                .into_iter()
                .any(|c| t.card(c).unwrap().card_type() == "rumors"),
            "the Last Stand has its rumor"
        );
    }

    /// Combat instantiates the virtual foes as **real cards** split off the Bestiary stacks, and returns
    /// them afterward — conservation-clean both ways (PC.2). A corner fields its tuned foe list; a solo its
    /// one keystone; the capstone its whole combined-arms warband.
    #[test]
    fn manual_combat_instantiates_foes_from_the_bestiary_and_returns_them() {
        let mut t = sample_table();
        let bestiary = deck(&t, "Bestiary");
        let arena = t.add_pile(t.root_id(), "Arena").unwrap();
        let total = t.card_count();
        let bestiary_before = t.physical_card_count(bestiary);

        // The Emberfall Hollow corner fields The Wall x2 (its tuned composition) → 2 real cards.
        let foes = t
            .instantiate_from_bank(
                bestiary,
                arena,
                &deckbound_content::catalog::encounter_roster("Emberfall Hollow"),
            )
            .unwrap();
        assert_eq!(
            foes.len(),
            2,
            "corner = its tuned foe list (Emberfall = Wall x2)"
        );
        assert_eq!(
            t.content_cards(arena).len(),
            2,
            "real foe cards now in the arena"
        );
        assert_eq!(
            t.physical_card_count(bestiary),
            bestiary_before - 2,
            "the Bestiary supply dropped by exactly the two drawn"
        );
        assert_eq!(
            t.card_count(),
            total,
            "instantiation split, minted nothing (PC.2)"
        );
        let walls = t
            .content_cards(arena)
            .iter()
            .filter(|&&c| t.card(c).unwrap().name() == "The Wall")
            .count();
        assert_eq!(walls, 2, "the doubled keystone fielded two");

        // Return them: the Bestiary is made whole (merged back to `×4`), count conserved.
        t.return_foes_to_bestiary(&foes, bestiary).unwrap();
        assert!(t.content_cards(arena).is_empty(), "all foes returned");
        assert_eq!(
            t.physical_card_count(bestiary),
            bestiary_before,
            "Bestiary made whole"
        );
        assert_eq!(t.card_count(), total, "return merged, conserved (PC.2)");

        // A solo fields just its keystone; the inn (no encounter) yields nothing.
        assert_eq!(
            t.instantiate_from_bank(
                bestiary,
                arena,
                &deckbound_content::catalog::encounter_roster("The Sundered Vault")
            )
            .unwrap()
            .len(),
            1,
            "solo = one keystone"
        );
        // Ashfen Crossing — the old inn — now stations the capstone: Swarm x1 + Wall x2 + Sniper x1.
        assert_eq!(
            t.instantiate_from_bank(
                bestiary,
                arena,
                &deckbound_content::catalog::encounter_roster("Ashfen Crossing")
            )
            .unwrap()
            .len(),
            4,
            "the capstone fields its whole warband"
        );
    }

    /// Find a top-level deck by label (test helper).
    fn deck(t: &Board, label: &str) -> PileId {
        *t.pile(t.root_id())
            .unwrap()
            .subpiles()
            .iter()
            .find(|&&id| t.pile(id).unwrap().label == label)
            .unwrap()
    }

    /// The party starts **already assembled** (a hero *is* its kit), so there is nothing to recruit: this
    /// just finds the character deck of the kit at `catalog::ROSTER[i]`, and its name.
    fn party_member(t: &Board, i: usize) -> (PileId, String) {
        let name = catalog::ROSTER[i].0.to_string();
        (deck(t, &name), name)
    }

    /// `character_recipe` reads a hero's build back out of its deck cards — the stats and ability
    /// round-trip, so combat can recover `[Might, Vitality, Grit, Cadence, Finesse]`.
    #[test]
    fn character_recipe_round_trips_a_recruited_build() {
        let t = sample_table();
        let (cdeck, _name) = party_member(&t, 0);
        let (_, stats, ability) = catalog::ROSTER[0];
        let recovered = t
            .character_recipe(cdeck, &deckbound_content::catalog::stat_names())
            .expect("a complete build");
        assert_eq!(recovered.stats, stats, "the kit's stats, spelled in cards");
        assert_eq!(recovered.ability, ability);
        // A deck that is not a character build yields nothing.
        assert_eq!(
            t.character_recipe(
                deck(&t, "Heroes"),
                &deckbound_content::catalog::stat_names()
            ),
            None
        );
    }

    /// Each generic attack's strike shape `(ranged, aoe)` — the four (reach x spread) combinations.
    #[test]
    fn ability_shape_covers_the_attacks() {
        assert_eq!(catalog::ability_shape("Jab"), (false, false)); // melee single
        assert_eq!(catalog::ability_shape("Shot"), (true, false)); // ranged single
        assert_eq!(catalog::ability_shape("Sweep"), (false, true)); // melee area
        assert_eq!(catalog::ability_shape("Salvo"), (true, true)); // ranged area
    }

    /// Each generic attack's reach `(melee, ranged)` — Jab/Sweep are melee, Shot/Salvo ranged; an unknown
    /// name defaults to melee. (The reach model permits both / neither; a single attack card is one reach.)
    #[test]
    fn ability_reach_covers_the_attacks() {
        assert_eq!(catalog::ability_reach("Jab"), (true, false)); // melee
        assert_eq!(catalog::ability_reach("Sweep"), (true, false)); // melee
        assert_eq!(catalog::ability_reach("Shot"), (false, true)); // ranged
        assert_eq!(catalog::ability_reach("Salvo"), (false, true)); // ranged
        assert_eq!(catalog::ability_reach("(unknown)"), (true, false)); // default melee
    }

    /// Assembling the party dealt each hero's **four** copies out of its `×4` Heroes stack — two to the
    /// character deck (Zone label + rank marker), one to the home cell (map position), one onto Progress
    /// (move marker) — emptying the reserve. Un-equipping returns all four, re-forming the `×4`. All
    /// conservation-clean (PC.2): the total card count is unchanged across the round-trip.
    #[test]
    fn the_party_was_dealt_four_copies_each_and_un_equipping_returns_them() {
        let mut t = sample_table();
        let (heroes, progress) = (deck(&t, "Heroes"), deck(&t, "Progress"));
        let ashfen = t.pile(deck(&t, "Locations")).unwrap().subpiles()[4];
        let total = t.card_count();
        let (cdeck, name) = party_member(&t, 0);

        let copies_in = |t: &Board, pile: PileId| -> usize {
            t.content_cards(pile)
                .iter()
                .filter(|&&c| {
                    let k = t.card(c).unwrap();
                    k.card_type() == "hero" && k.front_title() == name
                })
                .count()
        };

        // Four copies dealt: the Zone label (its own reflection), a rank marker in the deck's content, a
        // position copy at the home cell, a move marker on Progress. The Heroes reserve is emptied.
        assert_eq!(
            t.zone_card(cdeck)
                .and_then(|c| t.card(c))
                .map(|c| c.front_title()),
            Some(name.as_str())
        );
        assert_eq!(
            copies_in(&t, cdeck),
            1,
            "the rank marker in the character deck"
        );
        assert_eq!(
            copies_in(&t, ashfen),
            1,
            "the position copy at the home cell"
        );
        assert_eq!(copies_in(&t, progress), 1, "the move marker on Progress");
        assert_eq!(copies_in(&t, heroes), 0, "the Heroes stack was emptied");

        // Un-equip: the four copies return, re-forming the ×4 Heroes stack.
        let (stats, numbers, abilities) = (
            deck(&t, "Stats"),
            deck(&t, "Numbers"),
            deck(&t, "Abilities"),
        );
        t.unequip_character(cdeck, heroes, stats, numbers, abilities)
            .unwrap();
        let restacked = t
            .content_cards(heroes)
            .iter()
            .find(|&&c| {
                let k = t.card(c).unwrap();
                k.card_type() == "hero" && k.front_title() == name
            })
            .map(|&c| t.card(c).unwrap().quantity());
        assert_eq!(restacked, Some(4), "four copies merged back to x4");
        assert_eq!(t.card_count(), total, "conservation across the round-trip");
    }

    /// The movement loop (PC.5): the party is stationed at the home cell with a **move marker** each on
    /// Progress; moving a position copy to another location spends that character's day (`move_character`
    /// flips its marker), the day ends once the *last* hero has moved, and advancing ticks Day 0 → 1. All
    /// conservation-clean.
    #[test]
    fn moving_a_stationed_character_spends_its_day_and_can_advance() {
        let mut t = sample_table();
        let (progress, events, locations) = (
            deck(&t, "Progress"),
            deck(&t, "Events"),
            deck(&t, "Locations"),
        );
        let places = t.pile(locations).unwrap().subpiles();
        let (ashfen, thornmarch) = (places[4], places[5]); // centre = the home cell; a neighbour

        let (_cdeck, name) = party_member(&t, 0);

        // A hero copy stands at the home cell (map position); a move marker stands on Progress; Day 0.
        let named = |t: &Board, pile, n: &str| {
            t.content_cards(pile).into_iter().find(|&c| {
                let k = t.card(c).unwrap();
                k.front_title() == n && k.card_type() == "hero"
            })
        };
        assert!(named(&t, ashfen, &name).is_some(), "position copy at home");
        assert!(
            named(&t, progress, &name).is_some(),
            "move marker on Progress"
        );
        assert_eq!(t.current_day(progress), 0);

        // Move this character to a neighbouring location — it spends its day, but the rest of the party has
        // not moved, so the day is not over yet.
        let position = named(&t, ashfen, &name).unwrap();
        let total = t.card_count();
        let day_over = t.move_character(position, thornmarch, progress).unwrap();
        assert!(!day_over, "the rest of the party still has moves");
        assert!(
            named(&t, thornmarch, &name).is_some(),
            "stationed at the new location"
        );
        assert!(named(&t, ashfen, &name).is_none());

        // Spend every other hero's move — the last one ends the day.
        for &(other, _, _) in catalog::ROSTER.iter().skip(1) {
            t.mark_moved(progress, other).unwrap();
        }
        assert!(t.day_is_over(progress), "every hero has moved");

        // Advance: Day 0 -> 1, move markers stand back up, and cards are conserved through move + advance.
        t.advance_day(progress, events).unwrap();
        assert_eq!(t.current_day(progress), 1);
        assert!(!t.day_is_over(progress));
        assert_eq!(
            t.card_count(),
            total,
            "moving + advancing conserves cards (PC.2)"
        );
    }

    #[test]
    fn abilities_bank_holds_copies_of_every_ability() {
        let t = sample_table();
        let root = t.pile(t.root_id()).unwrap();
        let id = *root
            .subpiles()
            .iter()
            .find(|&&id| t.pile(id).unwrap().label == "Abilities")
            .unwrap();
        let abilities = t.pile(id).unwrap();

        // The bank holds one `×N` stack per catalog ability (drawn on equip). Every content card is an
        // ability, and each ability's stack carries a quantity > 1 (a real `×N`, PC.2).
        let cards = t.content_cards(abilities.id);
        for &cid in &cards {
            assert_eq!(t.card(cid).unwrap().card_type(), "ability");
        }
        for (name, _) in catalog::ABILITIES {
            let stack = cards
                .iter()
                .find(|&&c| t.card(c).unwrap().name() == name)
                .unwrap_or_else(|| panic!("the Abilities bank should hold {name}"));
            assert!(
                t.card(*stack).unwrap().quantity() > 1,
                "the {name} stack should be an xN (got {})",
                t.card(*stack).unwrap().quantity()
            );
        }
        let top = t.card(*abilities.cards().last().unwrap()).unwrap();
        assert_eq!(top.name(), "Abilities");
        assert_eq!(top.kind(), CardKind::Zone);
    }

    #[test]
    fn sample_table_round_trips_through_ron() {
        let t = sample_table();
        let text = ron::to_string(&t).expect("serialize to RON");
        let back: Board = ron::from_str(&text).expect("deserialize from RON");
        // Structure survives the round-trip (HashMaps keyed by integer ids and all card/pile data).
        assert_eq!(back.card_count(), t.card_count());
        let subs = |t: &Board| t.pile(t.root_id()).unwrap().subpiles().len();
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
            "Engage (4/6)"
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

    /// A hero **is** its kit: there is no Kit deck, so each hero card carries what a kit card used to — it is
    /// named for the kit, typed `hero`, and grows to that kit's stat line and ability. The four kits are
    /// `catalog::ROSTER` (Raider / Marksman / Bastion / Bombardier).
    #[test]
    fn each_hero_carries_its_kit_build() {
        let t = sample_table();
        assert_eq!(
            catalog::ROSTER.map(|(n, _, _)| n),
            ["Raider", "Marksman", "Bastion", "Bombardier"]
        );
        for &(name, stats, ability) in &catalog::ROSTER {
            // The hero's own card is its character deck's Zone label (dealt from the Heroes reserve).
            let label = t
                .zone_card(deck(&t, name))
                .and_then(|c| t.card(c))
                .unwrap_or_else(|| panic!("{name} has a Zone label"));
            assert_eq!(label.name(), name);
            assert_eq!(label.card_type(), "hero");
            let shows = |needle: &str| label.detail().iter().any(|l| l.contains(needle));

            // Its raw stats...
            let [might, vitality, grit, cadence, finesse] = stats;
            assert!(
                shows(&format!(
                    "Might {might} | Vitality {vitality} | Grit {grit}"
                )),
                "{name} shows its stat line: {:?}",
                label.detail()
            );
            assert!(shows(&format!("Cadence {cadence} | Finesse {finesse}")));
            assert!(shows(ability), "{name} names its ability");

            // ...and its **computed** stats: the two card pools its stats become in a fight.
            assert!(
                shows(&format!("Health {vitality} cards | Tempo {cadence} cards")),
                "{name} shows the card pools its stats become: {:?}",
                label.detail()
            );

            // But NOT a rank/intention indicator - that is obvious from the kit's name and, in a fight, from
            // the lane its tile sits in. (The derivation still exists; it just isn't printed on the card.)
            assert!(
                !shows(catalog::kit_intention(stats, ability)),
                "{name} must not print a rank indicator: {:?}",
                label.detail()
            );
        }
    }

    /// The kits' derived intentions — the roster is one Outrider, one Vanguard and two Rearguards, and the
    /// names say so (Raider raids, Bastion holds, Marksman and Bombardier shoot from the back).
    #[test]
    fn each_kit_is_shaped_for_a_position() {
        let by = |name: &str| {
            let &(_, stats, ability) = catalog::ROSTER
                .iter()
                .find(|&&(n, _, _)| n == name)
                .expect("a roster kit");
            catalog::kit_intention(stats, ability)
        };
        assert_eq!(by("Raider"), "Outrider", "Might 7 vs Grit 1: it raids");
        assert_eq!(by("Bastion"), "Vanguard", "Grit outweighs Might: it holds");
        assert_eq!(by("Marksman"), "Rearguard");
        assert_eq!(by("Bombardier"), "Rearguard");
    }

    /// The party starts **already assembled**: one character deck per kit (a hero *is* its kit), each
    /// stationed at Ashfen Crossing. There is no Inn and no Kit deck — dealing the party out empties the
    /// Heroes reserve, which is then only the home those cards return to on un-equip.
    #[test]
    fn the_party_starts_assembled_and_stationed_at_ashfen() {
        let t = sample_table();
        let root = t.pile(t.root_id()).unwrap();
        let find = |label: &str| {
            root.subpiles()
                .iter()
                .copied()
                .find(|&id| t.pile(id).unwrap().label == label)
        };

        // One character deck per kit, named for it, its Zone label being the hero's own card.
        for &(kit, _, _) in &catalog::ROSTER {
            let deck = find(kit).unwrap_or_else(|| panic!("a {kit} character deck was assembled"));
            assert!(
                t.pile(deck).unwrap().reflects().is_some(),
                "{kit}'s deck is labelled by its own hero card"
            );
        }

        // The Inn and the Kit deck are gone.
        assert!(
            find("Kit").is_none(),
            "the Kit deck was merged into the hero"
        );
        let locations = t.pile(find("Locations").unwrap()).unwrap();
        assert_eq!(locations.subpiles().len(), LOCATIONS.len());
        let ashfen = t.pile(locations.subpiles()[4]).unwrap();
        assert!(
            ashfen.subpiles().is_empty(),
            "Ashfen no longer holds an Inn"
        );

        // All four heroes stand at Ashfen Crossing — one map-position card each.
        let stationed: Vec<String> = t
            .content_cards(locations.subpiles()[4])
            .into_iter()
            .filter(|&c| t.card(c).unwrap().card_type() == "hero")
            .map(|c| t.card(c).unwrap().name().to_string())
            .collect();
        assert_eq!(stationed.len(), catalog::ROSTER.len());
        for &(kit, _, _) in &catalog::ROSTER {
            assert!(stationed.iter().any(|n| n == kit), "{kit} stands at Ashfen");
        }

        // Dealing the party out emptied the Heroes reserve (four copies each, all spent).
        assert!(
            t.content_cards(find("Heroes").unwrap()).is_empty(),
            "every hero copy was dealt into the party"
        );
    }

    /// The party is assembled into **real** character decks from the banks (a stat-name card then a number
    /// card per stat, then the ability, under the hero's own card as the deck's Zone label) — no reflection,
    /// no mint (PC.2). Un-equipping removes the deck and returns every card, conserving the total.
    #[test]
    fn equip_assembles_a_character_deck_from_the_banks() {
        let mut t = sample_table();
        let total = t.card_count();

        // The Raider: Might 6, Vitality 6, Grit 1, Cadence 2, Finesse 2, carrying Jab.
        let (cdeck, name) = party_member(&t, 0);

        // A top-level character deck, marked as reflecting the hero, spelling its stats as name+number.
        assert!(t.pile(cdeck).unwrap().reflects().is_some());
        let names: Vec<String> = t
            .content_cards(cdeck)
            .iter()
            .map(|&c| t.card(c).unwrap().name().to_string())
            .collect();
        assert_eq!(
            names,
            [
                "Might",
                "6",
                "Vitality",
                "6",
                "Grit",
                "1",
                "Cadence",
                "2",
                "Finesse",
                "2",
                "Jab",
                name.as_str() // the rank marker (a hero copy) sits with the character
            ]
        );
        assert_eq!(t.card(t.zone_card(cdeck).unwrap()).unwrap().name(), name);

        // Un-equip: the deck is gone and every card is back, total conserved.
        let (heroes, stats, numbers, abilities) = (
            deck(&t, "Heroes"),
            deck(&t, "Stats"),
            deck(&t, "Numbers"),
            deck(&t, "Abilities"),
        );
        t.unequip_character(cdeck, heroes, stats, numbers, abilities)
            .unwrap();
        assert!(t.pile(cdeck).is_none(), "character deck removed");
        assert_eq!(
            t.card_count(),
            total,
            "conservation across equip + un-equip"
        );
    }
}
