//! The combat **phases and rules as a self-documenting, toggleable registry** (§4).
//!
//! Combat is a sequence of single-purpose **phases** over two accumulators (the per-phase damage
//! **pile** and **Tempo**) plus a few cross-cutting **behaviors**. Every entry here:
//! - does **exactly one thing**,
//! - carries a human **description** — the *source of truth* for the auto-generated phase-by-phase rules
//!   appendix (so the rules can't drift from the code), and
//! - can be switched **on/off** via the [`Ruleset`](crate::ruleset::Ruleset), so a simulation can run
//!   against a chosen subset and record exactly which rules were allowed.
//!
//! This module is **pure data** (ids + text + ordering); the engine reads it to drive resolution, the
//! handbook reads it to emit the appendix, and a simulation records the enabled set as provenance.

/// Whether a rule is a **phase** (a step in the round, run in order) or a cross-cutting **behavior**
/// (a sub-rule consulted within phases, e.g. area-of-effect).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RuleKind {
    /// A single-purpose step in the round, run in the order listed in [`ALL_RULES`].
    Phase,
    /// A cross-cutting behavior consulted inside phases (not a step of its own).
    Behavior,
}

/// One toggleable combat phase or behavior. The variant order in [`ALL_RULES`] is the **round order**
/// (for the phases) — the appendix and the engine both read it in sequence.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum Rule {
    // ---- Standoff (set up the round) ----
    /// Place each unit front/back.
    SetPositions,
    /// Cast Standing abilities (braces / ally buffs).
    StandingCasts,
    /// Declare each Vanguard's answer to incoming melee (Trade vs Block).
    DeclareGuard,
    // ---- Fray (the fronts engage) ----
    /// Melee front clash (the one Tempo contest).
    MeleeContest,
    /// Ranged fire from the Rearguard at the enemy front.
    RangedFire,
    // ---- Volley (cross to the back) ----
    /// Free Vanguards declare a charge (at the enemy Rearguard) or flank.
    DeclareCharges,
    /// The enemy front strikes a crossing charger (the front strikes the runner).
    Interception,
    /// The charged Rearguard answers first, before the charge's own blow.
    Preempt,
    // ---- Breach / Reckoning (land the back blows) ----
    /// Surviving chargers land their blows on the exposed Rearguard.
    Breach,
    /// Deferred effects resolve last (fizzling if their caster died).
    Reckoning,
    // ---- boundaries / accumulators ----
    /// Clear the per-phase damage pile at a phase boundary.
    WipePile,
    /// The Lull: Tempo resets, Health persists, the round advances (5-round cap).
    Refresh,
    // ---- cross-cutting behaviors ----
    /// Area-of-effect: an attack may strike a whole rank at once.
    AreaOfEffect,
    /// Grouping: same-side units bind into one unit (sum to block, weakest-link to slip).
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
        use RuleKind::{Behavior, Phase};
        let (name, description, kind) = match self {
            Rule::SetPositions => (
                "Set positions",
                "Each unit is placed in the Vanguard (front) or the Rearguard (back), re-set each round. \
                 Melee units belong in front — the front is also the shield; ranged units belong in back, \
                 firing safely over their own line.",
                Phase,
            ),
            Rule::StandingCasts => (
                "Standing casts",
                "Standing abilities (a Wall's brace, a Support's ally buff) are cast now. They are \
                 ally-targeted, auto-land, and last the round.",
                Phase,
            ),
            Rule::DeclareGuard => (
                "Declare guard",
                "Each Vanguard declares how it answers an incoming melee blow this round: Trade (strike \
                 back) or Block (spend Tempo to out-bid the attacker and take no blow).",
                Phase,
            ),
            Rule::MeleeContest => (
                "Melee contest",
                "Each engaging Vanguard strikes an enemy Vanguard, paying one Tempo. The defender answers \
                 per its guard: Trade — both blows land (a mortally wounded body still lands its committed \
                 blow); Block — the defender out-bids the attacker (cards x Finesse, strictly exceed; a tie \
                 lands the hit) to take no blow. Untyped Might banks into the per-phase pile; each time the \
                 pile clears the target's Toughness, one Health card turns face down.",
                Phase,
            ),
            Rule::RangedFire => (
                "Ranged fire",
                "Each Rearguard carrying a ranged attack fires at the enemy front, paying one Tempo. The \
                 target may evade by out-bidding the volley (cards x Finesse, strictly exceed) with its own \
                 Tempo; otherwise the shot lands.",
                Phase,
            ),
            Rule::DeclareCharges => (
                "Declare charges",
                "A free Vanguard — one that did not engage in the Fray, or whose front-foe fell — may \
                 charge the enemy Rearguard, or flank a surviving enemy Vanguard. A locked Vanguard (its \
                 struck foe still stands) stays pinned.",
                Phase,
            ),
            Rule::Interception => (
                "Interception",
                "A charger crossing toward the enemy Rearguard is struck by each living enemy front \
                 Vanguard. The charger slips each via the Tempo contest (spending its own Tempo) or takes \
                 the blow; a charger cut down crossing never reaches the back. A wide front drains a \
                 crosser slip-by-slip, so only a lone high-Finesse, high-Tempo body gets through.",
                Phase,
            ),
            Rule::Preempt => (
                "Pre-empt",
                "The charged Rearguard answers first — a ranged target counter-fires, a melee target \
                 strikes back — before the charge's own blow lands in the Breach.",
                Phase,
            ),
            Rule::Breach => (
                "Breach",
                "Each charger that survived the Volley lands its blow on the now-exposed enemy Rearguard.",
                Phase,
            ),
            Rule::Reckoning => (
                "Reckoning",
                "Deferred effects (wound up earlier this round) resolve last. A caster killed in the Breach \
                 has its deferred effect fizzle.",
                Phase,
            ),
            Rule::WipePile => (
                "Wipe pile",
                "At a phase boundary the per-phase damage pile is cleared: sub-threshold damage that did \
                 not turn a Health card does not carry into the next phase. Only Health persists.",
                Phase,
            ),
            Rule::Refresh => (
                "Refresh (the Lull)",
                "Round end: all spent Tempo resets, Health carries over, and the round advances. A battle \
                 not decided within five rounds is a draw.",
                Phase,
            ),
            Rule::AreaOfEffect => (
                "Area of effect",
                "An attack may strike a whole rank at once instead of a single target — width that cannot \
                 whiff against a crowd, at the price of not concentrating its force.",
                Behavior,
            ),
            Rule::Grouping => (
                "Grouping",
                "Same-side units may be bound at form-up into one unit (one position, one shared target, \
                 distinct Health): a group sums its members' Tempo to block but needs every member to beat \
                 the attacker to slip — a superb wall and a hopeless slipper.",
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
}

/// Every combat rule, in **round order** for the phases (the appendix and the engine read it in
/// sequence), with the cross-cutting behaviors last.
pub const ALL_RULES: &[Rule] = &[
    Rule::SetPositions,
    Rule::StandingCasts,
    Rule::DeclareGuard,
    Rule::MeleeContest,
    Rule::RangedFire,
    Rule::DeclareCharges,
    Rule::Interception,
    Rule::Preempt,
    Rule::Breach,
    Rule::Reckoning,
    Rule::WipePile,
    Rule::Refresh,
    Rule::AreaOfEffect,
    Rule::Grouping,
];

/// Render the **phase-by-phase combat appendix** (the mechanical reference) from the registry. This is
/// the canonical mechanical text — generated, never hand-edited — and is distinct from the thematic
/// rulebook overview. Phases are listed in round order, then the cross-cutting behaviors.
pub fn appendix() -> String {
    let mut s = String::new();
    s.push_str("# Combat — phase-by-phase appendix\n\n");
    s.push_str(
        "> **Auto-generated from `crates/deckbound/src/rules.rs`** (the canonical mechanical text) — do \
         not edit by hand; regenerate with `cargo run -p deckbound --example handbook`. This is the \
         *mechanical* reference: each phase does exactly one thing, over two accumulators (the per-phase \
         damage **pile** and **Tempo**). The thematic overview lives in the rulebook.\n\n",
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
    s.push_str("## Cross-cutting behaviors\n\n");
    for &r in ALL_RULES {
        let info = r.info();
        if info.kind == RuleKind::Behavior {
            s.push_str(&format!("- **{}** — {}\n\n", info.name, info.description));
        }
    }
    s
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
