//! The booklet: loads the card list and scenario booklet from `data/booklet.ron`
//! and maps its keywords onto engine types. No fighter or scenario is defined in
//! code here — only the schema and the keyword handlers.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::actors::{Creature, Hero, ReadPolicy};
use crate::stats::Body;

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
    body: u32,
    toughness: u32,
    base: u32,
    bandwidth: u32,
}

#[derive(Debug, Deserialize)]
struct CreatureCard {
    name: String,
    role: String,
    body: u32,
    toughness: u32,
    base: u32,
    bandwidth: u32,
    policy: String,
}

#[derive(Debug, Deserialize)]
struct ScenarioCard {
    name: String,
    blurb: String,
    heroes: Vec<String>,
    foes: Vec<String>,
}

/// A selectable scenario: name, teaching blurb, and the cards it uses.
#[derive(Clone, Debug)]
pub struct Scenario {
    pub name: String,
    pub blurb: String,
    heroes: Vec<String>,
    foes: Vec<String>,
}

impl Scenario {
    pub fn roster(&self) -> (Vec<Hero>, Vec<Creature>) {
        let cat = catalog();
        let heroes = self.heroes.iter().map(|n| build_hero(cat, n)).collect();
        let foes = self.foes.iter().map(|n| build_creature(cat, n)).collect();
        (heroes, foes)
    }
}

pub fn campaign() -> Vec<Scenario> {
    catalog().campaign.iter().map(scenario_from).collect()
}

pub fn tutorials() -> Vec<Scenario> {
    catalog().tutorials.iter().map(scenario_from).collect()
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
    Hero {
        name: c.name.clone(),
        role: c.role.clone(),
        body: Body::new(c.body, c.toughness),
        base: c.base,
        bandwidth: c.bandwidth,
    }
}

fn build_creature(cat: &Catalog, name: &str) -> Creature {
    let c = cat
        .creatures
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("booklet has no creature named {name:?}"));
    Creature {
        name: c.name.clone(),
        role: c.role.clone(),
        body: Body::new(c.body, c.toughness),
        base: c.base,
        bandwidth: c.bandwidth,
        policy: policy(&c.policy),
    }
}

fn policy(keyword: &str) -> ReadPolicy {
    match keyword {
        "dummy" => ReadPolicy::Dummy,
        "brute" => ReadPolicy::Brute,
        "turtle" => ReadPolicy::Turtle,
        "duelist" => ReadPolicy::Duelist,
        "grappler" => ReadPolicy::Grappler,
        "aggressor" => ReadPolicy::Aggressor,
        other => panic!("unknown read-policy keyword {other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn booklet_parses_with_tutorials_and_campaign() {
        assert!(!campaign().is_empty());
        assert!(tutorials().len() >= 4);
    }

    #[test]
    fn every_scenario_builds_a_roster() {
        for s in campaign().into_iter().chain(tutorials()) {
            let (h, f) = s.roster();
            assert!(!h.is_empty() && !f.is_empty(), "{} empty roster", s.name);
        }
    }
}
