//! The booklet: loads the card list and scenario booklet from data
//! (`data/booklet.ron`) and maps its keywords onto engine types.
//!
//! No hero, creature, or scenario is defined in code here — only the schema for
//! the data file and the keyword handlers that turn data into engine values.
//! This is the "components + scenarios" tier of the design's three-tier model;
//! the engine (the rest of the crate) is the "rulebook".

use std::sync::OnceLock;

use serde::Deserialize;

use crate::actors::{Behavior, Creature, Hero, Line, Play, new_creature, new_hero};
use crate::read::Read;
use crate::stats::{Armor, Body, DamageType};

// ---- the data schema (mirrors booklet.ron) -----------------------------

#[derive(Debug, Deserialize)]
struct Catalog {
    heroes: Vec<HeroCard>,
    creatures: Vec<CreatureCard>,
    campaign: Vec<ScenarioCard>,
    tutorials: Vec<ScenarioCard>,
}

#[derive(Debug, Deserialize)]
struct HeroCard {
    name: String,
    role: String,
    speed: u32,
    power: u32,
    magic: u32,
    spirit: u32,
    resolve: i32,
    body: u32,
    toughness: u32,
    armor: Vec<(String, u32)>,
    line: String,
    strike: String,
    plays: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CreatureCard {
    name: String,
    speed: u32,
    power: u32,
    fear: u32,
    resolve: i32,
    body: u32,
    toughness: u32,
    armor: Vec<(String, u32)>,
    line: String,
    strike: String,
    behavior: String,
    count: u32,
    runner: bool,
}

#[derive(Debug, Deserialize)]
struct ScenarioCard {
    name: String,
    blurb: String,
    heroes: Vec<String>,
    foes: Vec<String>,
}

/// A selectable scenario: its name, its teaching blurb, and the cards it uses.
#[derive(Clone, Debug)]
pub struct Scenario {
    pub name: String,
    pub blurb: String,
    heroes: Vec<String>,
    foes: Vec<String>,
}

impl Scenario {
    /// Build this scenario's party and warband from the card list.
    pub fn roster(&self) -> (Vec<Hero>, Vec<Creature>) {
        let cat = catalog();
        let heroes = self.heroes.iter().map(|n| build_hero(cat, n)).collect();
        let foes = self.foes.iter().map(|n| build_creature(cat, n)).collect();
        (heroes, foes)
    }
}

/// The campaign scenarios.
pub fn campaign() -> Vec<Scenario> {
    catalog().campaign.iter().map(scenario_from).collect()
}

/// The tutorial scenarios, each isolating one mechanic.
pub fn tutorials() -> Vec<Scenario> {
    catalog().tutorials.iter().map(scenario_from).collect()
}

// ---- loading ------------------------------------------------------------

fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        ron::from_str(include_str!("../data/booklet.ron"))
            .expect("data/booklet.ron should parse")
    })
}

fn scenario_from(card: &ScenarioCard) -> Scenario {
    Scenario {
        name: card.name.clone(),
        blurb: card.blurb.clone(),
        heroes: card.heroes.clone(),
        foes: card.foes.clone(),
    }
}

fn build_hero(cat: &Catalog, name: &str) -> Hero {
    let c = cat
        .heroes
        .iter()
        .find(|h| h.name == name)
        .unwrap_or_else(|| panic!("booklet has no hero named {name:?}"));
    new_hero(
        c.name.clone(),
        c.role.clone(),
        c.speed,
        c.power,
        c.magic,
        c.spirit,
        c.resolve,
        Body::new(c.body, c.toughness),
        armor(&c.armor),
        line(&c.line),
        damage_type(&c.strike),
        c.plays.iter().map(|p| play(p)).collect(),
    )
}

fn build_creature(cat: &Catalog, name: &str) -> Creature {
    let c = cat
        .creatures
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("booklet has no creature named {name:?}"));
    new_creature(
        c.name.clone(),
        c.speed,
        c.power,
        c.fear,
        c.resolve,
        Body::new(c.body, c.toughness),
        armor(&c.armor),
        line(&c.line),
        damage_type(&c.strike),
        behavior(&c.behavior),
        c.count,
        c.runner,
    )
}

// ---- keyword handlers (data -> engine) ----------------------------------

fn armor(entries: &[(String, u32)]) -> Armor {
    if entries.is_empty() {
        return Armor::none();
    }
    Armor::new(entries.iter().map(|(t, v)| (damage_type(t), *v)).collect())
}

fn damage_type(keyword: &str) -> DamageType {
    match keyword {
        "sharp" => DamageType::Sharp,
        "blunt" => DamageType::Blunt,
        "heat" => DamageType::Heat,
        "cold" => DamageType::Cold,
        other => panic!("unknown damage-type keyword {other:?}"),
    }
}

fn line(keyword: &str) -> Line {
    match keyword {
        "front" => Line::Front,
        "back" => Line::Back,
        other => panic!("unknown line keyword {other:?}"),
    }
}

fn behavior(keyword: &str) -> Behavior {
    match keyword {
        "bluff" => Behavior::Bluff,
        "runner" => Behavior::Runner,
        "howl" => Behavior::Howl,
        "swarm" => Behavior::Swarm,
        other => panic!("unknown behavior keyword {other:?}"),
    }
}

fn play(keyword: &str) -> Play {
    match keyword {
        "block" => Play::Read(Read::Block),
        "evade" => Play::Read(Read::Evade),
        "scheme" => Play::Read(Read::Scheme),
        "strike" => Play::Read(Read::Strike),
        "bash" => Play::Bash,
        "riposte" => Play::Riposte,
        "firestorm" => Play::Firestorm,
        "frostbite" => Play::Frostbite,
        "rally" => Play::Rally,
        "dread" => Play::Dread,
        "steel" => Play::Steel,
        other => panic!("unknown play keyword {other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn booklet_parses_and_has_scenarios() {
        assert_eq!(campaign().len(), 1);
        assert_eq!(tutorials().len(), 5);
    }

    #[test]
    fn every_scenario_builds_a_roster() {
        for scenario in campaign().into_iter().chain(tutorials()) {
            let (heroes, foes) = scenario.roster();
            assert!(!heroes.is_empty(), "{} has no heroes", scenario.name);
            assert!(!foes.is_empty(), "{} has no foes", scenario.name);
        }
    }
}
