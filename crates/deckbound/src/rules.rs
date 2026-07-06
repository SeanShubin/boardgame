//! The combat **phases and rules as a self-documenting, toggleable registry** (§4 / §4.6).
//!
//! Combat is a sequence of single-purpose **phases** over two accumulators (the per-sub-phase damage
//! **pile** and **Tempo**) plus a few cross-cutting **behaviors**. Every entry here:
//! - does **exactly one thing**,
//! - carries a human **description** — the *source of truth* for the auto-generated phase-by-phase rules
//!   appendix (so the rules can't drift from the code), and
//! - can be switched **on/off** via the [`Ruleset`](crate::ruleset::Ruleset), so a simulation can run
//!   against a chosen subset and record exactly which rules were allowed.
//!
//! This module is **pure data** (ids + text + ordering); the engine reads it to drive resolution, the
//! handbook reads it to emit the appendix, and a simulation records the enabled set as provenance.

/// Whether a rule is a round-level **phase** (a step in the round, run in order), a **sub-phase** (a
/// combat sub-step of the Engage phase, resolved on the §4.6 schedule), or a cross-cutting **behavior**
/// (a sub-rule consulted within phases, e.g. area-of-effect).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RuleKind {
    /// A single-purpose round-level step, run in the order listed in [`ALL_RULES`].
    Phase,
    /// A combat sub-step **within the Engage phase**, resolved on the §4.6 sub-phase schedule.
    SubPhase,
    /// A cross-cutting behavior consulted inside phases (not a step of its own).
    Behavior,
}

/// One toggleable combat phase, sub-phase, or behavior. The variant order in [`ALL_RULES`] is the
/// **round order** (for the phases) — the appendix and the engine both read it in sequence. The five
/// sub-phase steps (Intercept … Breach) are the §4.6 **sub-phase schedule** of the Engage phase.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum Rule {
    // ---- set up the round ----
    /// Marshal: secretly assign each unit an intention (Vanguard / Outrider / Rearguard) and group them.
    Marshal,
    /// Reveal: reveal intentions and groups; positions lock. Nobody moves.
    Reveal,
    /// Ready: cast Standing abilities (braces / ally buffs).
    Ready,
    /// Engage: resolve the fixed §4.6 sub-phase schedule (Intercept → Volley → Raid → Clash → Breach).
    Engage,
    // ---- the sub-phase schedule of Engage (§4.6) ----
    /// Intercept: each Vanguard strikes an enemy Outrider (the front screens the crossers).
    Intercept,
    /// Volley: each Rearguard fires on an enemy Outrider (the back shoots crossers — the pre-empt).
    Volley,
    /// Raid: each surviving Outrider strikes an enemy Rearguard (the flank that got through).
    Raid,
    /// Clash: each Rearguard fires on an enemy Vanguard, and the fronts strike each other.
    Clash,
    /// Breach: the deep / trailing blows land last (Vanguard→Rearguard, Outrider→Vanguard/Outrider).
    Breach,
    // ---- boundaries / accumulators ----
    /// Clear the per-sub-phase damage pile at each sub-phase boundary.
    WipePile,
    /// The Lull: Tempo resets, Health persists, the round advances (5-round cap).
    Refresh,
    // ---- cross-cutting behaviors ----
    /// The one Tempo contest: attack vs block / slip / evade.
    TempoContest,
    /// Strike-back: a melee attacker may be answered by the defender's own blow.
    StrikeBack,
    /// Area-of-effect: an attack may strike a whole rank at once (bypassing a group's spillover).
    AreaOfEffect,
    /// Grouping: same-side units bind into one unit (spillover to block, weakest-link to slip).
    Grouping,
}

/// Static metadata for a [`Rule`] — its appendix name, one-thing description, and kind.
pub struct RuleInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub kind: RuleKind,
}

impl Rule {
    /// The appendix name, description, and kind for this rule. The descriptions are the **canonical
    /// mechanical text** — the rules appendix is generated from them.
    pub fn info(self) -> RuleInfo {
        use RuleKind::{Behavior, Phase, SubPhase};
        let (name, description, kind) = match self {
            Rule::Marshal => (
                "Marshal",
                "Each unit is secretly assigned an **intention** — Vanguard (hold the front), Outrider \
                 (break the line) or Rearguard (deal from the back) — and may be bound into a group. \
                 Re-declared every round; declaring is free and may fail (a misplaced unit is idle, not \
                 barred).",
                Phase,
            ),
            Rule::Reveal => (
                "Reveal",
                "Intentions and groups are revealed together and positions lock. Nobody moves; everything \
                 after resolves in the open.",
                Phase,
            ),
            Rule::Ready => (
                "Ready",
                "Standing abilities (a Wall's brace, a Support's ally buff) are cast now. They are \
                 ally-targeted, auto-land, and last the round.",
                Phase,
            ),
            Rule::Engage => (
                "Engage",
                "The two lines meet and trade blows: the fixed **sub-phase schedule** resolves in order — \
                 Intercept → Volley → Raid → Clash → Breach — each sub-phase a §1.9 boundary. Untyped \
                 Might banks into the per-sub-phase pile; clearing a target's Toughness flips a Health card.",
                Phase,
            ),
            Rule::Intercept => (
                "Intercept",
                "The front screens the flankers: each Vanguard strikes an enemy Outrider as it crosses, \
                 before it can raid. An Outrider cut down here never reaches the back.",
                SubPhase,
            ),
            Rule::Volley => (
                "Volley",
                "The back fires on the flankers: each Rearguard shoots an enemy Outrider — before it \
                 arrives (the pre-empt). A shot spent here is a shot not fired at the enemy front later.",
                SubPhase,
            ),
            Rule::Raid => (
                "Raid",
                "Surviving Outriders strike the enemy Rearguard they crossed for. The breaker that got \
                 through the Intercept and Volley lands on the exposed back.",
                SubPhase,
            ),
            Rule::Clash => (
                "Clash",
                "The lines meet: each Rearguard fires on an enemy Vanguard (the only answer to its \
                 Toughness), and each engaging Vanguard strikes an enemy Vanguard. Untyped Might banks \
                 into the per-sub-phase pile; clearing the target's Toughness flips a Health card.",
                SubPhase,
            ),
            Rule::Breach => (
                "Breach",
                "The deep, trailing blows land last: a Vanguard crosses to an enemy Rearguard whose own \
                 front has fallen, and Outriders with no reachable back fall on the enemy front or each \
                 other.",
                SubPhase,
            ),
            Rule::WipePile => (
                "Wipe pile",
                "At each sub-phase boundary the per-sub-phase damage pile is cleared: sub-threshold damage \
                 that did not turn a Health card does not carry into the next sub-phase. Only Health \
                 persists.",
                Behavior,
            ),
            Rule::Refresh => (
                "Refresh (the Lull)",
                "Round end: all spent Tempo resets, Health carries over, and the round advances. A battle \
                 not decided within five rounds is a draw.",
                Phase,
            ),
            Rule::TempoContest => (
                "Tempo contest",
                "The one attack-vs-defense mechanic: a single simultaneous Tempo bid (cards x Finesse); \
                 the defender must strictly exceed it (a tie lands the hit) to block a melee blow, slip a \
                 blocker, or evade ranged fire. Defending is Tempo-negative, so blows eventually land.",
                Behavior,
            ),
            Rule::StrikeBack => (
                "Strike back",
                "A melee attacker may be answered: the defender spends a Tempo card to deal its own Might \
                 back — but only when that blow can crack the attacker's Toughness, and only if the \
                 defender is still alive (a corpse cannot react).",
                Behavior,
            ),
            Rule::AreaOfEffect => (
                "Area of effect",
                "An attack may strike a whole rank at once instead of a single target — width that cannot \
                 whiff against a crowd and **bypasses a group's spillover** (hits every member), at the \
                 price of not concentrating its force.",
                Behavior,
            ),
            Rule::Grouping => (
                "Grouping",
                "Same-side units may be bound at form-up into one unit (one position, one shared target, \
                 distinct Health): single-target damage **spills** through the front member in declared \
                 order (a bodyguard soaks for the squishies), a group sums its members' Tempo to block but \
                 needs every member to beat the attacker to slip — a superb wall and a hopeless slipper.",
                Behavior,
            ),
        };
        RuleInfo {
            name,
            description,
            kind,
        }
    }

    pub fn name(self) -> &'static str {
        self.info().name
    }

    pub fn is_phase(self) -> bool {
        self.info().kind == RuleKind::Phase
    }

    /// This rule's bit in a [`Ruleset`](crate::ruleset::Ruleset) enabled-mask (a fieldless enum, so the
    /// discriminant is a stable small index). There are well under 16 rules, so the mask fits a `u16`.
    pub fn bit(self) -> u16 {
        1u16 << (self as u16)
    }

    /// Parse a [`Rule`] from its **variant identifier** (the `Debug` name, e.g. `"Grouping"`,
    /// `"AreaOfEffect"`) — the stable, code-facing key a data file (a balance level's `rules_off`) names
    /// it by. `None` if no variant matches. (Distinct from [`name`](Rule::name), the prose appendix title.)
    pub fn from_ident(s: &str) -> Option<Rule> {
        ALL_RULES.iter().copied().find(|r| format!("{r:?}") == s)
    }
}

/// Every combat rule, in **round order** for the phases (the appendix and the engine read it in
/// sequence — the five sub-phase steps are the §4.6 schedule of the Engage phase), with the
/// cross-cutting behaviors last.
pub const ALL_RULES: &[Rule] = &[
    Rule::Marshal,
    Rule::Reveal,
    Rule::Ready,
    Rule::Engage,
    Rule::Intercept,
    Rule::Volley,
    Rule::Raid,
    Rule::Clash,
    Rule::Breach,
    Rule::WipePile,
    Rule::Refresh,
    Rule::TempoContest,
    Rule::StrikeBack,
    Rule::AreaOfEffect,
    Rule::Grouping,
];

/// Render the **phase-by-phase combat appendix** (the mechanical reference) from the registry. This is
/// the canonical mechanical text — generated, never hand-edited — and is distinct from the thematic
/// rulebook overview. Round phases are listed in round order, then the sub-phases of the Engage phase
/// (in §4.6 schedule order), then the cross-cutting behaviors.
pub fn appendix() -> String {
    let mut s = String::new();
    s.push_str("# Combat — phase-by-phase appendix\n\n");
    s.push_str(
        "> **Auto-generated from `crates/deckbound/src/rules.rs`** (the canonical mechanical text) — do \
         not edit by hand; regenerate with `cargo run -p deckbound --example handbook`. This is the \
         *mechanical* reference: each phase does exactly one thing, over two accumulators (the \
         per-sub-phase damage **pile** and **Tempo**). The thematic overview lives in the rulebook.\n\n",
    );
    s.push_str("## Phases (in round order)\n\n");
    let mut n = 1;
    for &r in ALL_RULES {
        let info = r.info();
        if info.kind == RuleKind::Phase {
            s.push_str(&format!(
                "{n}. **{}** — {}\n\n",
                info.name, info.description
            ));
            n += 1;
        }
    }
    s.push_str("## Sub-phases of the Engage phase (in schedule order)\n\n");
    let mut m = 1;
    for &r in ALL_RULES {
        let info = r.info();
        if info.kind == RuleKind::SubPhase {
            s.push_str(&format!(
                "{m}. **{}** — {}\n\n",
                info.name, info.description
            ));
            m += 1;
        }
    }
    s.push_str("## Cross-cutting behaviors\n\n");
    for &r in ALL_RULES {
        let info = r.info();
        if info.kind == RuleKind::Behavior {
            s.push_str(&format!("- **{}** — {}\n\n", info.name, info.description));
        }
    }
    mdtable::pad_tables(&s)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every rule has non-empty appendix text (the rules appendix can't have blanks), and ALL_RULES is
    /// complete + duplicate-free.
    #[test]
    fn every_rule_is_documented_and_listed() {
        for &r in ALL_RULES {
            let info = r.info();
            assert!(!info.name.is_empty(), "{r:?} has no name");
            assert!(info.description.len() > 20, "{r:?} has no real description");
        }
        // No duplicates in the round-order list.
        let mut seen = std::collections::HashSet::new();
        for &r in ALL_RULES {
            assert!(seen.insert(r), "{r:?} listed twice in ALL_RULES");
        }
    }
}
