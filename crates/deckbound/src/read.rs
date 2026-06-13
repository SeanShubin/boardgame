//! The Mind's rock-paper-scissors read and the cycle that resolves two of them.
//!
//! `Strike → Scheme → Defense → Strike`: a Strike beats a Scheme (interrupting
//! the setup), a Defense (Block/Evade) beats a Strike (absorb or dodge), and a
//! Scheme beats a Defense (you set up while they guarded nothing). Same
//! categories mirror, and the magnitude layer (tempo, then damage) settles them.

/// A single read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Read {
    Strike,
    Block,
    Evade,
    Scheme,
}

impl Read {
    /// A short display name.
    pub fn name(self) -> &'static str {
        match self {
            Read::Strike => "Strike",
            Read::Block => "Block",
            Read::Evade => "Evade",
            Read::Scheme => "Scheme",
        }
    }

    /// Block and Evade are the two Defenses — same role on the cycle.
    pub fn is_defense(self) -> bool {
        matches!(self, Read::Block | Read::Evade)
    }

    fn category(self) -> Category {
        match self {
            Read::Strike => Category::Strike,
            Read::Block | Read::Evade => Category::Defense,
            Read::Scheme => Category::Scheme,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Category {
    Strike,
    Defense,
    Scheme,
}

/// Who prevails when an attacker's read meets a defender's read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clash {
    /// The attacker's read wins — its offense lands, the defender's is spoiled.
    AttackerWins,
    /// The defender's read wins — the attack is negated.
    DefenderWins,
    /// Same category — settled by the magnitude layer (tempo, then both land).
    Mirror,
}

/// Resolve `attacker` versus `defender` on the cycle.
pub fn clash(attacker: Read, defender: Read) -> Clash {
    use Category::*;
    let (a, d) = (attacker.category(), defender.category());
    if a == d {
        return Clash::Mirror;
    }
    let attacker_wins = matches!(
        (a, d),
        (Strike, Scheme) | (Scheme, Defense) | (Defense, Strike)
    );
    if attacker_wins {
        Clash::AttackerWins
    } else {
        Clash::DefenderWins
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defense_beats_strike() {
        assert_eq!(clash(Read::Strike, Read::Block), Clash::DefenderWins);
        assert_eq!(clash(Read::Strike, Read::Evade), Clash::DefenderWins);
    }

    #[test]
    fn strike_shatters_a_scheme() {
        assert_eq!(clash(Read::Strike, Read::Scheme), Clash::AttackerWins);
    }

    #[test]
    fn scheme_beats_a_guard() {
        assert_eq!(clash(Read::Scheme, Read::Block), Clash::AttackerWins);
    }

    #[test]
    fn like_categories_mirror() {
        assert_eq!(clash(Read::Strike, Read::Strike), Clash::Mirror);
        assert_eq!(clash(Read::Block, Read::Evade), Clash::Mirror);
    }
}
