//! The state of a duel-sandbox combat in progress.
//!
//! Combat is a sequence of one-on-one duels. The human and a creature are paired
//! (first living vs first living); the human picks stances, the creature reads back
//! through its policy, and each beat resolves until a strike lands and the duel
//! ends. Then the next pair forms. Edge is **per-duel** — it resets each duel.

use engine::{Outcome, Rng};

use crate::actors::{Creature, Hero};
use crate::scenarios::Scenario;

/// Which menu page is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Menu {
    Top,
    Scenarios,
    Tutorial,
}

/// Where the game is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Menu(Menu),
    /// Matchmaking: pick which hero duels which foe (skipped when there is only
    /// one of each).
    Choosing,
    Combat,
}

/// The active duel: who's in it, both Edge banks (public), the beat counter, and
/// a run of mutual-Marshals for the stall backstop.
#[derive(Clone, Copy, Debug)]
pub struct Duel {
    pub hero: usize,
    pub foe: usize,
    pub hero_edge: u32,
    pub foe_edge: u32,
    pub beat: u32,
    pub double_marshals: u32,
}

#[derive(Clone, Debug)]
pub struct State {
    /// Which duel of the fight this is (1-based).
    pub duel_no: u32,
    pub heroes: Vec<Hero>,
    pub creatures: Vec<Creature>,
    pub phase: Phase,
    pub duel: Option<Duel>,
    pub scenario: Option<Scenario>,
    pub exiting: bool,
    pub log: Vec<String>,
    pub rng: Rng,
    pub seed: u64,
    pub outcome: Option<Outcome>,
}

impl State {
    pub fn first_living_hero(&self) -> Option<usize> {
        self.heroes.iter().position(|h| !h.is_down())
    }

    pub fn first_living_creature(&self) -> Option<usize> {
        self.creatures.iter().position(|c| !c.is_down())
    }

    pub fn living_heroes(&self) -> usize {
        self.heroes.iter().filter(|h| !h.is_down()).count()
    }

    pub fn living_creatures(&self) -> usize {
        self.creatures.iter().filter(|c| !c.is_down()).count()
    }
}
