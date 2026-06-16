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
    /// The foe phase: each un-engaged foe attacks; the targeted hero picks Defend / Counter
    /// / Eat for the attack at the front of `foe_queue`.
    FoePhase,
}

/// The active Clash: who's in it, each side's **Force** (per-duel, public), the beat
/// counter, and a run of no-connect beats for the termination backstop. `defending` marks
/// a Focus-defense (the foe is reset afterward — the hero can survive but not damage it).
#[derive(Clone, Copy, Debug)]
pub struct Duel {
    pub hero: usize,
    pub foe: usize,
    pub hero_force: u32,
    pub foe_force: u32,
    pub beat: u32,
    pub stall: u32,
    /// True when the hero is **defending** a foe-initiated attack (foe reset on end);
    /// false when the hero **initiated** (mutual, results stick).
    pub defending: bool,
    /// True when this duel is resolved during the foe phase (so it returns to `FoePhase`
    /// to finish the queue, not to `Choosing`).
    pub from_foe_phase: bool,
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
    /// The foe phase work-list: `(foe, target_hero)` attacks still to resolve.
    pub foe_queue: Vec<(usize, usize)>,
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

    /// A hero may still take an action this round (pay-after: act while Tempo ≥ 0; the
    /// action that drives it negative is the last).
    pub fn hero_can_act(&self, i: usize) -> bool {
        self.heroes
            .get(i)
            .is_some_and(|h| !h.fallen && h.tempo >= 0)
    }

    /// Clear the per-round plan and size `engaged` to the current foes.
    pub fn reset_round_plan(&mut self) {
        self.engaged = vec![false; self.creatures.len()];
        self.queued_cards.clear();
        self.foe_queue.clear();
    }
}
