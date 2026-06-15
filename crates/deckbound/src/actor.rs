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
use crate::duel::Stance;
use crate::stats::{Defense, Offense};

/// Who drives an Actor's choices.
#[derive(Clone, Debug)]
pub enum Driver {
    /// A Character: the human (or a stand-in) improvises.
    Human,
    /// A Creature: a scripted instinct.
    Creature(Behavior),
}

/// A creature's scripted instinct: how it picks a stance, and whom it targets.
#[derive(Clone, Copy, Debug)]
pub struct Behavior {
    pub policy: StancePolicy,
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

/// How a creature picks its stance each beat — its "deck" of reads, off the public
/// Edge banks (never the human's hidden pick this beat).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StancePolicy {
    Dummy,
    Brute,
    Turtle,
    Duelist,
    Grappler,
    Aggressor,
}

impl StancePolicy {
    pub fn label(self) -> &'static str {
        match self {
            StancePolicy::Dummy => "Sandbag",
            StancePolicy::Brute => "Brute",
            StancePolicy::Turtle => "Turtle",
            StancePolicy::Duelist => "Duelist",
            StancePolicy::Grappler => "Grappler",
            StancePolicy::Aggressor => "Berserker",
        }
    }

    /// The stance for this beat. `own` is the creature's Edge, `foe` the hero's
    /// (both public); `rng` supplies the bluff.
    pub fn stance(self, own: u32, foe: u32, rng: &mut Rng) -> Stance {
        match self {
            StancePolicy::Dummy => Stance::Marshal,
            StancePolicy::Brute => {
                if own >= 3 {
                    Stance::Unleash
                } else {
                    Stance::Marshal
                }
            }
            StancePolicy::Turtle => {
                if rng.below(5) == 0 {
                    Stance::Marshal
                } else {
                    Stance::Parry
                }
            }
            StancePolicy::Grappler => {
                if own < 2 {
                    Stance::Marshal
                } else {
                    Stance::Overwhelm
                }
            }
            StancePolicy::Aggressor => {
                if rng.below(10) < 7 {
                    Stance::Unleash
                } else {
                    Stance::Marshal
                }
            }
            StancePolicy::Duelist => {
                if foe >= 2 {
                    match rng.below(4) {
                        0 => Stance::Marshal,
                        1 => Stance::Overwhelm,
                        _ => Stance::Parry,
                    }
                } else if own >= 2 {
                    if rng.below(3) == 0 {
                        Stance::Overwhelm
                    } else {
                        Stance::Unleash
                    }
                } else if rng.below(3) == 0 {
                    Stance::Unleash
                } else {
                    Stance::Marshal
                }
            }
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
    fn dummy_always_marshals() {
        let mut rng = Rng::new(1);
        assert_eq!(StancePolicy::Dummy.stance(0, 0, &mut rng), Stance::Marshal);
    }
}
