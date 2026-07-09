//! The **card-table world** as a [`contract::Game`] — the deckbound-side view emitter.
//!
//! This is the reunification's core (plan §14): instead of hand-building a `Tableau`, the product's
//! world is authored here as a `Game` whose [`view`](Game::view) emits a nested [`TableView`]. Its state
//! is **compact** (not a `Tableau`) — the renderer inflates the `Tableau` from the view via
//! `cardtable_model::from_table_view`, so there is no round-trip on the game side. Content is sourced
//! from [`cardtable_model::catalog`] for now (it moves to this side in a later reorg phase).
//!
//! Built one slice at a time, guarded by the characterization behavioral golden. The entire static world
//! is reproduced (all ten zones), and the full play loop now flows through the seam as **pairings**:
//! recruit a hero onto a kit at the Inn ([`Action::Equip`]), march a character onto a location
//! ([`Action::March`]), and fight the encounter — either [`Action::Fight`] (auto-resolve, outcome-parity
//! via [`resolve_fight`]) or the interactive [`Action::Arena`], which drives deckbound's resumable battle
//! blow-by-blow ([`Action::StepArena`]) and folds the result back. Still to come: per-blow player choices
//! in the arena (Target / Evade / StrikeBack prompts, replacing the foundation's greedy step), then
//! pointing `boardgame` at this emitter and deleting the hand-wired bypass.

use cardtable_model::catalog;
use contract::{Arrangement, CardView, Game, GameError, Outcome, PlayerId, TableView, ZoneView};
use deckbound::balance::{DuelUnit, Stat5, build_duel_unit};
use deckbound::combat::{PendingDecision, answer_pending_greedily_side};
use deckbound::game::{battle_state_with, hero_won};
use deckbound::ruleset::Ruleset;
use deckbound::{Actor, Deckbound, ManualStatus, State};

/// The card-table world game.
pub struct CardTableWorld;

/// The compact world state — **not** a `Tableau`. Holds the RNG seed, the recruited party, and resolved
/// fights (their logs / cleared encounters). The interactive per-blow arena is layered on next.
#[derive(Clone, Default)]
pub struct World {
    /// The RNG seed for this game's fights (from `new_game`).
    seed: u64,
    /// Recruited characters (a hero paired with a kit), in recruit order.
    party: Vec<Character>,
    /// Resolved fights, in order — the outcome + log per location.
    results: Vec<FightResult>,
    /// The **active interactive fight** (the arena), if one is underway. While set, the view is the arena.
    fight: Option<Fight>,
}

/// An in-progress interactive fight — deckbound's resumable battle [`State`] plus the location it is at.
#[derive(Clone)]
struct Fight {
    state: State,
    location: usize,
}

impl World {
    /// Whether the encounter at location `idx` has been cleared (a won fight).
    fn cleared(&self, idx: usize) -> bool {
        self.results.iter().any(|r| r.location == idx && r.won)
    }

    /// The latest fight result at location `idx`, if any.
    fn last_result(&self, idx: usize) -> Option<&FightResult> {
        self.results.iter().rev().find(|r| r.location == idx)
    }
}

/// A resolved fight — its outcome and turn-by-turn log, stationed at a location.
#[derive(Clone)]
struct FightResult {
    location: usize,
    won: bool,
    log: Vec<String>,
}

/// One recruited character — a hero index (into [`HEROES`]) equipped with a kit index (into
/// `catalog::ROSTER`), stationed at a map location.
#[derive(Clone)]
struct Character {
    hero: usize,
    kit: usize,
    /// Where the character is stationed — an index into [`LOCATIONS`]. Recruited at the inn (Ashfen
    /// Crossing); marching moves it.
    location: usize,
}

/// A player action.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum Action {
    /// Recruit at the inn: equip hero #`hero` with kit #`kit`.
    Equip { hero: usize, kit: usize },
    /// March party character #`character` to location #`location` (an index into [`LOCATIONS`]).
    March { character: usize, location: usize },
    /// Fight: character #`character` **auto-resolves** the encounter at its current location.
    Fight { character: usize },
    /// Open the interactive **arena**: character #`character` fights the encounter blow-by-blow.
    Arena { character: usize },
    /// Advance the active arena one step (foundation: answers the hero decision greedily; per-blow player
    /// choices replace this next). Folds the result when the fight ends.
    StepArena,
}

impl Game for CardTableWorld {
    type State = World;
    type Action = Action;

    fn new_game(&self, seed: u64, _players: usize) -> World {
        World {
            seed,
            ..World::default()
        }
    }

    fn current_player(&self, _state: &World) -> Option<PlayerId> {
        Some(PlayerId(0))
    }

    fn legal_actions(&self, world: &World) -> Vec<Action> {
        // While the arena is up, the only move is to step it (per-blow choices arrive here next).
        if world.fight.is_some() {
            return vec![Action::StepArena];
        }
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
        // March: each recruited character to any *other* location.
        for (character, ch) in world.party.iter().enumerate() {
            for location in 0..LOCATIONS.len() {
                if location != ch.location {
                    acts.push(Action::March {
                        character,
                        location,
                    });
                }
            }
        }
        // Fight / Arena: each character stationed where an un-cleared encounter waits.
        for (character, ch) in world.party.iter().enumerate() {
            if catalog::encounter_for(LOCATIONS[ch.location]).is_some()
                && !world.cleared(ch.location)
            {
                acts.push(Action::Fight { character });
                acts.push(Action::Arena { character });
            }
        }
        acts
    }

    fn action_label(&self, state: &World, action: &Action) -> String {
        match *action {
            Action::Equip { hero, kit } => {
                format!("Equip {} with {}", HEROES[hero], catalog::ROSTER[kit].0)
            }
            Action::March {
                character,
                location,
            } => {
                let who = state.party.get(character).map_or("?", |c| HEROES[c.hero]);
                format!("March {who} to {}", LOCATIONS[location])
            }
            Action::Fight { character } => {
                let who = state.party.get(character).map_or("?", |c| HEROES[c.hero]);
                format!("Fight with {who}")
            }
            Action::Arena { character } => {
                let who = state.party.get(character).map_or("?", |c| HEROES[c.hero]);
                format!("Enter the arena with {who}")
            }
            Action::StepArena => "Advance the arena".into(),
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
                world.party.push(Character {
                    hero,
                    kit,
                    location: ASHFEN,
                });
                Ok(())
            }
            Action::March {
                character,
                location,
            } => {
                if location >= LOCATIONS.len() {
                    return Err(GameError::new("no such location"));
                }
                let ch = world
                    .party
                    .get_mut(character)
                    .ok_or_else(|| GameError::new("no such character"))?;
                ch.location = location;
                Ok(())
            }
            Action::Fight { character } => {
                let (loc, kit_idx) = {
                    let ch = world
                        .party
                        .get(character)
                        .ok_or_else(|| GameError::new("no such character"))?;
                    (ch.location, ch.kit)
                };
                let loc_name = LOCATIONS[loc];
                if catalog::encounter_for(loc_name).is_none() {
                    return Err(GameError::new("no encounter at this location"));
                }
                if world.cleared(loc) {
                    return Err(GameError::new("this encounter is already cleared"));
                }
                let (won, log) = resolve_fight(catalog::ROSTER[kit_idx].0, loc_name, world.seed)
                    .ok_or_else(|| GameError::new("no fight to resolve"))?;
                world.results.push(FightResult {
                    location: loc,
                    won,
                    log,
                });
                Ok(())
            }
            Action::Arena { character } => {
                if world.fight.is_some() {
                    return Err(GameError::new("a fight is already underway"));
                }
                let (loc, kit_idx) = {
                    let ch = world
                        .party
                        .get(character)
                        .ok_or_else(|| GameError::new("no such character"))?;
                    (ch.location, ch.kit)
                };
                if catalog::encounter_for(LOCATIONS[loc]).is_none() {
                    return Err(GameError::new("no encounter at this location"));
                }
                if world.cleared(loc) {
                    return Err(GameError::new("this encounter is already cleared"));
                }
                let mut state = start_battle(kit_idx, loc, world.seed)
                    .ok_or_else(|| GameError::new("no fight to open"))?;
                drive_to_hero_decision(&mut state);
                if state.outcome.is_some() {
                    fold_fight(world, loc, &state);
                } else {
                    world.fight = Some(Fight {
                        state,
                        location: loc,
                    });
                }
                Ok(())
            }
            Action::StepArena => {
                {
                    let fight = world
                        .fight
                        .as_mut()
                        .ok_or_else(|| GameError::new("no arena is open"))?;
                    // Foundation: answer the hero decision greedily, then drive to the next pause.
                    answer_pending_greedily_side(&mut fight.state, 0);
                    drive_to_hero_decision(&mut fight.state);
                }
                if world
                    .fight
                    .as_ref()
                    .is_some_and(|f| f.state.outcome.is_some())
                {
                    let fight = world.fight.take().expect("just checked it is some");
                    fold_fight(world, fight.location, &fight.state);
                }
                Ok(())
            }
        }
    }

    fn outcome(&self, _state: &World) -> Option<Outcome> {
        None
    }

    fn view(&self, world: &World, _perspective: Option<PlayerId>) -> TableView {
        // While a fight is underway, the arena takes over the felt.
        if let Some(fight) = &world.fight {
            return TableView {
                status: "Arena".into(),
                zones: vec![arena_zone(fight)],
                ..Default::default()
            };
        }
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

/// The index of Ashfen Crossing (the inn) in [`LOCATIONS`] — where characters are recruited.
const ASHFEN: usize = 4;

/// The pairing-key base for **location** targets — a character marches by pairing onto a Location card.
/// Offset past the kit keys (0–3) so pairing keys never collide within a view.
const LOC_KEY_BASE: u32 = 100;
/// The pairing-key base for **encounter** targets — a character fights by pairing onto an encounter card.
const ENC_KEY_BASE: u32 = 200;

/// One place on the map: its `Location` name card (a **march target**); its **encounter** (a **fight
/// target**, shown until cleared) or the **Inn** recruit sub-zone (Ashfen); a **combat-log** card once
/// fought; and any **characters** stationed here — each pairing onto other locations to march and onto
/// the encounter to fight.
fn place_zone(idx: usize, place: &str, world: &World, acts: &[Action]) -> ZoneView {
    let encounter = if place == "Ashfen Crossing" {
        None
    } else {
        catalog::encounter_for(place)
    };
    let cleared = world.cleared(idx);
    let fightable = encounter.is_some() && !cleared;

    let mut cards = vec![
        CardView::up(place)
            .typed("Location")
            .pair_key(LOC_KEY_BASE + idx as u32),
    ];
    if let Some(enc) = encounter
        && !cleared
    {
        cards.push(encounter_card(enc).pair_key(ENC_KEY_BASE + idx as u32));
    }
    if let Some(r) = world.last_result(idx) {
        cards.push(
            CardView::up(if r.won { "Victory" } else { "Defeat" })
                .typed("log")
                .panel(r.log.clone()),
        );
    }
    // Characters stationed here — each marches onto another location, and fights the encounter here.
    for (c, ch) in world.party.iter().enumerate() {
        if ch.location != idx {
            continue;
        }
        let mut card = character_card(ch);
        for l in 0..LOCATIONS.len() {
            if l == idx {
                continue;
            }
            if let Some(a) = acts.iter().position(
                |a| matches!(a, Action::March { character, location } if *character == c && *location == l),
            ) {
                card = card.pairs_onto(LOC_KEY_BASE + l as u32, a);
            }
        }
        if fightable
            && let Some(a) = acts
                .iter()
                .position(|a| matches!(a, Action::Fight { character } if *character == c))
        {
            card = card.pairs_onto(ENC_KEY_BASE + idx as u32, a);
        }
        cards.push(card);
    }
    if place == "Ashfen Crossing" {
        ZoneView::new(place, cards).with_zones(vec![inn_zone(world, acts)])
    } else {
        ZoneView::new(place, cards)
    }
}

/// An encounter card — its flavor and the virtual `Foes:` list.
fn encounter_card(enc: &catalog::Encounter) -> CardView {
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
    CardView::up(enc.title).typed("encounter").body(vec![
        enc.flavor.to_string(),
        format!("Foes: {}", foes.join(", ")),
    ])
}

/// A recruited character card — "Hero · Kit".
fn character_card(ch: &Character) -> CardView {
    CardView::up(format!(
        "{} · {}",
        HEROES[ch.hero],
        catalog::ROSTER[ch.kit].0
    ))
    .typed("character")
}

fn locations_zone(world: &World, acts: &[Action]) -> ZoneView {
    let places: Vec<ZoneView> = LOCATIONS
        .iter()
        .enumerate()
        .map(|(idx, &p)| place_zone(idx, p, world, acts))
        .collect();
    ZoneView::new("Locations", vec![CardView::up("Location").typed("Label")])
        .with_arrangement(Arrangement::Grid { columns: 3 })
        .with_zones(places)
}

/// The inn's **recruit view**: each un-recruited hero card **pairs onto** a kit to equip (the pairing the
/// renderer performs as a drag-drop or tap-then-tap); the kits are the pairing **targets**. Recruited
/// characters appear at the Ashfen *place* (stationed there), not inside the inn.
fn inn_zone(world: &World, acts: &[Action]) -> ZoneView {
    let equipped: Vec<usize> = world.party.iter().map(|c| c.hero).collect();
    let mut cards = Vec::new();
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
    for (j, (name, _, _)) in catalog::ROSTER.iter().enumerate() {
        cards.push(CardView::up(*name).typed("Kit").pair_key(j as u32));
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

// --- interactive arena ---------------------------------------------------------------------------
//
// The arena drives deckbound's *resumable* battle: build a `State`, then advance one atomic transition
// at a time, answering the foe side greedily and pausing for the hero's per-blow choices. This is the
// same resolver as `resolve_fight` (so outcomes still match), stepped instead of played out at once.

/// Build the seeded battle `State` for `kit` vs the encounter at `location`, or `None` if there is none.
fn start_battle(kit_idx: usize, location: usize, seed: u64) -> Option<State> {
    let hero = kit_unit(catalog::ROSTER[kit_idx].0)?;
    let foes = foe_units(LOCATIONS[location]);
    if foes.is_empty() {
        return None;
    }
    let heroes = vec![build_duel_unit(&hero)];
    let creatures = foes.iter().map(build_duel_unit).collect();
    Some(battle_state_with(
        heroes,
        creatures,
        false,
        seed,
        Ruleset::analysis(),
    ))
}

/// Whether a pending decision is an **unanswered hero** (side 0) decision — the ones the player owns.
fn hero_undecided(pd: &PendingDecision) -> bool {
    matches!(
        pd,
        PendingDecision::Target {
            side: 0,
            answer: None,
            ..
        } | PendingDecision::Evade {
            side: 0,
            answer: None,
            ..
        } | PendingDecision::StrikeBack {
            side: 0,
            answer: None,
            ..
        }
    )
}

/// Advance the fight, answering the **foe** side greedily, until a hero decision awaits or it finishes.
fn drive_to_hero_decision(state: &mut State) {
    let db = Deckbound;
    loop {
        match db.advance_manual_battle(state) {
            ManualStatus::Finished => return,
            ManualStatus::Advanced => continue,
            ManualStatus::Deciding => {
                answer_pending_greedily_side(state, 1); // the foe AI answers its own decisions
                if state.pending.iter().any(hero_undecided) {
                    return; // the player must answer before we go on
                }
                // Only foe decisions were pending; they are answered now — loop to consume them.
            }
        }
    }
}

/// Record a finished fight's outcome + log at its location.
fn fold_fight(world: &mut World, location: usize, state: &State) {
    world.results.push(FightResult {
        location,
        won: hero_won(state),
        log: state.log.clone(),
    });
}

/// The arena view: the hero combatants, the foe combatants (each a card with its remaining Health), any
/// hero decision awaiting an answer, and the running log.
fn arena_zone(fight: &Fight) -> ZoneView {
    let mut cards = Vec::new();
    for a in &fight.state.heroes {
        cards.push(combatant_card(a, "hero"));
    }
    for a in &fight.state.creatures {
        cards.push(combatant_card(a, "foe"));
    }
    for pd in fight.state.pending.iter().filter(|pd| hero_undecided(pd)) {
        cards.push(CardView::up(pending_label(pd)).typed("decision"));
    }
    if !fight.state.log.is_empty() {
        cards.push(
            CardView::up("Combat log")
                .typed("log")
                .panel(fight.state.log.clone()),
        );
    }
    ZoneView::new("Arena", cards)
}

/// One combatant as a card — its name, a `remaining/max` Health line, and whether it has fallen.
fn combatant_card(a: &Actor, kind: &str) -> CardView {
    let hp = format!(
        "HP {}/{}",
        a.defense.health.remaining(),
        a.defense.health.max()
    );
    let line = if a.is_down() {
        format!("{hp} — down")
    } else {
        hp
    };
    CardView::up(a.name.clone()).typed(kind).body(vec![line])
}

/// A short label for a pending hero decision (the affordance the player will answer next).
fn pending_label(pd: &PendingDecision) -> String {
    match pd {
        PendingDecision::Target { attacker, .. } => format!("{attacker}: choose a target"),
        PendingDecision::Evade { .. } => "Evade the blow?".into(),
        PendingDecision::StrikeBack { .. } => "Strike back?".into(),
    }
}
