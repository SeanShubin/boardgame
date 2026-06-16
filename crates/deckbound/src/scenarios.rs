//! The booklet: loads cards, traits, actors, and scenarios from
//! `data/booklet.ron` and builds [`Actor`]s. All numbers live in data so they
//! retune without recompiling the engine.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::actor::{Actor, Behavior, Driver, Instinct, Line, Script, TargetRule};
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

#[derive(Debug, Deserialize)]
struct ActorCard {
    name: String,
    role: String,
    /// "hero" (human) or a creature stance-policy keyword (dummy/brute/turtle/…).
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
    #[serde(default)]
    runner: bool,
    /// "front" (default) or "back" — §4 formation.
    #[serde(default)]
    line: Option<String>,
    /// Reaches the enemy back line directly (§4).
    #[serde(default)]
    ranged: bool,
    #[serde(default)]
    target_rule: Option<TargetRule>,
}

fn one() -> u32 {
    1
}

#[derive(Debug, Deserialize)]
struct ScenarioCard {
    name: String,
    blurb: String,
    heroes: Vec<String>,
    foes: Vec<String>,
}

/// A selectable scenario.
#[derive(Clone, Debug)]
pub struct Scenario {
    pub name: String,
    pub blurb: String,
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

fn find_card(cat: &Catalog, name: &str) -> Card {
    cat.cards
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("booklet has no card named {name:?}"))
        .clone()
}

/// A creature's instinct, by keyword. **Tutorial dummies are algorithmic scripts** — they
/// punish the player for not learning the lesson and fold to the correct move. **Real foes
/// draw from a random deck** (the deck *is* their mixed strategy, §7). First-pass tunings.
fn instinct_for(keyword: &str) -> Instinct {
    use Move::*;
    match keyword {
        // --- Tutorial scripts (deterministic; one lesson each) ---
        // Winds Force up, then unloads — Strike it while it gathers, or Evade the killshot.
        "charger" => Instinct::Script(Script::ChargeThenStrike { until: 2 }),
        "leader" => Instinct::Script(Script::Always(Anticipate)), // Strike or Gather it; never Evade
        "dodger" => Instinct::Script(Script::Always(Evade)),      // Anticipate it; never Strike
        "counter" => Instinct::Script(Script::Counter), // Anticipate it; Strike it and it counters
        "brawler" | "dummy" => Instinct::Script(Script::Always(Strike)), // Evade it; anything else is hit
        "post" => Instinct::Script(Script::Always(Gather)),
        // --- Real-foe decks (random lean) ---
        "feint" => Instinct::Deck(vec![Strike, Anticipate]),
        "brute" => Instinct::Deck(vec![Gather, Gather, Strike]),
        "aggressor" => Instinct::Deck(vec![Strike, Strike, Anticipate]),
        "hunter" | "grappler" => Instinct::Deck(vec![Anticipate, Anticipate, Strike]),
        "skirmisher" => Instinct::Deck(vec![Evade, Strike, Anticipate]),
        "turtle" => Instinct::Deck(vec![Gather, Evade, Strike]),
        "duelist" => Instinct::Deck(vec![Strike, Anticipate, Gather, Evade]),
        other => panic!("unknown creature instinct keyword {other:?}"),
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
            instinct: instinct_for(&c.driver),
            target_rule: c.target_rule.unwrap_or(TargetRule::Front),
        })
    };

    let line = match c.line.as_deref() {
        Some("back") => Line::Back,
        _ => Line::Front,
    };

    let mut actor = Actor {
        name: c.name.clone(),
        role: c.role.clone(),
        offense,
        defense,
        weapon: find_card(cat, &c.weapon),
        actions: c.actions.iter().map(|n| find_card(cat, n)).collect(),
        driver,
        runner: c.runner,
        line,
        ranged: c.ranged,
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
    }

    #[test]
    fn every_scenario_builds_a_roster() {
        for s in campaign().into_iter().chain(god()).chain(tutorials()) {
            let (h, f) = s.roster();
            assert!(!h.is_empty() && !f.is_empty(), "{} empty roster", s.name);
        }
    }
}
