//! The **card-table world** as a [`contract::Game`] — the deckbound-side view emitter.
//!
//! This is the reunification's core (plan §14): instead of hand-building a `Tableau`, the product's
//! world is authored here as a `Game` whose [`view`](Game::view) emits a nested [`TableView`]. Its state
//! is **compact** (not a `Tableau`) — the renderer inflates the `Tableau` from the view via
//! `cardtable_model::from_table_view`, so there is no round-trip on the game side. Content is sourced
//! from [`cardtable_model::catalog`] for now (it moves to this side in a later reorg phase).
//!
//! Built one slice at a time, guarded by the characterization behavioral golden. The entire static world
//! is reproduced (all ten zones: the banks, the nested Locations grid, the Rules encyclopedia, the day
//! clock), and the first **interaction** now flows through the seam: the Inn is the recruit view where a
//! hero card *pairs onto* a kit to equip, and [`Action::Equip`] recruits the character. Still to come:
//! march + the interactive fight (combat state → view → apply), then pointing `boardgame` at this emitter
//! and deleting the hand-wired bypass. Fight *resolution* already delegates to deckbound with
//! outcome-parity (see [`resolve_fight`]).

use cardtable_model::catalog;
use contract::{Arrangement, CardView, Game, GameError, Outcome, PlayerId, TableView, ZoneView};
use deckbound::balance::{DuelUnit, Stat5, build_duel_unit};

/// The card-table world game.
pub struct CardTableWorld;

/// The compact world state — **not** a `Tableau`. Grows as interaction lands; for now it holds the
/// recruited party (equipped characters). Cleared locations, the day, and any active fight come next.
#[derive(Clone, Default)]
pub struct World {
    /// Recruited characters (a hero paired with a kit), in recruit order.
    party: Vec<Character>,
}

/// One recruited character — a hero index (into [`HEROES`]) equipped with a kit index (into
/// `catalog::ROSTER`).
#[derive(Clone)]
struct Character {
    hero: usize,
    kit: usize,
}

/// A player action.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum Action {
    /// Recruit at the inn: equip hero #`hero` with kit #`kit`.
    Equip { hero: usize, kit: usize },
}

impl Game for CardTableWorld {
    type State = World;
    type Action = Action;

    fn new_game(&self, _seed: u64, _players: usize) -> World {
        World::default()
    }

    fn current_player(&self, _state: &World) -> Option<PlayerId> {
        Some(PlayerId(0))
    }

    fn legal_actions(&self, world: &World) -> Vec<Action> {
        // Every un-recruited hero × every kit is a legal equip. Stable order (hero-major) so the view's
        // pairing indices line up with this list.
        let equipped: Vec<usize> = world.party.iter().map(|c| c.hero).collect();
        let mut acts = Vec::new();
        for hero in 0..HEROES.len() {
            if equipped.contains(&hero) {
                continue;
            }
            for kit in 0..catalog::ROSTER.len() {
                acts.push(Action::Equip { hero, kit });
            }
        }
        acts
    }

    fn action_label(&self, _state: &World, action: &Action) -> String {
        match *action {
            Action::Equip { hero, kit } => {
                format!("Equip {} with {}", HEROES[hero], catalog::ROSTER[kit].0)
            }
        }
    }

    fn apply(&self, world: &mut World, action: &Action) -> Result<(), GameError> {
        match *action {
            Action::Equip { hero, kit } => {
                if hero >= HEROES.len() || kit >= catalog::ROSTER.len() {
                    return Err(GameError::new("no such hero or kit"));
                }
                if world.party.iter().any(|c| c.hero == hero) {
                    return Err(GameError::new("that hero is already recruited"));
                }
                world.party.push(Character { hero, kit });
                Ok(())
            }
        }
    }

    fn outcome(&self, _state: &World) -> Option<Outcome> {
        None
    }

    fn view(&self, world: &World, _perspective: Option<PlayerId>) -> TableView {
        let acts = self.legal_actions(world);
        TableView {
            status: "Card table".into(),
            zones: vec![
                heroes_zone(),
                kit_zone(),
                abilities_zone(),
                stats_zone(),
                numbers_zone(),
                locations_zone(world, &acts),
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
fn place_zone(place: &str, world: &World, acts: &[Action]) -> ZoneView {
    let name = CardView::up(place).typed("Location");
    if place == "Ashfen Crossing" {
        ZoneView::new(place, vec![name]).with_zones(vec![inn_zone(world, acts)])
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

fn locations_zone(world: &World, acts: &[Action]) -> ZoneView {
    let places: Vec<ZoneView> = LOCATIONS
        .iter()
        .map(|&p| place_zone(p, world, acts))
        .collect();
    ZoneView::new("Locations", vec![CardView::up("Location").typed("Label")])
        .with_arrangement(Arrangement::Grid { columns: 3 })
        .with_zones(places)
}

/// The inn's **recruit view**: each un-recruited hero card **pairs onto** a kit to equip (the pairing the
/// renderer performs as a drag-drop or tap-then-tap); the kits are the pairing **targets**; recruited
/// characters are listed after. (Equipped-character display here is a first cut — a name · kit card.)
fn inn_zone(world: &World, acts: &[Action]) -> ZoneView {
    let equipped: Vec<usize> = world.party.iter().map(|c| c.hero).collect();
    let mut cards = Vec::new();
    // Un-recruited heroes, each pairing onto every kit target.
    for (i, hero) in HEROES.iter().enumerate() {
        if equipped.contains(&i) {
            continue;
        }
        let mut card = CardView::up(*hero).typed("hero");
        for (j, _) in catalog::ROSTER.iter().enumerate() {
            if let Some(idx) = acts
                .iter()
                .position(|a| matches!(a, Action::Equip { hero, kit } if *hero == i && *kit == j))
            {
                card = card.pairs_onto(j as u32, idx);
            }
        }
        cards.push(card);
    }
    // Kit pairing targets (keyed by kit index).
    for (j, (name, _, _)) in catalog::ROSTER.iter().enumerate() {
        cards.push(CardView::up(*name).typed("Kit").pair_key(j as u32));
    }
    // Recruited characters.
    for ch in &world.party {
        cards.push(
            CardView::up(format!(
                "{} · {}",
                HEROES[ch.hero],
                catalog::ROSTER[ch.kit].0
            ))
            .typed("character"),
        );
    }
    ZoneView::new("Inn", cards).with_arrangement(Arrangement::Rows)
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

// --- combat --------------------------------------------------------------------------------------
//
// Fight resolution delegates to deckbound's deterministic resolver. The emitter builds the same
// `DuelUnit`s the old `cardtable-combat` path built — a hero from its kit (catalog `ROSTER` + strike
// shape), the foes from the location's encounter (catalog) — so the outcome matches the old path by
// construction. This is the combat logic moving from `cardtable-combat` to the emitter side; the
// interactive arena (view + per-blow stepping) is authored on top of it next.

/// `[u8; 5]` → the resolver's `(Might, Vitality, Toughness, Cadence, Finesse)` tuple.
fn stat5(s: [u8; 5]) -> Stat5 {
    (
        s[0] as u32,
        s[1] as u32,
        s[2] as u32,
        s[3] as u32,
        s[4] as u32,
    )
}

/// A kit as a combat unit: its stat line plus the strike shape derived from its signature ability.
fn kit_unit(kit: &str) -> Option<DuelUnit> {
    let (name, stats, ability) = catalog::ROSTER.iter().find(|(n, _, _)| *n == kit)?;
    let (ranged, aoe) = catalog::ability_shape(ability);
    Some(DuelUnit {
        name: (*name).to_string(),
        ability: (*ability).to_string(),
        stats: stat5(*stats),
        ranged,
        aoe,
        count: 1,
        hoard: false,
        pos: None,
    })
}

/// The foes stationed at `location` as combat units — the encounter's `(creature, quantity)` roster from
/// the catalog, expanded so a `×2` keystone fields two. Empty if the location has no encounter.
fn foe_units(location: &str) -> Vec<DuelUnit> {
    let Some(enc) = catalog::encounter_for(location) else {
        return Vec::new();
    };
    let mut units = Vec::new();
    for (creature, qty) in catalog::encounter_foes(enc) {
        for _ in 0..qty {
            units.push(DuelUnit {
                name: creature.name.to_string(),
                ability: creature.ability.to_string(),
                stats: stat5(creature.stats),
                ranged: creature.ranged,
                aoe: creature.aoe,
                count: 1,
                hoard: creature.hoard,
                pos: creature.pos.map(str::to_string),
            });
        }
    }
    units
}

/// Resolve a fight: `kit` vs the encounter at `location`, seeded. Returns the turn-by-turn log and whether
/// the hero won (`Some(true)`), lost/drew (`Some(false)`), or the fight was a no-op (`None` — unknown kit
/// or a location with no encounter). Delegates to deckbound's deterministic resolver, so the outcome
/// matches the old `cardtable-combat` path for the same inputs and seed.
pub fn resolve_fight(kit: &str, location: &str, seed: u64) -> Option<(bool, Vec<String>)> {
    let hero = kit_unit(kit)?;
    let foes = foe_units(location);
    if foes.is_empty() {
        return None;
    }
    let hero_actors = vec![build_duel_unit(&hero)];
    let foe_actors = foes.iter().map(build_duel_unit).collect();
    let (won, log) = deckbound::resolve_logged(hero_actors, foe_actors, seed);
    Some((won == Some(true), log))
}
