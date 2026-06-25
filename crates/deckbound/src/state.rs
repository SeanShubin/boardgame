//! The state of a combat in progress (§4 charge-and-gauntlet system).
//!
//! A round is a phase machine: **Assemble** (declare who charges, and **Muster** standing/persistent
//! cards before the gauntlet) → on Deploy the **gauntlet** resolves (the two charge-columns thread
//! through each other, producing Outriders who broke through and Vanguard who stopped) → **Outrider**
//! (outriders strike the enemy Rearguard) → **Rearguard** (rearguards fire ranged) → refresh. A same-range
//! engagement is a **trade** unless the optional **Clash** module is on (then the four-card mix-up
//! runs, [`Phase::Clash`]). Resolution is order-independent within each phase.

use engine::{Outcome, Rng};

use crate::actor::Actor;
use crate::campaign::CampaignState;
use crate::ruleset::Ruleset;
use crate::scenarios::Scenario;

/// Which menu page is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// Where the round is (§4 charge-and-gauntlet). `Assemble` selects who charges **and** is the
/// **Muster** window (play standing/persistent cards before the gauntlet); on Deploy the **gauntlet**
/// resolves automatically (the two charge-columns thread through each other), producing Outriders
/// (broke through) and Vanguard (stopped); then the Outrider and Rearguard phases.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Menu(Menu),
    /// Select which heroes charge (vs. hold back as Rearguard) and **Muster** standing cards, then
    /// Deploy to run the gauntlet.
    Assemble,
    /// Outriders (broke through the gauntlet) strike the enemy Rearguard, then Vanguard.
    Outrider,
    /// Rearguards fire ranged at the enemy front.
    Rearguard,
    /// An interactive four-card Clash (the optional module) for a 1v1 same-range duel.
    Clash,
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

/// The per-round working plan for the §4 charge-and-gauntlet system.
#[derive(Clone, Debug, Default)]
pub struct Round {
    /// Per hero: did it **charge** (commit to the front)? `false` = held back (Rearguard). A charger is a
    /// Vanguard unless it also **flanks** (then an Outrider). Sized to heroes.
    pub hero_charging: Vec<bool>,
    /// Per creature: same.
    pub foe_charging: Vec<bool>,
    /// Per hero: does this charger **flank** — declare itself an **Outrider** (attempt to cross) rather
    /// than hold as a Vanguard? Only meaningful when charging. Sized to heroes.
    pub hero_flank: Vec<bool>,
    /// Per creature: same.
    pub foe_flank: Vec<bool>,
    /// Heroes / creatures who **broke through** the gauntlet and became Outriders. A charger that
    /// did *not* break through is a Vanguard (stopped at the front); a non-charger is a Rearguard.
    pub hero_skirmisher: Vec<bool>,
    pub foe_skirmisher: Vec<bool>,
    /// Actors who have already acted in the current target phase (Outrider / Rearguard).
    pub hero_acted: Vec<bool>,
    pub foe_acted: Vec<bool>,
    /// §4.4 per-role-per-round cap: the role tracks each actor has already played a card of this
    /// round (reset each round). A role card is playable only if its track is not yet present here.
    pub hero_roles_played: Vec<Vec<crate::currency::Currency>>,
    pub foe_roles_played: Vec<Vec<crate::currency::Currency>>,
    /// PvP: which side is currently committing this phase (0 = heroes, 1 = creatures). Always
    /// 0 in PvE.
    pub committing: u8,
    /// True once the deterministic-base trade is replaced by the interactive Clash module.
    pub clash_mode: bool,
}

impl Round {
    pub fn sized(heroes: usize, foes: usize) -> Self {
        Round {
            hero_charging: vec![false; heroes],
            foe_charging: vec![false; foes],
            hero_flank: vec![false; heroes],
            foe_flank: vec![false; foes],
            hero_skirmisher: vec![false; heroes],
            foe_skirmisher: vec![false; foes],
            hero_acted: vec![false; heroes],
            foe_acted: vec![false; foes],
            hero_roles_played: vec![Vec::new(); heroes],
            foe_roles_played: vec![Vec::new(); foes],
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
    /// Pre-game tuning parameters (§0 separable balance) — the round/roster bounds, set once before
    /// the battle like the seed and the Clash module. Live play uses [`Ruleset::default`]; analysis
    /// tooling uses [`Ruleset::analysis`]. See [`crate::ruleset`].
    pub ruleset: Ruleset,
    /// When `Some`, the menu launched the **world-map Campaign** (§8); the [`Deckbound`] game
    /// delegates to it. Boxed because the campaign embeds combat `State`s (its battles).
    ///
    /// [`Deckbound`]: crate::game::Deckbound
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
    pub fn s_charging(&self, side: u8) -> &[bool] {
        if side == 0 {
            &self.plan.hero_charging
        } else {
            &self.plan.foe_charging
        }
    }
    pub fn s_charging_mut(&mut self, side: u8) -> &mut Vec<bool> {
        if side == 0 {
            &mut self.plan.hero_charging
        } else {
            &mut self.plan.foe_charging
        }
    }
    pub fn s_flank(&self, side: u8) -> &[bool] {
        if side == 0 {
            &self.plan.hero_flank
        } else {
            &self.plan.foe_flank
        }
    }
    pub fn s_flank_mut(&mut self, side: u8) -> &mut Vec<bool> {
        if side == 0 {
            &mut self.plan.hero_flank
        } else {
            &mut self.plan.foe_flank
        }
    }
    pub fn s_skirm(&self, side: u8) -> &[bool] {
        if side == 0 {
            &self.plan.hero_skirmisher
        } else {
            &self.plan.foe_skirmisher
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
