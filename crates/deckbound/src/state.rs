//! The state of a combat in progress (§4 lane commitment system).
//!
//! A round is a phase machine: **Muster** (assign Vanguard / Reserve) → **Slip** (Vanguard
//! hold or slip) → resolve the Vanguard phase → **Skirmish** (skirmishers pick targets) →
//! resolve → **Reserve** (reserves pick targets / aid) → resolve → refresh. A same-range
//! engagement is a **trade** unless the optional **Clash** module is on (then the four-card
//! mix-up runs, [`Phase::Clash`]). Resolution is order-independent within each phase.

use engine::{Outcome, Rng};

use crate::actor::Actor;
use crate::scenarios::Scenario;

/// Which menu page is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Menu {
    Top,
    Cooperation,
    God,
    Tutorial,
    Versus,
}

/// Where the round is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Menu(Menu),
    /// Assign each hero to the Vanguard or the Reserve, then deploy.
    Muster,
    /// Place each Vanguard hero into a specific lane (stacking is the choice, §4).
    Assign,
    /// Each Vanguard hero chooses to hold its lane or slip past (→ Skirmisher).
    Slip,
    /// Skirmishers pick targets.
    Skirmish,
    /// Reserves pick targets (or aid allies).
    Reserve,
    /// An interactive four-card Clash (the optional module) for a 1v1 same-range duel.
    Clash,
}

/// A lane: the Vanguard Actors of each side that meet here (§4). The smaller side has one per
/// lane; the larger side **stacks** its surplus.
#[derive(Clone, Debug, Default)]
pub struct Lane {
    pub heroes: Vec<usize>,
    pub foes: Vec<usize>,
}

/// The active interactive Clash (module): the two duelists and their per-duel Force.
#[derive(Clone, Copy, Debug)]
pub struct Clash {
    pub hero: usize,
    pub foe: usize,
    pub hero_force: u32,
    pub foe_force: u32,
    pub beat: u32,
    pub stall: u32,
}

/// The per-round working plan for the lane commitment system.
#[derive(Clone, Debug, Default)]
pub struct Round {
    pub lanes: Vec<Lane>,
    /// Per hero: which lane it's a Vanguard in (`Some`) or `None` for Reserve. Sized to heroes.
    pub hero_lane: Vec<Option<usize>>,
    /// Per creature: same.
    pub foe_lane: Vec<Option<usize>>,
    /// Per Vanguard: `Some(true)` = slip, `Some(false)` = hold, `None` = not yet decided.
    pub hero_slip: Vec<Option<bool>>,
    /// Creature slip choices (PvP — set by the human side B; PvE computes from AI).
    pub foe_slip: Vec<Option<bool>>,
    /// Heroes / creatures who became Skirmishers this round (slipped a lane and survived).
    pub hero_skirmisher: Vec<bool>,
    pub foe_skirmisher: Vec<bool>,
    /// Actors who have already acted in the current target phase (Skirmish / Reserve).
    pub hero_acted: Vec<bool>,
    pub foe_acted: Vec<bool>,
    /// Vanguard awaiting a lane during the Assign phase (the side currently committing).
    pub assign_queue: Vec<usize>,
    /// PvP: which side is currently committing this phase (0 = heroes, 1 = creatures). Always
    /// 0 in PvE.
    pub committing: u8,
    /// True once the deterministic-base trade is replaced by the interactive Clash module.
    pub clash_mode: bool,
}

impl Round {
    pub fn sized(heroes: usize, foes: usize) -> Self {
        Round {
            lanes: Vec::new(),
            hero_lane: vec![None; heroes],
            foe_lane: vec![None; foes],
            hero_slip: vec![None; heroes],
            foe_slip: vec![None; foes],
            hero_skirmisher: vec![false; heroes],
            foe_skirmisher: vec![false; foes],
            hero_acted: vec![false; heroes],
            foe_acted: vec![false; foes],
            assign_queue: Vec::new(),
            committing: 0,
            clash_mode: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct State {
    pub round: u32,
    pub heroes: Vec<Actor>,
    pub creatures: Vec<Actor>,
    pub phase: Phase,
    pub plan: Round,
    pub clash: Option<Clash>,
    pub scenario: Option<Scenario>,
    pub exiting: bool,
    pub log: Vec<String>,
    pub rng: Rng,
    pub seed: u64,
    pub outcome: Option<Outcome>,
    /// True when this scenario uses the optional Clash module for same-range duels.
    pub clash_module: bool,
    /// True when this is a hotseat PvP scenario (both sides human, §3.4).
    pub pvp: bool,
}

impl State {
    /// The pool of the side currently committing (PvP) / always heroes in PvE.
    pub fn committing_is_foe(&self) -> bool {
        self.plan.committing == 1
    }

    // ---- side-generic accessors (side 0 = heroes, 1 = creatures) ----
    pub fn s_pool(&self, side: u8) -> &[Actor] {
        if side == 0 { &self.heroes } else { &self.creatures }
    }
    pub fn s_len(&self, side: u8) -> usize {
        self.s_pool(side).len()
    }
    pub fn s_lane(&self, side: u8) -> &[Option<usize>] {
        if side == 0 { &self.plan.hero_lane } else { &self.plan.foe_lane }
    }
    pub fn s_lane_mut(&mut self, side: u8) -> &mut Vec<Option<usize>> {
        if side == 0 { &mut self.plan.hero_lane } else { &mut self.plan.foe_lane }
    }
    pub fn s_slip_mut(&mut self, side: u8) -> &mut Vec<Option<bool>> {
        if side == 0 { &mut self.plan.hero_slip } else { &mut self.plan.foe_slip }
    }
    pub fn s_skirm(&self, side: u8) -> &[bool] {
        if side == 0 { &self.plan.hero_skirmisher } else { &self.plan.foe_skirmisher }
    }
    pub fn s_acted(&self, side: u8) -> &[bool] {
        if side == 0 { &self.plan.hero_acted } else { &self.plan.foe_acted }
    }
    pub fn s_acted_mut(&mut self, side: u8) -> &mut Vec<bool> {
        if side == 0 { &mut self.plan.hero_acted } else { &mut self.plan.foe_acted }
    }
}

impl State {
    pub fn first_living_hero(&self) -> Option<usize> {
        self.heroes.iter().position(|h| !h.is_down())
    }

    pub fn first_living_creature(&self) -> Option<usize> {
        self.creatures.iter().position(|c| !c.is_down())
    }

    pub fn living_heroes(&self) -> usize {
        self.heroes.iter().filter(|h| !h.fallen).count()
    }

    pub fn living_creatures(&self) -> usize {
        self.creatures.iter().filter(|c| !c.is_down()).count()
    }

    /// A hero may still take an action this round (pay-after: act while Tempo ≥ 0).
    pub fn hero_can_act(&self, i: usize) -> bool {
        self.heroes
            .get(i)
            .is_some_and(|h| !h.fallen && h.tempo >= 0)
    }
}
