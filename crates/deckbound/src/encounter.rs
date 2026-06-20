//! §8.4 — encounters as **parametric deck-recipes**. A location's threat is a single
//! [`EncounterCard`] drawn from its currency's threat deck and then *fixed* (a persistent,
//! learnable threat). The card is a recipe **evaluated at the attempted level**: a **roster** of
//! creatures (each count a level-formula) plus **thematic stat-scaling** (which stats grow with
//! level signals the counter to bring). The level is one dial scaling reward *and* threat together.
//!
//! Stat-scaling rides the stats-as-deck model (§2.3/§4.3): `scaling_at(level)` is a [`StatCard`] —
//! the scaling coefficients × level — grafted onto each foe's Form as an attachment.

use serde::Deserialize;

use crate::currency::Currency;
use crate::form::StatCard;

fn one() -> u32 {
    1
}

/// One creature line in a roster recipe. Its **count at a level** is
/// `base + growth × (level − from_level)` once `level ≥ from_level`, else 0. Defaults give the
/// common case "one of this creature at every level" (`from_level: 1, base: 1, growth: 0`).
///
/// *Examples (table-arithmetic, §8.4):* `A`/`B` always one; `C` from L2 (`from_level: 2`);
/// `D × (level − 1)` from L3 (`from_level: 3, base: 2, growth: 1`).
#[derive(Clone, Debug, Deserialize)]
pub struct RosterEntry {
    pub creature: String,
    #[serde(default = "one")]
    pub from_level: u32,
    #[serde(default = "one")]
    pub base: u32,
    #[serde(default)]
    pub growth: u32,
}

impl RosterEntry {
    /// How many of this creature appear at `level`.
    pub fn count(&self, level: u32) -> u32 {
        if level < self.from_level {
            0
        } else {
            self.base + self.growth * (level - self.from_level)
        }
    }
}

/// An encounter card: a parametric recipe for a location's foes (§8.4). Drawn once from the
/// matching currency's threat deck, then fixed.
#[derive(Clone, Debug, Deserialize)]
pub struct EncounterCard {
    pub name: String,
    /// Which threat deck this belongs to — must match the location's currency (§8.4).
    pub currency: Currency,
    /// The §7 instinct / behavior keyword the whole roster fights with (e.g. "brute").
    #[serde(default)]
    pub strategy: String,
    pub foes: Vec<RosterEntry>,
    /// Thematic per-level stat coefficients: each foe's Form gains `scaling × level` (§8.4).
    #[serde(default)]
    pub scaling: StatCard,
}

impl EncounterCard {
    /// The foe roster at `level`: `(creature name, count)` pairs (zero-count lines dropped).
    pub fn roster(&self, level: u32) -> Vec<(String, u32)> {
        self.foes
            .iter()
            .map(|e| (e.creature.clone(), e.count(level)))
            .filter(|(_, n)| *n > 0)
            .collect()
    }

    /// The stat scaling to attach to **each** foe's Form at `level`: the coefficients × level
    /// (only the scalar stats scale; armor/ward/keystone are not level-scaled).
    pub fn scaling_at(&self, level: u32) -> StatCard {
        let s = &self.scaling;
        StatCard {
            name: format!("{} +L{level}", self.name),
            power: s.power * level,
            precision: s.precision * level,
            speed: s.speed * level,
            spirit: s.spirit * level,
            body: s.body * level,
            toughness: s.toughness * level,
            resolve: s.resolve * level,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(creature: &str, from_level: u32, base: u32, growth: u32) -> RosterEntry {
        RosterEntry {
            creature: creature.into(),
            from_level,
            base,
            growth,
        }
    }

    fn brute_pack() -> EncounterCard {
        EncounterCard {
            name: "Brute pack".into(),
            currency: Currency::Iron,
            strategy: "brute".into(),
            foes: vec![
                entry("A", 1, 1, 0),
                entry("B", 1, 1, 0),
                entry("C", 2, 1, 0), // adds at L2
                entry("D", 3, 2, 1), // D × (level − 1) from L3
            ],
            // thematic: a brute scales Body — Body = level × 3.
            scaling: StatCard {
                body: 3,
                ..Default::default()
            },
        }
    }

    #[test]
    fn roster_evaluates_the_level_formulas() {
        let e = brute_pack();
        assert_eq!(e.roster(1), vec![("A".into(), 1), ("B".into(), 1)]);
        assert_eq!(
            e.roster(2),
            vec![("A".into(), 1), ("B".into(), 1), ("C".into(), 1)]
        );
        // L3: A,B,C plus D×2 (= level−1).
        let d3 = e.roster(3).into_iter().find(|(n, _)| n == "D").unwrap().1;
        assert_eq!(d3, 2);
        // L5: D = 2 + 1×(5−3) = 4 (= level−1).
        let d5 = e.roster(5).into_iter().find(|(n, _)| n == "D").unwrap().1;
        assert_eq!(d5, 4);
    }

    #[test]
    fn scaling_is_thematic_times_level() {
        let e = brute_pack();
        assert_eq!(e.scaling_at(3).body, 9); // Body = 3 × level
        assert_eq!(e.scaling_at(3).speed, 0); // a brute does not scale Speed
    }

    #[test]
    fn level_is_one_dial_more_foes_and_tougher() {
        let e = brute_pack();
        let foes_at = |lvl| e.roster(lvl).iter().map(|(_, n)| *n).sum::<u32>();
        assert!(foes_at(5) > foes_at(1)); // deeper = more bodies
        assert!(e.scaling_at(5).body > e.scaling_at(1).body); // ...and tougher
    }
}
