//! **Ruleset** — pre-game tuning parameters (§0 separable balance).
//!
//! Some of the game's bounds are not fixed laws but **settings chosen before a game starts**, exactly
//! like the seed (randomness) and the optional Clash module. The two that matter for *analysis* bound
//! the combat game tree so that optimal single-combat play is **finite and exactly searchable** (§0
//! "the core is computable"):
//!
//! - a **round cap** (reaching it ends the fight as a draw — for PvE, equivalent to a loss), and
//! - a **roster cap** (max distinct unit *types* per side; a swarm counts as one).
//!
//! Live play uses [`Ruleset::default`] (effectively unbounded — the historical termination backstop).
//! Analysis tooling uses [`Ruleset::analysis`], a short horizon and small roster, so the per-combat
//! objective becomes a clean boolean ("winnable within the horizon?") with no evaluation heuristic.
//! These are an **analysis envelope**: the balancer may assume them without the live game enforcing
//! them — encounters are *designed* to resolve within the envelope, and the solver is the oracle that
//! checks it.

use crate::rules::{ALL_RULES, Rule};

/// Tunable, pre-game combat parameters. Set once before a battle (see [`crate::game::battle_state_with`]
/// and [`crate::state::State::ruleset`]); never mutated mid-combat.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Ruleset {
    /// Which combat [`Rule`]s are **enabled** this game (a bitset, so the struct stays `Copy`). A
    /// disabled phase is skipped and a disabled behavior is not consulted, so a simulation can run
    /// against a chosen subset; default is **all on** (current behavior). The enabled set is also the
    /// **provenance** recorded with a simulation result.
    pub rules: u16,
    /// Hard cap on combat **rounds**. Reaching it ends the fight as a **draw** (PvE: a draw is, given
    /// current mechanics, no different from a loss). Live default is the historical backstop; analysis
    /// bounds it (e.g. 5) to make the game tree shallow and the win/lose question a finite,
    /// horizon-terminal reachability query (so backward induction is *exact* — no eval heuristic).
    pub max_rounds: u32,
    /// Cap on the number of distinct unit **types** per side (a swarm counts as **one**). A balance
    /// envelope consumed by the *analysis* setup to bound branching (identical instances are symmetric);
    /// it is **advisory** — not enforced during live play.
    pub max_unique_per_side: u32,
}

impl Default for Ruleset {
    /// Live play: effectively unbounded. `max_rounds` keeps the historical termination backstop (100),
    /// so existing behaviour and balance are unchanged.
    fn default() -> Self {
        Self {
            max_rounds: 100,
            max_unique_per_side: u32::MAX,
            rules: u16::MAX, // all rules on
        }
    }
}

impl Ruleset {
    /// The **analysis envelope**: a short horizon and a small roster, so optimal single-combat play is
    /// finite and exactly searchable (§0). Used by the par-solver / balance tooling.
    pub fn analysis() -> Self {
        Self {
            max_rounds: 5,
            max_unique_per_side: 5,
            rules: u16::MAX, // all rules on
        }
    }

    /// Is [`Rule`] `r` enabled this game? A disabled phase is skipped; a disabled behavior is not
    /// consulted. (All rules are on by default; `bit()`s outside [`ALL_RULES`] are unused.)
    pub fn allows(&self, r: Rule) -> bool {
        self.rules & r.bit() != 0
    }

    /// This ruleset with `off` rules **disabled** (builder; for running a simulation against a subset).
    pub fn without(mut self, off: &[Rule]) -> Self {
        for &r in off {
            self.rules &= !r.bit();
        }
        self
    }

    /// The enabled rules, in registry order — the **provenance** to record with a result.
    pub fn enabled_rules(&self) -> Vec<Rule> {
        ALL_RULES
            .iter()
            .copied()
            .filter(|&r| self.allows(r))
            .collect()
    }

    /// Is a side's distinct-type count within the roster envelope? Advisory — for analysis setup to
    /// assert it is solving a within-envelope encounter. (Counting *types*, with swarms pre-collapsed
    /// to one, is the caller's job; this only applies the bound.)
    pub fn roster_within(&self, unique_types: u32) -> bool {
        unique_types <= self.max_unique_per_side
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_carry_the_intended_bounds() {
        // Live default preserves the historical backstop (unbounded in practice); analysis bounds both.
        assert_eq!(Ruleset::default().max_rounds, 100);
        assert_eq!(Ruleset::default().max_unique_per_side, u32::MAX);
        assert_eq!(Ruleset::analysis().max_rounds, 5);
        assert_eq!(Ruleset::analysis().max_unique_per_side, 5);
        assert!(Ruleset::analysis().roster_within(5));
        assert!(!Ruleset::analysis().roster_within(6));
    }

    #[test]
    fn rules_default_on_and_without_disables() {
        // All rules on by default / under the analysis envelope.
        assert!(Ruleset::default().allows(Rule::Intercept));
        assert!(Ruleset::analysis().allows(Rule::Grouping));
        assert_eq!(Ruleset::analysis().enabled_rules().len(), ALL_RULES.len());
        // `without` disables exactly the named rules; the rest stay on.
        let subset = Ruleset::analysis().without(&[Rule::Grouping, Rule::AreaOfEffect]);
        assert!(!subset.allows(Rule::Grouping));
        assert!(!subset.allows(Rule::AreaOfEffect));
        assert!(subset.allows(Rule::Clash));
        assert_eq!(subset.enabled_rules().len(), ALL_RULES.len() - 2);
    }
}
