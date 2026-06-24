//! The per-round combat simulation and duel resolution.
//!
//! Resolution (per the design doc, multiplicative-armor revision):
//! 1. Each round, the attacker spends `speed_quantity` actions; each is a Strike
//!    of the best available weapon vs the defender's armor type.
//! 2. Stage 1 — Armor (type chart, soft): `bite = power × effectiveness`, where
//!    the multiplier is ×2 / ×1 / ×½. `Half` never zeroes, so the chart adds no
//!    immunity.
//! 3. Stage 2 — Toughness (accumulates within the round): bites pile onto the
//!    active card; at `>= toughness` it flips. Overflow discarded (unless
//!    `cleave`, which cascades). This Toughness floor is the *only* wall source.
//! 4. End of round: un-flipped accumulation is wiped (unless `persist`).

use crate::{ArmorType, Character, DamageType, Effect, effectiveness};

/// Hard cap on simulated rounds; reaching it means "never" (∞).
pub const ROUND_CAP: u32 = 1000;

/// One resolved strike, recorded for traces.
#[derive(Debug, Clone)]
pub struct StrikeRow {
    pub round: u32,
    pub action: u32,
    pub channel: DamageType,
    /// Raw weapon magnitude.
    pub power: u32,
    /// Type-chart effectiveness this strike landed at.
    pub effect: Effect,
    /// Post-multiplier damage this strike delivers.
    pub bite: u32,
    pub acc_before: u32,
    pub acc_after: u32,
    pub flips: u32,
    /// Overflow discarded on a (non-cleave) flip.
    pub waste: u32,
    /// Total cards flipped so far (for the card visual).
    pub flipped_total: u32,
    pub cards_total: u32,
    pub bounced: bool,
    /// Remaining armor cards if the defender is `brittle`.
    pub armor_left: Option<u32>,
}

/// A trace step: a strike, or a round boundary.
#[derive(Debug, Clone)]
pub enum Step {
    Strike(StrikeRow),
    /// End of a round that did not finish the fight.
    RoundEnd {
        round: u32,
        /// Un-flipped accumulation at round end.
        leftover: u32,
        /// True if `persist` carried it; false if it was wiped.
        carried: bool,
    },
}

/// Result of a one-way grind.
#[derive(Debug, Clone)]
pub struct Grind {
    /// Rounds for the attacker to kill the defender, or `None` for ∞.
    pub rounds: Option<u32>,
    pub steps: Vec<Step>,
}

struct Hit {
    bite: u32,
    channel: DamageType,
    power: u32,
    effect: Effect,
}

/// Pick the weapon with the largest bite against the armor. `shattered` brittle
/// armor is treated as neutral Cloth.
fn best_strike(attacker: &Character, armor: ArmorType, shattered: bool, pierce: bool) -> Hit {
    let armor = if shattered { ArmorType::Cloth } else { armor };
    attacker
        .weapons
        .iter()
        .map(|w| {
            let effect = effectiveness(w.channel, armor, pierce);
            Hit {
                bite: effect.apply(w.strike_magnitude),
                channel: w.channel,
                power: w.strike_magnitude,
                effect,
            }
        })
        .max_by_key(|h| h.bite)
        .unwrap_or(Hit {
            bite: 0,
            channel: DamageType::Pierce,
            power: 0,
            effect: Effect::Normal,
        })
}

/// Simulate the attacker grinding the defender down. `record` controls whether a
/// strike-by-strike trace is built (skip it for the bulk matchup matrix).
pub fn grind(attacker: &Character, defender: &Character, record: bool) -> Grind {
    let toughness = defender.health_magnitude.max(1);
    let cards_total = defender.health_quantity;
    let mut cards = cards_total;
    let mut steps = Vec::new();
    if cards == 0 {
        return Grind {
            rounds: Some(0),
            steps,
        };
    }

    let pierce = attacker.pierce_magnitude > 0;
    let armor_type = defender.armor;
    let mut armor_cards = defender.armor_quantity; // brittle pool
    let mut acc: u32 = 0;

    for round in 1..=ROUND_CAP {
        for action in 1..=attacker.speed_quantity {
            let shattered = defender.keywords.brittle && armor_cards == 0;
            let hit = best_strike(attacker, armor_type, shattered, pierce);

            // Brittle: each strike erodes the armor pool; once spent, the armor
            // type stops applying (treated as neutral Cloth thereafter).
            if defender.keywords.brittle && armor_cards > 0 {
                armor_cards -= 1;
            }

            let acc_before = acc;
            acc += hit.bite;
            let mut flips = 0u32;
            let mut waste = 0u32;
            if attacker.keywords.cleave {
                while acc >= toughness && cards > 0 {
                    cards -= 1;
                    acc -= toughness;
                    flips += 1;
                }
            } else if acc >= toughness {
                waste = acc - toughness;
                cards -= 1;
                acc = 0;
                flips += 1;
            }

            if record {
                steps.push(Step::Strike(StrikeRow {
                    round,
                    action,
                    channel: hit.channel,
                    power: hit.power,
                    effect: hit.effect,
                    bite: hit.bite,
                    acc_before,
                    acc_after: acc,
                    flips,
                    waste,
                    flipped_total: cards_total - cards,
                    cards_total,
                    bounced: hit.bite == 0,
                    armor_left: defender.keywords.brittle.then_some(armor_cards),
                }));
            }

            if cards == 0 {
                return Grind {
                    rounds: Some(round),
                    steps,
                };
            }
        }

        let carried = attacker.keywords.persist;
        if record {
            steps.push(Step::RoundEnd {
                round,
                leftover: acc,
                carried,
            });
        }
        if !carried {
            acc = 0;
        }

        // Early exits to ∞ — but never while the situation can still change:
        // `persist` keeps accumulating across rounds, and `brittle` armor erodes.
        let armor_can_erode = defender.keywords.brittle && armor_cards > 0;
        let shattered = defender.keywords.brittle && armor_cards == 0;
        let cur_bite = best_strike(attacker, armor_type, shattered, pierce).bite;
        if cur_bite == 0 && !armor_can_erode {
            return Grind {
                rounds: None,
                steps,
            };
        }
        if !attacker.keywords.persist
            && !armor_can_erode
            && attacker.speed_quantity.saturating_mul(cur_bite) < toughness
        {
            return Grind {
                rounds: None,
                steps,
            }; // can't reach Toughness in a round
        }
    }

    Grind {
        rounds: None,
        steps,
    }
}

/// Rounds for `attacker` to kill `defender` one-way, or `None` for ∞.
pub fn rounds_to_kill(attacker: &Character, defender: &Character) -> Option<u32> {
    grind(attacker, defender, false).rounds
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Win,
    Loss,
    Draw,
}

impl Outcome {
    /// Win > Draw > Loss, for dominance comparisons.
    pub fn score(self) -> u8 {
        match self {
            Outcome::Win => 2,
            Outcome::Draw => 1,
            Outcome::Loss => 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Duel {
    pub rtk_ab: Option<u32>,
    pub rtk_ba: Option<u32>,
    /// From A's perspective.
    pub outcome: Outcome,
}

/// Resolve a duel as two one-way grinds: the faster kill wins; equal rounds break
/// on initiative; mutual ∞ is a draw.
pub fn duel(a: &Character, b: &Character) -> Duel {
    let rtk_ab = rounds_to_kill(a, b);
    let rtk_ba = rounds_to_kill(b, a);
    let outcome = match (rtk_ab, rtk_ba) {
        (None, None) => Outcome::Draw,
        (Some(_), None) => Outcome::Win,
        (None, Some(_)) => Outcome::Loss,
        (Some(x), Some(y)) => {
            if x < y {
                Outcome::Win
            } else if x > y {
                Outcome::Loss
            } else if a.speed_magnitude > b.speed_magnitude {
                Outcome::Win
            } else if a.speed_magnitude < b.speed_magnitude {
                Outcome::Loss
            } else {
                Outcome::Draw
            }
        }
    };
    Duel {
        rtk_ab,
        rtk_ba,
        outcome,
    }
}

/// TTK ratio for texture analysis; ∞ when either side is walled.
pub fn margin(d: &Duel) -> f64 {
    match (d.rtk_ab, d.rtk_ba) {
        (Some(x), Some(y)) if x > 0 && y > 0 => {
            let (x, y) = (x as f64, y as f64);
            x.max(y) / x.min(y)
        }
        _ => f64::INFINITY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Keywords, Weapon};

    fn ch(name: &str) -> Character {
        Character {
            name: name.into(),
            health_quantity: 4,
            health_magnitude: 2,
            armor: ArmorType::Cloth,
            armor_quantity: 1,
            speed_quantity: 1,
            speed_magnitude: 1,
            pierce_magnitude: 0,
            weapons: vec![Weapon {
                strike_magnitude: 3,
                channel: DamageType::Slash,
            }],
            keywords: Keywords::default(),
        }
    }

    #[test]
    fn chart_is_a_regular_latin_square() {
        use DamageType::*;
        let armors = [ArmorType::Plate, ArmorType::Mail, ArmorType::Padded];
        let channels = [Pierce, Slash, Crush];
        // Each row (channel) and each column (armor) has one of each multiplier.
        for c in channels {
            let row: Vec<Effect> = armors.iter().map(|&a| effectiveness(c, a, false)).collect();
            assert!(
                row.contains(&Effect::Double)
                    && row.contains(&Effect::Normal)
                    && row.contains(&Effect::Half)
            );
        }
        for a in armors {
            let col: Vec<Effect> = channels
                .iter()
                .map(|&c| effectiveness(c, a, false))
                .collect();
            assert!(
                col.contains(&Effect::Double)
                    && col.contains(&Effect::Normal)
                    && col.contains(&Effect::Half)
            );
        }
        // Specific signature matchups.
        assert_eq!(
            effectiveness(Pierce, ArmorType::Mail, false),
            Effect::Double
        );
        assert_eq!(
            effectiveness(Crush, ArmorType::Plate, false),
            Effect::Double
        );
        assert_eq!(
            effectiveness(Slash, ArmorType::Padded, false),
            Effect::Double
        );
        // Cloth is always neutral.
        assert_eq!(
            effectiveness(Crush, ArmorType::Cloth, false),
            Effect::Normal
        );
    }

    #[test]
    fn pierce_upgrades_resisted_to_neutral() {
        // Slash is resisted by Mail (×½); armor-piercing makes it neutral.
        assert_eq!(
            effectiveness(DamageType::Slash, ArmorType::Mail, false),
            Effect::Half
        );
        assert_eq!(
            effectiveness(DamageType::Slash, ArmorType::Mail, true),
            Effect::Normal
        );
        // It does not turn a ×2 into anything more.
        assert_eq!(
            effectiveness(DamageType::Pierce, ArmorType::Mail, true),
            Effect::Double
        );
    }

    #[test]
    fn toughness_floor_is_the_only_wall() {
        // bite 1 per strike, one action, Toughness 5 → never reaches it.
        let mut att = ch("att");
        att.weapons[0].strike_magnitude = 1;
        att.speed_quantity = 1;
        let mut def = ch("def");
        def.health_magnitude = 5;
        assert_eq!(rounds_to_kill(&att, &def), None);
        // Enough actions to cross Toughness in a round → finite.
        att.speed_quantity = 5;
        assert!(rounds_to_kill(&att, &def).is_some());
    }

    #[test]
    fn double_damage_against_weak_armor() {
        // Crush vs Padded is ×½; crush vs Plate is ×2.
        let mut att = ch("att");
        att.weapons[0] = Weapon {
            strike_magnitude: 3,
            channel: DamageType::Crush,
        };
        att.speed_quantity = 1;
        let mut tanky = ch("def");
        tanky.health_quantity = 2;
        tanky.health_magnitude = 5;
        tanky.armor = ArmorType::Plate; // crush ×2 → bite 6 ≥ 5, one flip/round
        assert_eq!(rounds_to_kill(&att, &tanky), Some(2));
        tanky.armor = ArmorType::Padded; // crush ×½ → bite 1 < 5 → walled
        assert_eq!(rounds_to_kill(&att, &tanky), None);
    }

    #[test]
    fn cleave_cascades_overflow() {
        let mut att = ch("att");
        att.weapons[0].strike_magnitude = 6; // ×1 vs cloth → bite 6, toughness 2 → 3 flips
        att.speed_quantity = 1;
        att.keywords.cleave = true;
        let mut def = ch("def");
        def.health_quantity = 4;
        def.health_magnitude = 2;
        assert_eq!(rounds_to_kill(&att, &def), Some(2));
        att.keywords.cleave = false;
        assert_eq!(rounds_to_kill(&att, &def), Some(4));
    }

    #[test]
    fn persist_defeats_the_per_round_reset() {
        let mut att = ch("att");
        att.weapons[0].strike_magnitude = 2;
        att.speed_quantity = 1;
        let mut def = ch("def");
        def.health_quantity = 1;
        def.health_magnitude = 5; // 2 < 5 per round → never, without persist
        assert_eq!(rounds_to_kill(&att, &def), None);
        att.keywords.persist = true; // 2,4,6 → flips round 3
        assert_eq!(rounds_to_kill(&att, &def), Some(3));
    }

    #[test]
    fn brittle_armor_shatters_to_neutral() {
        // pierce ×½ vs Plate gives bite 1 (2 actions = 2 < Toughness 3) → walled
        // while intact; after the pool shatters it's ×1 (bite 2, 2×2=4 ≥ 3).
        let mut att = ch("att");
        att.weapons[0] = Weapon {
            strike_magnitude: 2,
            channel: DamageType::Pierce,
        };
        att.speed_quantity = 2;
        let mut def = ch("def");
        def.armor = ArmorType::Plate;
        def.health_quantity = 1;
        def.health_magnitude = 3;
        assert_eq!(rounds_to_kill(&att, &def), None); // intact forever → walled
        def.keywords.brittle = true;
        def.armor_quantity = 2;
        assert!(rounds_to_kill(&att, &def).is_some()); // shatters → breaks through
    }

    #[test]
    fn initiative_breaks_round_ties() {
        let mut a = ch("a");
        let mut b = ch("b");
        a.speed_magnitude = 5;
        b.speed_magnitude = 1;
        // Identical kill speed (both cloth, same stats) → initiative decides.
        assert_eq!(duel(&a, &b).outcome, Outcome::Win);
    }
}
