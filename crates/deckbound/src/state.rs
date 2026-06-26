//! The state of a combat in progress (§4.6 **six-phase** model).
//!
//! A round is a fixed six-phase machine ([`Phase`], §4.6): **Standoff** (reveal; positions lock;
//! `cast: Standing` buffs/braces auto-land) → **Fray** (the fronts engage — melee and instant ranged
//! resolve simultaneously; deaths here **fix the breach list**) → **Volley** (free Vanguards charge the
//! enemy Rearguard, or flank a survivor; the rear answers **first** — pre-empt) → **Breach** (chargers
//! who survived the Volley land their `resolve: Breach` blows; a kill here **disrupts** a deferred
//! spell) → **Reckoning** (`resolve: Reckoning` effects land last) → **Lull** (refresh: Tempo resets,
//! Health persists, round++). All Tempo across all phases is paid from **one shared per-round pool**.
//! The optional four-card **Clash** module ([`Phase::Clash`]) replaces a same-range trade when on.
//! Resolution is order-independent **within** a phase (§1.9); the phase order is the only timing.

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

/// Where the round is — the §4.6 **six phases**, in fixed order. The Lull (refresh) is not an
/// interactive phase: it is the transition `Breach`/`Reckoning` → next round's `Standoff`. The
/// interactive phases are **Standoff** (declare positions, cast `Standing` buffs), **Fray** (the
/// front clash: pick targets / cast `Strike` abilities), and **Volley** (free Vanguards charge or
/// flank; the rear answers first); **Breach** and **Reckoning** resolve automatically (a charger's
/// blow and a deferred spell, both committed earlier). [`Phase::Clash`] is the optional 1v1 module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Menu(Menu),
    /// §4.6 #1 — reveal commitments; pick each unit's **position** (Vanguard / Rearguard) and cast
    /// `Standing` buffs/braces (auto-land). Advancing begins the Fray.
    Standoff,
    /// §4.6 #2 — the fronts engage: Vanguards strike, instant (`cast: Strike, resolve: OnCast`)
    /// ranged fire and its defenses resolve simultaneously. Deaths here **fix the breach list**.
    Fray,
    /// §4.6 #3 — free Vanguards charge the enemy Rearguard (or flank a survivor); the rear answers
    /// **first** (pre-empt). Instant abilities may fire again here.
    Volley,
    /// §4.6 #4 — chargers who survived the Volley land their `resolve: Breach` blows; a breach-kill
    /// **disrupts** the victim's deferred spell. Resolved automatically from the Volley's charges.
    Breach,
    /// §4.6 #5 — deferred (`resolve: Reckoning`) effects from surviving casters resolve last.
    /// Resolved automatically.
    Reckoning,
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

/// A held (`resolve: Reckoning`) spell wound up in the Fray or the Volley, resolving last (§4.6 #5).
/// It **fizzles** if its caster is killed before the Reckoning (disrupt, §4.6).
#[derive(Clone, Debug)]
pub struct Deferred {
    /// Which side cast it (0 = heroes, 1 = creatures).
    pub side: u8,
    /// The caster's index in its pool.
    pub caster: usize,
    /// The wound-up card (its effects land in the Reckoning).
    pub card: crate::cards::Card,
    /// The caster's Offense snapshot at cast time (Might for the effect).
    pub offense: crate::stats::Offense,
    /// The caster's name (for the log, in case the index shifts).
    pub name: String,
}

/// A charge (or flank) declared in the Volley (§4.6 #3). A **charge** crosses open ground at the enemy
/// Rearguard and lands in the **Breach** (the rear pre-empts it); a **flank** is adjacent melee on a
/// surviving enemy Vanguard and **trades** in the Volley itself (a flank-kill intercepts the target's
/// own charge).
#[derive(Clone, Copy, Debug)]
pub struct Charge {
    /// The charger's side (0 = heroes, 1 = creatures).
    pub side: u8,
    /// The charger's index in its pool.
    pub attacker: usize,
    /// The target's index in the **enemy** pool.
    pub target: usize,
    /// `true` = a **flank** (trades in the Volley); `false` = a **charge** (lands in the Breach).
    pub flank: bool,
}

/// The per-round working plan for the §4.6 six-phase model. Positions are declared at the **Standoff**;
/// the **breach list** (`locked`) is fixed by the **Fray**; charges/flanks are declared in the
/// **Volley**; deferred spells resolve in the **Reckoning**.
#[derive(Clone, Debug, Default)]
pub struct Round {
    /// Per hero: is it a **Vanguard** (`true`, the front) or **Rearguard** (`false`, the back)? A group
    /// shares one position. Declared at the Standoff. Sized to heroes.
    pub hero_vanguard: Vec<bool>,
    /// Per creature: same.
    pub foe_vanguard: Vec<bool>,
    /// §4.6 **breach list / per-unit lock**: per Vanguard, is it **locked** — did it attack an enemy
    /// Vanguard in the Fray that is **still alive**? Only attacking locks; recomputed at the Fray
    /// boundary. A Vanguard that is not locked is **free** and may charge/flank in the Volley.
    pub hero_locked: Vec<bool>,
    pub foe_locked: Vec<bool>,
    /// §10 **Pin** (Artillery space-control): per unit, has it been **pinned** by suppressive fire this
    /// round? A pinned Vanguard is denied its charge — `fix_breach_list` ORs this into the computed lock
    /// (so a Fray-cast Pin survives the boundary recompute) and a charge declaration skips it. Cleared
    /// each round with the rest of the plan.
    pub hero_pinned: Vec<bool>,
    pub foe_pinned: Vec<bool>,
    /// §4.6 **attacked-map**: per actor, the **enemy-Vanguard indices it struck in the Fray** — the
    /// exact input to [`crate::combat::compute_locks`]. Recorded as each Fray melee strike resolves
    /// (interactive *and* foe-AI), so a Vanguard whose struck foe died is **free** even while other
    /// enemy Vanguards stand (the per-unit lock, not an all-or-nothing approximation). Cleared each
    /// round with the rest of the plan.
    pub hero_attacked: Vec<Vec<usize>>,
    pub foe_attacked: Vec<Vec<usize>>,
    /// Actors who have already acted in the current interactive phase (Standoff / Fray / Volley).
    pub hero_acted: Vec<bool>,
    pub foe_acted: Vec<bool>,
    /// §4.6 #3 — charges/flanks declared in the Volley, resolved at the Volley/Breach boundary.
    pub charges: Vec<Charge>,
    /// §4.6 #5 — deferred (`resolve: Reckoning`) spells wound up this round.
    pub deferred: Vec<Deferred>,
    /// PvP: which side is currently committing this phase (0 = heroes, 1 = creatures). Always 0 in PvE.
    pub committing: u8,
    /// True once the deterministic-base trade is replaced by the interactive Clash module.
    pub clash_mode: bool,
}

impl Round {
    pub fn sized(heroes: usize, foes: usize) -> Self {
        Round {
            // Default position: a melee unit fronts, a ranged/support unit holds back. `game` overrides
            // this per-actor from the attack profile when a round opens.
            hero_vanguard: vec![true; heroes],
            foe_vanguard: vec![true; foes],
            hero_locked: vec![false; heroes],
            foe_locked: vec![false; foes],
            hero_pinned: vec![false; heroes],
            foe_pinned: vec![false; foes],
            hero_attacked: vec![Vec::new(); heroes],
            foe_attacked: vec![Vec::new(); foes],
            hero_acted: vec![false; heroes],
            foe_acted: vec![false; foes],
            charges: Vec::new(),
            deferred: Vec::new(),
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
    /// Per-unit Vanguard flag of `side` (§4.6 position; `true` = front).
    pub fn s_vanguard(&self, side: u8) -> &[bool] {
        if side == 0 {
            &self.plan.hero_vanguard
        } else {
            &self.plan.foe_vanguard
        }
    }
    pub fn s_vanguard_mut(&mut self, side: u8) -> &mut Vec<bool> {
        if side == 0 {
            &mut self.plan.hero_vanguard
        } else {
            &mut self.plan.foe_vanguard
        }
    }
    /// Per-unit lock flag of `side` (§4.6 breach list; `true` = locked, may not charge).
    pub fn s_locked(&self, side: u8) -> &[bool] {
        if side == 0 {
            &self.plan.hero_locked
        } else {
            &self.plan.foe_locked
        }
    }
    pub fn s_locked_mut(&mut self, side: u8) -> &mut Vec<bool> {
        if side == 0 {
            &mut self.plan.hero_locked
        } else {
            &mut self.plan.foe_locked
        }
    }
    /// Per-unit §10 Pin flag of `side` (`true` = pinned this round, charge denied).
    pub fn s_pinned(&self, side: u8) -> &[bool] {
        if side == 0 {
            &self.plan.hero_pinned
        } else {
            &self.plan.foe_pinned
        }
    }
    pub fn s_pinned_mut(&mut self, side: u8) -> &mut Vec<bool> {
        if side == 0 {
            &mut self.plan.hero_pinned
        } else {
            &mut self.plan.foe_pinned
        }
    }
    /// Per-actor §4.6 attacked-map of `side` (the enemy-Vanguard indices each struck in the Fray).
    pub fn s_attacked_mut(&mut self, side: u8) -> &mut Vec<Vec<usize>> {
        if side == 0 {
            &mut self.plan.hero_attacked
        } else {
            &mut self.plan.foe_attacked
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
