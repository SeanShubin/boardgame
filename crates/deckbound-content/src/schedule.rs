//! The §4.6 **sub-phase schedule** — shared content: the fixed order in which rank-vs-rank strikes resolve.
//! Used by the sample game's resolver and by the card-table product's v2 combat, so it lives here so both
//! walk the same schedule.

use crate::rank::Intention;

/// §4.6 — the fixed **sub-phase schedule**: five sub-phases, each a list of `(attacker, target)` role
/// pairs resolved in order. This is the single source of truth shared by the sample resolver and the
/// steppable machine — they must walk it identically.
pub const SCHEDULE: &[&[(Intention, Intention)]] = {
    use Intention::{Outrider, Rearguard, Vanguard};
    &[
        &[(Vanguard, Outrider)],                        // Intercept
        &[(Rearguard, Outrider)],                       // Volley
        &[(Outrider, Rearguard)],                       // Raid
        &[(Rearguard, Vanguard), (Vanguard, Vanguard)], // Clash
        &[
            (Vanguard, Rearguard),
            (Outrider, Vanguard),
            (Outrider, Outrider),
            // §4.6 conditional pair: a Rearguard fires on the enemy back-line, but **only once the
            // enemy Vanguard has fallen** (the dropped screen opens the back). Gated by the back-access
            // rule in `policy::can_reach`, so it is a no-op while the enemy front lives.
            (Rearguard, Rearguard),
        ], // Breach
    ]
};

/// The §4.6 sub-phase names, indexed by [`SCHEDULE`] position.
pub const SUB_PHASE_NAMES: [&str; 5] = ["Intercept", "Volley", "Raid", "Clash", "Breach"];
