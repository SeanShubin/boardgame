//! The **rank / declared intention** (Vanguard / Outrider / Rearguard) — shared content: the sample game
//! calls it a declared *intention* (§4), the card-table product reframes it as a *rank*. Both depend on this
//! one definition so the vocabulary stays consistent across the split.

use serde::{Deserialize, Serialize};

/// A unit's declared **intention** for the round (§4) — the position it takes, and the role it plays in
/// the sub-phase schedule (§4.6). Re-declared each round; declaring is free and may *fail* (force-not-fiat).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Intention {
    /// Hold the line (front): the shield; screens enemy Outriders, fights the front, cleans up last.
    Vanguard,
    /// Break the line (flank): raids the enemy Rearguard directly, exposed to the enemy front and back first.
    Outrider,
    /// Deal from the back: fires/buffs/degrades from safety; the only answer to a Vanguard's Toughness.
    Rearguard,
}

impl Intention {
    /// The role this intention is **designed to beat** (its cycle prey, Hold>Break>Deal>Hold) — the
    /// efficient default spends scarce Tempo on its prey first, falling back only when none is crackable.
    pub fn prey(self) -> Intention {
        match self {
            Intention::Vanguard => Intention::Outrider,
            Intention::Outrider => Intention::Rearguard,
            Intention::Rearguard => Intention::Vanguard,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Intention::Vanguard => "Vanguard",
            Intention::Outrider => "Outrider",
            Intention::Rearguard => "Rearguard",
        }
    }
}
