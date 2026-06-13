//! The state of a Deckbound combat in progress.
//!
//! The fight is simultaneous and hidden, but it fits the turn-based `Game`
//! contract by collecting each living hero's secret declaration one at a time;
//! once the last is in, [`crate::resolve`] settles the whole round at once
//! (creatures decide, the gauntlet charges, the exchange resolves, recover).

use engine::{Outcome, Rng};

use crate::actors::{Creature, Hero, Line, Play};

/// One hero's committed choice for the round: a play, and a target if it needs
/// one (an index into [`State::creatures`]).
#[derive(Clone, Debug)]
pub struct Decl {
    pub play: Play,
    pub target: Option<usize>,
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
    /// This round's declarations, one slot per hero (`None` = not yet chosen,
    /// or a downed hero).
    pub declarations: Vec<Option<Decl>>,
    /// A play-by-play of the most recent resolution, for the table view.
    pub log: Vec<String>,
    /// All randomness (the Ironclad's bluff) flows from here.
    pub rng: Rng,
    /// Set once the fight ends. `Win(0)` = the party prevails; `Win(1)` = the
    /// party falls (the world prevails) — the trait has no "everyone lost".
    pub outcome: Option<Outcome>,
}

impl State {
    /// The first living hero still owing a declaration this round, or `None`
    /// when the fight is over.
    pub fn current_hero(&self) -> Option<usize> {
        if self.outcome.is_some() {
            return None;
        }
        self.heroes
            .iter()
            .enumerate()
            .find(|(i, h)| !h.is_down() && self.declarations[*i].is_none())
            .map(|(i, _)| i)
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
    }

    pub fn living_heroes(&self) -> usize {
        self.heroes.iter().filter(|h| !h.is_down()).count()
    }

    pub fn living_creatures(&self) -> usize {
        self.creatures.iter().filter(|c| c.alive()).count()
    }

    /// A hero is *holding* this round when its declared play is not attacking.
    /// (A hero with no declaration yet counts as not holding.)
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
