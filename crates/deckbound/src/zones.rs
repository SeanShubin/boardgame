//! §5 — the card **zone state-machine**: Hand / Active / Down, per-card **zone behavior**
//! (default-return · Spend · Lasting), and the **Recover** / **Disrupt** moves.
//!
//! Facing encodes *state, not secrecy* (§5.1): face-up = in play / available (Active, and the
//! held Hand), face-down = spent / dormant (Down). Cooldowns are emergent — Spend sends a card
//! Down, and only a Recover brings it back (§5.3). This is the substrate for stats-as-deck
//! (§2.3/§4.3): a character's Form (fundamental + attachments) lives in Active and derives its
//! stat block; Action cards move through these zones. See `canon/2-spec/README.md` §5.

use serde::{Deserialize, Serialize};

/// The three zones a card can occupy (§5.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Zone {
    /// *In your grip* — held, ready to play.
    Hand,
    /// *In play, in effect* — face-up on the table: Form, Lasting stances, charges.
    Active,
    /// *Spent / dormant* — face-down; recovered to Hand.
    Down,
}

impl Zone {
    /// Face-up = working / available (Active or the held Hand); face-down = spent (Down).
    pub fn is_face_up(self) -> bool {
        !matches!(self, Zone::Down)
    }
}

/// What a played card does to **itself** after resolving (§5.3). The **default is Return** —
/// the card goes back to Hand, reusable next turn (the Clash kit is all-Return). Keywords modify
/// it. (`Recover` and `Disrupt` are not self-behaviors — they are *effects* that move **other**
/// cards' zones; see [`Move`].)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ZoneBehavior {
    /// Default: a jab you keep throwing — play → Hand.
    #[default]
    Return,
    /// Used up, winded — play → Down (a one-shot until Recovered).
    Spend,
    /// A held stance / aura — play → Active (stays until removed / Disrupted / consumed).
    Lasting,
}

impl ZoneBehavior {
    /// The zone a card lands in after it is played from Hand.
    pub fn destination(self) -> Zone {
        match self {
            ZoneBehavior::Return => Zone::Hand,
            ZoneBehavior::Spend => Zone::Down,
            ZoneBehavior::Lasting => Zone::Active,
        }
    }
}

/// A zone-moving move a card's effect can apply to **another** card (§5.3). These are the two
/// verbs beyond the self-behaviors: the restore and the force-exhaust.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    /// *Catch your breath; stand a card back up* — Down → Hand (the restore; costs a beat/Tempo).
    Recover,
    /// *Stagger them — knock it down* — an enemy's Active / Hand → Down (force-exhaust).
    Disrupt,
}

impl Move {
    /// Resolve this move against a card currently in `from`, returning the zone it moves to —
    /// or `None` if the move does not apply to a card in that zone (e.g. Recover only acts on a
    /// Down card; Disrupt only on a face-up one).
    pub fn apply(self, from: Zone) -> Option<Zone> {
        match (self, from) {
            (Move::Recover, Zone::Down) => Some(Zone::Hand),
            (Move::Disrupt, Zone::Active | Zone::Hand) => Some(Zone::Down),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facing_encodes_state() {
        assert!(Zone::Hand.is_face_up());
        assert!(Zone::Active.is_face_up());
        assert!(!Zone::Down.is_face_up());
    }

    #[test]
    fn default_behavior_returns_to_hand() {
        assert_eq!(ZoneBehavior::default(), ZoneBehavior::Return);
        assert_eq!(ZoneBehavior::Return.destination(), Zone::Hand);
        assert_eq!(ZoneBehavior::Spend.destination(), Zone::Down);
        assert_eq!(ZoneBehavior::Lasting.destination(), Zone::Active);
    }

    #[test]
    fn cooldown_is_spend_then_recover() {
        // Play a Spend card → Down; only a Recover brings it back to Hand (the gap = the cooldown).
        let after_play = ZoneBehavior::Spend.destination();
        assert_eq!(after_play, Zone::Down);
        assert_eq!(Move::Recover.apply(after_play), Some(Zone::Hand));
        // A Recover does nothing to a card already in Hand/Active.
        assert_eq!(Move::Recover.apply(Zone::Hand), None);
    }

    #[test]
    fn disrupt_knocks_face_up_cards_down() {
        assert_eq!(Move::Disrupt.apply(Zone::Active), Some(Zone::Down));
        assert_eq!(Move::Disrupt.apply(Zone::Hand), Some(Zone::Down));
        // You cannot Disrupt a card that is already Down.
        assert_eq!(Move::Disrupt.apply(Zone::Down), None);
    }
}
