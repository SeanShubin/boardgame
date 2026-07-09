//! The **card-table world** as a [`contract::Game`] — the deckbound-side view emitter.
//!
//! This is the reunification's core (plan §14): instead of hand-building a `Tableau`, the product's
//! world is authored here as a `Game` whose [`view`](Game::view) emits a nested [`TableView`]. Its state
//! is **compact** (not a `Tableau`) — the renderer inflates the `Tableau` from the view via
//! `cardtable_model::from_table_view`, so there is no round-trip on the game side. Content is sourced
//! from [`cardtable_model::catalog`] for now (it moves to this side in a later reorg phase).
//!
//! Built one slice at a time, guarded by the characterization behavioral golden. **P2.1** reproduces the
//! flat banks (Heroes / Kit / Abilities / Stats / Numbers / Bestiary); the nested Locations, Rules,
//! Progress, Events, and the interactive fight follow.

use cardtable_model::catalog;
use contract::{Arrangement, CardView, Game, GameError, Outcome, PlayerId, TableView, ZoneView};

/// The card-table world game.
pub struct CardTableWorld;

/// The compact world state. Minimal for P2.1 (the banks are static); it grows with the party, cleared
/// locations, the day, and any active fight as later steps add interaction.
#[derive(Clone, Default)]
pub struct World;

/// A player action. Empty for P2.1 — the banks carry no interactions yet (equip / march / fight arrive
/// with the Locations and the arena).
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum Action {}

impl Game for CardTableWorld {
    type State = World;
    type Action = Action;

    fn new_game(&self, _seed: u64, _players: usize) -> World {
        World
    }

    fn current_player(&self, _state: &World) -> Option<PlayerId> {
        Some(PlayerId(0))
    }

    fn legal_actions(&self, _state: &World) -> Vec<Action> {
        Vec::new()
    }

    fn action_label(&self, _state: &World, action: &Action) -> String {
        match *action {}
    }

    fn apply(&self, _state: &mut World, action: &Action) -> Result<(), GameError> {
        match *action {}
    }

    fn outcome(&self, _state: &World) -> Option<Outcome> {
        None
    }

    fn view(&self, _state: &World, _perspective: Option<PlayerId>) -> TableView {
        TableView {
            status: "Card table".into(),
            zones: vec![
                heroes_zone(),
                kit_zone(),
                abilities_zone(),
                stats_zone(),
                numbers_zone(),
                locations_zone(),
                rules_zone(),
                progress_zone(),
                events_zone(),
                bestiary_zone(),
            ],
            ..Default::default()
        }
    }
}

// --- banks ---------------------------------------------------------------------------------------

/// The nine starter heroes — identity only; a hero gains stats + an ability when equipped with a kit.
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

/// Wrap a bank's content cards with its trailing name **Label** card (the deck's own label) — the last
/// card in every bank.
fn labeled(label: &str, mut cards: Vec<CardView>) -> ZoneView {
    cards.push(CardView::up(label).typed("Label"));
    ZoneView::new(label, cards)
}

/// The two stat lines shared by kit and foe cards, plus a trailing `tail` (the abilities line).
fn stat_lines(stats: &[u8; 5], tail: String) -> Vec<String> {
    vec![
        format!(
            "Might {} · Vitality {} · Toughness {}",
            stats[0], stats[1], stats[2]
        ),
        format!("Cadence {} · Finesse {}", stats[3], stats[4]),
        tail,
    ]
}

fn heroes_zone() -> ZoneView {
    let cards = HEROES
        .iter()
        .map(|name| CardView::up(*name).typed("hero").times(4))
        .collect();
    labeled("Heroes", cards)
}

fn kit_zone() -> ZoneView {
    let cards = catalog::ROSTER
        .iter()
        .map(|(name, stats, ability)| {
            CardView::up(*name)
                .typed("Kit")
                .body(stat_lines(stats, format!("Abilities: {ability}")))
        })
        .collect();
    labeled("Kit", cards)
}

fn abilities_zone() -> ZoneView {
    let cards = catalog::ABILITIES
        .iter()
        .map(|(name, desc)| {
            CardView::up(*name)
                .typed("ability")
                .body(vec![desc.to_string()])
                .times(5)
        })
        .collect();
    labeled("Abilities", cards)
}

fn stats_zone() -> ZoneView {
    let cards = catalog::STATS
        .iter()
        .map(|(name, desc)| {
            CardView::up(*name)
                .typed("stat")
                .body(vec![desc.to_string()])
                .times(5)
        })
        .collect();
    labeled("Stats", cards)
}

fn numbers_zone() -> ZoneView {
    let cards = (1..=9)
        .map(|n| CardView::up(n.to_string()).typed("number").times(12))
        .collect();
    labeled("Numbers", cards)
}

/// The nine map locations, in row-major grid order (Ashfen Crossing, the inn, is the centre cell).
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

/// One place on the map: its `Location` name card, then either the **Inn** sub-zone (Ashfen Crossing —
/// drill in to recruit) or the location's **encounter** card (its foes listed virtually).
fn place_zone(place: &str) -> ZoneView {
    let name = CardView::up(place).typed("Location");
    if place == "Ashfen Crossing" {
        // The inn's recruit view. In the shipped world it *projects* the Heroes + Kit decks; the emitter
        // authors it inline as the two row labels the projection showed (equip becomes a later action).
        let inn = ZoneView::new(
            "Inn",
            vec![
                CardView::up("Hero").typed("Label"),
                CardView::up("Kit").typed("Label"),
            ],
        )
        .with_arrangement(Arrangement::Rows);
        ZoneView::new(place, vec![name]).with_zones(vec![inn])
    } else if let Some(enc) = catalog::encounter_for(place) {
        let foes: Vec<String> = catalog::encounter_foes(enc)
            .iter()
            .map(|(c, q)| {
                if *q > 1 {
                    format!("{} ×{q}", c.name)
                } else {
                    c.name.to_string()
                }
            })
            .collect();
        let encounter = CardView::up(enc.title).typed("encounter").body(vec![
            enc.flavor.to_string(),
            format!("Foes: {}", foes.join(", ")),
        ]);
        ZoneView::new(place, vec![name, encounter])
    } else {
        ZoneView::new(place, vec![name])
    }
}

fn locations_zone() -> ZoneView {
    let places: Vec<ZoneView> = LOCATIONS.iter().map(|&p| place_zone(p)).collect();
    ZoneView::new("Locations", vec![CardView::up("Location").typed("Label")])
        .with_arrangement(Arrangement::Grid { columns: 3 })
        .with_zones(places)
}

/// A rules **phase** card — a title and its description lines.
fn phase(title: &str, detail: &[&str]) -> CardView {
    CardView::up(title)
        .typed("phase")
        .body(detail.iter().map(|s| s.to_string()).collect())
}

/// The rules encyclopedia: the six round-phases, with the five combat sub-phases nested under **Engage**.
fn rules_zone() -> ZoneView {
    let engage = ZoneView::new(
        "Engage",
        vec![
            phase(
                "Intercept (1/5)",
                &[
                    "The front screens the flankers: each Vanguard strikes an enemy Outrider as it crosses, before it can raid.",
                ],
            ),
            phase(
                "Volley (2/5)",
                &[
                    "The back fires on the flankers: each Rearguard shoots an enemy Outrider — the pre-empt, before it arrives.",
                ],
            ),
            phase(
                "Raid (3/5)",
                &[
                    "Surviving Outriders strike the enemy Rearguard they crossed for — the breaker lands on the exposed back.",
                ],
            ),
            phase(
                "Clash (4/5)",
                &[
                    "The lines meet: each Rearguard fires an enemy Vanguard, and each engaging Vanguard strikes an enemy Vanguard.",
                ],
            ),
            phase(
                "Breach (5/5)",
                &[
                    "The deep blows land last: a Vanguard crosses to an exposed enemy Rearguard; stranded Outriders fall on the front.",
                ],
            ),
            phase(
                "Engage (4/6)",
                &[
                    "Intercept — Vanguard -> Outrider",
                    "Volley — Rearguard -> Outrider",
                    "Raid — Outrider -> Rearguard",
                    "Clash — Rearguard / Vanguard -> Vanguard",
                    "Breach — the trailing blows land",
                    "Each combat phase banks its own damage pile and wipes it at that boundary: sub-Toughness damage does not carry to the next.",
                ],
            ),
        ],
    );
    let cards = vec![
        phase(
            "Marshal (1/6)",
            &[
                "Secretly assign each unit an intention — Vanguard, Outrider or Rearguard — and maybe bind a group. Re-declared each round.",
            ],
        ),
        phase(
            "Reveal (2/6)",
            &[
                "Intentions and groups are revealed together and positions lock. Nobody moves; everything after resolves in the open.",
            ],
        ),
        phase(
            "Ready (3/6)",
            &[
                "Standing abilities cast now (a Wall's brace, a Support's buff): ally-targeted, auto-land, last the round.",
            ],
        ),
        phase(
            "Wipe pile (5/6)",
            &[
                "The boundary rule of every combat phase above, not a step of its own: as each phase ends its damage pile clears — sub-Toughness damage that didn't flip a Health card does not carry to the next phase. Only Health persists; there is no separate end-of-round wipe.",
            ],
        ),
        phase(
            "Refresh (6/6)",
            &[
                "Round end (the Lull): spent Tempo resets, Health carries over, the round advances. Five undecided rounds is a draw.",
            ],
        ),
        CardView::up("Rules").typed("Label"),
    ];
    ZoneView::new("Rules", cards).with_zones(vec![engage])
}

/// The day clock — starts empty (Day 0); each passing day lays a `Day Passed` card here at runtime. Only
/// its name label at the start.
fn progress_zone() -> ZoneView {
    labeled("Progress", Vec::new()).with_arrangement(Arrangement::Grid { columns: 5 })
}

/// The reserve of `Day Passed` cards the day clock draws from.
fn events_zone() -> ZoneView {
    labeled(
        "Events",
        vec![CardView::up("Day Passed").typed("event").times(12)],
    )
}

fn bestiary_zone() -> ZoneView {
    let cards = catalog::CREATURES
        .iter()
        .map(|c| {
            let posture = format!(
                "{} · {}",
                catalog::creature_intention(c),
                catalog::creature_posture(c)
            );
            let ability = format!(
                "{}: {}",
                c.ability,
                catalog::creature_ability_description(c.ability)
            );
            let mut body = vec![posture];
            body.extend(stat_lines(&c.stats, ability));
            CardView::up(c.name).typed("foe").times(4).body(body)
        })
        .collect();
    labeled("Bestiary", cards)
}
