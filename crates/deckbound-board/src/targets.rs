//! **The target table** — every `(sub-phase, attacker rank, target rank)` the rules permit, denormalized, with
//! the reach it is thrown with and the condition (if any) that gates it.
//!
//! The schedule is a *terse* thing: five lists of rank pairs, with the range rule and the screen rule living
//! somewhere else entirely ([`combat::rank_is_ranged`], [`combat::back_access_ok`]). To answer "who can hit
//! whom, when?" you have to hold three files in your head at once and join them yourself. So join them here,
//! once, and generate the answer — rather than write it down by hand somewhere it can quietly go stale, which
//! is precisely what happened to the Grit card and the Raid rule text.
//!
//! Rendered to `docs/games/deckbound/reference/combat-targets.md` by `examples/targets`, and a test asserts the
//! committed file still matches. Change the schedule and the doc fails until it is regenerated.

use deckbound_content::rank::Intention as Rank;
use deckbound_content::schedule::{SCHEDULE, SUB_PHASE_NAMES};

use crate::combat;

/// One row of the table: in this sub-phase, this rank may reach that rank, with this reach, under this
/// condition.
pub struct TargetRow {
    pub phase: &'static str,
    pub attacker: Rank,
    pub target: Rank,
    /// The reach the blow is thrown with — decided by the *attacker's position*, not its body (spec 4.2).
    pub reach: &'static str,
    /// What must also be true, or `""` if the pairing is unconditional.
    pub condition: &'static str,
    /// Whether the target may answer along the edge: melee contact is mutual, ranged contact is one-way.
    pub answerable: bool,
}

fn word(r: Rank) -> &'static str {
    match r {
        Rank::Vanguard => "Vanguard",
        Rank::Outrider => "Outrider",
        Rank::Rearguard => "Rearguard",
    }
}

/// Every legal pairing, in schedule order. Nine of them — the schedule is a complete 3x3 — spread across the
/// five sub-phases by *when*, which is the only thing the schedule decides.
pub fn rows() -> Vec<TargetRow> {
    let mut out = Vec::new();
    for (i, pairs) in SCHEDULE.iter().enumerate() {
        for &(attacker, target) in *pairs {
            let ranged = combat::rank_is_ranged(attacker);
            // The screen (back-access): a Rearguard fights from behind its own line, so it is reachable by a
            // Vanguard or a Rearguard only once that line has fallen. An Outrider slips past it at any time -
            // that is what it crossed for, and what it ate the Intercept and the Volley to buy.
            let condition = if target == Rank::Rearguard && attacker != Rank::Outrider {
                "only once the target's own Vanguard has fallen (the screen)"
            } else {
                ""
            };
            out.push(TargetRow {
                phase: SUB_PHASE_NAMES[i],
                attacker,
                target,
                reach: if ranged { "ranged" } else { "melee" },
                condition,
                answerable: !ranged,
            });
        }
    }
    out
}

/// The table as Markdown — the committed reference.
pub fn table_md() -> String {
    let mut md = String::from(
        "# Combat - who can target whom, and when\n\n\
         > **Auto-generated** from `deckbound_content::schedule::SCHEDULE` joined with the range rule\n\
         > (`combat::rank_is_ranged`) and the screen rule (`combat::back_access_ok`) - do not edit by hand;\n\
         > regenerate with `cargo run -p deckbound-board --example targets`. A test fails if it drifts.\n\n\
         The schedule is a **complete 3x3**: every rank gets exactly one slot against each enemy rank, so it\n\
         does not decide *who* may hit *whom* - everyone eventually reaches everyone. It decides **when**. An\n\
         empty target rank simply voids that pairing, for every rank, with no exception.\n\n\
         **Answerable** means the target may spend its own tempo striking back along the edge. A melee contact\n\
         is mutual - the body you engaged did not choose the fight, and you could have let it pass. A ranged\n\
         contact is one-way: you cannot punch an archer at range.\n\n\
         | Sub-phase | Attacker | Reach | Target | Answerable | Condition |\n\
         |---|---|---|---|---|---|\n",
    );
    for r in rows() {
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            r.phase,
            word(r.attacker),
            r.reach,
            word(r.target),
            if r.answerable { "yes" } else { "no" },
            if r.condition.is_empty() {
                "-"
            } else {
                r.condition
            },
        ));
    }

    // The same nine pairings, read the other way: when does each rank reach each rank? This is the view that
    // makes the Outrider's whole bargain visible in one glance - it reaches the back FIRST and everything else
    // LAST, and it pays for that by being the only rank the front and the back both shoot at on the way in.
    md.push_str("\n## The 3x3, by *when*\n\n| Attacker \\ Target | Vanguard | Outrider | Rearguard |\n|---|---|---|---|\n");
    for a in [Rank::Vanguard, Rank::Outrider, Rank::Rearguard] {
        md.push_str(&format!("| **{}** ", word(a)));
        for t in [Rank::Vanguard, Rank::Outrider, Rank::Rearguard] {
            let when = rows()
                .iter()
                .find(|r| r.attacker == a && r.target == t)
                .map(|r| r.phase)
                .unwrap_or("-");
            md.push_str(&format!("| {when} "));
        }
        md.push_str("|\n");
    }
    md
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The committed doc is the schedule, joined with the rules that gate it. If the schedule moves and the
    /// doc does not, they disagree - and a reference that disagrees with the engine is worse than none.
    #[test]
    fn the_committed_table_is_current() {
        let want = include_str!("../../../docs/games/deckbound/reference/combat-targets.md");
        let norm = |s: &str| s.replace("\r\n", "\n");
        assert_eq!(
            norm(&table_md()),
            norm(want),
            "combat-targets.md drifted - regenerate with `cargo run -p deckbound-board --example targets`"
        );
    }

    /// Every legal pairing appears, exactly once - the 3x3 the schedule is.
    #[test]
    fn the_table_is_the_complete_three_by_three() {
        let rows = rows();
        assert_eq!(rows.len(), 9, "3 attacker ranks x 3 target ranks");
        for a in [Rank::Vanguard, Rank::Outrider, Rank::Rearguard] {
            for t in [Rank::Vanguard, Rank::Outrider, Rank::Rearguard] {
                assert_eq!(
                    rows.iter()
                        .filter(|r| r.attacker == a && r.target == t)
                        .count(),
                    1,
                    "{a:?} -> {t:?} appears exactly once"
                );
            }
        }
    }

    /// A Rearguard is screened from everything except an Outrider - which is the whole reason the back line is
    /// safe, and the whole reason the Outrider is worth its exposure.
    #[test]
    fn only_the_outrider_reaches_an_unbroken_back_line() {
        for r in rows().iter().filter(|r| r.target == Rank::Rearguard) {
            if r.attacker == Rank::Outrider {
                assert_eq!(r.condition, "", "the raid slips the screen");
            } else {
                assert!(r.condition.contains("Vanguard has fallen"), "screened");
            }
        }
    }
}
