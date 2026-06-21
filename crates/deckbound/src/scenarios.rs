//! The booklet: loads cards, traits, actors, and scenarios from `data/booklet.ron` and
//! builds [`Actor`]s. All numbers live in data so they retune without recompiling the engine.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

use engine::{Accent, CardView, ProseLine};

use crate::actor::{Actor, Attack, Behavior, Driver, Instinct, Script, TargetRule};
use crate::cards::{Card, Effect, RoleKind};
use crate::currency::Currency;
use crate::duel::Move;
use crate::encounter::EncounterCard;
use crate::form::{Form, StatCard};
use crate::stats::{Channel, DamageType};
use crate::zones::ZoneBehavior;

#[derive(Debug, Deserialize)]
struct Catalog {
    cards: Vec<Card>,
    traits: Vec<TraitCard>,
    actors: Vec<ActorCard>,
    campaign: Vec<ScenarioCard>,
    god: Vec<ScenarioCard>,
    tutorials: Vec<ScenarioCard>,
    #[serde(default)]
    versus: Vec<ScenarioCard>,
    /// The 25 role-card rewards (§8.3): 5 tracks × 5 levels, each an atomic set of role cards + a
    /// bundled Stat card. Replaces the currency/Upgrade economy.
    #[serde(default)]
    rewards: Vec<Reward>,
    /// Flavor for the enum content that has no data row of its own (§8.3 tracks): the prose is
    /// keyed by the `Currency` itself, so code references only the key and the text stays in data.
    #[serde(default)]
    currency_flavor: HashMap<Currency, String>,
    /// Flavor for the §4 combat roles, keyed by the role string actors carry (`role: "Wall"`).
    #[serde(default)]
    role_flavor: HashMap<String, String>,
    /// §2.3 — the clean-slate baseline a **character** starts from, as a **separate** Form card (its
    /// fundamental), so a character's identity Actor card carries no stats. Injected by
    /// [`build_character`]; creatures print their `base` instead.
    #[serde(default)]
    clean_slate: StatCard,
}

/// A role-card **reward** (§8.3): clearing `(track, level)` unlocks this atomic set — its role
/// `cards` (Base / Modifier / Mode) plus a bundled `stat` Form attachment. One physical copy.
#[derive(Debug, Deserialize)]
struct Reward {
    track: Currency,
    level: u32,
    #[serde(default)]
    cards: Vec<Card>,
    #[serde(default)]
    stat: StatCard,
    #[serde(default)]
    flavor: String,
}

/// The id of a reward (which `(track, level)` clear yields it). The campaign assigns rewards by id;
/// the content lives in the booklet's [`Reward`] table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RewardId {
    pub track: Currency,
    pub level: u32,
}

/// Whether a Stat card actually grants anything (so empty bundles don't clutter a Form).
fn stat_is_empty(s: &StatCard) -> bool {
    s.power == 0
        && s.precision == 0
        && s.speed == 0
        && s.dread == 0
        && s.inspiration == 0
        && s.body == 0
        && s.toughness == 0
        && s.resolve == 0
        && s.armor.is_empty()
        && s.ward.is_empty()
}

#[derive(Debug, Deserialize)]
struct TraitCard {
    name: String,
    #[serde(default)]
    flavor: String,
    #[serde(default)]
    armor: Vec<(DamageType, u32)>,
    #[serde(default)]
    ward: Vec<(DamageType, u32)>,
    #[serde(default)]
    resolve: u32,
}

fn five() -> u32 {
    5
}
fn melee() -> Attack {
    Attack::Melee
}

#[derive(Debug, Deserialize)]
struct ActorCard {
    name: String,
    role: String,
    #[serde(default)]
    flavor: String,
    /// "hero" (human) or a creature instinct keyword (brute / aggressor / charger / …).
    driver: String,
    /// The fundamental Form card (stats-as-deck, §2.3/§4.3): a **creature's** printed base stat block.
    /// A **character** leaves this empty — its baseline is the catalog's separate `clean_slate` card
    /// (§2.3, locked 2026-06-21), so the identity card carries no stats.
    #[serde(default)]
    base: StatCard,
    weapon: String,
    #[serde(default)]
    actions: Vec<String>,
    /// Attachment Form cards (armor / ward / …) — references into the trait library.
    #[serde(default)]
    traits: Vec<String>,
    /// Attack profile (§4.2): Melee / Ranged / Both / Neither. Defaults to Melee.
    #[serde(default = "melee")]
    attack: Attack,
    /// Creature commitment bias 0..=10 (how many Vanguard it fields, how readily it slips).
    #[serde(default = "five")]
    aggression: u32,
    #[serde(default)]
    target_rule: Option<TargetRule>,
}

#[derive(Debug, Deserialize)]
struct ScenarioCard {
    name: String,
    blurb: String,
    #[serde(default)]
    flavor: String,
    heroes: Vec<String>,
    foes: Vec<String>,
    /// Use the optional four-card Clash module for same-range duels (else deterministic trade).
    #[serde(default)]
    clash: bool,
    /// A hotseat PvP scenario — both sides are human (§3.4).
    #[serde(default)]
    pvp: bool,
}

/// A selectable scenario.
#[derive(Clone, Debug)]
pub struct Scenario {
    pub name: String,
    pub blurb: String,
    /// Evocative in-world flavor (prose), distinct from the tactical `blurb`.
    pub flavor: String,
    /// Whether this scenario runs the optional Clash module.
    pub clash: bool,
    /// Whether this scenario is a hotseat PvP battle (both sides human).
    pub pvp: bool,
    heroes: Vec<String>,
    foes: Vec<String>,
}

impl Scenario {
    pub fn roster(&self) -> (Vec<Actor>, Vec<Actor>) {
        let cat = catalog();
        let heroes = self.heroes.iter().map(|n| build_actor(cat, n)).collect();
        let foes = self.foes.iter().map(|n| build_actor(cat, n)).collect();
        (heroes, foes)
    }
}

pub fn campaign() -> Vec<Scenario> {
    catalog().campaign.iter().map(scenario_from).collect()
}

pub fn god() -> Vec<Scenario> {
    catalog().god.iter().map(scenario_from).collect()
}

pub fn tutorials() -> Vec<Scenario> {
    catalog().tutorials.iter().map(scenario_from).collect()
}

pub fn versus() -> Vec<Scenario> {
    catalog().versus.iter().map(scenario_from).collect()
}

/// The Spec is the single source of truth for the rules glossary (canon/2-spec/README.md),
/// embedded at compile time so the encyclopedia can't drift from the canon.
const SPEC: &str = include_str!("../../../docs/games/deckbound/canon/2-spec/README.md");

/// The category order for the encyclopedia sidebar (rule categories from the Spec, then Powers).
const CATEGORY_ORDER: &[&str] = &[
    "Roles",
    "Combat",
    "Resources",
    "Round",
    "Clash module",
    "Powers",
];

/// The in-app rules reference (encyclopedia), generated — never hand-authored — so it cannot
/// drift from its sources:
/// - **rule entries** are parsed from the Spec's `**TERM.** \`Name\` (Category) — text` lines
///   (skipping superseded sections), the same Spec that defines the rules;
/// - **Powers** are generated from the passive power cards' `text` in `booklet.ron` (the card is
///   the source of truth for what it does).
///
/// Entries are grouped by [`CATEGORY_ORDER`]; order within a category is source order.
pub fn glossary() -> Vec<engine::RefEntry> {
    static GLOSSARY: OnceLock<Vec<engine::RefEntry>> = OnceLock::new();
    GLOSSARY
        .get_or_init(|| {
            let mut entries = parse_spec_terms(SPEC);
            // Powers: one entry per distinct passive power card, from its `text`.
            for card in catalog()
                .cards
                .iter()
                .filter(|c| c.passive && !c.text.is_empty())
            {
                entries.push(engine::RefEntry::new(
                    "Powers",
                    card.name.as_str(),
                    card.text.as_str(),
                ));
            }
            // Group by the sidebar's category order; keep source order within each category.
            entries.sort_by_key(|e| {
                CATEGORY_ORDER
                    .iter()
                    .position(|c| *c == e.category)
                    .unwrap_or(CATEGORY_ORDER.len())
            });
            entries
        })
        .clone()
}

/// Parse the Spec's `**TERM.**` annotation lines into encyclopedia entries. Each is a single
/// (optionally bulleted) line of the form `**TERM.** \`Name\` (Category) — readable text`. Lines
/// inside a **superseded** section (one whose blockquote banner begins `> **SUPERSEDED` /
/// `> **PARTIALLY SUPERSEDED`) are skipped, so the encyclopedia tracks only the live rules.
fn parse_spec_terms(spec: &str) -> Vec<engine::RefEntry> {
    let mut out = Vec::new();
    let mut superseded = false;
    for line in spec.lines() {
        let t = line.trim_start();
        if t.starts_with("## ") || t.starts_with("### ") {
            superseded = false; // supersession is per-section; a heading starts fresh
        }
        if t.starts_with("> **SUPERSEDED") || t.starts_with("> **PARTIALLY SUPERSEDED") {
            superseded = true;
        }
        // Allow a leading list-bullet so the markers render as a tidy list in the Spec.
        let body = t
            .strip_prefix("- ")
            .or_else(|| t.strip_prefix("* "))
            .unwrap_or(t);
        if superseded {
            continue;
        }
        if let Some(rest) = body.strip_prefix("**TERM.**")
            && let Some(entry) = parse_term_line(rest)
        {
            out.push(entry);
        }
    }
    out
}

/// Parse the part of a `**TERM.**` line after the marker: `` `Name` (Category) — text ``.
fn parse_term_line(rest: &str) -> Option<engine::RefEntry> {
    let rest = rest.trim_start();
    // `Name` in backticks.
    let rest = rest.strip_prefix('`')?;
    let (term, rest) = rest.split_once('`')?;
    // (Category) immediately after.
    let rest = rest.trim_start().strip_prefix('(')?;
    let (category, rest) = rest.split_once(')')?;
    // Remaining text, minus a leading dash / colon separator.
    let text = rest.trim_start().trim_start_matches(['—', '-', ':']).trim();
    if term.trim().is_empty() || category.trim().is_empty() || text.is_empty() {
        return None;
    }
    Some(engine::RefEntry::new(category.trim(), term.trim(), text))
}

fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        ron::from_str(include_str!("../data/booklet.ron")).expect("data/booklet.ron should parse")
    })
}

fn scenario_from(card: &ScenarioCard) -> Scenario {
    Scenario {
        name: card.name.clone(),
        blurb: card.blurb.clone(),
        flavor: card.flavor.clone(),
        clash: card.clash,
        pvp: card.pvp,
        heroes: card.heroes.clone(),
        foes: card.foes.clone(),
    }
}

fn find_card(cat: &Catalog, name: &str) -> Card {
    cat.cards
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("booklet has no card named {name:?}"))
        .clone()
}

/// A creature's Clash instinct, by keyword (used only when the Clash module is on). Tutorial
/// dummies are deterministic scripts; real foes draw from a deck (the deck is their mixed
/// strategy).
fn instinct_for(keyword: &str) -> Instinct {
    use Move::*;
    match keyword {
        "charger" => Instinct::Script(Script::ChargeThenStrike { until: 2 }),
        "leader" => Instinct::Script(Script::Always(Anticipate)),
        "dodger" => Instinct::Script(Script::Always(Evade)),
        "counter" => Instinct::Script(Script::Counter),
        "brawler" | "dummy" => Instinct::Script(Script::Always(Strike)),
        "post" => Instinct::Script(Script::Always(Gather)),
        "feint" => Instinct::Deck(vec![Strike, Anticipate]),
        "brute" => Instinct::Deck(vec![Gather, Gather, Strike]),
        "aggressor" => Instinct::Deck(vec![Strike, Strike, Anticipate]),
        "hunter" | "grappler" => Instinct::Deck(vec![Anticipate, Anticipate, Strike]),
        "skirmisher" => Instinct::Deck(vec![Evade, Strike, Anticipate]),
        "turtle" => Instinct::Deck(vec![Gather, Evade, Strike]),
        "duelist" => Instinct::Deck(vec![Strike, Anticipate, Gather, Evade]),
        _ => Instinct::Deck(vec![Strike, Anticipate]),
    }
}

fn build_actor(cat: &Catalog, name: &str) -> Actor {
    build_actor_with(cat, name, &[], None)
}

/// Build an Actor from the catalog, grafting any `extras` Form cards (e.g. an encounter's per-level
/// scaling, §8.4, or bought Upgrades, §8.3) and optionally overriding the instinct keyword.
fn build_actor_with(
    cat: &Catalog,
    name: &str,
    extras: &[StatCard],
    driver_kw: Option<&str>,
) -> Actor {
    let c = cat
        .actors
        .iter()
        .find(|a| a.name == name)
        .unwrap_or_else(|| panic!("booklet has no actor named {name:?}"));

    // Stats-as-deck (§2.3/§4.3): the stat block is read off the **Form** — a fundamental card plus
    // attachments (traits), plus any `extra` attachment (encounter per-level scaling, §8.4).
    let mut fundamental = c.base.clone();
    if fundamental.name.is_empty() {
        fundamental.name = format!("{} (base)", c.name);
    }
    let mut form = Form::new(vec![fundamental]);
    for tname in &c.traits {
        let t = cat
            .traits
            .iter()
            .find(|t| &t.name == tname)
            .unwrap_or_else(|| panic!("booklet has no trait named {tname:?}"));
        form.cards.push(StatCard {
            name: t.name.clone(),
            armor: t.armor.clone(),
            ward: t.ward.clone(),
            resolve: t.resolve,
            ..Default::default()
        });
    }
    for extra in extras {
        form.cards.push(extra.clone());
    }
    let offense = form.offense();
    let defense = form.defense();

    let driver_kw = driver_kw
        .filter(|s| !s.is_empty())
        .unwrap_or(c.driver.as_str());
    let driver = if driver_kw == "hero" {
        Driver::Human
    } else {
        Driver::Creature(Behavior {
            aggression: c.aggression,
            target_rule: c.target_rule.unwrap_or(TargetRule::Front),
            instinct: instinct_for(driver_kw),
        })
    };

    let mut actor = Actor {
        name: c.name.clone(),
        role: c.role.clone(),
        offense,
        defense,
        weapon: find_card(cat, &c.weapon),
        actions: c.actions.iter().map(|n| find_card(cat, n)).collect(),
        driver,
        attack: c.attack,
        tempo: 0,
        cannot_fall: false,
        stunned: false,
        shoved: false,
        disarmed: false,
        free_slip_used: false,
        power_bonus: 0,
        fallen: false,
    };
    actor.refresh_round();
    actor
}

/// Build the foe roster for an encounter at `level` (§8.4): each creature in the recipe's roster
/// gets the encounter's per-level **scaling** grafted onto its Form and the encounter's
/// **strategy** as its instinct. This is the encounter → combat bridge.
pub fn build_encounter_foes(enc: &EncounterCard, level: u32) -> Vec<Actor> {
    let cat = catalog();
    let scaling = enc.scaling_at(level);
    let strategy = (!enc.strategy.is_empty()).then_some(enc.strategy.as_str());
    let mut foes = Vec::new();
    for (name, count) in enc.roster(level) {
        for _ in 0..count {
            foes.push(build_actor_with(
                cat,
                &name,
                std::slice::from_ref(&scaling),
                strategy,
            ));
        }
    }
    foes
}

fn reward(cat: &Catalog, id: RewardId) -> Option<&Reward> {
    cat.rewards
        .iter()
        .find(|r| r.track == id.track && r.level == id.level)
}

/// Build a character (§8.5): the clean-slate `base` identity plus its assigned **rewards** — each
/// reward's Stat card grafts onto the Form (stats-as-deck) and its role cards join the kit. A
/// character **is** its assigned role cards (§8.5). Curse (M5) folds at build time: +1 debuff
/// target on the owner's Controller bases.
pub fn build_character(base: &str, rewards: &[RewardId]) -> Actor {
    let cat = catalog();
    // §2.3: build_character is for a *character* (a bare identity that gains the clean-slate baseline
    // + rewards). A creature prints its own base and must be built with `build_creature` — building it
    // here would double-count, stacking the clean-slate baseline on top of the printed base.
    if let Some(c) = cat.actors.iter().find(|a| a.name == base) {
        debug_assert!(
            stat_is_empty(&c.base),
            "build_character('{base}') on an actor with a printed base — use build_creature for creatures (§2.3)"
        );
    }
    // §2.3 (locked 2026-06-21): a character's identity card is bare — its clean-slate baseline is a
    // *separate* Form card (the fundamental), never stats printed on the Actor. Creatures keep a
    // printed `base`; a character gets the catalog's `clean_slate` card here.
    let mut stats: Vec<StatCard> = vec![cat.clean_slate.clone()];
    let mut role_cards: Vec<Card> = Vec::new();
    for &id in rewards {
        if let Some(r) = reward(cat, id) {
            if !stat_is_empty(&r.stat) {
                let mut s = r.stat.clone();
                if s.name.is_empty() {
                    s.name = format!("{} {} · Stat", id.track.label(), id.level);
                }
                stats.push(s);
            }
            role_cards.extend(r.cards.iter().cloned());
        }
    }
    // Curse modifier fold (M5): a played Controller debuff hits +1 extra foe while Curse is owned.
    if role_cards
        .iter()
        .any(|c| c.kind == RoleKind::Modifier && c.name == "Curse")
    {
        for c in role_cards.iter_mut() {
            if c.role == Some(Currency::Bone) && c.kind == RoleKind::Base {
                c.targets += 1;
            }
        }
    }
    let mut actor = build_actor_with(cat, base, &stats, None);
    actor.actions.extend(role_cards);
    actor
}

/// Build a **creature** (§2.3 carve-out): a non-progressing foe whose stats are **printed** on its
/// actor card. Unlike [`build_character`], it receives no clean-slate baseline and no rewards — the
/// printed `base` *is* its Form, already summed.
pub fn build_creature(name: &str) -> Actor {
    build_actor(catalog(), name)
}

/// All reward ids of a track (its five levels), in level order — the full specialist kit (§8.3),
/// used by the reference scenario's combat-band probe.
pub fn rewards_for(track: Currency) -> Vec<RewardId> {
    let cat = catalog();
    let mut ids: Vec<RewardId> = cat
        .rewards
        .iter()
        .filter(|r| r.track == track)
        .map(|r| RewardId {
            track: r.track,
            level: r.level,
        })
        .collect();
    ids.sort_by_key(|r| r.level);
    ids
}

// --- Flavor lookups (§8.3 style) ---------------------------------------------------------------
// All flavor prose lives in `data/booklet.ron`; these read it by key. Card-like content carries a
// per-row `flavor` field; the enum content (currencies, roles) is keyed in the `*_flavor` maps.

/// In-world flavor for a currency, looked up from data. Empty if none authored.
pub fn currency_flavor(currency: Currency) -> &'static str {
    catalog()
        .currency_flavor
        .get(&currency)
        .map(String::as_str)
        .unwrap_or_default()
}

/// In-world flavor for a §4 combat role (keyed by the role string actors carry). Empty if none.
pub fn role_flavor(role: &str) -> &'static str {
    catalog()
        .role_flavor
        .get(role)
        .map(String::as_str)
        .unwrap_or_default()
}

/// In-world flavor for a named card / weapon. Empty if none authored.
pub fn card_flavor(name: &str) -> &'static str {
    catalog()
        .cards
        .iter()
        .find(|c| c.name == name)
        .map(|c| c.flavor.as_str())
        .unwrap_or_default()
}

/// In-world flavor for a named trait. Empty if none authored.
pub fn trait_flavor(name: &str) -> &'static str {
    catalog()
        .traits
        .iter()
        .find(|t| t.name == name)
        .map(|t| t.flavor.as_str())
        .unwrap_or_default()
}

/// In-world flavor for a named actor. Empty if none authored.
pub fn actor_flavor(name: &str) -> &'static str {
    catalog()
        .actors
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.flavor.as_str())
        .unwrap_or_default()
}

// --- Card catalog -------------------------------------------------------------------------------
// Every printable card in the game, as a browsable visual + a rules-grounded description.
// Generated from `booklet.ron` (like the glossary), so it can't drift from the cards themselves.

/// One entry in the in-app **card catalog**: a printable card visual ([`CardView`]) plus a detailed,
/// rules-grounded description ([`ProseLine`]s) shown when the card is opened.
#[derive(Clone)]
pub struct CatalogEntry {
    /// The section this card belongs to: Actions / Weapons / Powers / Form / Upgrades / Characters.
    pub kind: &'static str,
    pub name: String,
    /// What the card looks like printed.
    pub view: CardView,
    /// How the card works and interacts with the rules.
    pub detail: Vec<ProseLine>,
}

/// Every printable card, grouped by kind — the source for the catalog browser. Cached.
pub fn card_catalog() -> Vec<CatalogEntry> {
    static CATALOG: OnceLock<Vec<CatalogEntry>> = OnceLock::new();
    CATALOG.get_or_init(build_catalog).clone()
}

fn build_catalog() -> Vec<CatalogEntry> {
    let cat = catalog();
    // Weapons are the `cards` an actor wields; the rest of the non-passive cards are Actions.
    let weapons: std::collections::HashSet<&str> =
        cat.actors.iter().map(|a| a.weapon.as_str()).collect();
    let mut out = Vec::new();
    for c in cat
        .cards
        .iter()
        .filter(|c| !c.passive && !weapons.contains(c.name.as_str()))
    {
        out.push(action_entry(c));
    }
    for c in cat
        .cards
        .iter()
        .filter(|c| !c.passive && weapons.contains(c.name.as_str()))
    {
        out.push(weapon_entry(c));
    }
    for c in cat.cards.iter().filter(|c| c.passive) {
        out.push(power_entry(c));
    }
    for t in &cat.traits {
        out.push(trait_entry(t));
    }
    for r in &cat.rewards {
        out.push(reward_entry(r));
    }
    for a in &cat.actors {
        out.push(actor_entry(a));
    }
    out
}

/// A tiny word-wrap for card bodies (proportional font, so this is a rough budget). Ellipsizes if
/// the text doesn't fit in `max` lines.
fn wrap(text: &str, width: usize, max: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    let mut truncated = false;
    for word in text.split_whitespace() {
        if !line.is_empty() && line.len() + 1 + word.len() > width {
            lines.push(std::mem::take(&mut line));
            if lines.len() == max {
                truncated = true;
                break;
            }
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !truncated && !line.is_empty() {
        lines.push(line);
    }
    if truncated && let Some(last) = lines.last_mut() {
        last.push('…');
    }
    lines
}

fn reach_label(reach: [u32; 2]) -> String {
    match reach {
        [1, 1] => "Reach: melee".into(),
        [a, _] if a >= 2 => "Reach: ranged".into(),
        [a, b] => format!("Reach: {a}\u{2013}{b}"),
    }
}

fn reach_sentence(reach: [u32; 2]) -> String {
    match reach {
        [1, 1] => "Played at melee range (the front / a Skirmisher).".into(),
        [a, _] if a >= 2 => "Played at range — a Reserve firing on the enemy front (§4).".into(),
        [a, b] => format!("Reach {a}\u{2013}{b} jumps."),
    }
}

fn zone_behavior_label(z: ZoneBehavior) -> &'static str {
    match z {
        ZoneBehavior::Return => "After: returns to Hand",
        ZoneBehavior::Spend => "After: spent (Down)",
        ZoneBehavior::Lasting => "After: stays in play",
    }
}

pub(crate) fn zone_behavior_rule(z: ZoneBehavior) -> &'static str {
    match z {
        ZoneBehavior::Return => {
            "Zone \u{2014} Return (default): after it resolves the card goes back to your Hand, \
             reusable next round."
        }
        ZoneBehavior::Spend => {
            "Zone \u{2014} Spend: the card goes face-down (Down) after use; only a Recover stands it \
             back up. That gap is its cooldown (\u{00A7}5.3)."
        }
        ZoneBehavior::Lasting => {
            "Zone \u{2014} Lasting: the card stays in play (Active) as a stance or aura until it is \
             removed, Disrupted, or consumed (\u{00A7}5.3)."
        }
    }
}

/// A rules sentence for one card effect. `pub(crate)` so the transcript glossary defines a card's
/// keywords from the same single source of truth as the encyclopedia (no drift).
pub(crate) fn effect_rule(e: &Effect) -> String {
    match e {
        Effect::Damage { power, dtype } => match dtype.channel() {
            // Outer channel: Armor cut → Toughness bar → the Body pool.
            Channel::Body => format!(
                "Deals {} damage (base {power}). It is reduced by the target's {} Armor, then absorbed \
                 by its Body pool (cut \u{2192} bar \u{2192} pool, \u{00A7}2); Edge scales this.",
                dtype.label(),
                dtype.label()
            ),
            // Inner channel: Ward cut → the Resolve bar; no Body pool — the will *breaks* on a
            // crossing (Freeze / Flee / Scared-to-death) rather than turning Health cards.
            Channel::Fear => format!(
                "Deals {} damage (base {power}) to the will (the inner channel): reduced by the \
                 target's Ward, then tested against its Resolve bar \u{2014} no Body pool. If it \
                 clears Resolve the will breaks (Freeze / Flee / Scared-to-death, \u{00A7}2); Edge \
                 scales this.",
                dtype.label()
            ),
        },
        Effect::Guard { tempo } => format!(
            "Braces: +{tempo} Tempo to the holder this round — more initiative to answer incoming blows (M2)."
        ),
        Effect::Lifeline => {
            "Last Stand: this round the holder cannot be downed \u{2014} damage that would fell it \
             leaves it at 1 Body (M3)."
                .into()
        }
        Effect::Stagger => "On a landed hit, the target loses its action this round.".into(),
        Effect::Sunder { armor } => {
            format!(
                "Shears {armor} Armor off the target's plate (a Sunder), so later hits bite deeper."
            )
        }
        Effect::Disarm => "Rips a card from the target's Hand (knocks it Down).".into(),
        Effect::Shove => "Breaks the target out of its lane (a Shove).".into(),
        Effect::Rally { resolve } => {
            format!(
                "Raises allies' Resolve by {resolve} (a Rally) \u{2014} a Lasting effect in the party zone."
            )
        }
        Effect::Steel => "Clears accumulated Fear and steadies the nerve (a Steel).".into(),
        Effect::Recover => {
            "Turns a face-down card back up \u{2014} Down \u{2192} Hand (a Recover, \u{00A7}5.3)."
                .into()
        }
        Effect::BankSpeed { amount } => format!("Banks +{amount} Speed as extra Tempo this round."),
        Effect::Mend { body } => format!("Restores {body} Body to the most-wounded ally (a Mend)."),
        Effect::Ward => {
            "Grants a melee attack to a defenceless ally for the round (a Ward, \u{00A7}4.2), so a \
             ranged / support actor can self-defend."
                .into()
        }
        Effect::Haste { tempo } => format!("Grants +{tempo} Tempo to an ally (a Haste)."),
        Effect::Empower { power } => format!(
            "Raises allies' Power by {power} this round (an Empower) \u{2014} the Support force-multiplier's indirect offense."
        ),
        Effect::Fortify { armor } => {
            format!(
                "Raises a shield wall: +{armor} Armor to the whole line this round (a Fortify)."
            )
        }
        Effect::Suppress { tempo } => format!("Strips {tempo} Tempo from a foe (a Suppress)."),
        Effect::Slow { speed } => {
            format!("Cuts {speed} Speed from a foe (a Slow) \u{2014} cheaper to block or engage.")
        }
        Effect::Confuse { tempo } => {
            format!(
                "Drains {tempo} Tempo from a foe (a Confuse) — less initiative to act or defend."
            )
        }
    }
}

fn flavor_tail(detail: &mut Vec<ProseLine>, flavor: &str) {
    if !flavor.is_empty() {
        detail.push(ProseLine::Gap);
        detail.push(ProseLine::Body(flavor.to_string()));
    }
}

fn action_entry(c: &Card) -> CatalogEntry {
    let mut body = Vec::new();
    let summary = c.summary();
    if !summary.is_empty() {
        body.push(summary);
    }
    if c.targets > 1 {
        body.push(format!("AoE \u{00D7}{}", c.targets));
    }
    body.push(reach_label(c.reach));
    body.push(zone_behavior_label(c.zone).into());
    let view = CardView::up(c.name.clone())
        .typed("Action")
        .body(body)
        .accent(Accent::Ally);

    let mut detail = vec![
        ProseLine::Heading(c.name.clone()),
        ProseLine::Term("Action card".into()),
    ];
    for e in &c.effects {
        detail.push(ProseLine::Body(effect_rule(e)));
    }
    if c.targets > 1 {
        detail.push(ProseLine::Body(format!(
            "Area effect: resolves against up to {} distinct foes (\u{00A7}4 AoE).",
            c.targets
        )));
    }
    detail.push(ProseLine::Body(reach_sentence(c.reach)));
    detail.push(ProseLine::Body(zone_behavior_rule(c.zone).into()));
    if !c.tags.is_empty() {
        detail.push(ProseLine::Body(format!(
            "Tags: {} \u{2014} charge / combo interaction (\u{00A7}5.4).",
            c.tags.join(", ")
        )));
    }
    flavor_tail(&mut detail, card_flavor(&c.name));
    CatalogEntry {
        kind: "Actions",
        name: c.name.clone(),
        view,
        detail,
    }
}

fn weapon_entry(c: &Card) -> CatalogEntry {
    let (power, dtype) = c.primary_damage().unwrap_or((0, DamageType::Blunt));
    let body = vec![
        format!("{} damage", dtype.label()),
        if power > 0 {
            format!("+{power} Power")
        } else {
            "base weapon".into()
        },
    ];
    let view = CardView::up(c.name.clone())
        .typed("Weapon")
        .body(body)
        .accent(Accent::Neutral);

    let mut detail = vec![
        ProseLine::Heading(c.name.clone()),
        ProseLine::Term("Weapon".into()),
        ProseLine::Body(format!(
            "Supplies the {} damage type to its wielder's strike; its Power ({power}) adds to the \
             strike's magnitude (\u{00A7}4.2).",
            dtype.label()
        )),
        ProseLine::Body(format!(
            "Against a target, {} Armor reduces each hit before the Body pool absorbs the rest.",
            dtype.label()
        )),
    ];
    flavor_tail(&mut detail, card_flavor(&c.name));
    CatalogEntry {
        kind: "Weapons",
        name: c.name.clone(),
        view,
        detail,
    }
}

fn power_entry(c: &Card) -> CatalogEntry {
    let view = CardView::up(c.name.clone())
        .typed("Power")
        .body(wrap(&c.text, 24, 6))
        .accent(Accent::Good);

    let mut detail = vec![
        ProseLine::Heading(c.name.clone()),
        ProseLine::Term("Power (passive)".into()),
        ProseLine::Body(c.text.clone()),
        ProseLine::Body(
            "A \u{00A7}4 power \u{2014} always on, detected by name; it shapes the lane round rather \
             than being played as a card."
                .into(),
        ),
    ];
    flavor_tail(&mut detail, card_flavor(&c.name));
    CatalogEntry {
        kind: "Powers",
        name: c.name.clone(),
        view,
        detail,
    }
}

fn trait_entry(t: &TraitCard) -> CatalogEntry {
    let mut body = Vec::new();
    for (dt, v) in &t.armor {
        body.push(format!("Armor {} {v}", dt.label()));
    }
    for (dt, v) in &t.ward {
        body.push(format!("Ward {} {v}", dt.label()));
    }
    if t.resolve > 0 {
        body.push(format!("+{} Resolve", t.resolve));
    }
    let view = CardView::up(t.name.clone())
        .typed("Form \u{00B7} attachment")
        .body(body)
        .accent(Accent::Warn);

    let mut detail = vec![
        ProseLine::Heading(t.name.clone()),
        ProseLine::Term("Form attachment (stats-as-deck, \u{00A7}2.3)".into()),
        ProseLine::Body(
            "An attachment card added to an actor's Form (in the Active zone). Its stats sum into \
             the Form's block \u{2014} Armor and Ward merge per damage type (\u{00A7}4.3)."
                .into(),
        ),
    ];
    for (dt, v) in &t.armor {
        detail.push(ProseLine::Body(format!(
            "Armor {} {v}: reduces each incoming {} hit by {v} before the Body pool.",
            dt.label(),
            dt.label()
        )));
    }
    for (dt, v) in &t.ward {
        detail.push(ProseLine::Body(format!(
            "Ward {} {v}: blunts {} by {v} \u{2014} the defence Armor can't provide (\u{00A7}4.2).",
            dt.label(),
            dt.label()
        )));
    }
    flavor_tail(&mut detail, trait_flavor(&t.name));
    CatalogEntry {
        kind: "Form",
        name: t.name.clone(),
        view,
        detail,
    }
}

/// The non-zero stat boosts of a `StatCard`, as `"+4 Body"`-style strings.
fn stat_grants(s: &StatCard) -> Vec<String> {
    let mut v = Vec::new();
    for (n, label) in [
        (s.power, "Power"),
        (s.precision, "Precision"),
        (s.speed, "Speed"),
        (s.daring, "Daring"),
        (s.dread, "Dread"),
        (s.inspiration, "Inspiration"),
        (s.body, "Body"),
        (s.toughness, "Tough"),
        (s.resolve, "Resolve"),
    ] {
        if n > 0 {
            v.push(format!("+{n} {label}"));
        }
    }
    for (dt, n) in &s.armor {
        v.push(format!("+{n} {} Armor", dt.label()));
    }
    for (dt, n) in &s.ward {
        v.push(format!("+{n} {} Ward", dt.label()));
    }
    v
}

/// A short "Wall · III" provenance label for a reward (§3.5).
fn reward_provenance(r: &Reward) -> String {
    let role = r.track.role().unwrap_or_else(|| r.track.label());
    format!("{role} \u{00B7} L{}", r.level)
}

fn reward_entry(r: &Reward) -> CatalogEntry {
    let prov = reward_provenance(r);
    let card_names: Vec<String> = r.cards.iter().map(|c| c.name.clone()).collect();
    let grants = stat_grants(&r.stat);
    let mut body: Vec<String> = card_names.clone();
    body.extend(grants.iter().map(|g| format!("Stat {g}")));
    let view = CardView::up(prov.clone())
        .typed("Reward")
        .body(body)
        .corner(format!("L{}", r.level))
        .accent(Accent::Good);

    let role = r.track.role().unwrap_or_else(|| r.track.label());
    let mut detail = vec![
        ProseLine::Heading(prov.clone()),
        ProseLine::Term(format!(
            "Role-card reward (\u{00A7}8.3) \u{2014} the {role} track"
        )),
        ProseLine::Body(format!(
            "Cleared {role} level {}: an atomic set, assigned permanently to one character (§8.3). \
             A character **is** its assigned role cards (§8.5).",
            r.level
        )),
    ];
    for c in &r.cards {
        let kind = match c.kind {
            RoleKind::Base => "Base",
            RoleKind::Modifier => "Modifier",
            RoleKind::Mode => "Mode",
        };
        let how = if c.passive {
            "passive".into()
        } else {
            c.summary()
        };
        detail.push(ProseLine::Body(format!(
            "{}: {kind} \u{2014} {how}.",
            c.name
        )));
    }
    if !grants.is_empty() {
        detail.push(ProseLine::Body(format!(
            "Bundled Stat card: {} (stats-as-deck, §2.3).",
            grants.join(", ")
        )));
    }
    flavor_tail(&mut detail, &r.flavor);
    CatalogEntry {
        kind: "Rewards",
        name: prov,
        view,
        detail,
    }
}

fn actor_entry(a: &ActorCard) -> CatalogEntry {
    let is_hero = a.driver == "hero";
    // §2.3: a *bare-identity* character (empty printed base — the clean-slate Novice) must be built via
    // build_character so its separate clean-slate baseline card shows. Anything with a printed base —
    // creatures AND fixed specialist hero kits — displays its printed Form directly.
    let actor = if stat_is_empty(&a.base) {
        build_character(&a.name, &[])
    } else {
        build_actor(catalog(), &a.name)
    };
    let off = &actor.offense;
    let def = &actor.defense;
    let body = vec![
        format!("Spd {} \u{00B7} Drv {}", off.speed, off.daring.max(1)),
        format!("Pow {} \u{00B7} Body {}", off.power, def.body.max),
        format!("Res {} \u{00B7} Tempo {}", def.resolve, off.speed),
    ];
    let view = CardView::up(a.name.clone())
        .typed(format!(
            "{} \u{00B7} {}",
            if is_hero { "Hero" } else { "Creature" },
            a.role
        ))
        .body(body)
        .corner(def.body.max.to_string())
        .accent(if is_hero { Accent::Ally } else { Accent::Foe });

    let mut detail = vec![
        ProseLine::Heading(a.name.clone()),
        ProseLine::Term(format!(
            "{} \u{2014} {}",
            if is_hero { "Character" } else { "Creature" },
            a.role
        )),
        ProseLine::Body(
            "An actor is a bare identity plus a starting deck (stats-as-deck, \u{00A7}2.3): a \
             fundamental Form card (its base stats), attachment cards, Action cards, and a weapon."
                .into(),
        ),
        ProseLine::Body(format!(
            "Stats \u{2014} Speed {} (Tempo cards) · Daring {} (crossing grade), Power {}, Precision {}; \
             Body pool {} (toughness {}), Resolve {}.",
            off.speed,
            off.daring.max(1),
            off.power,
            off.precision,
            def.body.max,
            def.body.toughness,
            def.resolve,
        )),
    ];
    if !def.armor.is_empty() {
        let armor = def
            .armor
            .iter()
            .map(|(dt, v)| format!("{} {v}", dt.label()))
            .collect::<Vec<_>>()
            .join(", ");
        detail.push(ProseLine::Body(format!("Armor: {armor}.")));
    }
    if !a.traits.is_empty() {
        detail.push(ProseLine::Body(format!(
            "Form attachments: {}.",
            a.traits.join(", ")
        )));
    }
    if !a.actions.is_empty() {
        detail.push(ProseLine::Body(format!(
            "Action cards: {}.",
            a.actions.join(", ")
        )));
    }
    detail.push(ProseLine::Body(format!(
        "Weapon: {} (supplies the strike's damage type).",
        a.weapon
    )));
    flavor_tail(&mut detail, actor_flavor(&a.name));
    CatalogEntry {
        kind: "Characters",
        name: a.name.clone(),
        view,
        detail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn booklet_parses() {
        assert!(!campaign().is_empty());
        assert!(!god().is_empty());
        assert!(tutorials().len() >= 4);
        assert!(glossary().len() >= 10, "the encyclopedia has rules entries");
    }

    /// The card catalog is generated from the booklet: every section is present, and every entry
    /// has a printable visual titled with its name plus a non-empty rules description.
    #[test]
    fn card_catalog_is_generated_with_visual_and_detail() {
        let cat = card_catalog();
        assert!(cat.len() >= 30, "the catalog has cards (got {})", cat.len());

        let kinds: std::collections::HashSet<&str> = cat.iter().map(|e| e.kind).collect();
        for section in [
            "Actions",
            "Weapons",
            "Powers",
            "Form",
            "Rewards",
            "Characters",
        ] {
            assert!(
                kinds.contains(section),
                "catalog missing the {section} section"
            );
        }
        // Entries are grouped by section (each kind is one contiguous block, for the grid).
        let mut seen: Vec<&str> = Vec::new();
        for e in &cat {
            if seen.last() != Some(&e.kind) {
                assert!(
                    !seen.contains(&e.kind),
                    "section {} is split across the catalog",
                    e.kind
                );
                seen.push(e.kind);
            }
        }
        for e in &cat {
            assert!(!e.name.is_empty());
            assert!(!e.detail.is_empty(), "{} has no rules detail", e.name);
            match &e.view.face {
                engine::CardFace::Up { title, .. } => assert_eq!(title, &e.name),
                engine::CardFace::Down => panic!("{} is face-down in the catalog", e.name),
            }
        }
    }

    /// The encyclopedia is fully **generated** (no hand-authored glossary): rule entries come
    /// from the Spec's `**TERM.**` lines, Powers from the passive cards' `text`. This checks the
    /// generation actually produced both halves and the curated categories — so a broken Spec
    /// marker or a power that lost its `text` fails the build instead of silently emptying a page.
    #[test]
    fn glossary_is_generated_from_spec_and_cards() {
        let g = glossary();

        // Every entry has all three fields (no empty term/category/text slips through).
        for e in &g {
            assert!(
                !e.category.is_empty() && !e.term.is_empty() && !e.text.is_empty(),
                "incomplete glossary entry: {e:?}"
            );
            assert!(
                CATEGORY_ORDER.contains(&e.category.as_str()),
                "entry {:?} has a category outside CATEGORY_ORDER",
                e.term
            );
        }

        let has = |term: &str| g.iter().any(|e| e.term == term);
        // Rule terms parsed from the Spec, across several sections.
        for term in [
            "Vanguard",
            "The gauntlet",
            "Charge",
            "Speed",
            "Daring",
            "Tempo",
            "Trade",
            "The Clash",
            "Suit",
        ] {
            assert!(
                has(term),
                "Spec TERM `{term}` was not parsed into the glossary"
            );
        }
        assert!(
            has("Phases"),
            "Spec TERM `Phases` was not parsed into the glossary"
        );
        // Powers generated from card data (passive power cards' `text`).
        for power in ["Phalanx", "Blitz", "Longshot"] {
            assert!(
                has(power),
                "power `{power}` was not generated from card data"
            );
        }

        // Pin the counts so a dropped/typo'd TERM line (or a power that lost its text) fails here
        // instead of silently shrinking a page. Bump these when the Spec/cards intentionally grow.
        let powers = g.iter().filter(|e| e.category == "Powers").count();
        assert_eq!(powers, 7, "expected 7 generated Powers entries");
        assert_eq!(
            g.len() - powers,
            20,
            "expected 20 Spec TERM entries — a marker may have failed to parse"
        );

        // Entries are grouped by the sidebar's category order (non-decreasing).
        let rank = |c: &str| CATEGORY_ORDER.iter().position(|x| *x == c).unwrap();
        assert!(
            g.windows(2)
                .all(|w| rank(&w[0].category) <= rank(&w[1].category)),
            "glossary entries are not grouped by CATEGORY_ORDER"
        );
    }

    #[test]
    fn every_scenario_builds_a_roster() {
        for s in campaign()
            .into_iter()
            .chain(god())
            .chain(tutorials())
            .chain(versus())
        {
            let (h, f) = s.roster();
            assert!(!h.is_empty() && !f.is_empty(), "{} empty roster", s.name);
        }
    }

    #[test]
    fn encounter_foes_scale_with_level() {
        use crate::currency::Currency;
        use crate::encounter::{EncounterCard, RosterEntry};
        let enc = EncounterCard {
            name: "Test pack".into(),
            currency: Currency::Iron,
            strategy: "brute".into(),
            foes: vec![
                RosterEntry {
                    creature: "Husk".into(),
                    from_level: 1,
                    base: 1,
                    growth: 0,
                },
                RosterEntry {
                    creature: "Brute".into(),
                    from_level: 2,
                    base: 1,
                    growth: 0,
                },
            ],
            scaling: StatCard {
                body: 3,
                ..Default::default()
            },
        };
        // L1: only Husk; its body = base 2 + scaling 3×1 = 5.
        let l1 = build_encounter_foes(&enc, 1);
        assert_eq!(l1.len(), 1);
        assert_eq!(l1[0].name, "Husk");
        assert_eq!(l1[0].defense.body.max, 5);
        // L2: Husk + Brute; Husk body = 2 + 3×2 = 8.
        let l2 = build_encounter_foes(&enc, 2);
        assert_eq!(l2.len(), 2);
        let husk = l2.iter().find(|a| a.name == "Husk").unwrap();
        assert_eq!(husk.defense.body.max, 8);
        // The encounter's strategy overrode the instinct: foes are creatures, not humans.
        assert!(l2.iter().all(|a| !a.is_human()));
    }

    #[test]
    fn rewards_strengthen_a_clean_slate_character() {
        let bare = build_character("Novice", &[]);
        let wall = build_character("Novice", &rewards_for(Currency::Iron));
        // The Wall track's bundled Stat cards make the character tougher; its role cards join the kit.
        assert!(wall.defense.body.max > bare.defense.body.max);
        assert!(wall.actions.len() > bare.actions.len());
        // Five levels per track (§8.3).
        assert_eq!(rewards_for(Currency::Iron).len(), 5);
    }

    #[test]
    fn character_identity_card_is_bare_and_baseline_comes_from_clean_slate() {
        // §2.3 (locked 2026-06-21): a character carries NO printed stats — the Novice identity card's
        // own `base` must contribute nothing; the baseline lives in the separate `clean_slate` card.
        let cat = catalog();
        let novice = cat.actors.iter().find(|a| a.name == "Novice").unwrap();
        assert!(
            stat_is_empty(&novice.base),
            "a character's identity card must print no stats (§2.3)"
        );
        // The baseline is preserved by the separate clean-slate card: a bare-built Novice still
        // fields the old numbers (body 5 / toughness 1 / resolve 1 / speed 3 / power 1).
        let bare = build_character("Novice", &[]);
        assert_eq!(bare.defense.body.max, 5);
        assert_eq!(bare.defense.body.toughness, 1);
        assert_eq!(bare.defense.resolve, 1);
        assert_eq!(bare.offense.speed, 3);
        assert_eq!(bare.offense.power, 1);
        // A creature, by contrast, still prints its base on the identity card.
        let brute = cat.actors.iter().find(|a| a.name == "Brute").unwrap();
        assert!(
            !stat_is_empty(&brute.base),
            "a creature keeps a printed base (§2.3 carve-out)"
        );
    }
}
