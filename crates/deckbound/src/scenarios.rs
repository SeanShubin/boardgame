//! The booklet: loads cards, traits, actors, and scenarios from `data/booklet.ron` and
//! builds [`Actor`]s. All numbers live in data so they retune without recompiling the engine.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::actor::{Actor, Attack, Behavior, Driver, Instinct, Script, TargetRule};
use crate::cards::Card;
use crate::currency::{Coins, Currency};
use crate::duel::Move;
use crate::encounter::EncounterCard;
use crate::form::{Form, StatCard};
use crate::stats::{Aspect, DamageType};

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
    #[serde(default)]
    upgrades: Vec<UpgradeCard>,
    /// Flavor for the enum content that has no data row of its own (§8.3 currencies): the prose is
    /// keyed by the `Currency` itself, so code references only the key and the text stays in data.
    #[serde(default)]
    currency_flavor: HashMap<Currency, String>,
    /// Flavor for the §4 combat roles, keyed by the role string actors carry (`role: "Wall"`).
    #[serde(default)]
    role_flavor: HashMap<String, String>,
}

/// A purchasable Upgrade (§8.3): a `price` in one currency, and a `grant` (Form attachment, the
/// stat boosts it adds when bought).
#[derive(Debug, Deserialize)]
struct UpgradeCard {
    name: String,
    price: Coins,
    grant: StatCard,
    #[serde(default)]
    flavor: String,
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
    #[serde(default)]
    mind: u32,
    #[serde(default)]
    keystone: Option<Aspect>,
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
    /// The fundamental Form card (stats-as-deck, §2.3/§4.3): the actor's base stat block.
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
            mind: t.mind,
            keystone: t.keystone,
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
        focus: 0,
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

fn upgrade<'a>(cat: &'a Catalog, name: &str) -> &'a UpgradeCard {
    cat.upgrades
        .iter()
        .find(|u| u.name == name)
        .unwrap_or_else(|| panic!("booklet has no upgrade named {name:?}"))
}

/// Build a clean-slate character (§8.5): the `base` identity plus its bought `upgrades`, each
/// grafted onto the Form as an attachment (stats-as-deck, §8.3). The character's strength is
/// entirely a function of the Upgrades it has bought.
pub fn build_character(base: &str, upgrades: &[String]) -> Actor {
    let cat = catalog();
    let grants: Vec<StatCard> = upgrades
        .iter()
        .map(|u| upgrade(cat, u).grant.clone())
        .collect();
    build_actor_with(cat, base, &grants, None)
}

/// The price of an Upgrade (§8.3).
pub fn upgrade_price(name: &str) -> Coins {
    upgrade(catalog(), name).price
}

/// The Upgrade names purchasable with a given currency (one role's shop).
pub fn upgrades_for(currency: Currency) -> Vec<String> {
    catalog()
        .upgrades
        .iter()
        .filter(|u| u.price.currency == currency)
        .map(|u| u.name.clone())
        .collect()
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

/// In-world flavor for a named Upgrade. Empty if none authored.
pub fn upgrade_flavor(name: &str) -> &'static str {
    catalog()
        .upgrades
        .iter()
        .find(|u| u.name == name)
        .map(|u| u.flavor.as_str())
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
    fn upgrades_strengthen_a_clean_slate_character() {
        let bare = build_character("Novice", &[]);
        let upgraded = build_character("Novice", &["Bulwark".into()]);
        // Bulwark grants +4 body — the character is tougher only because it bought the Upgrade.
        assert!(upgraded.defense.body.max > bare.defense.body.max);
        assert_eq!(upgrade_price("Bulwark").currency, Currency::Iron);
        assert!(upgrades_for(Currency::Iron).contains(&"Bulwark".to_string()));
    }
}
