//! The state of a Deckbound combat in progress.
//!
//! The fight is simultaneous and hidden, but it fits the turn-based `Game`
//! contract by walking a small state machine: first set the [`Phase::Formation`]
//! (assign each hero a line), then each round gather declarations one staged
//! choice at a time ([`Phase::Declaring`] — pick a hero, a play, then a target),
//! with a `Back` step that rewinds. Once the last living hero has committed,
//! [`crate::resolve`] settles the whole round at once.

use engine::{Outcome, Rng};

use crate::actors::{Creature, Hero, Line, Play};

/// One hero's committed choice for the round.
#[derive(Clone, Debug)]
pub struct Decl {
    pub play: Play,
    pub target: Option<usize>,
}

/// Where the round is in its decision flow.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Assigning each hero to the front or back line, before the fight.
    Formation,
    /// Gathering this round's declarations. `hero` is the one being declared for
    /// (`None` while choosing whom); `play` is their chosen play awaiting a
    /// target.
    Declaring {
        hero: Option<usize>,
        play: Option<Play>,
    },
}

/// The full state of a combat.
#[derive(Clone, Debug)]
pub struct State {
    /// The round number, counting from 1.
    pub round: u32,
    /// The party, in seat order.
    pub heroes: Vec<Hero>,
    /// The warband.
    pub creatures: Vec<Creature>,
    /// This round's committed declarations, one slot per hero.
    pub declarations: Vec<Option<Decl>>,
    /// The order declarations were committed this round (so `Back` can undo the
    /// most recent).
    pub declared_order: Vec<usize>,
    /// Formation progress: each hero's assigned line, or `None` until placed.
    pub formation: Vec<Option<Line>>,
    /// The order heroes were placed (so `Back` can undo the most recent).
    pub formation_order: Vec<usize>,
    /// Where the decision flow currently sits.
    pub phase: Phase,
    /// A play-by-play of the most recent resolution, for the table view.
    pub log: Vec<String>,
    /// All randomness (the Ironclad's bluff) flows from here.
    pub rng: Rng,
    /// The seed this battle was built from, so it can be replayed.
    pub seed: u64,
    /// Set once the fight ends. `Win(0)` = the party prevails; `Win(1)` = the
    /// party falls (the world prevails) — the trait has no "everyone lost".
    pub outcome: Option<Outcome>,
}

impl State {
    /// Whether every hero has been assigned a line.
    pub fn formation_complete(&self) -> bool {
        self.formation.iter().all(Option::is_some)
    }

    /// Living heroes that still owe a declaration this round.
    pub fn undeclared_living(&self) -> Vec<usize> {
        self.heroes
            .iter()
            .enumerate()
            .filter(|(i, h)| !h.is_down() && self.declarations[*i].is_none())
            .map(|(i, _)| i)
            .collect()
    }

    /// Whether every living hero has declared (time to resolve the round).
    pub fn all_alive_declared(&self) -> bool {
        self.heroes
            .iter()
            .enumerate()
            .all(|(i, h)| h.is_down() || self.declarations[i].is_some())
    }

    /// Clears the round's declarations for the next round.
    pub fn clear_declarations(&mut self) {
        for slot in &mut self.declarations {
            *slot = None;
        }
        self.declared_order.clear();
    }

    pub fn living_heroes(&self) -> usize {
        self.heroes.iter().filter(|h| !h.is_down()).count()
    }

    pub fn living_creatures(&self) -> usize {
        self.creatures.iter().filter(|c| c.alive()).count()
    }

    /// A hero is *holding* this round when its declared play is not attacking.
    pub fn is_holding(&self, hero: usize) -> bool {
        match &self.declarations[hero] {
            Some(decl) => !decl.play.is_attacking(),
            None => false,
        }
    }

    /// The combined drag of the front line: the summed Speed of living
    /// front-line heroes who are holding the wall this round.
    pub fn front_drag(&self) -> u32 {
        self.heroes
            .iter()
            .enumerate()
            .filter(|(i, h)| !h.is_down() && h.line == Line::Front && self.is_holding(*i))
            .map(|(_, h)| h.speed)
            .sum()
    }
}
