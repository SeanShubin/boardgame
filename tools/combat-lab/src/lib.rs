//! Combat lab — a deterministic melee-matchup engine and the sandbox for the
//! **gear system** deckbound defers (Spec §2.2: armor/damage-types return as a
//! "pre-pile subtract"). The chassis uses deckbound's five stats —
//! **might · vitality · toughness · speed · daring** — and gear adds the deferred
//! layer: per-type **resistance** (the pre-pile cut) and typed weapons.
//!
//! Resistance is **subtractive** (canon-aligned) and **capped low** (≤3) so a hit
//! whose magnitude exceeds the cut always lands — no immunity / stalemate.
//!
//! The domain model lives here; simulation, balance detectors, report rendering,
//! and roster loading live in the sibling modules.

pub mod detect;
pub mod report;
pub mod resolver;
pub mod roster;

use serde::Deserialize;

/// A damage type. The set is open — fire/ice/electric/poison slot in by adding
/// variants and widening the resistance vector; nothing else changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum DamageType {
    Pierce,
    Slash,
    Crush,
}

impl DamageType {
    pub const ALL: [DamageType; 3] = [DamageType::Pierce, DamageType::Slash, DamageType::Crush];

    /// Index into a resistance vector.
    pub fn index(self) -> usize {
        match self {
            DamageType::Pierce => 0,
            DamageType::Slash => 1,
            DamageType::Crush => 2,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            DamageType::Pierce => "pierce",
            DamageType::Slash => "slash",
            DamageType::Crush => "crush",
        }
    }
}

/// Rule-bending keywords (attacker-side).
#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct Keywords {
    /// Round-end accumulation carries instead of resetting. Anti-tank.
    #[serde(default)]
    pub persist: bool,
    /// On a flip, overflow cascades into the next card. Anti-swarm.
    #[serde(default)]
    pub cleave: bool,
}

/// A full character sheet: the five-stat chassis plus the gear layer (weapon
/// damage-types and per-type resistance).
#[derive(Debug, Clone, Deserialize)]
pub struct Character {
    pub name: String,

    /// **Might** — base attack magnitude per strike (≤9).
    pub might: u32,
    /// The damage-types this character can strike with. Each strike uses the
    /// type the defender resists *least* (coverage: multi-type "finds the gap").
    /// A specialist carries one; a god collects all.
    pub weapon: Vec<DamageType>,

    /// **Vitality** — health card count; the "many small hits" answer (≤9).
    pub vitality: u32,
    /// **Toughness** — the bar a round's pile must clear to flip a card (≤9).
    pub toughness: u32,

    /// **Speed** — actions (strikes) per round (≤9).
    pub speed: u32,
    /// **Daring** — tempo-contest grade; here, the duel tie-breaker (≤9).
    #[serde(default = "one")]
    pub daring: u32,

    /// Gear: per-type **resistance** `[pierce, slash, crush]`, the pre-pile cut
    /// subtracted from a matching strike. Capped at 3 so it tilts, never walls.
    #[serde(default)]
    pub resistance: [u32; 3],

    #[serde(default)]
    pub keywords: Keywords,
}

fn one() -> u32 {
    1
}
