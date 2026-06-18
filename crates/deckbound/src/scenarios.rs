//! The booklet: loads cards, traits, actors, and scenarios from `data/booklet.ron` and
//! builds [`Actor`]s. All numbers live in data so they retune without recompiling the engine.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::actor::{Actor, Attack, Behavior, Driver, Instinct, Script, TargetRule};
use crate::cards::Card;
use crate::duel::Move;
use crate::stats::{Aspect, DamageType, Defense, Offense};

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
}

#[derive(Debug, Deserialize)]
struct TraitCard {
    name: String,
    #[serde(default)]
    armor: Vec<(DamageType, u32)>,
    #[serde(default)]
    ward: Vec<(DamageType, u32)>,
    #[serde(default)]
    resolve: u32,
    #[serde(default)]
    mind: u32,
    #[serde(default)]
    keystone: Option<Aspect>,
}

fn one() -> u32 {
    1
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
    /// "hero" (human) or a creature instinct keyword (brute / aggressor / charger / …).
    driver: String,
    speed: u32,
    power: u32,
    #[serde(default)]
    precision: u32,
    #[serde(default)]
    spirit: u32,
    body: u32,
    #[serde(default = "one")]
    toughness: u32,
    #[serde(default)]
    resolve: u32,
    #[serde(default = "one")]
    mind: u32,
    weapon: String,
    #[serde(default)]
    actions: Vec<String>,
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
    "Lanes",
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
    let c = cat
        .actors
        .iter()
        .find(|a| a.name == name)
        .unwrap_or_else(|| panic!("booklet has no actor named {name:?}"));

    let mut defense = Defense::new(c.body, c.toughness, c.resolve, c.mind);
    let mut armor: BTreeMap<DamageType, u32> = BTreeMap::new();
    let mut ward: BTreeMap<DamageType, u32> = BTreeMap::new();
    for tname in &c.traits {
        let t = cat
            .traits
            .iter()
            .find(|t| &t.name == tname)
            .unwrap_or_else(|| panic!("booklet has no trait named {tname:?}"));
        for (dt, v) in &t.armor {
            *armor.entry(*dt).or_insert(0) += v;
        }
        for (dt, v) in &t.ward {
            *ward.entry(*dt).or_insert(0) += v;
        }
        defense.resolve += t.resolve;
        defense.mind += t.mind;
        if let Some(k) = t.keystone {
            defense.keystone = k;
        }
    }
    defense.armor = armor;
    defense.ward = ward;

    let offense = Offense {
        power: c.power,
        precision: c.precision,
        speed: c.speed,
        spirit: c.spirit,
    };

    let driver = if c.driver == "hero" {
        Driver::Human
    } else {
        Driver::Creature(Behavior {
            aggression: c.aggression,
            target_rule: c.target_rule.unwrap_or(TargetRule::Front),
            instinct: instinct_for(&c.driver),
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
        focus: 0,
        fallen: false,
    };
    actor.refresh_round();
    actor
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
        for term in ["Vanguard", "Lanes", "Tempo", "Focus", "Trade", "The Clash"] {
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
            18,
            "expected 18 Spec TERM entries — a marker may have failed to parse"
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
}
