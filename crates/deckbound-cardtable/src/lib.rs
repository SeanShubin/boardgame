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
use contract::{CardView, Game, GameError, Outcome, PlayerId, TableView, ZoneView};

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
