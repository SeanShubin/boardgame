//! §8.3 — the **role tracks** (formerly currencies). The five role colours survive as **track
//! ids** (SD4): they key which role a reward belongs to and carry the role/colour provenance (§3.5).
//! The old spend economy is gone — there is **no currency, no balance** (§0.1 no path-dependent
//! budget); clearing a track-level *directly* unlocks its reward (§8.3).

use serde::{Deserialize, Serialize};

/// A role track id (§8.3). Five role colours + a generic `Gold` retained for non-track locations
/// (the reference scenario's neutral A / Final tiles). The five **role tracks** are [`Currency::TRACKS`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Currency {
    /// Wall — hold the line.
    Iron,
    /// Infiltrator — slip and assassinate.
    Silver,
    /// Artillery — ranged damage.
    Brass,
    /// Controller — strip foes / Fear.
    Bone,
    /// Support — heal / ward / aid.
    Salt,
    /// Generic — non-track (neutral locations); never a reward track (§8.5: the generic is a bundled
    /// Stat layer, not a sixth role).
    Gold,
}

/// The role track ids (a clearer name for the surviving enum, SD4).
pub type Track = Currency;

impl Currency {
    /// All six ids, in canonical order (Gold last — the generic non-track).
    pub const ALL: [Currency; 6] = [
        Currency::Iron,
        Currency::Silver,
        Currency::Brass,
        Currency::Bone,
        Currency::Salt,
        Currency::Gold,
    ];

    /// The five **role tracks** (Gold excluded — it is the generic, not a track, §8.5).
    pub const TRACKS: [Currency; 5] = [
        Currency::Iron,
        Currency::Silver,
        Currency::Brass,
        Currency::Bone,
        Currency::Salt,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Currency::Iron => "Iron",
            Currency::Silver => "Silver",
            Currency::Brass => "Brass",
            Currency::Bone => "Bone",
            Currency::Salt => "Salt",
            Currency::Gold => "Gold",
        }
    }

    /// The combat role this track funds — `None` for the generic Gold (§8.5).
    pub fn role(self) -> Option<&'static str> {
        Some(match self {
            Currency::Iron => "Wall",
            Currency::Silver => "Infiltrator",
            Currency::Brass => "Artillery",
            Currency::Bone => "Controller",
            Currency::Salt => "Support",
            Currency::Gold => return None,
        })
    }

    /// The **ranged weapon** a track confers when a character invests in it (§8.5 — a character *is*
    /// its role, so investing in a Rearguard role makes it deal from the back): Artillery's Bow,
    /// Controller's Wand — matching the booklet's role actors (Sear / Hex). `None` for the melee
    /// roles (Wall / Infiltrator) and Support, whose characters keep their base weapon and range.
    pub fn ranged_weapon(self) -> Option<&'static str> {
        match self {
            Currency::Brass => Some("Bow"),
            Currency::Bone => Some("Wand"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_role_tracks_plus_a_generic() {
        assert_eq!(Currency::TRACKS.len(), 5);
        let roles: Vec<_> = Currency::TRACKS.iter().filter_map(|c| c.role()).collect();
        assert_eq!(roles.len(), 5); // every track names a role
        assert_eq!(Currency::Gold.role(), None); // the generic is not a track
        assert!(!Currency::TRACKS.contains(&Currency::Gold));
    }
}
