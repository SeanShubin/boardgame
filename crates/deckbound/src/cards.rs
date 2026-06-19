//! Data-driven cards & effects.
//!
//! A Clash **Move** lives in [`crate::duel`]; an **Action card** carries effect(s) played in the
//! round. Each card declares a **zone behavior** ([`ZoneBehavior`], §5.3: Return / Spend / Lasting)
//! and optional **tags** (charge/combo interaction, §5.4). Cards are loaded from
//! `data/booklet.ron`, so numbers retune without recompiling; a card's magnitude flows through the
//! [`crate::stats`] cut→bar→pool pipeline.

use serde::Deserialize;

use crate::stats::DamageType;
use crate::zones::ZoneBehavior;

/// A single effect a card can carry. Edge scales the **primary** (first) effect's
/// natural unit (+1 per Edge); see [`Card::primary_damage`].
#[derive(Clone, Copy, Debug, Deserialize)]
pub enum Effect {
    /// Deal `power` damage of `dtype` (Edge adds on top, per target).
    Damage { power: u32, dtype: DamageType },
    /// On a landed hit, the target loses its action this round.
    Stagger,
    /// Shear `armor` off the target's plate (a Sunder).
    Sunder { armor: u32 },
    /// Rip a card from the target's hand.
    Disarm,
    /// Break the target out of the line (a Shove).
    Shove,
    /// Raise allies' Resolve by `resolve` (a Rally; lives in the party zone).
    Rally { resolve: u32 },
    /// Clear accumulated fear / steady the nerve (a Steel).
    Steel,
    /// Turn a face-down card back up (a Recover).
    Recover,
    /// Bank +`amount` Speed (extra tempo this round).
    BankSpeed { amount: u32 },
    /// Restore `body` Health to the most-wounded ally (a Mend).
    Mend { body: u32 },
    /// Grant a melee attack to a defenseless ally for the round (a Ward, §4.2).
    Ward,
    /// Grant +`tempo` Tempo to an ally (a Haste).
    Haste { tempo: u32 },
    /// Strip `tempo` Tempo from a foe (a Suppress).
    Suppress { tempo: u32 },
    /// Cut `speed` Speed from a foe (a Slow — cheaper to block/engage).
    Slow { speed: u32 },
    /// Strip `focus` Focus from a foe so it cannot block (a Confuse).
    Confuse { focus: u32 },
}

/// An Action card: its effect(s), how many foes it hits, its §5 zone behavior, and tags.
#[derive(Clone, Debug, Deserialize)]
pub struct Card {
    pub name: String,
    /// A human rules description, used to generate the encyclopedia's **Powers** entries from
    /// card data (the card is the source of truth for what it does). Empty for plain weapons.
    #[serde(default)]
    pub text: String,
    /// Evocative in-world flavor — prose, not rules (the `text` field carries the mechanics).
    /// Lives entirely in `data/booklet.ron`; never authored in code.
    #[serde(default)]
    pub flavor: String,
    /// Distinct foes hit (AoE). 1 = single target.
    #[serde(default = "one")]
    pub targets: u32,
    /// Reach in jumps `[min, max]` (melee `[1,1]`, ranged `[2,2]`). Informational
    /// for now; positioning is approximated.
    #[serde(default = "melee")]
    pub reach: [u32; 2],
    /// §5.3 — what the card does to itself after it is played (default **Return** to Hand).
    #[serde(default)]
    pub zone: ZoneBehavior,
    /// §5.4 — type tags for charge/combo interaction (e.g. `["Charge(fire)"]`). Empty by default.
    #[serde(default)]
    pub tags: Vec<String>,
    /// A passive ability (a §4 power detected by name) rather than a played effect card.
    #[serde(default)]
    pub passive: bool,
    #[serde(default)]
    pub effects: Vec<Effect>,
}

fn one() -> u32 {
    1
}
fn melee() -> [u32; 2] {
    [1, 1]
}

impl Card {
    /// The card's primary damage (power, type), if it deals damage. This is what
    /// Edge scales when the card is Unleashed/Overwhelmed.
    pub fn primary_damage(&self) -> Option<(u32, DamageType)> {
        self.effects.iter().find_map(|e| match e {
            Effect::Damage { power, dtype } => Some((*power, *dtype)),
            _ => None,
        })
    }

    pub fn has_stagger(&self) -> bool {
        self.effects.iter().any(|e| matches!(e, Effect::Stagger))
    }

    /// A short one-line summary for the card UI.
    pub fn summary(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        for e in &self.effects {
            parts.push(match e {
                Effect::Damage { power, dtype } => format!("{} {power}", dtype.label()),
                Effect::Stagger => "stagger".into(),
                Effect::Sunder { armor } => format!("sunder -{armor}"),
                Effect::Disarm => "disarm".into(),
                Effect::Shove => "shove".into(),
                Effect::Rally { resolve } => format!("rally +{resolve}"),
                Effect::Steel => "steel".into(),
                Effect::Recover => "recover".into(),
                Effect::BankSpeed { amount } => format!("+{amount} speed"),
                Effect::Mend { body } => format!("mend +{body}"),
                Effect::Ward => "ward (grant melee)".into(),
                Effect::Haste { tempo } => format!("haste +{tempo}"),
                Effect::Suppress { tempo } => format!("suppress -{tempo} tempo"),
                Effect::Slow { speed } => format!("slow -{speed} speed"),
                Effect::Confuse { focus } => format!("confuse -{focus} focus"),
            });
        }
        if self.targets > 1 {
            parts.push(format!("x{} targets", self.targets));
        }
        parts.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn firestorm() -> Card {
        Card {
            name: "Firestorm".into(),
            text: String::new(),
            flavor: String::new(),
            targets: 5,
            reach: [2, 2],
            zone: ZoneBehavior::Spend,
            tags: vec![],
            passive: false,
            effects: vec![Effect::Damage {
                power: 5,
                dtype: DamageType::Heat,
            }],
        }
    }

    #[test]
    fn primary_damage_reads_the_first_damage_effect() {
        let (p, t) = firestorm().primary_damage().unwrap();
        assert_eq!(p, 5);
        assert_eq!(t, DamageType::Heat);
    }

    #[test]
    fn summary_mentions_aoe() {
        assert!(firestorm().summary().contains("targets"));
    }
}
