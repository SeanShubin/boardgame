//! Combatants — **Actors** — and how non-player ones decide.
//!
//! An Actor is the umbrella (see `docs/games/deckbound/design/entities.md`): a
//! **Character** is human-driven (improvises); a **Creature** follows a scripted
//! `Behavior` (a stance policy + a target rule). Both carry the full stat block —
//! [`Offense`](crate::stats::Offense) and [`Defense`](crate::stats::Defense) — plus
//! a weapon and action cards, and round budgets (**tempo** = Speed, **focus** = Mind).

use engine::Rng;
use serde::Deserialize;

use crate::cards::Card;
use crate::duel::Move;
use crate::stats::{Defense, Offense};

/// Who drives an Actor's choices.
#[derive(Clone, Debug)]
pub enum Driver {
    /// A Character: the human (or a stand-in) improvises.
    Human,
    /// A Creature: a scripted instinct.
    Creature(Behavior),
}

/// A creature's scripted instinct: how it picks a move, and whom it targets.
#[derive(Clone, Copy, Debug)]
pub struct Behavior {
    pub policy: MovePolicy,
    pub target_rule: TargetRule,
}

/// Whom a creature goes for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum TargetRule {
    /// The front of the line (default).
    Front,
    /// The most fragile (fewest Body cards).
    LowestBody,
    /// The shakiest nerve (lowest Resolve).
    LeastResolute,
    /// Runs the gauntlet for the back line.
    Runner,
}

/// How a creature picks its move each beat — its instinct (one-way; a Creature does not
/// read you back, §7). `rng` supplies the variation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MovePolicy {
    Dummy,
    Brute,
    Turtle,
    Duelist,
    Grappler,
    Aggressor,
}

impl MovePolicy {
    pub fn label(self) -> &'static str {
        match self {
            MovePolicy::Dummy => "Sandbag",
            MovePolicy::Brute => "Charger",
            MovePolicy::Turtle => "Turtle",
            MovePolicy::Duelist => "Duelist",
            MovePolicy::Grappler => "Grappler",
            MovePolicy::Aggressor => "Berserker",
        }
    }

    /// The move for this beat. `up`/`down`/`max` are the creature's own Charge state;
    /// `rng` supplies the variation. (One-way: it does not read the hero, §7.)
    pub fn pick_move(self, up: u32, down: u32, max: u32, rng: &mut Rng) -> Move {
        let can_charge = up + down < max;
        match self {
            // A sandbag just swings — trivially read and Parried.
            MovePolicy::Dummy => Move::Strike,
            // Wind up to a killshot, then swing; recover if its charge was knocked down.
            MovePolicy::Brute => {
                if down > 0 {
                    Move::Recover
                } else if can_charge {
                    Move::Charge
                } else {
                    Move::Strike
                }
            }
            // Mostly blocks; occasionally dodges.
            MovePolicy::Turtle => {
                if rng.below(4) == 0 {
                    Move::Evade
                } else {
                    Move::Parry
                }
            }
            // Throws through guards; charges when it can.
            MovePolicy::Grappler => {
                if down > 0 {
                    Move::Recover
                } else if can_charge && rng.below(2) == 0 {
                    Move::Charge
                } else {
                    Move::Throw
                }
            }
            // Relentless swings.
            MovePolicy::Aggressor => Move::Strike,
            // Mixes the whole kit.
            MovePolicy::Duelist => match rng.below(5) {
                0 if can_charge => Move::Charge,
                0 => Move::Strike,
                1 => Move::Throw,
                2 => Move::Parry,
                3 => Move::Evade,
                _ => Move::Strike,
            },
        }
    }
}

/// A combatant.
#[derive(Clone, Debug)]
pub struct Actor {
    pub name: String,
    pub role: String,
    pub offense: Offense,
    pub defense: Defense,
    /// The default strike for Unleash / Overwhelm.
    pub weapon: Card,
    /// Extra action cards (AoE, Rally, Dread, …) playable in the round.
    pub actions: Vec<Card>,
    pub driver: Driver,
    /// This actor crosses the gauntlet rather than holding a line.
    pub runner: bool,
    /// Charge capacity — how many durable Charges this fighter can stack in a Clash.
    pub charges_max: u32,

    // round-scoped budgets
    pub tempo: i32,
    pub focus: u32,
    pub exposed: bool,
    /// Finalized dead. Body reaching 0 is "mortally wounded" — the actor fights on and
    /// lands its committed blows; death is tallied at the round boundary (§1.9), which
    /// sets this. Once set it persists (the actor is out of the fight).
    pub fallen: bool,
}

impl Actor {
    pub fn is_down(&self) -> bool {
        self.defense.is_down()
    }

    pub fn is_human(&self) -> bool {
        matches!(self.driver, Driver::Human)
    }

    pub fn behavior(&self) -> Option<&Behavior> {
        match &self.driver {
            Driver::Creature(b) => Some(b),
            Driver::Human => None,
        }
    }

    /// Refresh tempo & focus and clear round-scoped state.
    pub fn refresh_round(&mut self) {
        self.tempo = self.offense.speed as i32;
        self.focus = self.defense.mind;
        self.exposed = false;
        self.defense.end_round();
    }

    /// Overextended — Tempo went negative. Go all-in (§3.3): Focus drops to 0, so the
    /// actor can read no one and every foe free-hits this round.
    pub fn expose(&mut self) {
        self.exposed = true;
        self.focus = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::{Card, Effect, Lifecycle};
    use crate::stats::DamageType;

    fn weapon() -> Card {
        Card {
            name: "Fist".into(),
            targets: 1,
            reach: [1, 1],
            lifecycle: Lifecycle::Fleeting,
            effects: vec![Effect::Damage {
                power: 1,
                dtype: DamageType::Blunt,
            }],
        }
    }

    #[test]
    fn refresh_resets_budgets_to_speed_and_mind() {
        let mut a = Actor {
            name: "X".into(),
            role: "Y".into(),
            offense: Offense {
                speed: 5,
                ..Default::default()
            },
            defense: Defense::new(8, 1, 4, 3),
            weapon: weapon(),
            actions: vec![],
            driver: Driver::Human,
            runner: false,
            charges_max: 3,
            tempo: 0,
            focus: 0,
            exposed: true,
            fallen: false,
        };
        a.refresh_round();
        assert_eq!(a.tempo, 5);
        assert_eq!(a.focus, 3);
        assert!(!a.exposed);
    }

    #[test]
    fn dummy_just_swings() {
        let mut rng = Rng::new(1);
        assert_eq!(MovePolicy::Dummy.pick_move(0, 0, 3, &mut rng), Move::Strike);
    }
}
