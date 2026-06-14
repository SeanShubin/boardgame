//! Combatants and the read-policies creatures duel with.
//!
//! A hero is human-driven; a creature plays the duel through a **read-policy** —
//! its "deck" of reads. Policies see only the *public* Edge banks (both sides),
//! never the human's hidden pick this beat, so the duel stays a blind read.

use engine::Rng;

use crate::duel::Read;
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

/// How a creature picks its reads — the AI "deck".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadPolicy {
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

impl ReadPolicy {
    pub fn label(self) -> &'static str {
        match self {
            ReadPolicy::Dummy => "Sandbag",
            ReadPolicy::Brute => "Brute",
            ReadPolicy::Turtle => "Turtle",
            ReadPolicy::Duelist => "Duelist",
            ReadPolicy::Grappler => "Grappler",
            ReadPolicy::Aggressor => "Berserker",
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
    pub policy: ReadPolicy,
}

impl Creature {
    pub fn is_down(&self) -> bool {
        self.body.is_down()
    }

    /// The creature's read this beat. `own` is its Edge, `foe` the hero's Edge
    /// (both public); `rng` supplies the bluff.
    pub fn read(&self, own: u32, foe: u32, rng: &mut Rng) -> Read {
        match self.policy {
            ReadPolicy::Dummy => Read::Marshal,
            ReadPolicy::Brute => {
                if own >= 3 {
                    Read::Unleash
                } else {
                    Read::Marshal
                }
            }
            ReadPolicy::Turtle => {
                // Mostly guards; occasionally gathers so it isn't a pure wall.
                if rng.below(5) == 0 {
                    Read::Marshal
                } else {
                    Read::Parry
                }
            }
            ReadPolicy::Grappler => {
                // Builds, then throws - so an Unleash beats it.
                if own < 2 {
                    Read::Marshal
                } else {
                    Read::Overwhelm
                }
            }
            ReadPolicy::Aggressor => {
                // Mostly swings; occasionally winds up for a bigger one.
                if rng.below(10) < 7 {
                    Read::Unleash
                } else {
                    Read::Marshal
                }
            }
            ReadPolicy::Duelist => {
                if foe >= 2 {
                    // You're loaded — it expects a big blow. Mostly parry, but
                    // sometimes overwhelm-bait or gather to stay honest.
                    match rng.below(4) {
                        0 => Read::Marshal,
                        1 => Read::Overwhelm,
                        _ => Read::Parry,
                    }
                } else if own >= 2 {
                    // It's loaded — cash, or feint a parry-punish.
                    if rng.below(3) == 0 {
                        Read::Overwhelm
                    } else {
                        Read::Unleash
                    }
                } else {
                    // Neutral — gather, with the odd poke.
                    if rng.below(3) == 0 {
                        Read::Unleash
                    } else {
                        Read::Marshal
                    }
                }
            }
        }
    }
}
