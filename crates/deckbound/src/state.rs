//! The state of a combat in progress (§4 **sub-phase-schedule** model).
//!
//! A round: **Marshal** (declare each unit's intention — Vanguard / Outrider / Rearguard; **Reveal** locks
//! positions; **Ready** lands `cast: Standing` buffs/braces) → **Engage** (the fixed sub-phase schedule
//! resolves: Intercept `V→O`, Volley `R→O`, Raid `O→R`, Clash `R→V`+`V→V`, Breach `V→R`+`O→V`+`O→O`, §4.6) →
//! **Refresh** (the Lull: Tempo resets, Health persists, round++). All Tempo across the whole schedule is paid
//! from **one shared per-round pool**. The optional four-card **Clash** module ([`Phase::Clash`]) replaces
//! a same-range trade when on. Resolution is order-independent **within** an sub-phase (§1.9); the
//! schedule order is the only timing.

use contract::Outcome;
use engine::Rng;
use serde::{Deserialize, Serialize};

use crate::actor::{Actor, Intention};
use crate::campaign::CampaignState;
use crate::ruleset::Ruleset;
use crate::scenarios::Scenario;

/// Which menu page is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Menu {
    Top,
    Cooperation,
    God,
    Tutorial,
    Versus,
    /// The encyclopedia's category list (the top of the rules hierarchy).
    Rules,
    /// The entries within one rules category (by its index in `categories()`).
    Category(usize),
    /// The card catalog: every card, browsable by section.
    Catalog,
    /// One card's detail page (by its index in `card_catalog()`).
    CardDetail(usize),
}

/// Where the round is (§4). The interactive phase is **Marshal** (pick each unit's intention +
/// grouping, cast `Standing` buffs); **Engage** resolves the fixed sub-phase schedule (§4.6) and the
/// **Refresh** (the Lull) is the transition to the next round's Marshal. [`Phase::Clash`] is the
/// optional 1v1 module. (Reveal and Ready are conceptual sub-steps of Marshal, not separate variants.)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Menu(Menu),
    /// §4 Marshal — pick each unit's **intention** (Vanguard / Outrider / Rearguard), group them, and
    /// cast `Standing` buffs/braces (auto-land at the Ready sub-step). Advancing begins the sub-phase schedule.
    Marshal,
    /// §4.6 — the sub-phase schedule resolves (Intercept → Volley → Raid → Clash → Breach), each step a
    /// §1.9 boundary, over the one shared Tempo pool. Resolved by the resolver (`combat`).
    Engage,
    /// An interactive four-card Clash (the optional module) for a 1v1 same-range duel.
    Clash,
}

/// The active interactive Clash (module): the two duelists and their per-duel Force.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Clash {
    pub hero: usize,
    pub foe: usize,
    pub hero_force: u32,
    pub foe_force: u32,
    pub beat: u32,
    pub stall: u32,
}

/// A held (`resolve: Reckoning`) spell wound up earlier in the round, resolving in the **last** sub-phase
/// (§4.6). It **fizzles** if its caster is killed before then (disrupt, §4.6).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Deferred {
    /// Which side cast it (0 = heroes, 1 = creatures).
    pub side: u8,
    /// The caster's index in its pool.
    pub caster: usize,
    /// The wound-up card (its effects land last).
    pub card: crate::cards::Card,
    /// The caster's Offense snapshot at cast time (Might for the effect).
    pub offense: crate::stats::Offense,
    /// The caster's name (for the log, in case the index shifts).
    pub name: String,
}

/// The per-round working plan for the §4 sub-phase-schedule model. Intentions and grouping are declared
/// at **Marshal**; the schedule then resolves over them (§4.6).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Round {
    /// Per hero: its declared **intention** this round (§4). A group shares one intention. Sized to heroes.
    pub hero_intent: Vec<Intention>,
    /// Per creature: same.
    pub foe_intent: Vec<Intention>,
    /// Per hero: its **group id** (units sharing an id are bound into one group, §4.5). Default = the
    /// unit's own index (ungrouped). Sized to heroes.
    pub hero_group: Vec<usize>,
    pub foe_group: Vec<usize>,
    /// Actors who have already acted in the current interactive phase (Marshal).
    pub hero_acted: Vec<bool>,
    pub foe_acted: Vec<bool>,
    /// §4.6 — deferred (`resolve: Reckoning`) spells wound up this round (resolve in the last sub-phase).
    pub deferred: Vec<Deferred>,
    /// PvP: which side is currently committing this phase (0 = heroes, 1 = creatures). Always 0 in PvE.
    pub committing: u8,
    /// True once the deterministic-base trade is replaced by the interactive Clash module.
    pub clash_mode: bool,
}

impl Round {
    pub fn sized(heroes: usize, foes: usize) -> Self {
        Round {
            // Default intention: a melee unit fronts (Vanguard), a ranged unit deals (Rearguard); `game`
            // overrides this per-actor from the attack profile / stats when a round opens.
            hero_intent: vec![Intention::Vanguard; heroes],
            foe_intent: vec![Intention::Vanguard; foes],
            hero_group: (0..heroes).collect(),
            foe_group: (0..foes).collect(),
            hero_acted: vec![false; heroes],
            foe_acted: vec![false; foes],
            deferred: Vec::new(),
            committing: 0,
            clash_mode: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub round: u32,
    pub heroes: Vec<Actor>,
    pub creatures: Vec<Actor>,
    pub phase: Phase,
    /// §4.6 — the in-flight sub-phase-schedule resolution cursor. `Some` while a round is resolving
    /// (between [`combat::step`](crate::combat::step) calls); `None` at rest (Marshal, Refresh).
    /// Today resolution runs to completion synchronously inside `apply(Deploy)`, so live play never
    /// rests with this set — it exists so the resolution is observable one atomic step at a time and so
    /// the whole machine round-trips through RON.
    #[serde(default)]
    pub resolution: Option<crate::combat::Resolution>,
    pub plan: Round,
    pub clash: Option<Clash>,
    #[serde(skip)]
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
    /// Pre-game tuning parameters (§0 separable balance) — the round/roster bounds, set once before
    /// the battle like the seed and the Clash module. Live play uses [`Ruleset::default`]; analysis
    /// tooling uses [`Ruleset::analysis`]. See [`crate::ruleset`].
    pub ruleset: Ruleset,
    /// When `Some`, the menu launched the **world-map Campaign** (§8); the [`Deckbound`] game
    /// delegates to it. Boxed because the campaign embeds combat `State`s (its battles).
    ///
    /// [`Deckbound`]: crate::game::Deckbound
    #[serde(skip)]
    pub campaign: Option<Box<CampaignState>>,
}

impl State {
    /// The pool of the side currently committing (PvP) / always heroes in PvE.
    pub fn committing_is_foe(&self) -> bool {
        self.plan.committing == 1
    }

    // ---- side-generic accessors (side 0 = heroes, 1 = creatures) ----
    pub fn s_pool(&self, side: u8) -> &[Actor] {
        if side == 0 {
            &self.heroes
        } else {
            &self.creatures
        }
    }
    pub fn s_len(&self, side: u8) -> usize {
        self.s_pool(side).len()
    }
    /// Per-unit declared intention of `side` (§4).
    pub fn s_intent(&self, side: u8) -> &[Intention] {
        if side == 0 {
            &self.plan.hero_intent
        } else {
            &self.plan.foe_intent
        }
    }
    pub fn s_intent_mut(&mut self, side: u8) -> &mut Vec<Intention> {
        if side == 0 {
            &mut self.plan.hero_intent
        } else {
            &mut self.plan.foe_intent
        }
    }
    /// Per-unit group id of `side` (§4.5; units sharing an id are one group).
    pub fn s_group(&self, side: u8) -> &[usize] {
        if side == 0 {
            &self.plan.hero_group
        } else {
            &self.plan.foe_group
        }
    }
    pub fn s_group_mut(&mut self, side: u8) -> &mut Vec<usize> {
        if side == 0 {
            &mut self.plan.hero_group
        } else {
            &mut self.plan.foe_group
        }
    }
    pub fn s_acted(&self, side: u8) -> &[bool] {
        if side == 0 {
            &self.plan.hero_acted
        } else {
            &self.plan.foe_acted
        }
    }
    pub fn s_acted_mut(&mut self, side: u8) -> &mut Vec<bool> {
        if side == 0 {
            &mut self.plan.hero_acted
        } else {
            &mut self.plan.foe_acted
        }
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
