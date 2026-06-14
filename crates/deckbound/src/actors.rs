//! Combatants and the stance-policies creatures duel with.
//!
//! A hero is human-driven; a creature plays the duel through a **stance-policy** —
//! its "deck" of stances. Policies see only the *public* Edge banks (both sides),
//! never the human's hidden pick this beat, so the duel stays a blind read.

use engine::Rng;

use crate::duel::Stance;
use crate::stats::Body;

/// A player-controlled fighter.
#[derive(Clone, Debug)]
pub struct Hero {
    pub name: String,
    pub role: String,
    pub body: Body,
    /// Damage of a 0-Edge strike.
    pub base: u32,
    /// How many foes it can keep track of; foes beyond this free-hit it each
    /// beat. Normal fighters have 1; gods have more.
    pub bandwidth: u32,
}

impl Hero {
    pub fn is_down(&self) -> bool {
        self.body.is_down()
    }
}

/// How a creature picks its stances — the AI "deck".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StancePolicy {
    /// Always gathers — never threatens. A sandbag to learn Build + Unleash on.
    Dummy,
    /// Winds up, then swings: Marshal until loaded, then Unleash. Telegraphs a
    /// big blow to read and Parry-steal.
    Brute,
    /// Hides behind the guard — almost always Parry. Teaches Overwhelm.
    Turtle,
    /// A real opponent: parries when you're loaded, swings when it is, gathers
    /// otherwise, with enough noise to stay unreadable.
    Duelist,
    /// Winds up, then **throws**: Marshal until loaded, then Overwhelm. Teaches
    /// that an Unleash beats an Overwhelm (don't Parry a thrower).
    Grappler,
    /// Relentless swinger: mostly Unleash. Teaches that trading Unleashes hurts
    /// you both - Parry it instead.
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
}

/// A non-player fighter.
#[derive(Clone, Debug)]
pub struct Creature {
    pub name: String,
    pub role: String,
    pub body: Body,
    pub base: u32,
    pub bandwidth: u32,
    pub policy: StancePolicy,
}

impl Creature {
    pub fn is_down(&self) -> bool {
        self.body.is_down()
    }

    /// The creature's stance this beat. `own` is its Edge, `foe` the hero's Edge
    /// (both public); `rng` supplies the bluff.
    pub fn stance(&self, own: u32, foe: u32, rng: &mut Rng) -> Stance {
        match self.policy {
            StancePolicy::Dummy => Stance::Marshal,
            StancePolicy::Brute => {
                if own >= 3 {
                    Stance::Unleash
                } else {
                    Stance::Marshal
                }
            }
            StancePolicy::Turtle => {
                // Mostly guards; occasionally gathers so it isn't a pure wall.
                if rng.below(5) == 0 {
                    Stance::Marshal
                } else {
                    Stance::Parry
                }
            }
            StancePolicy::Grappler => {
                // Builds, then throws - so an Unleash beats it.
                if own < 2 {
                    Stance::Marshal
                } else {
                    Stance::Overwhelm
                }
            }
            StancePolicy::Aggressor => {
                // Mostly swings; occasionally winds up for a bigger one.
                if rng.below(10) < 7 {
                    Stance::Unleash
                } else {
                    Stance::Marshal
                }
            }
            StancePolicy::Duelist => {
                if foe >= 2 {
                    // You're loaded — it expects a big blow. Mostly parry, but
                    // sometimes overwhelm-bait or gather to stay honest.
                    match rng.below(4) {
                        0 => Stance::Marshal,
                        1 => Stance::Overwhelm,
                        _ => Stance::Parry,
                    }
                } else if own >= 2 {
                    // It's loaded — cash, or feint a parry-punish.
                    if rng.below(3) == 0 {
                        Stance::Overwhelm
                    } else {
                        Stance::Unleash
                    }
                } else {
                    // Neutral — gather, with the odd poke.
                    if rng.below(3) == 0 {
                        Stance::Unleash
                    } else {
                        Stance::Marshal
                    }
                }
            }
        }
    }
}
