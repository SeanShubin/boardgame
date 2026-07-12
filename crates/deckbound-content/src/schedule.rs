//! The §4.6 **sub-phase schedule** — shared content: the fixed order in which rank-vs-rank strikes resolve.
//! Used by the sample game's resolver and by the card-table product's v2 combat, so it lives here so both
//! walk the same schedule.

use crate::rank::Intention;

/// §4.6 — the fixed **sub-phase schedule**: five sub-phases, each a list of `(attacker, target)` role
/// pairs resolved in order. This is the single source of truth shared by the sample resolver and the
/// steppable machine — they must walk it identically.
///
/// **It is a complete 3x3.** Every role gets exactly **one slot against each enemy rank** — the spec's
/// invariant that *every legal pair appears exactly once*:
///
/// |               | -> Vanguard | -> Outrider   | -> Rearguard |
/// |---------------|-------------|---------------|--------------|
/// | **Vanguard**  | Clash       | **Intercept** | Breach       |
/// | **Outrider**  | Breach      | Breach        | **Raid**     |
/// | **Rearguard** | Clash       | **Volley**    | Breach       |
///
/// So the schedule does not decide *who may strike whom* — everyone may eventually strike everyone. It
/// decides **when**. That is the whole of the interception / pre-empt machinery, and it is the Outrider's
/// signature: its **Rearguard** slot comes **early** (the Raid, 3rd) while every other role reaches the back
/// **last** (the Breach). It pays for that with total exposure first — the enemy front screens it (Intercept)
/// and the enemy back shoots it (Volley) before it lands. Tempo is the single budget across all three slots,
/// so no role can actually take every opportunity it is offered.
///
/// **An empty target rank simply voids that pairing — for every role, with no exception.** A Vanguard facing
/// no enemy Outriders loses its Intercept; a Rearguard facing none loses its Volley; an Outrider facing no
/// enemy Rearguard loses its Raid — and, exactly like the others, still has its remaining two slots in the
/// Breach. A misdeclared intent is punished by *timing*, not by silence: you crossed for a back line that was
/// not there, you ate the Intercept for it, and your blows now land **last**.
///
/// (An earlier version made the Raid *re-aim* down a priority list and deleted the Outrider's Breach pairs.
/// That was a mistake: it left the Outrider with one slot while the other roles kept three, on the false
/// premise that it had no other slot to fall back on. The real defect was that the log made a voided pairing
/// look like the unit had done nothing all round.)
pub const SCHEDULE: &[&[(Intention, Intention)]] = {
    use Intention::{Outrider, Rearguard, Vanguard};
    &[
        &[(Vanguard, Outrider)],  // Intercept - the front screens the crossers
        &[(Rearguard, Outrider)], // Volley - the back shoots the crossers (pre-empt)
        &[(Outrider, Rearguard)], // Raid - the flanker strikes the exposed back
        &[(Rearguard, Vanguard), (Vanguard, Vanguard)], // Clash - the lines meet
        &[
            // The deep / trailing blows land last. `V->R` and `R->R` pour through a *broken* line: both are
            // gated by the back-access rule (the target's Vanguard must have fallen), so they are no-ops
            // while the enemy front stands. The Outrider's other two slots sit here — it reached the back
            // early, and pays for that by reaching everything else late.
            (Vanguard, Rearguard),
            (Outrider, Vanguard),
            (Outrider, Outrider),
            (Rearguard, Rearguard),
        ], // Breach
    ]
};

/// The §4.6 sub-phase names, indexed by [`SCHEDULE`] position.
pub const SUB_PHASE_NAMES: [&str; 5] = ["Intercept", "Volley", "Raid", "Clash", "Breach"];

#[cfg(test)]
mod tests {
    use super::*;
    use Intention::{Outrider, Rearguard, Vanguard};

    /// **The schedule is a complete 3x3: every legal pair appears exactly once.** This is the spec's own
    /// invariant, and it is what makes the roles symmetric — each gets one slot against each enemy rank, and
    /// only the *timing* differs. Breaking it (as an earlier cascade did, by deleting the Outrider's Breach
    /// pairs) silently leaves one role with fewer opportunities than the others.
    #[test]
    fn every_legal_pair_appears_exactly_once() {
        let mut seen: Vec<(Intention, Intention)> = Vec::new();
        for pairs in SCHEDULE {
            for &p in *pairs {
                assert!(!seen.contains(&p), "{p:?} appears twice in the schedule");
                seen.push(p);
            }
        }
        assert_eq!(seen.len(), 9, "3 attacker ranks x 3 target ranks");
        for a in [Vanguard, Outrider, Rearguard] {
            for t in [Vanguard, Outrider, Rearguard] {
                assert!(seen.contains(&(a, t)), "{a:?} -> {t:?} is never scheduled");
            }
        }
    }

    /// Each role reaches the enemy **back** at a different time — and that difference *is* the Outrider.
    #[test]
    fn only_the_outrider_reaches_the_back_early() {
        let slot = |a: Intention, t: Intention| {
            SCHEDULE
                .iter()
                .position(|pairs| pairs.contains(&(a, t)))
                .expect("every pair is scheduled")
        };
        const RAID: usize = 2;
        const BREACH: usize = 4;

        assert_eq!(
            slot(Outrider, Rearguard),
            RAID,
            "the raid is early - the point of the role"
        );
        assert_eq!(
            slot(Vanguard, Rearguard),
            BREACH,
            "everyone else reaches the back last"
        );
        assert_eq!(slot(Rearguard, Rearguard), BREACH);

        // ...and it pays for that by reaching everything else last.
        assert_eq!(slot(Outrider, Vanguard), BREACH);
        assert_eq!(slot(Outrider, Outrider), BREACH);
    }
}
