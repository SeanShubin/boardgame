//! Combat lab — a deterministic melee-matchup engine over the
//! `quantity / magnitude / flippable-card` model.
//!
//! The domain model lives here; the simulation, balance detectors, report
//! rendering, and roster loading live in the sibling modules.

pub mod detect;
pub mod report;
pub mod resolver;
pub mod roster;

use serde::Deserialize;

/// The three melee damage channels. Armor is indexed by channel; each weapon
/// strikes on exactly one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum DamageType {
    Pierce,
    Slash,
    Crush,
}

impl DamageType {
    pub const ALL: [DamageType; 3] = [DamageType::Pierce, DamageType::Slash, DamageType::Crush];

    /// Index into a `[_; 3]` armor array.
    pub fn index(self) -> usize {
        match self {
            DamageType::Pierce => 0,
            DamageType::Slash => 1,
            DamageType::Crush => 2,
        }
    }

    /// One-letter tag for compact report tables.
    pub fn short(self) -> char {
        match self {
            DamageType::Pierce => 'p',
            DamageType::Slash => 's',
            DamageType::Crush => 'c',
        }
    }
}

/// Armor type — the categorical defense identity. Plate / Mail / Padded form the
/// regular counter cycle; Cloth is the neutral "unarmored" null, off the wheel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum ArmorType {
    Plate,
    Mail,
    Padded,
    Cloth,
}

impl ArmorType {
    pub fn name(self) -> &'static str {
        match self {
            ArmorType::Plate => "plate",
            ArmorType::Mail => "mail",
            ArmorType::Padded => "padded",
            ArmorType::Cloth => "cloth",
        }
    }
}

/// Type-chart effectiveness of a strike against an armor type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    Double,
    Normal,
    Half,
}

impl Effect {
    /// Apply to raw power. `Half` never zeroes (min 1) — the chart introduces no
    /// immunity; the only wall source is the Toughness floor.
    pub fn apply(self, power: u32) -> u32 {
        match self {
            Effect::Double => power.saturating_mul(2),
            Effect::Normal => power,
            Effect::Half => (power / 2).max(1),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Effect::Double => "×2",
            Effect::Normal => "×1",
            Effect::Half => "×½",
        }
    }
}

/// The weapon/armor type chart — a regular Latin square (each row and column has
/// exactly one Double, one Normal, one Half): pierce beats mail, slash beats
/// padding, crush beats plate, each failing against the next armor around. Cloth
/// is neutral. `pierce` (armor-piercing) upgrades a resisted Half to Normal.
pub fn effectiveness(channel: DamageType, armor: ArmorType, pierce: bool) -> Effect {
    use ArmorType::*;
    use DamageType::*;
    let base = match (channel, armor) {
        (Pierce, Plate) => Effect::Half,
        (Pierce, Mail) => Effect::Double,
        (Pierce, Padded) => Effect::Normal,
        (Slash, Plate) => Effect::Normal,
        (Slash, Mail) => Effect::Half,
        (Slash, Padded) => Effect::Double,
        (Crush, Plate) => Effect::Double,
        (Crush, Mail) => Effect::Normal,
        (Crush, Padded) => Effect::Half,
        (_, Cloth) => Effect::Normal,
    };
    if pierce && base == Effect::Half {
        Effect::Normal
    } else {
        base
    }
}

/// A single weapon: power per hit, on one channel.
#[derive(Debug, Clone, Deserialize)]
pub struct Weapon {
    pub strike_magnitude: u32,
    pub channel: DamageType,
}

/// Rule-bending keywords. See the design doc for exact semantics.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct Keywords {
    /// Attacker: within-round accumulation carries *across* rounds (defeats the
    /// per-round reset). Anti-tank grind.
    #[serde(default)]
    pub persist: bool,
    /// Attacker: on a flip, overflow cascades to the next card. Anti-swarm.
    #[serde(default)]
    pub cleave: bool,
    /// Defender: armor is a depletable pool of `armor_quantity` cards; once spent,
    /// all armor drops to zero.
    #[serde(default)]
    pub brittle: bool,
}

/// A full character sheet. Every stat is a `(quantity, magnitude)` pair in the
/// underlying model, flattened here to named fields.
#[derive(Debug, Clone, Deserialize)]
pub struct Character {
    pub name: String,

    /// Body cards (quantity) and Toughness — damage a card accumulates within a
    /// round before it flips (magnitude).
    pub health_quantity: u32,
    pub health_magnitude: u32,

    /// Armor type — sets the type-chart multiplier against each damage channel.
    #[serde(default = "default_armor")]
    pub armor: ArmorType,
    /// Strikes a `brittle` armor withstands before it shatters to neutral.
    #[serde(default = "one")]
    pub armor_quantity: u32,

    /// Actions (Strikes) per round (quantity) and initiative (magnitude).
    pub speed_quantity: u32,
    #[serde(default = "one")]
    pub speed_magnitude: u32,

    /// Armor-piercing: any value > 0 upgrades a resisted (×½) matchup to neutral.
    #[serde(default)]
    pub pierce_magnitude: u32,

    /// One or more weapons; the engine picks the best one against the current
    /// defender each strike.
    pub weapons: Vec<Weapon>,

    #[serde(default)]
    pub keywords: Keywords,
}

fn one() -> u32 {
    1
}

fn default_armor() -> ArmorType {
    ArmorType::Cloth
}
