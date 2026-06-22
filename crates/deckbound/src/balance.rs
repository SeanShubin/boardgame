//! The **balance-property harness** (§0.3 / §8.3) — *detecting* balance problems.
//!
//! A balance property is a **falsifiable claim about combat outcomes**, checked against the
//! **resolver-of-record** (the bounded [`auto_resolve`], §4 ratified v1 / §0.4 analysis envelope). Each
//! check builds a `(party, encounter)` and asserts a **win / loss**; a [`Violation`] is a property the
//! current content does not satisfy. The harness *measures* — it does not assert in CI yet (the
//! tutorial encounters are deliberately gentle and will show violations until calibrated); run the
//! ignored `probe_grind_balance` to print the report.
//!
//! The properties encode the design intent (the user's words, §8.3):
//! - **Tutorial / necessity** — a location's suit powers should *decide* it: the equipped party wins,
//!   the **unequipped party loses** (you suffer the absence of a treasure before gaining it).
//! - **Depth gating** — going deeper in a suit should be *much harder without the lower rewards*: an
//!   L4-equipped party wins L5; an L1-only party does not.
//! - **God viability** — an L5 should fall to **5 characters with the suit's L4**, *or* to a **single
//!   god** carrying many suits' L4 with the rest fodder (the god build is viable, if not optimal).
//!
//! The comparative-par properties (even-advancement ≤ god; 2–3 teams ≤ blob/solos) are run-level, not
//! encounter-level, and are the next layer (they need par over the world, not a single fight).

use crate::actor::Actor;
use crate::campaign::grind_encounter;
use crate::currency::Currency;
use crate::scenarios::{RewardId, build_character, build_encounter_foes, rewards_for};
use crate::solver::auto_resolve;
use crate::world::REWARD_SUITS;

/// A balance property the current content fails to satisfy against the resolver-of-record.
#[derive(Clone, Debug)]
pub struct Violation {
    /// `"<Suit>: <property>"` — what was being checked.
    pub property: String,
    /// The expected vs actual outcome.
    pub detail: String,
}

/// The reward ids of `suit` up to and including level `k` (the cumulative kit a `k`-deep clear yields).
fn rewards_up_to(suit: Currency, k: u32) -> Vec<RewardId> {
    rewards_for(suit)
        .into_iter()
        .filter(|r| r.level <= k)
        .collect()
}

/// A **god** kit: every suit's rewards up to level `k`, on one character (broad, not deep-per-suit).
fn god_rewards(k: u32) -> Vec<RewardId> {
    REWARD_SUITS
        .iter()
        .flat_map(|&s| rewards_up_to(s, k))
        .collect()
}

/// `n` clean-slate Novices each carrying `rewards`.
fn party(n: usize, rewards: &[RewardId]) -> Vec<Actor> {
    (0..n).map(|_| build_character("Novice", rewards)).collect()
}

/// Record a check: if the resolved outcome disagrees with `want_win`, it is a violation. `outcome`
/// is `Some(true)` win / `Some(false)` loss-or-draw / `None` non-resolving (also a failure to win).
fn check(
    v: &mut Vec<Violation>,
    suit: Currency,
    property: &str,
    outcome: Option<bool>,
    want_win: bool,
) {
    let got_win = outcome == Some(true);
    if got_win != want_win {
        v.push(Violation {
            property: format!("{}: {property}", suit.label()),
            detail: format!(
                "expected {}, got {}",
                if want_win { "win" } else { "loss/draw" },
                match outcome {
                    Some(true) => "win",
                    Some(false) => "loss/draw",
                    None => "non-resolving",
                }
            ),
        });
    }
}

/// Run the encounter-level balance properties over the 25-card grind ladder; returns every violation
/// (empty ⇒ the ladder satisfies the properties under the resolver-of-record). Checks each suit's **L5**
/// (the capstone band) — see the module docs for the properties.
pub fn check_grind_balance(seed: u64) -> Vec<Violation> {
    let mut v = Vec::new();
    for &suit in REWARD_SUITS.iter() {
        let enc = grind_encounter(suit, 5);
        let foes = || build_encounter_foes(&enc, 5);

        // Necessity: the suit's full kit wins; an unequipped party loses (the lesson decides it).
        let r = auto_resolve(party(5, &rewards_up_to(suit, 5)), foes(), seed);
        check(&mut v, suit, "equipped party (5×L5) wins L5", r, true);
        let r = auto_resolve(party(5, &[]), foes(), seed);
        check(
            &mut v,
            suit,
            "unequipped party loses L5 (lesson decides)",
            r,
            false,
        );

        // Depth gating: L4 suffices for L5; L1-only does not (you need the lower rewards).
        let r = auto_resolve(party(5, &rewards_up_to(suit, 4)), foes(), seed);
        check(&mut v, suit, "depth: 5×L4 wins L5", r, true);
        let r = auto_resolve(party(5, &rewards_up_to(suit, 1)), foes(), seed);
        check(&mut v, suit, "depth: 5×L1 loses L5", r, false);

        // God viability: one god (L4 across suits) + 4 fodder clears the L5.
        let mut god = vec![build_character("Novice", &god_rewards(4))];
        god.extend(party(4, &[]));
        let r = auto_resolve(god, foes(), seed);
        check(
            &mut v,
            suit,
            "god (L4 many suits) + fodder wins L5",
            r,
            true,
        );
    }
    v
}

/// A human-readable report of the violations (for the diagnostic probe / a future balance runner).
pub fn report(violations: &[Violation]) -> String {
    if violations.is_empty() {
        return "BALANCED: the grind ladder satisfies every checked property.".into();
    }
    let mut s = format!("{} balance violation(s):\n", violations.len());
    for vi in violations {
        s.push_str(&format!("  - {} — {}\n", vi.property, vi.detail));
    }
    s
}

/// T3 — **stat decisiveness** (§8.6 no-redundant-stat, coarse view): zero each offensive magnitude
/// stat across the grind-ladder parties and report whether it **flips** any L5 win/loss. This is a
/// *decisiveness* probe, not a consumption proof: a stat that is consumed but never tips an outcome
/// (e.g. it adds damage to a fight already won) reads as "not decisive here". The precise
/// no-redundant-stat guards are the focused `*_is_consumed_by_*` unit tests in `combat.rs`; a stat
/// that is decisive **nowhere** *and* consumed nowhere is dead (the old "Spirit"). Diagnostic;
/// run with `--ignored`. *(Defensive / pool stats are structurally consumed by `Defense::take`.)*
pub fn stat_necessity_report(seed: u64) -> String {
    type Zeroer = (&'static str, fn(&mut Actor));
    let zeroers: [Zeroer; 5] = [
        ("power (Strike)", |a| a.offense.power = 0),
        ("precision (Pierce)", |a| a.offense.precision = 0),
        ("daring", |a| a.offense.daring = 0),
        ("dread", |a| a.offense.dread = 0),
        ("inspiration", |a| a.offense.inspiration = 0),
    ];
    let mut out =
        String::from("stat decisiveness — zero-and-flip over the 5 suits' L5 (§8.6 T3, coarse):\n");
    for (name, zero) in zeroers {
        let mut flipped = 0;
        for &suit in REWARD_SUITS.iter() {
            let enc = grind_encounter(suit, 5);
            let rw = rewards_up_to(suit, 5);
            let base = auto_resolve(party(5, &rw), build_encounter_foes(&enc, 5), seed);
            let mut zeroed_party = party(5, &rw);
            zeroed_party.iter_mut().for_each(zero);
            let zeroed = auto_resolve(zeroed_party, build_encounter_foes(&enc, 5), seed);
            if base != zeroed {
                flipped += 1;
            }
        }
        out.push_str(&format!(
            "  {name:<20} {}\n",
            if flipped > 0 {
                format!("decisive ({flipped}/5 fights flip)")
            } else {
                "not decisive in the grind ladder (consumed-but-not-tipping, or unexercised)"
                    .to_string()
            }
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Diagnostic (run on demand): print the current balance violations.
    /// `cargo test -p deckbound probe_grind_balance -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_grind_balance() {
        let v = check_grind_balance(1);
        println!("{}", report(&v));
    }

    /// T3 probe (§8.6 no-redundant-stat): print which offensive magnitude stats are load-bearing.
    /// `cargo test -p deckbound probe_stat_necessity -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_stat_necessity() {
        println!("{}", stat_necessity_report(1));
    }

    #[test]
    fn an_equipped_party_never_loses_its_own_l5() {
        // Regression guard for the Salt anomaly (a suit's own powers made the party *worse* than bare):
        // a party fully equipped in a suit must clear that suit's L5. The remaining violations should
        // all be "too easy" (encounter calibration) — never an "equipped party / 5×L4 should win" that
        // came back a loss.
        // The "should win" checks all read "… wins L5"; the too-easy ones read "… loses L5". A
        // violated "wins L5" means a sufficiently-equipped party lost — the anomaly class.
        let equipped_losses: Vec<_> = check_grind_balance(1)
            .into_iter()
            .filter(|vi| vi.property.contains("wins L5"))
            .collect();
        assert!(
            equipped_losses.is_empty(),
            "an equipped party must never lose its own L5 (the Salt anomaly): {equipped_losses:?}"
        );
    }

    #[test]
    fn the_harness_runs_and_is_deterministic() {
        // The harness itself must be sound: it runs without panicking and is a pure function of the
        // seed (same seed ⇒ same verdict), so a violation count is a stable measurement to tune against.
        let a = check_grind_balance(1);
        let b = check_grind_balance(1);
        assert_eq!(
            a.len(),
            b.len(),
            "the harness is deterministic for a fixed seed"
        );
    }
}
