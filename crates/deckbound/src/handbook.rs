//! **Generated player reference** — the same content the in-app encyclopedia and card catalog show,
//! rendered to committed markdown under `docs/games/deckbound/reference/` so it can be browsed and
//! diffed outside the running game.
//!
//! Two documents, both **generated projections** (source-of-truth: human-readable card / rules sheets
//! are generated from the print master, never hand-kept):
//! 1. **Card library** ([`card_library_md`]) — every card by cardset, with suit, level, kind, and a
//!    one-line effect, plus a per-set count. The lookup table for "what cards exist."
//! 2. **Rules reference** ([`rules_reference_md`]) — the encyclopedia, generated from the Spec's
//!    `TERM` lines and the passive powers' text (exactly [`crate::scenarios::glossary`]).
//!
//! Regenerate with `cargo run -p deckbound --example handbook`; the golden tests below fail the build
//! if the committed docs drift from the booklet / Spec.

use crate::scenarios::{LibraryRow, card_library, glossary};

const BANNER: &str = "> **GENERATED — do not edit.** Regenerate with `cargo run -p deckbound --example handbook`.\n\
                      > A projection of `crates/deckbound/data/booklet.ron` (the print master) and the Spec.";

/// Group rows into `(set, rows)` buckets, preserving first-seen set order.
fn by_set(rows: &[LibraryRow]) -> Vec<(String, Vec<&LibraryRow>)> {
    let mut out: Vec<(String, Vec<&LibraryRow>)> = Vec::new();
    for r in rows {
        match out.iter_mut().find(|(s, _)| *s == r.set) {
            Some((_, bucket)) => bucket.push(r),
            None => out.push((r.set.clone(), vec![r])),
        }
    }
    out
}

/// The **card library**: a counts summary, then one table per cardset (Level · Card · Kind · Effect).
pub fn card_library_md() -> String {
    let rows = card_library();
    let sets = by_set(&rows);
    let mut out = String::new();
    out.push_str("# Deckbound — Card Library\n\n");
    out.push_str(BANNER);
    out.push_str("\n\n");

    // Counts summary.
    out.push_str("## Counts\n\n| Cardset | Cards |\n| --- | ---: |\n");
    for (set, bucket) in &sets {
        out.push_str(&format!("| {set} | {} |\n", bucket.len()));
    }
    out.push_str(&format!("| **Total** | **{}** |\n\n", rows.len()));

    // One section per cardset.
    for (set, bucket) in &sets {
        out.push_str(&format!("## {set} ({} cards)\n\n", bucket.len()));
        out.push_str("| Level | Card | Kind | Effect |\n| ---: | --- | --- | --- |\n");
        for r in bucket {
            let level = r.level.map(|l| l.to_string()).unwrap_or_else(|| "—".into());
            let effect = if r.summary.is_empty() {
                "—"
            } else {
                &r.summary
            };
            out.push_str(&format!(
                "| {level} | {} | {} | {effect} |\n",
                r.name, r.kind
            ));
        }
        out.push('\n');
    }
    out
}

/// The **rules reference**: the encyclopedia entries grouped by category (the in-app Rules menu).
pub fn rules_reference_md() -> String {
    let entries = glossary(); // already ordered by the sidebar's category order
    let mut out = String::new();
    out.push_str("# Deckbound — Rules Reference\n\n");
    out.push_str(BANNER);
    out.push_str(
        "\n\nThe in-game encyclopedia, generated from the Spec's `TERM` definitions and the \
                  passive powers' card text.\n\n",
    );

    let mut cur = "";
    for e in &entries {
        if e.category != cur {
            if !cur.is_empty() {
                out.push('\n'); // blank line between categories
            }
            out.push_str(&format!("## {}\n\n", e.category));
            cur = &e.category;
        }
        out.push_str(&format!("- **{}** — {}\n", e.term, e.text));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn norm(s: &str) -> String {
        s.replace("\r\n", "\n")
    }

    /// Golden: the committed card library matches what the booklet generates. To update after an
    /// intended card change: `cargo run -p deckbound --example handbook`, then commit the docs.
    #[test]
    fn card_library_doc_is_current() {
        let got = card_library_md();
        let want = include_str!("../../../docs/games/deckbound/reference/card-library.md");
        assert_eq!(
            norm(&got),
            norm(want),
            "card-library.md drifted — regenerate with `cargo run -p deckbound --example handbook`"
        );
    }

    /// Golden: the committed rules reference matches the Spec/powers source.
    #[test]
    fn rules_reference_doc_is_current() {
        let got = rules_reference_md();
        let want = include_str!("../../../docs/games/deckbound/reference/rules-reference.md");
        assert_eq!(
            norm(&got),
            norm(want),
            "rules-reference.md drifted — regenerate with `cargo run -p deckbound --example handbook`"
        );
    }

    /// Golden: the committed phase-by-phase appendix matches the `rules.rs` registry.
    #[test]
    fn combat_phases_doc_is_current() {
        let got = crate::rules::appendix();
        let want = include_str!("../../../docs/games/deckbound/reference/combat-phases.md");
        assert_eq!(
            norm(&got),
            norm(want),
            "combat-phases.md drifted — regenerate with `cargo run -p deckbound --example handbook`"
        );
    }

    /// Each suit track covers all five levels (a level may bundle >1 card), and every role card in it
    /// carries its `(suit, level)`. Filtered by the track's set name so pool cards that share a name
    /// with a reward (e.g. Sunder) don't leak in.
    #[test]
    fn each_suit_track_covers_levels_1_to_5() {
        use crate::currency::Currency;
        let rows = card_library();
        for suit in [
            Currency::Iron,
            Currency::Silver,
            Currency::Brass,
            Currency::Bone,
            Currency::Salt,
        ] {
            let set = format!("{} — {}", suit.label(), suit.role().unwrap());
            let track: Vec<_> = rows.iter().filter(|r| r.set == set).collect();
            let levels: std::collections::BTreeSet<u32> =
                track.iter().filter_map(|r| r.level).collect();
            assert_eq!(
                levels,
                (1..=5).collect(),
                "{suit:?} track must cover levels 1..=5"
            );
            assert!(
                track
                    .iter()
                    .all(|r| r.suit == Some(suit) && r.level.is_some()),
                "{suit:?} role cards must carry their suit and level"
            );
        }
    }

    /// **Completeness guard** (the lesson of the silently-dropped Stat cards): a treasure grants *both*
    /// an ability card and a Stat card, and the library must show both — plus the Human baseline. This
    /// is the kind of invariant a golden snapshot can't enforce (a snapshot happily locks in an
    /// *incomplete* output); only an explicit "is everything accounted for?" check catches an omission.
    #[test]
    fn the_library_accounts_for_the_baseline_and_every_treasures_stat_card() {
        use crate::currency::Currency;
        let rows = card_library();
        assert!(
            rows.iter().any(|r| r.name == "Human" && r.kind == "stat"),
            "the Human baseline card must appear in the library"
        );
        for suit in [
            Currency::Iron,
            Currency::Silver,
            Currency::Brass,
            Currency::Bone,
            Currency::Salt,
        ] {
            let set = format!("{} — {}", suit.label(), suit.role().unwrap());
            let stat_levels: std::collections::BTreeSet<u32> = rows
                .iter()
                .filter(|r| r.set == set && r.kind == "stat")
                .filter_map(|r| r.level)
                .collect();
            assert_eq!(
                stat_levels,
                (1..=5).collect(),
                "{suit:?} must show its treasure's Stat card at every level"
            );
        }
    }
}
