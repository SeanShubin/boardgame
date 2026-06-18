//! §2.3 / §4.3 — **stats-as-deck**: a character's stat block is *read off its Form*, not authored
//! on the actor. A [`StatCard`] carries one card's stat contribution; a [`Form`] is the
//! fundamental card + its attachments (in the Active zone, §5.2), summed into the
//! [`Offense`](crate::stats::Offense) / [`Defense`](crate::stats::Defense) the rest of the engine
//! reads. Combination is **commutative** (§5.5). This is number-preserving: a fundamental built
//! from an actor's base stats plus its trait attachments derives exactly the old stat block.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::stats::{Aspect, DamageType, Defense, Offense};

/// One card's contribution to the Form stat block. The **fundamental** card sets the base; each
/// **attachment** (a trait, or a bought Upgrade, §8.3) adds on top. Body pool = `body` (count) ×
/// `toughness` (value); both grow by attachment (the two depth/breadth dials, §5.5).
#[derive(Clone, Debug, Default, Deserialize)]
pub struct StatCard {
    #[serde(default)]
    pub name: String,
    // offense
    #[serde(default)]
    pub power: u32,
    #[serde(default)]
    pub precision: u32,
    #[serde(default)]
    pub speed: u32,
    #[serde(default)]
    pub spirit: u32,
    // defense (Body pool = count × value)
    #[serde(default)]
    pub body: u32,
    #[serde(default)]
    pub toughness: u32,
    #[serde(default)]
    pub resolve: u32,
    #[serde(default)]
    pub mind: u32,
    #[serde(default)]
    pub armor: Vec<(DamageType, u32)>,
    #[serde(default)]
    pub ward: Vec<(DamageType, u32)>,
    #[serde(default)]
    pub keystone: Option<Aspect>,
}

/// A character's **Form**: the fundamental card + attachments. Sums to the Offense/Defense the
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
            o.power += c.power;
            o.precision += c.precision;
            o.speed += c.speed;
            o.spirit += c.spirit;
        }
        o
    }

    /// The defensive stats, summed across the Form. Armor/Ward merge per damage type; the keystone
    /// is the last card to name one (so an attachment can move it), defaulting to Body.
    pub fn defense(&self) -> Defense {
        let body = self.cards.iter().map(|c| c.body).sum();
        let toughness = self.cards.iter().map(|c| c.toughness).sum();
        let resolve = self.cards.iter().map(|c| c.resolve).sum();
        let mind = self.cards.iter().map(|c| c.mind).sum();
        let mut d = Defense::new(body, toughness, resolve, mind);
        let mut armor: BTreeMap<DamageType, u32> = BTreeMap::new();
        let mut ward: BTreeMap<DamageType, u32> = BTreeMap::new();
        for c in &self.cards {
            for (dt, v) in &c.armor {
                *armor.entry(*dt).or_insert(0) += v;
            }
            for (dt, v) in &c.ward {
                *ward.entry(*dt).or_insert(0) += v;
            }
            if let Some(k) = c.keystone {
                d.keystone = k;
            }
        }
        d.armor = armor;
        d.ward = ward;
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_sums_fundamental_and_attachments_commutatively() {
        let fundamental = StatCard {
            name: "Anvil (base)".into(),
            power: 4,
            precision: 1,
            speed: 2,
            body: 10,
            toughness: 2,
            resolve: 5,
            mind: 5,
            ..Default::default()
        };
        let plate = StatCard {
            name: "Heavy-Plate".into(),
            armor: vec![(DamageType::Sharp, 4), (DamageType::Blunt, 3)],
            ..Default::default()
        };
        let ward_charm = StatCard {
            name: "Ward-charm".into(),
            ward: vec![(DamageType::Fear, 2)],
            resolve: 1,
            ..Default::default()
        };

        let form = Form::new(vec![fundamental.clone(), plate.clone(), ward_charm.clone()]);
        let o = form.offense();
        assert_eq!((o.power, o.precision, o.speed), (4, 1, 2));
        let d = form.defense();
        assert_eq!(d.body.max, 10);
        assert_eq!(d.body.toughness, 2);
        assert_eq!(d.resolve, 6); // 5 + 1 from the charm
        assert_eq!(d.mind, 5);
        assert_eq!(d.armor.get(&DamageType::Sharp), Some(&4));
        assert_eq!(d.ward.get(&DamageType::Fear), Some(&2));

        // Commutative: reordering the attachments yields the same block.
        let reordered = Form::new(vec![ward_charm, plate, fundamental]);
        assert_eq!(reordered.defense().resolve, d.resolve);
        assert_eq!(
            reordered.defense().armor.get(&DamageType::Sharp),
            d.armor.get(&DamageType::Sharp)
        );
    }
}
