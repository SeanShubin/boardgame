//! The state of a combat in progress.
//!
//! Combat is a sequence of **rounds**. In a round the human engages foes (spending
//! **tempo** = Speed) through interactive duels; at round end the creatures act and
//! foes the heroes couldn't **cover** (focus = Mind) free-hit. Edge is per-duel.

use engine::{Outcome, Rng};

use crate::actor::Actor;
use crate::duel::Move;
use crate::scenarios::Scenario;

/// Which menu page is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Menu {
    Top,
    Scenarios,
    God,
    Tutorial,
    Versus,
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
    /// A hero is diving the enemy gauntlet toward a back-line foe (§4): push through (pay
    /// the guards' combined Speed as Tempo and eat their hits) or halt.
    HeroDive,
    /// A creature runner is diving the hero gauntlet (§4): the player picks which front-line
    /// heroes intercept (each pays Tempo = the runner's Speed), then lets it resolve.
    FoeDive,
    /// A hotseat PvP duel (§3.4 PvP): two human sides commit moves in lockstep — hidden,
    /// simultaneous — beat by beat until one falls.
    Versus,
}

/// A hotseat PvP duel in progress (§3.4). Side A is `heroes[0]` (PlayerId 0), side B is
/// `creatures[0]` (PlayerId 1); both are human. Commit is **hidden + simultaneous**: side A
/// commits into `committed` (unrevealed), then side B replies and the beat resolves.
#[derive(Clone, Copy, Debug)]
pub struct Versus {
    pub a_force: u32,
    pub b_force: u32,
    pub beat: u32,
    /// No-connect beats in a row — a stalemate backstop (§1.6).
    pub stall: u32,
    /// Side A's hidden move, awaiting side B's reply.
    pub committed: Option<Move>,
}

impl Versus {
    pub fn new() -> Self {
        Versus {
            a_force: 0,
            b_force: 0,
            beat: 0,
            stall: 0,
            committed: None,
        }
    }
}

impl Default for Versus {
    fn default() -> Self {
        Self::new()
    }
}

/// A dive in progress across the gauntlet (§4). For [`Phase::HeroDive`] the runner is a hero
/// and the guards/target are foes; for [`Phase::FoeDive`] the runner is a creature and the
/// guards/target are heroes.
#[derive(Clone, Debug)]
pub struct Dive {
    /// The diving actor's index (a hero for HeroDive, a creature for FoeDive).
    pub runner: usize,
    /// The back-line target on the far side.
    pub target: usize,
    /// Living front-line guards on the far side who may intercept.
    pub guards: Vec<usize>,
    /// Guards chosen to intercept so far (FoeDive — the player builds this up).
    pub chosen: Vec<usize>,
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
    /// The gauntlet dive in progress (HeroDive / FoeDive), if any.
    pub dive: Option<Dive>,
    /// The hotseat PvP duel in progress (Phase::Versus), if any.
    pub versus: Option<Versus>,
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
        self.dive = None;
    }
}
