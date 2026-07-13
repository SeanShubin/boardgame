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

/// **The 3x3, as a card for the sidebar** — who reaches whom, and *when*. Same [`rows`] the reference doc is
/// built from, so the card on the table and the doc on disk cannot tell different stories.
///
/// The full table does not fit: the Condition column alone wants ~400px and the sidebar has 213. But the full
/// table is not what you need mid-fight - you need the one thing the schedule actually decides, which is
/// **when** each rank reaches each rank. Everything else on the card is a footnote to that.
pub fn schedule_card() -> Vec<String> {
    let letter = |r: Rank| match r {
        Rank::Vanguard => "V",
        Rank::Outrider => "O",
        Rank::Rearguard => "R",
    };
    let rows = rows();
    let when = |a: Rank, t: Rank| {
        rows.iter()
            .find(|r| r.attacker == a && r.target == t)
            .map(|r| {
                // Mark the pairings the screen gates - they are no-ops while the enemy front stands, and a
                // player who does not know that reads an empty Breach as a bug.
                let star = if r.condition.is_empty() { "" } else { "*" };
                format!("{}{star}", r.phase)
            })
            .unwrap_or_else(|| "-".into())
    };

    const RANKS: [Rank; 3] = [Rank::Vanguard, Rank::Outrider, Rank::Rearguard];

    // **Each column is as wide as its own widest cell**, and no wider. A fixed width does not align anything:
    // `{:<6}` is a *minimum*, so one long cell ("Intercept", 9) simply shoves every column after it along, and
    // the table is only a table by accident. Measure the content and lay the columns out to it.
    let cell = |a: Rank, t: Rank| when(a, t);
    let widths: Vec<usize> = RANKS
        .iter()
        .map(|&t| {
            RANKS
                .iter()
                .map(|&a| cell(a, t).len())
                .chain(std::iter::once(3)) // the "->V" header
                .max()
                .unwrap_or(3)
        })
        .collect();

    let mut header = "     ".to_string(); // clears the "  V  " row label
    for (i, &t) in RANKS.iter().enumerate() {
        header.push_str(&format!(
            "{:<w$} ",
            format!("->{}", letter(t)),
            w = widths[i]
        ));
    }
    let mut out = vec![
        "Who reaches whom".to_string(),
        header.trim_end().to_string(),
    ];
    for &a in &RANKS {
        let mut row = format!("  {}  ", letter(a));
        for (i, &t) in RANKS.iter().enumerate() {
            row.push_str(&format!("{:<w$} ", cell(a, t), w = widths[i]));
        }
        out.push(row.trim_end().to_string());
    }
    out.push("  * needs their Vanguard down".to_string());
    out
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

    /// **The card is a table, so its columns must actually line up.** They are aligned with spaces in a
    /// monospace face, which means alignment is a property of the *string* - and `{:<6}` does not give it to
    /// you, because a fixed width is a MINIMUM: one long cell ("Intercept", 9 chars) shoves every column after
    /// it along, and the table becomes a table only by accident.
    ///
    /// So assert what the eye asserts: every cell in a column starts at the same offset, on every row.
    #[test]
    fn the_schedule_card_columns_line_up() {
        let card = schedule_card();
        let rows: Vec<&String> = card
            .iter()
            .filter(|l| l.starts_with("  V") || l.starts_with("  O") || l.starts_with("  R"))
            .collect();
        assert_eq!(rows.len(), 3, "one row per attacker rank");

        // The column starts of a line: the index of each run of non-space that follows a space.
        let starts = |l: &str| -> Vec<usize> {
            let b = l.as_bytes();
            (1..b.len())
                .filter(|&i| b[i] != b' ' && b[i - 1] == b' ')
                .collect()
        };
        let want = starts(rows[0]);
        assert_eq!(want.len(), 4, "a row label and three cells: {}", rows[0]);
        for r in &rows[1..] {
            assert_eq!(
                starts(r),
                want,
                "every column must start at the same offset on every row:\n  {}\n  {}",
                rows[0],
                r
            );
        }
        // ...and the header's arrows sit over the cells they head.
        let header = card
            .iter()
            .find(|l| l.contains("->V"))
            .expect("the card has a header");
        assert_eq!(
            starts(header),
            want[1..].to_vec(),
            "the ->V/->O/->R headers must sit over their columns:\n  {header}\n  {}",
            rows[0]
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
