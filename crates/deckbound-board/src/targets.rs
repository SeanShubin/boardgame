//! **The target table** - every `(step, attacker rank, target rank)` the canon round sequence permits,
//! denormalized, with the reach it is thrown with and the condition (if any) that gates it.
//!
//! The rules encode this across the step machine's eligibility and resolvers
//! (`rules::combat::step_game::StepState` and `rules::combat::steps`); to answer "who can hit whom, when?"
//! you would have to hold them in your head at once and join them yourself. So join them here, once, and
//! generate the answer - rather than write it down by hand somewhere it can quietly go stale.
//!
//! Rendered to `docs/games/deckbound/reference/combat-targets.md` by `examples/targets` (through
//! `mdtable::pad_tables`, the repo's one table style), and a test asserts the committed file still matches.
//! Change the schedule and the doc fails until it is regenerated. The canonical prose is
//! `docs/games/deckbound/combat-round-sequence.md`; this is the same schedule as a lookup table.

use rules::combat::step_game::{STEPS, Step, step_coord};

/// One row of the table: at this step, this rank may reach that rank, with this reach, under this condition.
pub struct TargetRow {
    pub step: Step,
    pub attacker: &'static str,
    pub target: &'static str,
    /// The reach the blow is thrown with.
    pub reach: &'static str,
    /// What must also be true, or `""` if the pairing is unconditional.
    pub condition: &'static str,
    /// Whether the target may answer along the edge: a mutual melee step trades both ways, a ranged shot is
    /// one-way.
    pub answerable: bool,
}

/// Every legal strike pairing, in schedule order - the step machine's reach rules, written out. The two
/// movement steps (Withdraw, Crossing) move bodies instead of striking, so they have no rows here.
pub fn rows() -> Vec<TargetRow> {
    vec![
        TargetRow {
            step: Step::Havoc,
            attacker: "Outrider",
            target: "anyone in its region",
            reach: "weapon",
            condition: "point-blank: both tiers, no screen; its hosts strike back in the same wave",
            answerable: true,
        },
        TargetRow {
            step: Step::Skirmish,
            attacker: "Vanguard",
            target: "Vanguard",
            reach: "melee",
            condition: "the early trade; a line strike here bars your own crossing this round",
            answerable: true,
        },
        TargetRow {
            step: Step::Volley,
            attacker: "Rearguard",
            target: "Outrider",
            reach: "ranged",
            condition: "one-way, the opening blow only",
            answerable: false,
        },
        TargetRow {
            step: Step::Raid,
            attacker: "Outrider",
            target: "Rearguard",
            reach: "melee",
            condition: "this round's arrivals only, in the region they landed in; opening blow only, evadable",
            answerable: false,
        },
        TargetRow {
            step: Step::Assault,
            attacker: "Vanguard",
            target: "Vanguard",
            reach: "melee",
            condition: "the late trade - every vanguard that held back swings here",
            answerable: true,
        },
        TargetRow {
            step: Step::Assault,
            attacker: "Rearguard",
            target: "Vanguard",
            reach: "ranged",
            condition: "",
            answerable: false,
        },
        TargetRow {
            step: Step::Advance,
            attacker: "Vanguard",
            target: "Rearguard",
            reach: "melee",
            condition: "only a rearguard with NO living vanguard at this step (the same-round advance)",
            answerable: true,
        },
        TargetRow {
            step: Step::Advance,
            attacker: "Rearguard",
            target: "Rearguard",
            reach: "ranged",
            condition: "only a rearguard with NO living vanguard at this step",
            answerable: false,
        },
    ]
}

/// The table as Markdown - the committed reference, emitted through the repo's one table style.
pub fn table_md() -> String {
    let mut md = String::from(
        "# Combat - who can target whom, and when\n\n\
         > **Auto-generated** from `deckbound_board::targets` (the step machine's reach rules, written out) -\n\
         > do not edit by hand; regenerate with `cargo run -p deckbound-board --example targets`. A test fails\n\
         > if it drifts. The canonical prose is `docs/games/deckbound/combat-round-sequence.md`.\n\n\
         The round is EIGHT steps, each its own declare/reveal wave, resolved on the spot - so a death at an\n\
         early step silences every later one. The two movement steps (2 Withdraw, 4 Crossing) move bodies\n\
         instead of striking: an outrider may rejoin its own line, and a vanguard that declared no line strike\n\
         may cross, landing as an Outrider.\n\n\
         **Answerable** means the target may strike back along the edge in the same wave: a mutual melee step\n\
         trades both ways because both declared; a ranged shot is one-way.\n\n",
    );
    md.push_str("| # | Step | Attacker | Reach | Target | Answerable | Condition |\n");
    md.push_str("|---|---|---|---|---|---|---|\n");
    for r in rows() {
        let (k, name) = step_coord(r.step);
        md.push_str(&format!(
            "| {k} | {name} | {} | {} | {} | {} | {} |\n",
            r.attacker,
            r.reach,
            r.target,
            if r.answerable { "yes" } else { "no" },
            if r.condition.is_empty() {
                "-"
            } else {
                r.condition
            },
        ));
    }
    md.push_str(
        "\n## The schedule at a glance\n\n\
         | # | Step | Who -> whom |\n\
         |---|---|---|\n\
         | 1 | Havoc | O->RV, RV->O (in-region, mutual) |\n\
         | 2 | Withdraw | O may move to its own line |\n\
         | 3 | Skirmish | V->V (bars the striker's crossing) |\n\
         | 4 | Crossing | V may move to their line (if it did not strike) |\n\
         | 5 | Defensive Volley | R->O (one-way, opening only) |\n\
         | 6 | Raid | O->R (this round's arrivals, opening only) |\n\
         | 7 | Assault | RV->V (all firepower to bear) |\n\
         | 8 | Advance | RV->R (only an unscreened back, AT this step) |\n",
    );
    // Emit through the repo's one table style, so the committed doc is byte-identical to what the tree-wide
    // `pad_tables` pass would produce - the golden test and the formatter can never fight.
    mdtable::pad_tables(&md)
}

/// **The schedule, as a card for the sidebar** - the eight steps and who reaches whom, compact enough for
/// 213px. The full table does not fit; mid-fight you need the one thing the schedule actually decides, which
/// is *when*.
pub fn schedule_card() -> Vec<String> {
    let mut out = vec!["The round - eight steps".to_string()];
    let lines = [
        "O <-> hosts, point-blank",
        "O may rejoin its line",
        "V -> V (bars your crossing)",
        "V may cross (if it held)",
        "R -> O, one-way",
        "O -> R, arrivals only",
        "RV -> V, all firepower",
        "RV -> exposed R",
    ];
    for (s, line) in STEPS.into_iter().zip(lines) {
        let (k, name) = step_coord(s);
        out.push(format!("{k} {name}: {line}"));
    }
    out.push(String::new());
    out.push("Minor steps: target, bid, strike, resolve".to_string());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The committed doc is the schedule written out. If the rules move and the doc does not, they disagree -
    /// and a reference that disagrees with the engine is worse than none.
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

    /// The sidebar card names all eight steps, in order.
    #[test]
    fn the_schedule_card_names_every_step() {
        let card = schedule_card();
        for s in STEPS {
            let (_, name) = step_coord(s);
            assert!(
                card.iter().any(|l| l.contains(name)),
                "the schedule card is missing {name}"
            );
        }
    }

    /// Only the raid reaches a SCREENED back line; every other route to a rearguard waits for its vanguard
    /// to fall - which is the whole reason the back line is safe, and the whole reason crossing is worth its
    /// exposure.
    #[test]
    fn only_the_raid_reaches_a_screened_back_line() {
        for r in rows().iter().filter(|r| r.target == "Rearguard") {
            if r.attacker == "Outrider" {
                assert!(
                    !r.condition.contains("NO living vanguard"),
                    "the raid slips the screen"
                );
            } else {
                assert!(
                    r.condition.contains("NO living vanguard"),
                    "screened: {} -> Rearguard must wait for the collapse",
                    r.attacker
                );
            }
        }
    }
}
