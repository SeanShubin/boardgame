//! The §4.6 **sub-phase schedule** — shared content: the fixed order in which rank-vs-rank strikes resolve.
//! Used by the sample game's resolver and by the card-table product's v2 combat, so it lives here so both
//! walk the same schedule.

use crate::rank::Intention;

/// §4.6 — the fixed **sub-phase schedule**: five sub-phases, each a list of `(attacker, target priority)`
/// entries resolved in order. This is the single source of truth shared by the sample resolver and the
/// steppable machine — they must walk it identically.
///
/// A target is not a single rank but an **ordered preference**: an attacker strikes the first rank in its
/// list that has anything reachable in it (see [`target_rank`]). Most entries name one rank and so are
/// simply "attacks that rank, or nobody".
///
/// **The Outrider is why.** Declaring Outrider is a statement of intent — *go for the Rearguard first, then
/// the Vanguard, then the other Outriders* — and it is the only role whose one offensive slot (Raid) can be
/// voided by the enemy simply not fielding the rank it crossed for. A Vanguard and a Rearguard each get a
/// separate slot against each rank (Intercept / Clash / Breach), so an empty rank costs them an opportunity
/// they never had; there is nothing to re-aim *to*. The Outrider has one, and standing idle beside a foe
/// because the back line it wanted does not exist is the one moment the game would say "you may not react to
/// what you see" — which contradicts its own rule that *your declaration fixes what happens **to** you, the
/// field fixes what you **do***.
///
/// So the cascade is **data, not an exception**: every entry has a priority list, the Outrider's simply has
/// three ranks in it. And it re-aims **in its own slot** (Raid), rather than waiting for a late fallback —
/// which is why Breach no longer carries `O->V` / `O->O`. The Outrider has exactly one offensive slot; it is
/// never a second bite.
pub const SCHEDULE: &[&[(Intention, &[Intention])]] = {
    use Intention::{Outrider, Rearguard, Vanguard};
    &[
        &[(Vanguard, &[Outrider] as &[Intention])], // Intercept — the front screens the flankers
        &[(Rearguard, &[Outrider])],                // Volley — the back fires on the flankers
        // Raid — the Outrider's one offensive slot. It crossed for the Rearguard; failing that it falls on
        // the front; failing that, on whoever else crossed.
        &[(Outrider, &[Rearguard, Vanguard, Outrider])],
        &[(Rearguard, &[Vanguard]), (Vanguard, &[Vanguard])], // Clash — the lines meet
        &[
            (Vanguard, &[Rearguard]),
            // §4.6 conditional pair: a Rearguard fires on the enemy back-line, but **only once the
            // enemy Vanguard has fallen** (the dropped screen opens the back). Gated by the back-access
            // rule in `policy::can_reach`, so it is a no-op while the enemy front lives.
            (Rearguard, &[Rearguard]),
        ], // Breach
    ]
};

/// The rank an attacker of rank `atk` actually strikes in sub-phase `sub`: the **first** rank in its target
/// priority that `reachable` accepts. `None` if it has no entry this sub-phase, or nothing it wants is there.
///
/// `reachable(rank)` answers "is there a living enemy in that rank that this attacker can actually get at" —
/// which includes the back-access screen, and is why it is the caller's to supply: each engine knows its own
/// units. Both engines must resolve targets through this one function, or they stop walking the same schedule.
pub fn target_rank(
    sub: usize,
    atk: Intention,
    reachable: impl Fn(Intention) -> bool,
) -> Option<Intention> {
    SCHEDULE
        .get(sub)?
        .iter()
        .find(|(a, _)| *a == atk)?
        .1
        .iter()
        .copied()
        .find(|&t| reachable(t))
}

/// The §4.6 sub-phase names, indexed by [`SCHEDULE`] position.
pub const SUB_PHASE_NAMES: [&str; 5] = ["Intercept", "Volley", "Raid", "Clash", "Breach"];

#[cfg(test)]
mod tests {
    use super::*;
    use Intention::{Outrider, Rearguard, Vanguard};

    const RAID: usize = 2;
    const BREACH: usize = 4;

    /// The Outrider crossed for the Rearguard, so it takes one when there is one.
    #[test]
    fn an_outrider_raids_the_rearguard_when_there_is_one() {
        let all_there = |_r| true;
        assert_eq!(target_rank(RAID, Outrider, all_there), Some(Rearguard));
    }

    /// With no enemy Rearguard, it does not stand idle beside the foe in front of it — it falls on the front.
    /// This is the hole the cascade closes: the strike used to be skipped ("no legal target") and the unit
    /// did nothing until Breach.
    #[test]
    fn a_stranded_outrider_falls_on_the_front_in_its_own_slot() {
        let no_back_line = |r| r != Rearguard;
        assert_eq!(target_rank(RAID, Outrider, no_back_line), Some(Vanguard));

        // ...and failing even that, on whoever else crossed.
        let only_outriders = |r| r == Outrider;
        assert_eq!(target_rank(RAID, Outrider, only_outriders), Some(Outrider));

        // With nothing at all left, it strikes nothing (rather than something it was never aimed at).
        assert_eq!(target_rank(RAID, Outrider, |_| false), None);
    }

    /// **No double dip.** The Outrider re-aims in its own slot, so Breach must not hand it a second bite: its
    /// old `O->V` / `O->O` fallback pairs are gone.
    #[test]
    fn breach_gives_the_outrider_no_second_strike() {
        assert_eq!(target_rank(BREACH, Outrider, |_| true), None);
    }

    /// A Vanguard and a Rearguard need no cascade: each already has a separate slot per rank, so an empty
    /// rank costs an opportunity they never had. Their entries name exactly one target.
    #[test]
    fn the_other_roles_name_exactly_one_target_each() {
        for (sub, entries) in SCHEDULE.iter().enumerate() {
            for (atk, targets) in entries.iter() {
                if *atk != Outrider {
                    assert_eq!(
                        targets.len(),
                        1,
                        "sub-phase {sub}: {atk:?} should name exactly one target, got {targets:?}"
                    );
                }
            }
        }
        assert_eq!(target_rank(0, Vanguard, |_| true), Some(Outrider)); // Intercept
        assert_eq!(target_rank(1, Rearguard, |_| true), Some(Outrider)); // Volley
        // An empty rank simply costs the opportunity - there is nothing to re-aim to.
        assert_eq!(target_rank(0, Vanguard, |r| r != Outrider), None);
    }
}
