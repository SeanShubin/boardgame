//! The state of a combat in progress.
//!
//! Combat is a sequence of **rounds**. In a round the human engages foes (spending
//! **tempo** = Speed) through interactive duels; at round end the creatures act and
//! foes the heroes couldn't **cover** (focus = Mind) free-hit. Edge is per-duel.

use engine::{Outcome, Rng};

use crate::actor::Actor;
use crate::scenarios::Scenario;

/// Which menu page is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Menu {
    Top,
    Scenarios,
    God,
    Tutorial,
}

/// Where the game is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Menu(Menu),
    /// The player's phase: engage foes, play actions, or end the round.
    Choosing,
    /// An interactive duel is running, beat by beat.
    Combat,
}

/// The active duel: who's in it, both Edge banks (public), the beat counter, and a
/// run of mutual-Marshals for the stall backstop.
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
    pub round: u32,
    pub heroes: Vec<Actor>,
    pub creatures: Vec<Actor>,
    pub phase: Phase,
    pub duel: Option<Duel>,
    pub scenario: Option<Scenario>,
    pub exiting: bool,
    pub log: Vec<String>,
    pub rng: Rng,
    pub seed: u64,
    pub outcome: Option<Outcome>,
    /// Foes dueled or traded this round — they do not also free-hit (§1.8).
    pub engaged: Vec<bool>,
    /// Action cards played this round, applied in tiers at round end (§1.9): (hero, idx).
    pub queued_cards: Vec<(usize, usize)>,
}

impl State {
    pub fn first_living_hero(&self) -> Option<usize> {
        self.heroes.iter().position(|h| !h.is_down())
    }

    pub fn first_living_creature(&self) -> Option<usize> {
        self.creatures.iter().position(|c| !c.is_down())
    }

    pub fn living_heroes(&self) -> usize {
        // A mortally-wounded hero (Body 0, not yet fallen) is still in the fight this
        // round — defeat is tallied at the round boundary (§1.9), so count `!fallen`.
        self.heroes.iter().filter(|h| !h.fallen).count()
    }

    pub fn living_creatures(&self) -> usize {
        self.creatures.iter().filter(|c| !c.is_down()).count()
    }

    /// A hero may still take an offensive engagement this round.
    pub fn hero_can_act(&self, i: usize) -> bool {
        self.heroes
            .get(i)
            .is_some_and(|h| !h.fallen && !h.exposed && h.tempo >= 0)
    }

    /// Clear the per-round plan and size `engaged` to the current foes.
    pub fn reset_round_plan(&mut self) {
        self.engaged = vec![false; self.creatures.len()];
        self.queued_cards.clear();
    }
}
