//! The per-round combat simulation and duel resolution.
//!
//! Resolution (canon-aligned, Spec §2.2 with the deferred gear cut):
//! 1. Each round the attacker spends `speed` actions; each is one strike of
//!    magnitude **might**, on the weapon type the defender resists least.
//! 2. **Pre-pile subtract (gear):** `damage = max(0, might − resistance[type])`.
//!    Resistance is capped at 3, so `might > 3` always lands — no immunity.
//! 3. **Bar (accumulate within the round):** add `damage` to the active health
//!    card's pile; at `>= toughness` flip one card; overflow discarded (unless
//!    `cleave`, which cascades).
//! 4. End of round: un-flipped accumulation is wiped (unless `persist`).

use crate::{Character, DamageType};

/// Hard cap on simulated rounds; reaching it means "never" (∞).
pub const ROUND_CAP: u32 = 1000;

/// One resolved strike, recorded for traces.
#[derive(Debug, Clone)]
pub struct StrikeRow {
    pub round: u32,
    pub action: u32,
    /// The type chosen this strike (the defender's least-resisted).
    pub channel: DamageType,
    pub might: u32,
    /// Resistance faced on the chosen type.
    pub resist: u32,
    /// Post-cut damage delivered.
    pub damage: u32,
    pub acc_before: u32,
    pub acc_after: u32,
    pub flips: u32,
    pub waste: u32,
    pub flipped_total: u32,
    pub cards_total: u32,
    pub bounced: bool,
}

/// A trace step: a strike, or a round boundary.
#[derive(Debug, Clone)]
pub enum Step {
    Strike(StrikeRow),
    RoundEnd {
        round: u32,
        leftover: u32,
        carried: bool,
    },
}

/// Result of a one-way grind.
#[derive(Debug, Clone)]
pub struct Grind {
    pub rounds: Option<u32>,
    pub steps: Vec<Step>,
}

/// The best strike against a resistance vector: the weapon type the defender
/// resists least (coverage). Returns `(chosen type, resistance faced, damage)`.
fn best_strike(attacker: &Character, resistance: &[u32; 3]) -> (DamageType, u32, u32) {
    attacker
        .weapon
        .iter()
        .map(|&t| {
            let r = resistance[t.index()];
            (t, r, attacker.might.saturating_sub(r))
        })
        .max_by_key(|&(_, _, dmg)| dmg)
        .unwrap_or((DamageType::Pierce, 0, attacker.might))
}

/// Simulate the attacker grinding the defender down. `record` builds a
/// strike-by-strike trace (skip it for the bulk matchup matrix).
pub fn grind(attacker: &Character, defender: &Character, record: bool) -> Grind {
    let toughness = defender.toughness.max(1);
    let cards_total = defender.vitality;
    let mut cards = cards_total;
    let mut steps = Vec::new();
    if cards == 0 {
        return Grind {
            rounds: Some(0),
            steps,
        };
    }

    let (_, _, per_strike) = best_strike(attacker, &defender.resistance);
    let mut acc: u32 = 0;

    for round in 1..=ROUND_CAP {
        for action in 1..=attacker.speed {
            let (channel, resist, damage) = best_strike(attacker, &defender.resistance);
            let acc_before = acc;
            acc += damage;
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
                    channel,
                    might: attacker.might,
                    resist,
                    damage,
                    acc_before,
                    acc_after: acc,
                    flips,
                    waste,
                    flipped_total: cards_total - cards,
                    cards_total,
                    bounced: damage == 0,
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

        // Early exits to ∞ (resistance is static, so per-strike damage is fixed):
        if per_strike == 0 {
            return Grind {
                rounds: None,
                steps,
            }; // never penetrates
        }
        if !attacker.keywords.persist && attacker.speed.saturating_mul(per_strike) < toughness {
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
    pub outcome: Outcome,
}

/// Resolve a duel as two one-way grinds: the faster kill wins; equal rounds break
/// on **daring**; mutual ∞ is a draw.
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
            } else if a.daring > b.daring {
                Outcome::Win
            } else if a.daring < b.daring {
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
    use crate::Keywords;

    fn ch(name: &str) -> Character {
        Character {
            name: name.into(),
            might: 5,
            weapon: vec![DamageType::Slash],
            vitality: 4,
            toughness: 4,
            speed: 2,
            daring: 1,
            resistance: [0, 0, 0],
            keywords: Keywords::default(),
        }
    }

    #[test]
    fn resistance_subtracts_per_strike() {
        let mut att = ch("att");
        att.might = 5;
        let mut def = ch("def");
        def.resistance[DamageType::Slash.index()] = 3; // 5 − 3 = 2 lands
        let (_, r, dmg) = best_strike(&att, &def.resistance);
        assert_eq!(r, 3);
        assert_eq!(dmg, 2);
    }

    #[test]
    fn capped_resistance_never_immunises_a_real_hit() {
        // might > 3 always penetrates a ≤3 cut — no stalemate.
        let mut att = ch("att");
        att.might = 4;
        let mut def = ch("def");
        def.resistance = [3, 3, 3];
        def.vitality = 1;
        def.toughness = 1;
        assert!(rounds_to_kill(&att, &def).is_some());
    }

    #[test]
    fn weak_hit_into_full_resistance_is_walled() {
        let mut att = ch("att");
        att.might = 3;
        let mut def = ch("def");
        def.resistance[DamageType::Slash.index()] = 3; // 3 − 3 = 0
        assert_eq!(rounds_to_kill(&att, &def), None);
    }

    #[test]
    fn multi_type_picks_the_gap() {
        let mut god = ch("god");
        god.might = 5;
        god.weapon = vec![DamageType::Pierce, DamageType::Slash, DamageType::Crush];
        let resistance = [3, 0, 3]; // slash is the gap
        let (t, r, dmg) = best_strike(&god, &resistance);
        assert_eq!(t, DamageType::Slash);
        assert_eq!(r, 0);
        assert_eq!(dmg, 5);
    }

    #[test]
    fn emergent_rps_from_rotated_resistance() {
        // Single-type specialists, rotated resistance — no counter table written.
        let base = |name: &str, t: DamageType, r: [u32; 3]| Character {
            name: name.into(),
            might: 6,
            weapon: vec![t],
            vitality: 4,
            toughness: 4,
            speed: 2,
            daring: 1,
            resistance: r,
            keywords: Keywords::default(),
        };
        let p = base("P", DamageType::Pierce, [0, 3, 0]); // resists slash
        let s = base("S", DamageType::Slash, [0, 0, 3]); // resists crush
        let c = base("C", DamageType::Crush, [3, 0, 0]); // resists pierce
        assert_eq!(duel(&p, &s).outcome, Outcome::Win); // P > S
        assert_eq!(duel(&s, &c).outcome, Outcome::Win); // S > C
        assert_eq!(duel(&c, &p).outcome, Outcome::Win); // C > P
    }

    #[test]
    fn cleave_cascades_overflow() {
        let mut att = ch("att");
        att.might = 6;
        att.speed = 1;
        att.keywords.cleave = true;
        let mut def = ch("def");
        def.vitality = 4;
        def.toughness = 2;
        assert_eq!(rounds_to_kill(&att, &def), Some(2));
        att.keywords.cleave = false;
        assert_eq!(rounds_to_kill(&att, &def), Some(4));
    }

    #[test]
    fn persist_defeats_the_per_round_reset() {
        let mut att = ch("att");
        att.might = 2;
        att.speed = 1;
        let mut def = ch("def");
        def.vitality = 1;
        def.toughness = 5;
        assert_eq!(rounds_to_kill(&att, &def), None);
        att.keywords.persist = true; // 2,4,6 → flip round 3
        assert_eq!(rounds_to_kill(&att, &def), Some(3));
    }

    #[test]
    fn daring_breaks_round_ties() {
        let mut a = ch("a");
        let mut b = ch("b");
        a.daring = 5;
        b.daring = 1;
        assert_eq!(duel(&a, &b).outcome, Outcome::Win);
    }
}
