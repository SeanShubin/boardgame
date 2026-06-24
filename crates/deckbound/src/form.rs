//! §2.3 / §4.3 — **stats-as-deck**: a character's stat block is *read off its Form*, not authored
//! on the actor. A [`StatCard`] carries one card's stat contribution over the **five** stats; a
//! [`Form`] is the fundamental (`base`) card + its attachments (in the Active zone, §5.2), summed into
//! the [`Offense`](crate::stats::Offense) / [`Defense`](crate::stats::Defense) the rest of the engine
//! reads. Combination is **commutative** (§5.5).

use crate::stats::{Defense, Offense};

use serde::Deserialize;

/// One card's contribution to the Form stat block over the **five** stats (Spec §2.4): `might`,
/// `vitality` (Health-card count), `toughness` (per-card bar), `cadence` (Tempo count), `finesse`
/// (per-Tempo-card grade). The **fundamental** (`base`) card sets the base; each **attachment** (a
/// reward, or a bought Upgrade, §8.3) adds on top. Health pool = `vitality` (count) × `toughness`
/// (value). No channel / armor / damage-type fields — those are deferred with gear (§2.2).
#[derive(Clone, Debug, Default, Deserialize)]
pub struct StatCard {
    #[serde(default)]
    pub name: String,
    /// Flat strike force (Power-only magnitude, §2.4). Formerly `power`.
    #[serde(default)]
    pub might: u32,
    /// Health-pool **count** (§2.4): how many Health cards. Formerly `body`.
    #[serde(default)]
    pub vitality: u32,
    /// Per-Health-card **bar** (§2.4): the magnitude a hit must clear to flip a card.
    #[serde(default)]
    pub toughness: u32,
    /// Tempo-pool **count** (§3): how many Tempo cards.
    #[serde(default)]
    pub cadence: u32,
    /// Tempo-card **grade** (§3): Finesse — the per-card magnitude weighed in a crossing or evade
    /// contest.
    #[serde(default)]
    pub finesse: u32,
}

/// A character's **Form**: the fundamental (`base`) card + attachments. Sums to the Offense/Defense the
/// engine reads (stats-as-deck, §2.3/§4.3). Form cards are permanent (§5.2) — this is *what you
/// are*, derived from the table, never a maintained meter (§2.1).
#[derive(Clone, Debug, Default)]
pub struct Form {
    pub cards: Vec<StatCard>,
}

impl Form {
    pub fn new(cards: Vec<StatCard>) -> Self {
        Self { cards }
    }

    /// The offensive stats, summed across the Form.
    pub fn offense(&self) -> Offense {
        let mut o = Offense::default();
        for c in &self.cards {
            o.might += c.might;
            o.cadence += c.cadence;
            o.finesse += c.finesse;
        }
        o
    }

    /// The defensive stats, summed across the Form: Vitality (count) × Toughness (bar).
    pub fn defense(&self) -> Defense {
        let vitality = self.cards.iter().map(|c| c.vitality).sum();
        let toughness = self.cards.iter().map(|c| c.toughness).sum();
        Defense::new(vitality, toughness)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_sums_fundamental_and_attachments_commutatively() {
        let fundamental = StatCard {
            name: "Anvil (base)".into(),
            might: 4,
            cadence: 2,
            finesse: 1,
            vitality: 10,
            toughness: 2,
        };
        let reward_a = StatCard {
            name: "Toughen".into(),
            toughness: 1,
            ..Default::default()
        };
        let reward_b = StatCard {
            name: "Swiften".into(),
            cadence: 1,
            ..Default::default()
        };

        let form = Form::new(vec![
            fundamental.clone(),
            reward_a.clone(),
            reward_b.clone(),
        ]);
        let o = form.offense();
        assert_eq!((o.might, o.cadence, o.finesse), (4, 3, 1));
        let d = form.defense();
        assert_eq!(d.health.max, 10);
        assert_eq!(d.health.toughness, 3); // 2 + 1 from the reward

        // Commutative: reordering the attachments yields the same block.
        let reordered = Form::new(vec![reward_b, reward_a, fundamental]);
        assert_eq!(reordered.defense().health.toughness, d.health.toughness);
        assert_eq!(reordered.offense().cadence, o.cadence);
    }
}
