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
    /// The in-app rules reference (encyclopedia) entries.
    #[serde(default)]
    glossary: Vec<GlossaryCard>,
}

#[derive(Debug, Deserialize)]
struct GlossaryCard {
    category: String,
    term: String,
    text: String,
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

/// The in-app rules reference (encyclopedia), as engine `RefEntry`s.
pub fn glossary() -> Vec<engine::RefEntry> {
    catalog()
        .glossary
        .iter()
        .map(|g| engine::RefEntry::new(g.category.clone(), g.term.clone(), g.text.clone()))
        .collect()
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

    /// Anti-drift: every *current* (non-superseded) `MANUAL` line in the Spec must appear
    /// verbatim in the encyclopedia glossary. If the Spec wording changes, this fails until the
    /// matching glossary entry is brought back in sync — the digital/printed one-liner can't drift
    /// from the canon. (The Spec is the source of truth; the glossary is authored to match it.)
    #[test]
    fn glossary_carries_current_spec_manual_lines() {
        const SPEC: &str = include_str!("../../../docs/games/deckbound/canon/2-spec/README.md");

        // One searchable corpus of every glossary entry's text.
        let corpus = glossary()
            .iter()
            .map(|e| e.text.clone())
            .collect::<Vec<_>>()
            .join("\n");

        // Walk the Spec, tracking whether the current section is superseded, and collect each
        // current MANUAL block (the marker line through the next blank line), normalized.
        let lines: Vec<&str> = SPEC.lines().collect();
        let mut manuals: Vec<String> = Vec::new();
        let mut superseded = false;
        let mut i = 0;
        while i < lines.len() {
            let t = lines[i].trim_start();
            if t.starts_with("## ") || t.starts_with("### ") {
                superseded = false; // supersession is per-section; a heading starts fresh
            }
            if t.starts_with("> **SUPERSEDED") || t.starts_with("> **PARTIALLY SUPERSEDED") {
                superseded = true;
            }
            if t.contains("**MANUAL.**") && !superseded {
                let mut block = String::new();
                let mut j = i;
                while j < lines.len() && !lines[j].trim().is_empty() {
                    block.push(' ');
                    block.push_str(lines[j]);
                    j += 1;
                }
                let block = block.replace("**MANUAL.**", " ");
                let norm = block.split_whitespace().collect::<Vec<_>>().join(" ");
                manuals.push(norm.trim_matches('*').trim().to_string());
                i = j;
                continue;
            }
            i += 1;
        }

        assert!(
            manuals.len() >= 2,
            "expected at least the current §1.0 and §4 MANUAL lines, found {}",
            manuals.len()
        );
        for m in &manuals {
            assert!(
                corpus.contains(m.as_str()),
                "a current Spec MANUAL line is not in the glossary verbatim — add/sync a \
                 GlossaryCard so the encyclopedia can't drift from the Spec:\n  {m}"
            );
        }
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
