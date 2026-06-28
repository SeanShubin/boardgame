//! Deckbound as an [`engine::Game`] — the §4.6 six-phase battle.
//!
//! A scenario is either **base mode** (deterministic: same-range = trade, mismatch = auto-hit,
//! §4.2) run through the six-phase round (Standoff → Fray → Volley → Breach → Reckoning → Lull,
//! §4.6), or a **Clash-module** 1v1 duel (the optional four-card mix-up, [`crate::duel`]). All
//! numbers live in `data/booklet.ron`.

use engine::{
    Accent, CardView, Game, GameError, Layout, Outcome, PlayerId, Rng, TableView, ZoneView,
};

use crate::actor::{Actor, Range};
use crate::campaign::{Campaign, reference_campaign};
use crate::combat;
use crate::duel::{self, Move, Side};
use crate::rules::Rule;
use crate::ruleset::Ruleset;
use crate::scenarios::{self, Scenario};
use crate::state::{Clash, Menu, Phase, Round, State};

/// Break off a Clash after this many no-connect beats (termination backstop, §1.6).
const STALL_CAP: u32 = 12;

/// One step a player can take.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Action {
    OpenCooperation,
    OpenGod,
    OpenTutorial,
    OpenVersus,
    /// Open the world-map reference Campaign (§8).
    OpenCampaign,
    /// Open the rules encyclopedia (its category list).
    OpenEncyclopedia,
    /// Open the card catalog.
    OpenCatalog,
    /// Take the campaign's *n*-th legal action (an index into [`Campaign::legal_actions`], so the
    /// whole campaign rides through this enum while it stays `Copy`).
    CampaignMove(usize),
    /// Open one rules category (by index in `categories()`).
    OpenCategory(usize),
    /// Open one card's detail page (by index in `card_catalog()`).
    OpenCard(usize),
    PickScenario(usize),
    Exit,
    ToMenu,
    Back,
    Replay,
    /// Standoff (§4.6 #1): set this unit's **position** — front (Vanguard) or back (Rearguard).
    SetVanguard(usize),
    SetRearguard(usize),
    /// Standoff (§4): toggle this hero unit's melee answer for the round — **Block** (out-bid to slip the
    /// blow) vs the default **Trade** (strike back). The slipper sets Block to survive the front untouched.
    Guard(usize),
    /// Volley (§4.6 #3): declare a **flank** — this freed Vanguard attacks a surviving enemy
    /// Vanguard (a trade in the Volley). `Flank(actor)` flanks the first enemy Vanguard.
    Flank(usize),
    /// Advance the current phase (Standoff → Fray → Volley → Breach → Reckoning → next round).
    Deploy,
    /// Fray (§4.6 #2) strike / Volley (§4.6 #3) **flank**: this actor attacks that enemy.
    Target(usize, usize),
    /// Volley (§4.6 #3): this freed Vanguard **charges** that enemy Rearguard (lands in the Breach).
    Charge(usize, usize),
    PlayCard(usize, usize),
    Pass(usize),
    /// Clash module (1v1): play one move.
    Play(Move),
}

/// The ruleset. Holds no state of its own.
#[derive(Clone, Copy, Debug, Default)]
pub struct Deckbound;

fn menu_state(seed: u64) -> State {
    State {
        round: 0,
        heroes: Vec::new(),
        creatures: Vec::new(),
        phase: Phase::Menu(Menu::Top),
        plan: Round::default(),
        clash: None,
        scenario: None,
        exiting: false,
        log: vec!["Deckbound — choose a scenario set.".into()],
        rng: Rng::new(seed),
        seed,
        outcome: None,
        clash_module: false,
        pvp: false,
        ruleset: Ruleset::default(),
        campaign: None,
    }
}

/// The campaign ruleset, delegated to while [`State::campaign`] is `Some`. A unit struct, so this
/// is just a namespace for its `Game` methods.
const CAMPAIGN: Campaign = Campaign;

/// A stable session key for a combat scenario, derived from its name (FNV-1a). The high bit is set
/// so it can never collide with the reserved menu (0) / campaign (1) keys.
fn scenario_key(name: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in name.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h | (1 << 63)
}

fn list_for(menu: Menu) -> Vec<Scenario> {
    match menu {
        Menu::Cooperation => scenarios::campaign(),
        Menu::God => scenarios::god(),
        Menu::Tutorial => scenarios::tutorials(),
        Menu::Versus => scenarios::versus(),
        Menu::Top | Menu::Rules | Menu::Category(_) | Menu::Catalog | Menu::CardDetail(_) => {
            Vec::new()
        }
    }
}

/// The encyclopedia's categories, in first-seen order (the top of the hierarchy).
fn categories() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for e in scenarios::glossary() {
        if !out.contains(&e.category) {
            out.push(e.category);
        }
    }
    out
}

/// The rules entries within one category.
fn entries_in(category: &str) -> Vec<engine::RefEntry> {
    scenarios::glossary()
        .into_iter()
        .filter(|e| e.category == category)
        .collect()
}

fn load_scenario(state: &mut State, scenario: Scenario) {
    let (heroes, creatures) = scenario.roster();
    let nh = heroes.len();
    let nf = creatures.len();
    state.heroes = heroes;
    state.creatures = creatures;
    state.round = 1;
    state.outcome = None;
    state.clash = None;
    state.clash_module = scenario.clash;
    state.pvp = scenario.pvp;
    state.plan = Round::sized(nh, nf);
    if scenario.clash {
        // 1v1 Clash-module duel: skip the lane machinery, run the four-card mix-up.
        state.plan.clash_mode = true;
        state.clash = Some(Clash {
            hero: 0,
            foe: 0,
            hero_force: 0,
            foe_force: 0,
            beat: 0,
            stall: 0,
        });
        state.phase = Phase::Clash;
        state.log = vec![scenario.blurb.clone(), "-- the duel begins --".into()];
    } else {
        state.phase = Phase::Standoff;
        default_positions(state);
        state.log = vec![scenario.blurb.clone(), "-- Round 1: the Standoff --".into()];
    }
    state.scenario = Some(scenario);
}

/// §4.6 #1 — seed each unit's default **position** from its attack profile: a melee unit fronts
/// (Vanguard), a ranged / support unit holds back (Rearguard). The Standoff lets the human (or the AI)
/// override this per unit before the Fray.
fn default_positions(state: &mut State) {
    for side in 0u8..2 {
        let n = state.s_len(side);
        for i in 0..n {
            let melee = state.s_pool(side)[i].can_contest(Range::Melee);
            state.s_vanguard_mut(side)[i] = melee;
        }
    }
}

/// Build a battle [`State`] directly from explicit rosters — for **headless auto-resolution** (the
/// par-solver, §8). `clash` selects the optional RPS module; **off → deterministic** (§4.2), so a
/// greedy hero policy can play the battle to an `Outcome`. No `Scenario` is attached. Uses the live
/// [`Ruleset::default`]; analysis callers want [`battle_state_with`].
pub fn battle_state(heroes: Vec<Actor>, creatures: Vec<Actor>, clash: bool, seed: u64) -> State {
    battle_state_with(heroes, creatures, clash, seed, Ruleset::default())
}

/// As [`battle_state`], but with an explicit [`Ruleset`] (the pre-game round/roster bounds). Analysis
/// tooling passes [`Ruleset::analysis`] so the combat tree is finite and exactly searchable (§0).
pub fn battle_state_with(
    heroes: Vec<Actor>,
    creatures: Vec<Actor>,
    clash: bool,
    seed: u64,
    ruleset: Ruleset,
) -> State {
    let nh = heroes.len();
    let nf = creatures.len();
    let mut state = menu_state(seed);
    state.ruleset = ruleset;
    state.heroes = heroes;
    state.creatures = creatures;
    state.round = 1;
    state.clash_module = clash;
    state.plan = Round::sized(nh, nf);
    if clash {
        state.plan.clash_mode = true;
        state.clash = Some(Clash {
            hero: 0,
            foe: 0,
            hero_force: 0,
            foe_force: 0,
            beat: 0,
            stall: 0,
        });
        state.phase = Phase::Clash;
        // Replace the menu log seeded by `menu_state` — else the battle opens showing the stale
        // "choose a scenario set" line until combat events push it off.
        state.log = vec!["-- the duel begins --".into()];
    } else {
        state.phase = Phase::Standoff;
        default_positions(&mut state);
        state.log = vec!["-- Round 1: the Standoff --".into()];
    }
    state
}

pub(crate) fn check_outcome(state: &mut State) {
    if state.living_creatures() == 0 {
        state.outcome = Some(Outcome::Win(PlayerId(0)));
        state.log.push("Every foe is down — victory!".into());
    } else if state.living_heroes() == 0 {
        state.outcome = Some(Outcome::Win(PlayerId(1)));
        state.log.push("The party has fallen.".into());
    }
}

/// A single-purpose round-resolution step (§4 exploded-phase model): it does exactly one thing to the
/// state — resolve a strike-set, finalize deaths, or wipe an accumulator.
type ResolutionStep = fn(&Deckbound, &mut State);

/// The **§4 post-Volley resolution as one editable list** (the exploded-phase model): pre-empt → wipe,
/// then Breach → wipe, then Reckoning → wipe. Each resolve step finalizes its own deaths (tally); every
/// `clear_piles` is an **isolated accumulator-wipe that does *only* that**, so reordering/removing one
/// directly tunes how much focus-fire accumulates across a phase boundary (the Toughness-as-wall dial).
/// The runner ([`Deckbound::end_volley`]) checks the outcome after every step and stops the instant the
/// battle ends. To change the model, edit this list.
const POST_VOLLEY_SCHEDULE: &[(Rule, ResolutionStep)] = &[
    (Rule::Interception, Deckbound::step_intercept),
    (Rule::Preempt, Deckbound::step_preempt),
    (Rule::WipePile, Deckbound::step_clear_piles),
    (Rule::Breach, Deckbound::step_breach),
    (Rule::WipePile, Deckbound::step_clear_piles),
    (Rule::Reckoning, Deckbound::step_reckoning),
    (Rule::WipePile, Deckbound::step_clear_piles),
];

impl Deckbound {
    // ---- §4.6 six-phase round -------------------------------------------------

    /// A unit of `side` that may still act in the current interactive phase: living, not staggered,
    /// and not yet flagged `acted`. In the **Fray** only Vanguards (front) and ranged Rearguards act;
    /// in the **Volley** only **free** (un-locked) Vanguards may charge/flank and ranged Rearguards
    /// may fire again.
    fn pending(&self, state: &State, side: u8) -> Vec<usize> {
        (0..state.s_len(side))
            .filter(|&i| {
                let a = &state.s_pool(side)[i];
                if a.fallen || a.is_down() || a.stunned || state.s_acted(side)[i] {
                    return false;
                }
                match state.phase {
                    Phase::Fray => self.can_act_in_fray(state, side, i),
                    Phase::Volley => self.can_act_in_volley(state, side, i),
                    _ => false,
                }
            })
            .collect()
    }

    /// In the Fray a unit acts if it is a Vanguard with a melee answer (it strikes the front) or a
    /// Rearguard that can fire instant ranged at the front.
    fn can_act_in_fray(&self, state: &State, side: u8, i: usize) -> bool {
        let a = &state.s_pool(side)[i];
        if state.s_vanguard(side)[i] {
            a.can_contest_now(Range::Melee) && a.tempo > 0
        } else {
            a.can_contest(Range::Ranged) && a.tempo > 0
        }
    }

    /// In the Volley a **free** Vanguard may charge/flank, and a Rearguard with ranged fire may shoot
    /// again (instant). A locked Vanguard stays pinned (no action).
    fn can_act_in_volley(&self, state: &State, side: u8, i: usize) -> bool {
        let a = &state.s_pool(side)[i];
        if state.s_vanguard(side)[i] {
            // §10 Rout: a routed Vanguard is driven off the line — it cannot charge/flank (neither holds
            // the front nor crosses). A pinned/locked Vanguard likewise stays put.
            !state.s_locked(side)[i] && !a.routed && a.can_contest_now(Range::Melee) && a.tempo > 0
        } else {
            a.can_contest(Range::Ranged) && a.tempo > 0
        }
    }

    /// Advance the round one phase (§4.6 fixed order). Called by `Deploy`, or automatically when a
    /// phase has no remaining interactive choices.
    fn advance_phase(&self, state: &mut State) {
        match state.phase {
            Phase::Standoff => self.begin_fray(state),
            Phase::Fray => self.end_fray(state),
            Phase::Volley => self.end_volley(state),
            _ => {}
        }
    }

    /// Standoff → Fray. (Positions are already declared; in PvE the creature AI keeps its profile
    /// defaults. Nothing else to set up — the Fray is interactive.)
    fn begin_fray(&self, state: &mut State) {
        state.plan.committing = 0;
        state.plan.hero_acted.iter_mut().for_each(|v| *v = false);
        state.plan.foe_acted.iter_mut().for_each(|v| *v = false);
        state.phase = Phase::Fray;
        state.log.push("-- the Fray --".into());
        // Run the creature side's Fray first (PvE), so the human front sees the incoming blows.
        if !state.pvp {
            self.foe_fray(state);
            // §4 / solver dedup: the hero **guard** is spent the instant the foe's Fray melee has
            // resolved against it (nothing downstream reads it this round). Reset it to the default so a
            // now-irrelevant stance no longer distinguishes states in the transposition table — two
            // Standoff guard-choices with the same Fray outcome converge, collapsing the Block branching.
            state
                .plan
                .hero_guard
                .iter_mut()
                .for_each(|g| *g = combat::Guard::Trade);
        }
        if self.pending(state, 0).is_empty() {
            self.advance_phase(state);
        }
    }

    /// End the Fray: finalize deaths, **fix the breach list** (§4.6: only attacking an enemy Vanguard
    /// that is still alive locks you), wipe the per-phase piles, then open the Volley.
    fn end_fray(&self, state: &mut State) {
        combat::tally(&mut state.heroes, &mut state.log); // finalize Fray deaths
        combat::tally(&mut state.creatures, &mut state.log);
        check_outcome(state);
        if state.outcome.is_some() {
            return;
        }
        // The breach list: a Vanguard is locked iff some enemy Vanguard it *attacked* in the Fray is
        // still alive. We recompute it from the current board — a Vanguard that struck no living enemy
        // Vanguard (its target dead, or it never engaged) is free.
        self.fix_breach_list(state);
        // The **Fray** accumulator-wipe (§4): the per-phase pile clears at the Fray boundary. This is the
        // single point that bounds **focus-fire accumulation** within the front clash — the Toughness-as-
        // wall / Sunder-necessity dial. Same named step as the post-Volley wipes (toggle/move to tune).
        self.step_clear_piles(state);
        state.plan.committing = 0;
        state.plan.hero_acted.iter_mut().for_each(|v| *v = false);
        state.plan.foe_acted.iter_mut().for_each(|v| *v = false);
        state.plan.charges.clear();
        state.phase = Phase::Volley;
        state.log.push("-- the Volley --".into());
        if !state.pvp {
            self.foe_volley(state);
        }
        if self.pending(state, 0).is_empty() {
            self.advance_phase(state);
        }
    }

    /// §4.6 **breach list / per-unit lock** — the exact primitive (replaces the old all-or-nothing
    /// approximation). A Vanguard is locked iff **some enemy Vanguard it struck in the Fray is still
    /// alive** (only *attacking* locks, §4.6). We feed [`combat::compute_locks`] the per-actor
    /// attacked-map recorded as each Fray strike resolved (`fray_strike`) against the enemy pool with
    /// its Fray deaths already finalized — so a Vanguard whose struck foe **died** is **free** even
    /// while other enemy Vanguards stand (a line breaks per-unit, not all-or-nothing).
    fn fix_breach_list(&self, state: &mut State) {
        state.plan.hero_locked = combat::compute_locks(&state.plan.hero_attacked, &state.creatures);
        state.plan.foe_locked = combat::compute_locks(&state.plan.foe_attacked, &state.heroes);
        // §10 **Pin** (Artillery): a Vanguard pinned by suppressive fire is locked regardless of the Fray
        // breach list — OR it in here so a Fray-cast Pin survives this boundary recompute (it denies the
        // pinned unit its Volley charge).
        for (l, &p) in state
            .plan
            .hero_locked
            .iter_mut()
            .zip(state.plan.hero_pinned.iter())
        {
            *l |= p;
        }
        for (l, &p) in state
            .plan
            .foe_locked
            .iter_mut()
            .zip(state.plan.foe_pinned.iter())
        {
            *l |= p;
        }
    }

    /// End the Volley → Breach → Reckoning, then the Lull (next round). Runs [`POST_VOLLEY_SCHEDULE`] —
    /// the §4 resolution as an ordered list of single-purpose steps — checking the outcome after each and
    /// stopping the instant the battle ends. The behavior is identical to the old hardcoded flow; the
    /// difference is that the sequence (and the accumulator-wipes) is now **data we can edit**.
    fn end_volley(&self, state: &mut State) {
        for (rule, step) in POST_VOLLEY_SCHEDULE {
            if !state.ruleset.allows(*rule) {
                continue; // this phase is toggled off for this game
            }
            step(self, state);
            check_outcome(state);
            if state.outcome.is_some() {
                return;
            }
        }
        self.next_round(state);
    }

    // ---- §4 round-resolution steps (single-purpose; sequenced by POST_VOLLEY_SCHEDULE) ----

    /// **Interception** (§4): each declared charge is struck by the *enemy front* Vanguards as it crosses
    /// — the front strikes the runner. The charger slips each via the Finesse contest (spending Tempo) or
    /// is cut down at the line; a charger downed here never reaches the back (its Breach blow fizzles). So
    /// crossing a guarded front is a specialist play — only a lone high-Finesse/high-Tempo body survives.
    fn step_intercept(&self, state: &mut State) {
        let charges = state.plan.charges.clone();
        for c in charges.iter().filter(|c| !c.flank) {
            if c.side == 0 {
                // A hero charges the enemy Rearguard — the creature front intercepts.
                if c.attacker >= state.heroes.len() {
                    continue;
                }
                let mut creatures = std::mem::take(&mut state.creatures);
                combat::intercept(
                    &mut state.heroes[c.attacker],
                    &mut creatures,
                    &state.plan.foe_vanguard,
                    &mut state.log,
                );
                state.creatures = creatures;
            } else {
                // A foe charges the hero Rearguard — the hero front intercepts.
                if c.attacker >= state.creatures.len() {
                    continue;
                }
                let mut heroes = std::mem::take(&mut state.heroes);
                combat::intercept(
                    &mut state.creatures[c.attacker],
                    &mut heroes,
                    &state.plan.hero_vanguard,
                    &mut state.log,
                );
                state.heroes = heroes;
            }
        }
        combat::tally(&mut state.heroes, &mut state.log);
        combat::tally(&mut state.creatures, &mut state.log);
    }

    /// **Pre-empt:** the rear answers the declared charges first (counter-fire / strike-back / dodge),
    /// then finalize deaths.
    fn step_preempt(&self, state: &mut State) {
        let charges = state.plan.charges.clone();
        let mut log = std::mem::take(&mut state.log);
        combat::resolve_volley(&mut state.heroes, &mut state.creatures, &charges, &mut log);
        combat::tally(&mut state.heroes, &mut log);
        combat::tally(&mut state.creatures, &mut log);
        state.log = log;
    }

    /// **Breach:** surviving chargers land their blows on the exposed Rearguard (§4.6 #4), then tally.
    fn step_breach(&self, state: &mut State) {
        state.phase = Phase::Breach;
        state.log.push("-- the Breach --".into());
        let charges = state.plan.charges.clone();
        let mut log = std::mem::take(&mut state.log);
        combat::resolve_breach(&mut state.heroes, &mut state.creatures, &charges, &mut log);
        combat::tally(&mut state.heroes, &mut log);
        combat::tally(&mut state.creatures, &mut log);
        state.log = log;
    }

    /// **Reckoning:** deferred spells resolve, fizzling if their caster died in the Breach (§4.6 #5),
    /// then tally.
    fn step_reckoning(&self, state: &mut State) {
        state.phase = Phase::Reckoning;
        state.log.push("-- the Reckoning --".into());
        let deferred = state.plan.deferred.clone();
        let mut log = std::mem::take(&mut state.log);
        combat::resolve_reckoning(&mut state.heroes, &mut state.creatures, &deferred, &mut log);
        combat::tally(&mut state.heroes, &mut log);
        combat::tally(&mut state.creatures, &mut log);
        state.log = log;
    }

    /// **Accumulator-wipe** (does ONLY this, §4): clear the per-phase pile on both sides so
    /// sub-threshold damage never crosses a phase boundary. Its placement is the focus-fire /
    /// Toughness-as-wall dial — moving or removing it changes how much damage accumulates before a wall.
    fn step_clear_piles(&self, state: &mut State) {
        combat::clear_phase_piles(&mut state.heroes);
        combat::clear_phase_piles(&mut state.creatures);
    }

    /// PvE creature **Fray**: each living foe Vanguard strikes the first living hero Vanguard (a melee
    /// trade), and each ranged foe Rearguard fires at the hero front. Deterministic (§0.1 fixed
    /// instinct): a foe acts while it has Tempo and a legal target.
    /// The living creature with the most missing Health (a healer's mend target); `None` if all are full.
    fn most_wounded_creature(&self, state: &State) -> Option<usize> {
        (0..state.creatures.len())
            .filter(|&i| !state.creatures[i].fallen && !state.creatures[i].is_down())
            .filter(|&i| {
                state.creatures[i].defense.health.remaining < state.creatures[i].defense.health.max
            })
            .max_by_key(|&i| {
                state.creatures[i].defense.health.max - state.creatures[i].defense.health.remaining
            })
    }

    fn foe_fray(&self, state: &mut State) {
        for f in 0..state.creatures.len() {
            if state.creatures[f].fallen
                || state.creatures[f].is_down()
                || state.creatures[f].stunned
            {
                continue;
            }
            // §13 healer: a support creature mends its most-wounded ally instead of attacking — undoing
            // the party's attrition, so the front never falls to damage alone and the healer must be
            // *reached and killed*.
            let heal = state.creatures[f].behavior().map_or(0, |b| b.heal);
            if heal > 0 && state.creatures[f].tempo > 0 {
                if let Some(t) = self.most_wounded_creature(state) {
                    state.creatures[f].tempo -= 1;
                    let restored: u32 = (0..heal)
                        .map(|_| state.creatures[t].defense.recover_card())
                        .sum();
                    if restored > 0 {
                        let hn = state.creatures[f].name.clone();
                        let tn = state.creatures[t].name.clone();
                        state
                            .log
                            .push(format!("  {hn} mends {tn} (+{restored} Health)."));
                    }
                }
                continue; // a healer does not attack
            }
            if state.plan.foe_vanguard[f] {
                // Front clash: strike the first living hero Vanguard. One committed strike per foe
                // Vanguard in the Fray (keeps the trade clean).
                if state.creatures[f].tempo > 0
                    && state.creatures[f].can_contest_now(Range::Melee)
                    && let Some(t) = (0..state.heroes.len())
                        .find(|&h| state.plan.hero_vanguard[h] && !state.heroes[h].is_down())
                {
                    self.fray_strike(state, false, f, t);
                }
            } else if state.creatures[f].can_contest(Range::Ranged) && state.creatures[f].tempo > 0
            {
                // One instant shot at the hero front this Fray.
                if let Some(t) = self.front_target(state, 0) {
                    self.fray_shot(state, false, f, t);
                }
            }
        }
        combat::tally(&mut state.heroes, &mut state.log);
    }

    /// PvE creature **Volley**: a free foe Vanguard charges the hero Rearguard (or flanks a surviving
    /// hero Vanguard); a ranged foe Rearguard fires again at the hero front. Deterministic.
    fn foe_volley(&self, state: &mut State) {
        for f in 0..state.creatures.len() {
            if state.creatures[f].fallen
                || state.creatures[f].is_down()
                || state.creatures[f].stunned
            {
                continue;
            }
            if state.plan.foe_vanguard[f]
                && !state.plan.foe_locked[f]
                && !state.creatures[f].routed
                && state.creatures[f].can_contest_now(Range::Melee)
                && state.creatures[f].tempo > 0
            {
                // §10 Rout refinement: a routed foe is driven off the line and **cannot charge** as a
                // Vanguard (it neither holds the front nor crosses). Charge a hero Rearguard if one
                // exists; else flank a surviving hero Vanguard.
                if let Some(t) = self.rear_target(state, 0) {
                    state.plan.charges.push(crate::state::Charge {
                        side: 1,
                        attacker: f,
                        target: t,
                        flank: false,
                    });
                } else if let Some(t) = (0..state.heroes.len())
                    .find(|&h| state.plan.hero_vanguard[h] && !state.heroes[h].is_down())
                {
                    state.plan.charges.push(crate::state::Charge {
                        side: 1,
                        attacker: f,
                        target: t,
                        flank: true,
                    });
                }
            } else if !state.plan.foe_vanguard[f]
                && state.creatures[f].can_contest(Range::Ranged)
                && state.creatures[f].tempo > 0
                && let Some(t) = self.front_target(state, 0)
            {
                self.fray_shot(state, false, f, t);
            }
        }
    }

    /// The first living enemy **front** (Vanguard) of `enemy_side`, for a ranged shot.
    fn front_target(&self, state: &State, enemy_side: u8) -> Option<usize> {
        (0..state.s_len(enemy_side))
            .find(|&i| state.s_vanguard(enemy_side)[i] && !state.s_pool(enemy_side)[i].is_down())
    }

    /// The first living enemy **Rearguard** of `enemy_side` (a charge target). `None` if the enemy
    /// back is empty — then a charger must flank the front instead.
    fn rear_target(&self, state: &State, enemy_side: u8) -> Option<usize> {
        (0..state.s_len(enemy_side))
            .find(|&i| !state.s_vanguard(enemy_side)[i] && !state.s_pool(enemy_side)[i].is_down())
    }

    /// A **Fray melee strike** (§4.6 #2): a trade — `attacker` strikes `target`, who strikes back if
    /// it can. **Records the struck enemy Vanguard** in the attacked-map (the input to
    /// [`combat::compute_locks`]) so the per-unit lock is exact: a Vanguard whose struck foe dies is
    /// freed even while other enemy Vanguards stand.
    fn fray_strike(&self, state: &mut State, hero_attacker: bool, attacker: usize, target: usize) {
        // Only a Vanguard's front strike contributes a lock (attacking is what pins, §4.6).
        let side: u8 = if hero_attacker { 0 } else { 1 };
        if state.s_vanguard(side)[attacker] {
            let map = state.s_attacked_mut(side);
            if !map[attacker].contains(&target) {
                map[attacker].push(target);
            }
        }
        if hero_attacker {
            // The defender is a creature — it answers with its fixed instinct (Trade, §0.1).
            let mut heroes = std::mem::take(&mut state.heroes);
            combat::fray_one(
                &mut heroes[attacker],
                &mut state.creatures[target],
                combat::Guard::Trade,
                &mut state.log,
            );
            state.heroes = heroes;
        } else {
            // The defender is a hero — it answers per its declared §4 stance (Trade / Block).
            let guard = state.plan.hero_guard[target];
            let mut creatures = std::mem::take(&mut state.creatures);
            combat::fray_one(
                &mut creatures[attacker],
                &mut state.heroes[target],
                guard,
                &mut state.log,
            );
            state.creatures = creatures;
        }
    }

    /// A **Fray / Volley instant ranged shot** (§4.6: `cast: Strike, resolve: OnCast`): an evade
    /// contest, then the shot lands if not dodged.
    fn fray_shot(&self, state: &mut State, hero_attacker: bool, attacker: usize, target: usize) {
        if hero_attacker {
            let mut heroes = std::mem::take(&mut state.heroes);
            combat::ranged_one(
                &mut heroes[attacker],
                &mut state.creatures[target],
                &mut state.log,
            );
            state.heroes = heroes;
        } else {
            let mut creatures = std::mem::take(&mut state.creatures);
            combat::ranged_one(
                &mut creatures[attacker],
                &mut state.heroes[target],
                &mut state.log,
            );
            state.creatures = creatures;
        }
    }

    fn next_round(&self, state: &mut State) {
        // Round cap (§0 Ruleset): a fight not closed within `max_rounds` is a **draw** (PvE: no
        // different from a loss). The Lull (§4.6 #6): Tempo resets, Health persists, round++.
        if state.round >= state.ruleset.max_rounds {
            state.outcome = Some(Outcome::Tie(vec![PlayerId(0), PlayerId(1)]));
            state
                .log
                .push("The battle reaches the round cap — a draw.".into());
            return;
        }
        for a in state.heroes.iter_mut().chain(state.creatures.iter_mut()) {
            if !a.is_down() {
                a.refresh_round();
            }
        }
        state.round += 1;
        state.plan = Round::sized(state.heroes.len(), state.creatures.len());
        default_positions(state);
        state.phase = Phase::Standoff;
        state
            .log
            .push(format!("-- Round {}: the Standoff --", state.round));
    }

    /// §4.4 — may actor `i` of `side` play this `card` right now? There is **no per-suit/per-side cap**
    /// (casting is bounded only by Tempo + evade). It enforces Disarm, the §4.6 cast window, and the
    /// **target-classification position rule**: an **offensive** (foe-targeting) card is positioned by
    /// reach (§4.2) — a **ranged** one needs the **Rearguard**, a **melee** one the **Vanguard**;
    /// **support** (ally/self) cards are rank-free. A `cast: Standing` card is only legal in the Standoff;
    /// a `cast: Strike` card in the Fray/Volley.
    fn card_playable_now(
        &self,
        state: &State,
        side: u8,
        i: usize,
        card: &crate::cards::Card,
    ) -> bool {
        use crate::cards::Cast;
        if card.passive {
            return false;
        }
        if state.s_pool(side)[i].disarmed {
            return false;
        }
        // §4.4 — casting spends a Tempo card; with no per-suit/per-side cap, **Tempo is the limiter**.
        // A unit with no Tempo cannot cast (consistent with strikes, which also need `tempo > 0`).
        if state.s_pool(side)[i].tempo <= 0 {
            return false;
        }
        // §4.6 cast window.
        let window_ok = matches!(
            (state.phase, card.cast),
            (Phase::Standoff, Cast::Standing)
                | (Phase::Fray, Cast::Strike)
                | (Phase::Volley, Cast::Strike)
        );
        if !window_ok {
            return false;
        }
        // §4.4 — offensive casting is positioned by reach (§4.2). A foe-targeting card has its casting
        // position fixed by whether it is ranged: an offensive *ranged* spell fires only from the
        // Rearguard (so a Vanguard cannot rain ranged spells); an offensive *melee* ability is cast from
        // the front. Support (ally/self) is rank-free (the "a Vanguard can't rain spells" gate falls out
        // of §4.2 — it is not a separate mechanism).
        if card.is_offensive() {
            let in_vanguard = state.s_vanguard(side)[i];
            let ranged = card.is_ranged();
            if ranged && in_vanguard {
                return false; // an offensive ranged spell needs the Rearguard
            }
            if !ranged && !in_vanguard {
                return false; // an offensive melee ability needs the Vanguard
            }
        }
        true
    }

    /// Play `card` from actor `i` of `side`. A `resolve: Reckoning` card is **wound up** (deferred to
    /// the Reckoning, disruptable); everything else resolves immediately (`resolve: OnCast`).
    fn do_play_card(&self, state: &mut State, side: u8, i: usize, card: crate::cards::Card) {
        let off = state.s_pool(side)[i].offense;
        let name = state.s_pool(side)[i].name.clone();
        // Casting spends a Tempo card (§4.4) — pay-after.
        if side == 0 {
            state.heroes[i].tempo -= 1;
        } else {
            state.creatures[i].tempo -= 1;
        }
        if card.resolve == crate::cards::Resolve::Reckoning {
            state.plan.deferred.push(crate::state::Deferred {
                side,
                caster: i,
                card,
                offense: off,
                name: name.clone(),
            });
            state.log.push(format!(
                "{name} winds up a held effect (resolves at the Reckoning)."
            ));
            return;
        }
        // §10 Silence (Controller): cancel one *enemy* deferred (`resolve: Reckoning`) spell — a
        // non-lethal disrupt (§4.6). Resolved here (the deferred list lives in the round plan); the
        // `play_card` effect arm only narrates. The earliest wound-up enemy spell is removed.
        if card
            .effects
            .iter()
            .any(|e| matches!(e, crate::cards::Effect::Silence))
        {
            let enemy: u8 = 1 - side;
            if let Some(pos) = state.plan.deferred.iter().position(|d| d.side == enemy) {
                let d = state.plan.deferred.remove(pos);
                state.log.push(format!(
                    "{name} silences {}'s held {}.",
                    d.name, d.card.name
                ));
            }
        }
        // §10 Pin (Artillery): suppressive fire denies a free enemy Vanguard its charge. Resolved here
        // (the lock/charge lists live in the round plan); the `play_card` effect arm only narrates. We
        // pin up to `targets` free enemy Vanguards — set the pinned flag (so `fix_breach_list` keeps the
        // lock across the Fray boundary), lock it now, and drop any charge it has already declared.
        if card
            .effects
            .iter()
            .any(|e| matches!(e, crate::cards::Effect::Pin))
        {
            let enemy: u8 = 1 - side;
            let want = (card.targets as usize).max(1);
            let victims: Vec<usize> = (0..state.s_len(enemy))
                .filter(|&t| {
                    state.s_vanguard(enemy)[t]
                        && !state.s_pool(enemy)[t].is_down()
                        && !state.s_pinned(enemy)[t]
                })
                .take(want)
                .collect();
            for t in victims {
                state.s_pinned_mut(enemy)[t] = true;
                state.s_locked_mut(enemy)[t] = true;
                let vname = state.s_pool(enemy)[t].name.clone();
                // Drop a charge/flank this pinned unit may have already declared (e.g. a Volley cast).
                state
                    .plan
                    .charges
                    .retain(|c| !(c.side == enemy && c.attacker == t));
                state
                    .log
                    .push(format!("{name} pins {vname} — its charge is denied."));
            }
        }
        if side == 0 {
            let mut allies = std::mem::take(&mut state.heroes);
            combat::play_card(
                &card,
                &name,
                off,
                &mut state.creatures,
                &mut allies,
                Some(i),
                &mut state.log,
            );
            state.heroes = allies;
        } else {
            let mut allies = std::mem::take(&mut state.creatures);
            combat::play_card(
                &card,
                &name,
                off,
                &mut state.heroes,
                &mut allies,
                Some(i),
                &mut state.log,
            );
            state.creatures = allies;
        }
        combat::tally(&mut state.heroes, &mut state.log);
        combat::tally(&mut state.creatures, &mut state.log);
        check_outcome(state);
    }

    /// After an interactive Fray/Volley action: if the fight ended, stop; else if the committing side
    /// has nothing left to do, advance — in PvP hand the phase to side B first, otherwise close the
    /// phase (which resolves the rest and steps to the next phase).
    fn after_combat_action(&self, state: &mut State) {
        if state.outcome.is_some() {
            return;
        }
        if !self.pending(state, state.plan.committing).is_empty() {
            return;
        }
        if state.pvp && state.plan.committing == 0 {
            state.plan.committing = 1;
            return;
        }
        state.plan.committing = 0;
        self.advance_phase(state);
    }

    // ---- Clash module (1v1) -------------------------------------------------

    fn clash_beat(&self, state: &mut State, hero_move: Move) {
        let Some(c) = state.clash else { return };
        let key = state.seed
            ^ (c.beat as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (state.round as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
        let mut drng = Rng::new(key);
        let creature_move = state.creatures[c.foe]
            .behavior()
            .map(|b| b.pick(c.foe_force, &mut drng))
            .unwrap_or(Move::Strike);

        let hp = combat::base_strike(&state.heroes[c.hero]);
        let cp = combat::base_strike(&state.creatures[c.foe]);
        let hn = state.heroes[c.hero].name.clone();
        let cn = state.creatures[c.foe].name.clone();
        let a = Side {
            power: hp,
            force: c.hero_force,
            name: &hn,
        };
        let b = Side {
            power: cp,
            force: c.foe_force,
            name: &cn,
        };
        let r = duel::resolve(&a, hero_move, &b, creature_move);
        state.log.push(r.note);
        if let Some(s) = r.on_a {
            combat::apply_strike(&mut state.heroes[c.hero], s, &cn, &mut state.log);
        }
        if let Some(s) = r.on_b {
            combat::apply_strike(&mut state.creatures[c.foe], s, &hn, &mut state.log);
        }
        let hero_down = state.heroes[c.hero].is_down();
        let foe_down = state.creatures[c.foe].is_down();
        if r.ends || hero_down || foe_down {
            if hero_down {
                state.heroes[c.hero].fallen = true;
            }
            if foe_down {
                state.creatures[c.foe].fallen = true;
            }
            state.clash = None;
            check_outcome(state);
            if state.outcome.is_none() {
                // a duel that merely connected (no down) just restarts a fresh exchange
                state.clash = Some(Clash {
                    hero: c.hero,
                    foe: c.foe,
                    hero_force: 0,
                    foe_force: 0,
                    beat: 0,
                    stall: 0,
                });
            }
        } else {
            let stall = c.stall + 1;
            if stall >= STALL_CAP {
                state.clash = Some(Clash {
                    hero: c.hero,
                    foe: c.foe,
                    hero_force: 0,
                    foe_force: 0,
                    beat: 0,
                    stall: 0,
                });
                state.log.push("(they break and reset)".into());
            } else {
                state.clash = Some(Clash {
                    hero: c.hero,
                    foe: c.foe,
                    hero_force: r.a_force,
                    foe_force: r.b_force,
                    beat: c.beat + 1,
                    stall,
                });
            }
        }
    }

    /// The combat event feed (the play-by-play), oldest first — surfaced as the renderer's side
    /// panel (`TableView::log`), separate from the one-line `status` caption. Empty in menus.
    fn feed(&self, state: &State) -> Vec<String> {
        if matches!(state.phase, Phase::Menu(_)) {
            return Vec::new();
        }
        state.log.iter().rev().take(200).rev().cloned().collect()
    }

    fn status(&self, state: &State) -> String {
        let prompt = match (&state.outcome, &state.phase) {
            (Some(Outcome::Win(PlayerId(0))), _) => "Victory! Replay, or Main menu.".to_string(),
            (Some(_), _) => "Defeat. Replay, or Main menu.".to_string(),
            (None, Phase::Menu(Menu::Top)) => "Deckbound — pick a scenario set.".to_string(),
            (None, Phase::Menu(Menu::Rules)) => "Rules — pick a category. (Esc: back)".to_string(),
            (None, Phase::Menu(Menu::Category(i))) => format!(
                "{} — rules. (Esc: back)",
                categories()
                    .get(*i)
                    .cloned()
                    .unwrap_or_else(|| "Rules".into())
            ),
            (None, Phase::Menu(Menu::Catalog)) => {
                "Card catalog — click a card for its rules. (Esc: back)".to_string()
            }
            (None, Phase::Menu(Menu::CardDetail(i))) => format!(
                "{} — how it works. (Esc: back)",
                scenarios::card_catalog()
                    .get(*i)
                    .map(|e| e.name.clone())
                    .unwrap_or_else(|| "Card".into())
            ),
            (None, Phase::Menu(_)) => "Pick a scenario. (Esc: back)".to_string(),
            (None, Phase::Standoff) => format!(
                "Round {} — the Standoff: set positions & cast standing buffs, then advance. (Esc: menu)",
                state.round
            ),
            (None, Phase::Fray) => "The Fray — the fronts clash. (Esc: menu)".to_string(),
            (None, Phase::Volley) => {
                "The Volley — free Vanguards charge or flank; the rear answers first. (Esc: menu)"
                    .to_string()
            }
            (None, Phase::Breach) => "The Breach.".to_string(),
            (None, Phase::Reckoning) => "The Reckoning.".to_string(),
            (None, Phase::Clash) => match state.clash {
                Some(c) => format!(
                    "Clash: {} vs the {} — Strike/Anticipate/Gather/Evade. (Esc: menu)",
                    state.heroes[c.hero].name, state.creatures[c.foe].name
                ),
                None => "...".to_string(),
            },
        };
        // Hotseat: announce whose turn it is (pass-and-play); never reveal the other side's
        // committed choices — they aren't rendered until resolution. The play-by-play now lives in
        // the event feed (`TableView::log`), so the caption is just this one-line prompt.
        if state.pvp
            && state.outcome.is_none()
            && matches!(state.phase, Phase::Standoff | Phase::Fray | Phase::Volley)
        {
            format!("[Player {}] {prompt}", state.plan.committing + 1)
        } else {
            prompt
        }
    }
}

impl Game for Deckbound {
    type State = State;
    type Action = Action;

    fn new_game(&self, seed: u64, _players: usize) -> State {
        menu_state(seed)
    }

    fn current_player(&self, state: &State) -> Option<PlayerId> {
        if let Some(camp) = &state.campaign {
            return CAMPAIGN.current_player(camp);
        }
        if state.outcome.is_some() {
            return None;
        }
        // In a hotseat PvP round, the committing side is the current player (pass-and-play).
        if state.pvp {
            return Some(PlayerId(state.plan.committing as usize));
        }
        Some(PlayerId(0))
    }

    fn legal_actions(&self, state: &State) -> Vec<Action> {
        // In the campaign, expose its moves as indices into its own action list, plus an escape
        // back to the main menu (always available, including after the run is won/lost).
        if let Some(camp) = &state.campaign {
            let n = CAMPAIGN.legal_actions(camp).len();
            let mut a: Vec<Action> = (0..n).map(Action::CampaignMove).collect();
            a.push(Action::ToMenu);
            return a;
        }
        if state.outcome.is_some() {
            return vec![Action::Replay, Action::ToMenu];
        }
        match &state.phase {
            Phase::Menu(Menu::Top) => vec![
                Action::OpenTutorial,
                Action::OpenCooperation,
                Action::OpenGod,
                Action::OpenVersus,
                Action::OpenCampaign,
                Action::OpenCatalog,
                Action::OpenEncyclopedia,
                Action::Exit,
            ],
            // Rules top level: a category card per category (bound to OpenCategory) + Back.
            Phase::Menu(Menu::Rules) => {
                let mut a: Vec<Action> =
                    (0..categories().len()).map(Action::OpenCategory).collect();
                a.push(Action::Back);
                a
            }
            // A category page is the prose reading pane; only Back (to the category cards).
            Phase::Menu(Menu::Category(_)) => vec![Action::Back],
            // The catalog: one clickable card per catalog entry (bound to OpenCard) + Back.
            Phase::Menu(Menu::Catalog) => {
                let mut a: Vec<Action> = (0..scenarios::card_catalog().len())
                    .map(Action::OpenCard)
                    .collect();
                a.push(Action::Back);
                a
            }
            // A card detail page shows the card + its rules; only Back (to the catalog).
            Phase::Menu(Menu::CardDetail(_)) => vec![Action::Back],
            Phase::Menu(m) => {
                let mut a: Vec<Action> =
                    (0..list_for(*m).len()).map(Action::PickScenario).collect();
                a.push(Action::Back);
                a
            }
            // §4.6 #1 Standoff: set each unit's position (front/back) and cast `Standing` buffs;
            // advance to start the Fray.
            Phase::Standoff => {
                let side = state.plan.committing;
                let mut a = Vec::new();
                // §4 Block is only worth a decision branch when a live enemy melee Vanguard can actually
                // strike a hero this round — otherwise slipping nothing is a wasted toggle (perf prune).
                let melee_threat = side == 0
                    && (0..state.s_len(1)).any(|f| {
                        state.s_vanguard(1)[f]
                            && !state.s_pool(1)[f].is_down()
                            && state.s_pool(1)[f].can_contest(Range::Melee)
                    });
                for i in 0..state.s_len(side) {
                    if state.s_pool(side)[i].fallen {
                        continue;
                    }
                    if state.s_vanguard(side)[i] {
                        a.push(Action::SetRearguard(i));
                        // §4: a hero Vanguard (the position that eats melee) may set its answer to
                        // **Block** (out-bid to slip the blow) instead of the default Trade. One-way
                        // (offered only while still Trade) so the solver explores the *set* of blockers
                        // without value-neutral on/off toggling. Heroes only — creatures use instinct.
                        if melee_threat
                            && state.ruleset.allows(Rule::DeclareGuard)
                            && state.plan.hero_guard[i] == combat::Guard::Trade
                        {
                            a.push(Action::Guard(i));
                        }
                    } else {
                        a.push(Action::SetVanguard(i));
                    }
                    // One Standing cast per unit per Standoff: skip a unit that has already cast.
                    if state.s_acted(side)[i] {
                        continue;
                    }
                    for idx in 0..state.s_pool(side)[i].actions.len() {
                        if self.card_playable_now(
                            state,
                            side,
                            i,
                            &state.s_pool(side)[i].actions[idx],
                        ) {
                            a.push(Action::PlayCard(i, idx));
                        }
                    }
                }
                a.push(Action::Deploy);
                a.push(Action::ToMenu);
                a
            }
            // §4.6 #2 Fray: the next pending unit strikes the enemy front (melee Vanguards) or fires
            // instant ranged (ranged Rearguards), casts a `Strike` ability, or passes.
            Phase::Fray => {
                let side = state.plan.committing;
                let other = 1 - side;
                let mut a = Vec::new();
                if let Some(&i) = self.pending(state, side).first() {
                    let targets: Vec<usize> = if state.s_vanguard(side)[i] {
                        // Vanguards strike the enemy front.
                        (0..state.s_len(other))
                            .filter(|&t| {
                                state.s_vanguard(other)[t] && !state.s_pool(other)[t].is_down()
                            })
                            .collect()
                    } else {
                        // Ranged Rearguards fire at the enemy front.
                        (0..state.s_len(other))
                            .filter(|&t| {
                                state.s_vanguard(other)[t] && !state.s_pool(other)[t].is_down()
                            })
                            .collect()
                    };
                    for t in targets {
                        a.push(Action::Target(i, t));
                    }
                    for idx in 0..state.s_pool(side)[i].actions.len() {
                        if self.card_playable_now(
                            state,
                            side,
                            i,
                            &state.s_pool(side)[i].actions[idx],
                        ) {
                            a.push(Action::PlayCard(i, idx));
                        }
                    }
                    a.push(Action::Pass(i));
                }
                a.push(Action::ToMenu);
                a
            }
            // §4.6 #3 Volley: a **free** Vanguard charges an enemy Rearguard (`Charge`) or flanks a
            // surviving enemy Vanguard (`Target`); a ranged Rearguard fires again (`Target`).
            Phase::Volley => {
                let side = state.plan.committing;
                let other = 1 - side;
                let mut a = Vec::new();
                if let Some(&i) = self.pending(state, side).first() {
                    if state.s_vanguard(side)[i] {
                        // Free Vanguard: charge the rear, or flank a surviving enemy Vanguard.
                        for t in (0..state.s_len(other)).filter(|&t| {
                            !state.s_vanguard(other)[t] && !state.s_pool(other)[t].is_down()
                        }) {
                            a.push(Action::Charge(i, t));
                        }
                        for t in (0..state.s_len(other)).filter(|&t| {
                            state.s_vanguard(other)[t] && !state.s_pool(other)[t].is_down()
                        }) {
                            a.push(Action::Target(i, t)); // a flank (a trade)
                        }
                    } else {
                        // Ranged Rearguard fires again at the enemy front.
                        for t in (0..state.s_len(other)).filter(|&t| {
                            state.s_vanguard(other)[t] && !state.s_pool(other)[t].is_down()
                        }) {
                            a.push(Action::Target(i, t));
                        }
                    }
                    for idx in 0..state.s_pool(side)[i].actions.len() {
                        if self.card_playable_now(
                            state,
                            side,
                            i,
                            &state.s_pool(side)[i].actions[idx],
                        ) {
                            a.push(Action::PlayCard(i, idx));
                        }
                    }
                    a.push(Action::Pass(i));
                }
                a.push(Action::ToMenu);
                a
            }
            // Breach & Reckoning resolve automatically; surface only an escape.
            Phase::Breach | Phase::Reckoning => vec![Action::ToMenu],
            Phase::Clash => vec![
                Action::Play(Move::Strike),
                Action::Play(Move::Anticipate),
                Action::Play(Move::Gather),
                Action::Play(Move::Evade),
                Action::ToMenu,
            ],
        }
    }

    fn action_label(&self, state: &State, action: &Action) -> String {
        if let Some(camp) = &state.campaign {
            if let Action::CampaignMove(i) = action {
                let acts = CAMPAIGN.legal_actions(camp);
                return acts
                    .get(*i)
                    .map(|ca| CAMPAIGN.action_label(camp, ca))
                    .unwrap_or_default();
            }
            if matches!(action, Action::ToMenu) {
                return "Leave the campaign".into();
            }
        }
        // Names resolve against the committing side (heroes in PvE / side A); the foe-name helper
        // against the other side. In PvE committing is always 0, so these are heroes/creatures.
        let side = state.plan.committing;
        let hname = |h: usize| {
            state
                .s_pool(side)
                .get(h)
                .map(|x| x.name.clone())
                .unwrap_or_else(|| "?".into())
        };
        let fname = |f: usize| {
            state
                .s_pool(1 - side)
                .get(f)
                .map(|x| x.name.clone())
                .unwrap_or_else(|| "?".into())
        };
        match action {
            Action::OpenCooperation => "Cooperation".into(),
            Action::OpenGod => "God-tier".into(),
            Action::OpenTutorial => "Duels".into(),
            Action::OpenVersus => "Versus (hotseat)".into(),
            Action::OpenCampaign => "Campaign".into(),
            Action::CampaignMove(_) => String::new(),
            Action::OpenEncyclopedia => "Rules".into(),
            Action::OpenCategory(i) => categories().get(*i).cloned().unwrap_or_else(|| "?".into()),
            Action::OpenCatalog => "Cards".into(),
            Action::OpenCard(i) => scenarios::card_catalog()
                .get(*i)
                .map(|e| e.name.clone())
                .unwrap_or_else(|| "?".into()),
            Action::Exit => "Exit".into(),
            Action::ToMenu => "Main menu".into(),
            Action::Back => "< Back".into(),
            Action::Replay => "Replay this scenario".into(),
            Action::PickScenario(i) => match &state.phase {
                Phase::Menu(m) => list_for(*m)
                    .get(*i)
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| "?".into()),
                _ => "?".into(),
            },
            Action::SetVanguard(h) => format!("Send {} to the Vanguard (front)", hname(*h)),
            Action::Flank(h) => format!("{} flanks the enemy front", hname(*h)),
            Action::SetRearguard(h) => format!("Pull {} back to the Rearguard", hname(*h)),
            Action::Guard(h) => format!("{}: block/slip incoming melee (vs trade)", hname(*h)),
            Action::Deploy => "Advance the phase".into(),
            Action::Charge(a, f) => format!("{} → charge the {}", hname(*a), fname(*f)),
            Action::Target(a, f) => format!("{} → strike the {}", hname(*a), fname(*f)),
            Action::PlayCard(h, idx) => {
                let c = state.s_pool(side).get(*h).and_then(|x| x.actions.get(*idx));
                match c {
                    Some(c) => format!("{}: {} ({})", hname(*h), c.name, c.summary()),
                    None => format!("{}: ?", hname(*h)),
                }
            }
            Action::Pass(h) => format!("{}: pass", hname(*h)),
            Action::Play(Move::Strike) => "Strike (beats Gather)".into(),
            Action::Play(Move::Anticipate) => "Anticipate (beats Evade)".into(),
            Action::Play(Move::Gather) => "Gather — build Force (beats Anticipate)".into(),
            Action::Play(Move::Evade) => "Evade (beats Strike)".into(),
        }
    }

    fn apply(&self, state: &mut State, action: &Action) -> Result<(), GameError> {
        match action {
            Action::Exit => {
                state.exiting = true;
                return Ok(());
            }
            Action::ToMenu => {
                *state = menu_state(state.seed);
                return Ok(());
            }
            Action::Replay => {
                if state.outcome.is_none() {
                    return Err(GameError::new("the fight is not over yet"));
                }
                let seed = state.seed.wrapping_add(1);
                state.seed = seed;
                state.rng = Rng::new(seed);
                if let Some(s) = state.scenario.clone() {
                    load_scenario(state, s);
                }
                return Ok(());
            }
            // Delegate a campaign move to the campaign game (resolve its index against its own
            // action list — the same list `legal_actions` numbered).
            Action::CampaignMove(i) => {
                let camp = state
                    .campaign
                    .as_mut()
                    .ok_or_else(|| GameError::new("not in a campaign"))?;
                let acts = CAMPAIGN.legal_actions(camp);
                let ca = acts
                    .get(*i)
                    .cloned()
                    .ok_or_else(|| GameError::new("no such campaign move"))?;
                return CAMPAIGN.apply(camp, &ca);
            }
            _ => {}
        }
        if state.outcome.is_some() {
            return Err(GameError::new("the fight is over"));
        }
        match (&state.phase, action) {
            (Phase::Menu(Menu::Top), Action::OpenCooperation) => {
                state.phase = Phase::Menu(Menu::Cooperation)
            }
            (Phase::Menu(Menu::Top), Action::OpenGod) => state.phase = Phase::Menu(Menu::God),
            (Phase::Menu(Menu::Top), Action::OpenTutorial) => {
                state.phase = Phase::Menu(Menu::Tutorial)
            }
            (Phase::Menu(Menu::Top), Action::OpenVersus) => state.phase = Phase::Menu(Menu::Versus),
            (Phase::Menu(Menu::Top), Action::OpenCampaign) => {
                state.campaign = Some(Box::new(reference_campaign()));
            }
            (Phase::Menu(Menu::Top), Action::OpenEncyclopedia) => {
                state.phase = Phase::Menu(Menu::Rules)
            }
            (Phase::Menu(Menu::Top), Action::OpenCatalog) => {
                state.phase = Phase::Menu(Menu::Catalog)
            }
            // Click a category card → open that category's rules (the prose reading pane).
            (Phase::Menu(Menu::Rules), Action::OpenCategory(i)) => {
                if *i >= categories().len() {
                    return Err(GameError::new("no such category"));
                }
                state.phase = Phase::Menu(Menu::Category(*i));
            }
            // Click a catalog card → open that card's detail page.
            (Phase::Menu(Menu::Catalog), Action::OpenCard(i)) => {
                if *i >= scenarios::card_catalog().len() {
                    return Err(GameError::new("no such card"));
                }
                state.phase = Phase::Menu(Menu::CardDetail(*i));
            }
            (Phase::Menu(m), Action::PickScenario(i)) if *m != Menu::Top => {
                let s = list_for(*m)
                    .into_iter()
                    .nth(*i)
                    .ok_or_else(|| GameError::new("no such scenario"))?;
                load_scenario(state, s);
            }
            // Back climbs the hierarchy: an entry list → categories → top.
            (Phase::Menu(Menu::Category(_)), Action::Back) => {
                state.phase = Phase::Menu(Menu::Rules)
            }
            // A card detail → back to the catalog grid.
            (Phase::Menu(Menu::CardDetail(_)), Action::Back) => {
                state.phase = Phase::Menu(Menu::Catalog)
            }
            (Phase::Menu(_), Action::Back) => state.phase = Phase::Menu(Menu::Top),

            // ---- §4.6 #1 Standoff: positions + Standing buffs ----
            (Phase::Standoff, Action::SetVanguard(i)) => {
                let side = state.plan.committing;
                state.s_vanguard_mut(side)[*i] = true;
            }
            (Phase::Standoff, Action::SetRearguard(i)) => {
                let side = state.plan.committing;
                state.s_vanguard_mut(side)[*i] = false;
            }
            // §4: set a hero unit's melee answer to Block for the round (one-way; resets to Trade at the
            // next Standoff). The solver explores the set of blockers without on/off toggle noise.
            (Phase::Standoff, Action::Guard(i)) => {
                state.plan.hero_guard[*i] = combat::Guard::Block;
            }
            (Phase::Standoff, Action::PlayCard(i, idx)) => {
                let side = state.plan.committing;
                let card = state.s_pool(side)[*i]
                    .actions
                    .get(*idx)
                    .cloned()
                    .ok_or_else(|| GameError::new("no such card"))?;
                if !self.card_playable_now(state, side, *i, &card) {
                    return Err(GameError::new("that card can't be cast in the Standoff"));
                }
                self.do_play_card(state, side, *i, card);
                // One Standing cast per unit per Standoff (the same one-action-per-phase discipline the
                // Fray/Volley use via `pending`); the unit may still act again in later phases. Without
                // this the greedy would replay a returning buff until its Tempo drained (§4.4).
                state.s_acted_mut(side)[*i] = true;
            }
            (Phase::Standoff, Action::Deploy) => {
                if state.pvp && state.plan.committing == 0 {
                    state.plan.committing = 1;
                    state.log.push("-- side B: the Standoff --".into());
                } else {
                    state.plan.committing = 0;
                    self.advance_phase(state);
                }
            }

            // ---- §4.6 #2 Fray ----
            (Phase::Fray, Action::Target(i, t)) => {
                let side = state.plan.committing;
                if state.s_vanguard(side)[*i] {
                    self.fray_strike(state, side == 0, *i, *t);
                } else {
                    self.fray_shot(state, side == 0, *i, *t);
                }
                state.s_acted_mut(side)[*i] = true;
                self.after_combat_action(state);
            }
            (Phase::Fray, Action::PlayCard(i, idx)) => {
                let side = state.plan.committing;
                let card = state.s_pool(side)[*i]
                    .actions
                    .get(*idx)
                    .cloned()
                    .ok_or_else(|| GameError::new("no such card"))?;
                if !self.card_playable_now(state, side, *i, &card) {
                    return Err(GameError::new("that card can't be cast in the Fray"));
                }
                self.do_play_card(state, side, *i, card);
                state.s_acted_mut(side)[*i] = true;
                self.after_combat_action(state);
            }
            (Phase::Fray, Action::Pass(i)) => {
                let side = state.plan.committing;
                state.s_acted_mut(side)[*i] = true;
                self.after_combat_action(state);
            }

            // ---- §4.6 #3 Volley: charge / flank / fire ----
            (Phase::Volley, Action::Charge(i, t)) => {
                let side = state.plan.committing;
                state.plan.charges.push(crate::state::Charge {
                    side,
                    attacker: *i,
                    target: *t,
                    flank: false,
                });
                state.log.push(format!(
                    "{} charges the {}.",
                    state.s_pool(side)[*i].name,
                    state.s_pool(1 - side)[*t].name
                ));
                state.s_acted_mut(side)[*i] = true;
                self.after_combat_action(state);
            }
            (Phase::Volley, Action::Target(i, t)) => {
                let side = state.plan.committing;
                if state.s_vanguard(side)[*i] {
                    // A flank (a Volley trade) — recorded, resolved at the Volley boundary.
                    state.plan.charges.push(crate::state::Charge {
                        side,
                        attacker: *i,
                        target: *t,
                        flank: true,
                    });
                    state.log.push(format!(
                        "{} moves to flank the {}.",
                        state.s_pool(side)[*i].name,
                        state.s_pool(1 - side)[*t].name
                    ));
                } else {
                    // A Rearguard fires again (instant ranged).
                    self.fray_shot(state, side == 0, *i, *t);
                }
                state.s_acted_mut(side)[*i] = true;
                self.after_combat_action(state);
            }
            (Phase::Volley, Action::Flank(i)) => {
                // Convenience: flank the first surviving enemy Vanguard.
                let side = state.plan.committing;
                let other = 1 - side;
                if let Some(t) = (0..state.s_len(other))
                    .find(|&t| state.s_vanguard(other)[t] && !state.s_pool(other)[t].is_down())
                {
                    state.plan.charges.push(crate::state::Charge {
                        side,
                        attacker: *i,
                        target: t,
                        flank: true,
                    });
                }
                state.s_acted_mut(side)[*i] = true;
                self.after_combat_action(state);
            }
            (Phase::Volley, Action::PlayCard(i, idx)) => {
                let side = state.plan.committing;
                let card = state.s_pool(side)[*i]
                    .actions
                    .get(*idx)
                    .cloned()
                    .ok_or_else(|| GameError::new("no such card"))?;
                if !self.card_playable_now(state, side, *i, &card) {
                    return Err(GameError::new("that card can't be cast in the Volley"));
                }
                self.do_play_card(state, side, *i, card);
                state.s_acted_mut(side)[*i] = true;
                self.after_combat_action(state);
            }
            (Phase::Volley, Action::Pass(i)) => {
                let side = state.plan.committing;
                state.s_acted_mut(side)[*i] = true;
                self.after_combat_action(state);
            }

            (Phase::Clash, Action::Play(m)) => {
                if !self.legal_actions(state).contains(action) {
                    return Err(GameError::new("that move is not available"));
                }
                self.clash_beat(state, *m);
            }
            _ => return Err(GameError::new("that action is not legal right now")),
        }
        Ok(())
    }

    fn outcome(&self, state: &State) -> Option<Outcome> {
        // The campaign is a sub-activity, not the app's terminal outcome — winning the run shows a
        // victory and offers "Leave the campaign", rather than ending the whole session.
        if state.campaign.is_some() {
            return None;
        }
        state.outcome.clone()
    }

    fn suggest(&self, state: &State) -> Option<Action> {
        let camp = state.campaign.as_ref()?;
        let want = CAMPAIGN.suggest(camp)?;
        let pos = CAMPAIGN
            .legal_actions(camp)
            .iter()
            .position(|a| *a == want)?;
        Some(Action::CampaignMove(pos))
    }

    fn is_suggested(&self, state: &State, action: &Action) -> bool {
        let Some(camp) = &state.campaign else {
            return false;
        };
        let Action::CampaignMove(i) = action else {
            return false;
        };
        let Some(want) = CAMPAIGN.suggest(camp) else {
            return false;
        };
        CAMPAIGN.legal_actions(camp).get(*i) == Some(&want)
    }

    fn cancel_action(&self, state: &State) -> Option<Action> {
        // Esc leaves the campaign (back to the menu where it was launched).
        if state.campaign.is_some() {
            return Some(Action::ToMenu);
        }
        if state.outcome.is_some() {
            return None;
        }
        match &state.phase {
            Phase::Menu(Menu::Top) => None,
            Phase::Menu(_) => Some(Action::Back),
            _ => Some(Action::ToMenu),
        }
    }

    fn session_key(&self, state: &State) -> u64 {
        // Each scenario / the campaign is its own sticky session with local undo; the menu is the
        // shared hub. A combat scenario is keyed by its name so re-picking it resumes the same one,
        // and the campaign (world *and* its battles) is one session.
        const MENU: u64 = 0;
        const CAMPAIGN: u64 = 1;
        if state.campaign.is_some() {
            return CAMPAIGN;
        }
        match &state.scenario {
            Some(s) => scenario_key(&s.name),
            None => MENU,
        }
    }

    fn exit_requested(&self, state: &State) -> bool {
        state.exiting
    }

    fn is_exit_action(&self, _state: &State, action: &Action) -> bool {
        matches!(action, Action::Exit)
    }

    fn reference(&self) -> Vec<engine::RefEntry> {
        scenarios::glossary()
    }

    fn view(&self, state: &State, perspective: Option<PlayerId>) -> TableView {
        if let Some(camp) = &state.campaign {
            return CAMPAIGN.view(camp, perspective);
        }
        let mut zones = Vec::new();
        let mut prose: Vec<engine::ProseLine> = Vec::new();
        match &state.phase {
            Phase::Menu(Menu::Top) => zones.push(menu_zone()),
            // Categories are just names → clickable cards; the *content* of a category is the
            // reading pane (prose), since long rules text doesn't belong on a card.
            Phase::Menu(Menu::Rules) => zones.push(category_zone()),
            Phase::Menu(Menu::Category(i)) => {
                let cat = categories().into_iter().nth(*i).unwrap_or_default();
                prose.push(engine::ProseLine::Heading(cat.clone()));
                for e in entries_in(&cat) {
                    prose.push(engine::ProseLine::Term(e.term));
                    prose.push(engine::ProseLine::Body(e.text));
                    prose.push(engine::ProseLine::Gap);
                }
                // RPS-ish charts, shown in the category they belong to (discoverable in place):
                // the role triangle under Roles, the Clash counter-grid under the Clash module.
                match cat.as_str() {
                    "Roles" => append_triangle_chart(&mut prose),
                    "Clash module" => append_clash_chart(&mut prose),
                    _ => {}
                }
            }
            // The catalog: every card, grouped into a section per kind (clickable to open detail).
            Phase::Menu(Menu::Catalog) => {
                let entries = scenarios::card_catalog();
                let mut i = 0;
                while i < entries.len() {
                    let kind = entries[i].kind;
                    let mut cards = Vec::new();
                    while i < entries.len() && entries[i].kind == kind {
                        cards.push(entries[i].view.clone().action(i));
                        i += 1;
                    }
                    zones.push(ZoneView {
                        label: kind.to_string(),
                        layout: Layout::Row,
                        owner: None,
                        cards,
                    });
                }
            }
            // A card's detail: the printed card itself, plus its rules description as a reading pane.
            Phase::Menu(Menu::CardDetail(idx)) => {
                let entries = scenarios::card_catalog();
                if let Some(e) = entries.get(*idx) {
                    zones.push(ZoneView {
                        label: e.kind.to_string(),
                        layout: Layout::Row,
                        owner: None,
                        cards: vec![e.view.clone()],
                    });
                    prose = e.detail.clone();
                }
            }
            Phase::Menu(m) => zones.push(scenario_zone(*m)),
            Phase::Clash => {
                if let Some(c) = state.clash {
                    zones.push(creature_zone(state, Some(c.foe)));
                    zones.push(hero_zone(state, Some(c.hero)));
                }
            }
            // §4.6 #1 Standoff reads as **card placement**: the enemy on top, then your Vanguard
            // (front) and Rearguard (back) zones, each character card clickable to toggle its position.
            Phase::Standoff => {
                let side = state.plan.committing;
                zones.push(if side == 0 {
                    creature_zone(state, None)
                } else {
                    hero_zone(state, None)
                });
                let acts = self.legal_actions(state);
                let idx_of = |want: &Action| acts.iter().position(|a| a == want);
                let mut front = Vec::new();
                let mut back = Vec::new();
                for i in 0..state.s_len(side) {
                    let actor = &state.s_pool(side)[i];
                    if actor.fallen {
                        continue;
                    }
                    let is_front = state.s_vanguard(side)[i];
                    // Clicking a card sends it to the *other* position.
                    let toggle = if is_front {
                        Action::SetRearguard(i)
                    } else {
                        Action::SetVanguard(i)
                    };
                    let mut card = actor_card(actor, Accent::Ally);
                    if let Some(idx) = idx_of(&toggle) {
                        card = card.action(idx);
                    }
                    if is_front {
                        front.push(card);
                    } else {
                        back.push(card);
                    }
                }
                zones.push(ZoneView {
                    label: "Vanguard — hold the front".into(),
                    layout: Layout::Row,
                    owner: None,
                    cards: front,
                });
                zones.push(ZoneView {
                    label: "Rearguard — hold back, fire from the rear".into(),
                    layout: Layout::Row,
                    owner: None,
                    cards: back,
                });
            }
            _ => {
                zones.push(creature_zone(state, None));
                zones.push(hero_zone(state, None));
            }
        }
        TableView {
            status: self.status(state),
            zones,
            prose,
            map: None,
            log: self.feed(state),
        }
    }
}

// ---- view helpers -------------------------------------------------------

fn pips(remaining: u32, max: u32) -> String {
    let lost = max.saturating_sub(remaining) as usize;
    format!("{}{}", "#".repeat(remaining as usize), ".".repeat(lost))
}

fn actor_card(a: &crate::actor::Actor, accent: Accent) -> CardView {
    let d = &a.defense;
    CardView::up(format!("{} — {}", a.name, a.role))
        .body(vec![
            format!("HP [{}]", pips(d.health.remaining, d.health.max)),
            format!(
                "Cad {} Fin {} Mgt {} {}",
                a.offense.cadence,
                a.offense.finesse.max(1),
                a.offense.might,
                a.attack.label()
            ),
            format!("Tempo {}", a.tempo),
        ])
        .accent(accent)
}

fn hero_zone(state: &State, focus: Option<usize>) -> ZoneView {
    ZoneView {
        label: "Your party".into(),
        layout: Layout::Row,
        owner: None,
        cards: state
            .heroes
            .iter()
            .enumerate()
            .map(|(i, h)| {
                let accent = if Some(i) == focus {
                    Accent::Good
                } else {
                    Accent::Ally
                };
                actor_card(h, accent)
            })
            .collect(),
    }
}

fn creature_zone(state: &State, focus: Option<usize>) -> ZoneView {
    ZoneView {
        label: "Foes".into(),
        layout: Layout::Row,
        owner: None,
        cards: state
            .creatures
            .iter()
            .enumerate()
            .filter(|(_, c)| !c.fallen)
            .map(|(i, c)| {
                let accent = if Some(i) == focus {
                    Accent::Foe
                } else {
                    Accent::Neutral
                };
                actor_card(c, accent)
            })
            .collect(),
    }
}

/// Wrap `text` to `width`-ish columns over at most `max` lines (a tiny word-wrap for card
/// bodies; the last line is ellipsized if the text doesn't fit).
fn wrap(text: &str, width: usize, max: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.len() + 1 + word.len() > width {
            lines.push(std::mem::take(&mut line));
            if lines.len() == max {
                break;
            }
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if lines.len() < max && !line.is_empty() {
        lines.push(line);
    }
    if max > 0 && lines.len() == max {
        lines.last_mut().unwrap().push('…');
    }
    lines
}

/// The encyclopedia's top level: each category is a **clickable card** (just a name) bound to
/// `OpenCategory(i)` — index `i` in `legal_actions` for `Menu(Rules)` — showing its entry count.
/// Picking one opens that category's rules as a prose reading pane (the content, not cards).
fn category_zone() -> ZoneView {
    ZoneView {
        label: "Rules — pick a category".into(),
        layout: Layout::Row,
        owner: None,
        cards: categories()
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let n = entries_in(c).len();
                CardView::up(c.clone())
                    .typed(format!("{n} entries"))
                    .action(i)
            })
            .collect(),
    }
}

/// The top menu: each scenario set and Rules is a **clickable card** bound to its open action
/// (indices 0..4 in `legal_actions` for `Menu(Top)`). Buttons are left only for the meta (Exit).
fn menu_zone() -> ZoneView {
    // Order must match `legal_actions` for `Menu::Top`: each card binds to action index `i`.
    let items = [
        ("Duels", "Learn the game, one lesson at a time."),
        (
            "Cooperation",
            "Party scenarios — specialists who cover each other.",
        ),
        ("God-tier", "Solo power fantasy vs the odds."),
        ("Versus", "Hotseat PvP — pass and play."),
        (
            "Campaign",
            "The world-map reference run — travel, fight, grow, win.",
        ),
        ("Cards", "Browse every card and how it works."),
        ("Rules", "The rulebook — browse by category."),
    ];
    ZoneView {
        label: "Deckbound — choose a set".into(),
        layout: Layout::Row,
        owner: None,
        cards: items
            .iter()
            .enumerate()
            .map(|(i, (t, d))| CardView::up(*t).body(wrap(d, 22, 4)).action(i))
            .collect(),
    }
}

/// A scenario list: each scenario is a **clickable card** (bound to `PickScenario(i)`) carrying
/// its blurb. The only button is **Back**.
fn scenario_zone(menu: Menu) -> ZoneView {
    ZoneView {
        label: "Pick a scenario".into(),
        layout: Layout::Row,
        owner: None,
        cards: list_for(menu)
            .iter()
            .enumerate()
            .map(|(i, s)| {
                // The combat-mode tell, driven from the same flag the engine switches on.
                let mode = if s.clash {
                    "Clash duel · RPS"
                } else if s.pvp {
                    "Lane battle · PvP"
                } else {
                    "Lane battle"
                };
                CardView::up(s.name.clone())
                    .typed(mode)
                    .body(wrap(&s.blurb, 22, 7))
                    .action(i)
            })
            .collect(),
    }
}

/// The §4 / §8.5 **playstyle triangle** (Aggressor ▸ Glass-Cannon ▸ Turtle ▸ Aggressor) as a small
/// chart — the three damage Roles mediated by the Tempo economy.
fn append_triangle_chart(prose: &mut Vec<engine::ProseLine>) {
    prose.push(engine::ProseLine::Gap);
    prose.push(engine::ProseLine::Heading("The triangle".into()));
    for line in [
        "Aggressor (Infiltrator) ▸ beats Glass-Cannon — cracks the thin shield before the cannons win",
        "Glass-Cannon (Artillery) ▸ beats Turtle — out-guns a passive defender it never has to reach",
        "Turtle (Wall) ▸ beats Aggressor — drains the pusher dry, so it reaches the back empty",
    ] {
        prose.push(engine::ProseLine::Body(line.into()));
    }
}

/// The Clash four-card counter-grid ("what beats what"): row vs column, from the row's view.
fn append_clash_chart(prose: &mut Vec<engine::ProseLine>) {
    let win = |t: &str| engine::GridCell::new(t, Accent::Good);
    let lose = engine::GridCell::new("lose", Accent::Foe);
    let trade = engine::GridCell::new("trade", Accent::Warn);
    let none = engine::GridCell::new("—", Accent::Neutral);
    let row = |label: &str, cells: Vec<engine::GridCell>| engine::GridRow {
        label: label.into(),
        cells,
    };
    prose.push(engine::ProseLine::Gap);
    prose.push(engine::ProseLine::Heading("What beats what".into()));
    prose.push(engine::ProseLine::Grid(engine::Grid {
        headers: ["Strike", "Antic.", "Gather", "Evade"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        rows: vec![
            row(
                "Strike",
                vec![trade.clone(), win("win"), win("win"), lose.clone()],
            ),
            row(
                "Anticipate",
                vec![lose.clone(), none.clone(), lose.clone(), win("win")],
            ),
            row(
                "Gather",
                vec![lose.clone(), win("win"), none.clone(), none.clone()],
            ),
            row(
                "Evade",
                vec![win("win"), lose.clone(), none.clone(), none.clone()],
            ),
        ],
    }));
    prose.push(engine::ProseLine::Body(
        "Strike vs Strike trades; Evade vs a Strike also steals the striker's Force.".into(),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn launch(game: &Deckbound, s: &mut State, open: Action, index: usize) {
        game.apply(s, &open).unwrap();
        game.apply(s, &Action::PickScenario(index)).unwrap();
    }

    /// Drive a scenario to an outcome with a rough auto-strategy.
    fn autoplay(game: &Deckbound, s: &mut State) -> Outcome {
        let mut guard = 0;
        while game.current_player(s).is_some() {
            let acts = game.legal_actions(s);
            let action = match s.phase {
                Phase::Clash => {
                    let beat = s.clash.map(|c| c.beat).unwrap_or(0);
                    if beat % 2 == 0 {
                        Action::Play(Move::Strike)
                    } else {
                        Action::Play(Move::Anticipate)
                    }
                }
                Phase::Standoff => Action::Deploy,
                Phase::Fray | Phase::Volley => acts
                    .iter()
                    .find(|a| matches!(a, Action::Charge(..)))
                    .or_else(|| acts.iter().find(|a| matches!(a, Action::Target(..))))
                    .or_else(|| acts.iter().find(|a| matches!(a, Action::Pass(_))))
                    .copied()
                    .unwrap_or(Action::ToMenu),
                _ => break,
            };
            game.apply(s, &action).unwrap();
            guard += 1;
            assert!(guard < 20_000, "scenario should terminate");
        }
        game.outcome(s).unwrap()
    }

    #[test]
    fn new_game_starts_in_menu() {
        assert_eq!(Deckbound.new_game(1, 1).phase, Phase::Menu(Menu::Top));
    }

    /// The in-app encyclopedia exposes the rules reference (Game::reference).
    #[test]
    fn reference_exposes_the_rules() {
        let r = Deckbound.reference();
        assert!(r.len() >= 10, "the encyclopedia has entries");
        assert!(r.iter().any(|e| e.term == "Vanguard"));
        assert!(r.iter().any(|e| e.category == "Clash module"));
    }

    /// The encyclopedia is reachable as an action and navigable category → entries → back.
    #[test]
    fn encyclopedia_hierarchy_navigates() {
        let game = Deckbound;
        let mut s = game.new_game(1, 1);
        // "Rules" is offered as an action (a left-panel button) on the top menu.
        assert!(game.legal_actions(&s).contains(&Action::OpenEncyclopedia));
        game.apply(&mut s, &Action::OpenEncyclopedia).unwrap();
        assert_eq!(s.phase, Phase::Menu(Menu::Rules));
        // The category list offers one open-action per category.
        let cats = super::categories().len();
        assert!(cats >= 4 && game.legal_actions(&s).contains(&Action::OpenCategory(0)));
        game.apply(&mut s, &Action::OpenCategory(0)).unwrap();
        assert_eq!(s.phase, Phase::Menu(Menu::Category(0)));
        // Back climbs the hierarchy: entries → categories → top.
        game.apply(&mut s, &Action::Back).unwrap();
        assert_eq!(s.phase, Phase::Menu(Menu::Rules));
        game.apply(&mut s, &Action::Back).unwrap();
        assert_eq!(s.phase, Phase::Menu(Menu::Top));
    }

    /// The Campaign is reachable as a top-menu card, and the guide drives it to victory entirely
    /// through `Deckbound`'s wrapped actions (`OpenCampaign` → `CampaignMove`s) — i.e. the merge
    /// preserves the standalone campaign's win.
    #[test]
    fn campaign_is_playable_from_the_menu() {
        let game = Deckbound;
        let mut s = game.new_game(1, 1);
        assert!(game.legal_actions(&s).contains(&Action::OpenCampaign));
        game.apply(&mut s, &Action::OpenCampaign).unwrap();
        assert!(s.campaign.is_some(), "the campaign sub-state is live");
        // outcome() stays None in campaign mode (a sub-activity, not the session's end).
        assert!(game.outcome(&s).is_none());

        for _ in 0..10_000 {
            // Win shows as the campaign's own outcome; the menu stays reachable via ToMenu.
            if s.campaign
                .as_ref()
                .and_then(|c| c.outcome.clone())
                .is_some()
            {
                break;
            }
            let suggested = game.suggest(&s).expect("the guide always has a next move");
            assert!(game.is_suggested(&s, &suggested), "suggest() round-trips");
            game.apply(&mut s, &suggested).unwrap();
        }
        let won = matches!(
            s.campaign.as_ref().and_then(|c| c.outcome.clone()),
            Some(Outcome::Win(PlayerId(0)))
        );
        assert!(
            won,
            "the guide wins the reference run through the merged menu"
        );

        // Leaving returns to the top menu, campaign cleared.
        game.apply(&mut s, &Action::ToMenu).unwrap();
        assert!(s.campaign.is_none());
        assert_eq!(s.phase, Phase::Menu(Menu::Top));
    }

    /// Event-sourcing invariant (the basis of the renderer's save/load + undo): an action log,
    /// serialized through RON and back, replays from a fresh game to the *identical* result.
    #[test]
    fn action_log_round_trips_through_ron() {
        let game = Deckbound;
        // Record the guided campaign run as a flat action log (what the UI persists).
        let mut s = game.new_game(1, 1);
        let mut log: Vec<Action> = vec![Action::OpenCampaign];
        game.apply(&mut s, &Action::OpenCampaign).unwrap();
        for _ in 0..10_000 {
            if s.campaign
                .as_ref()
                .and_then(|c| c.outcome.clone())
                .is_some()
            {
                break;
            }
            let a = game.suggest(&s).expect("the guide always has a next move");
            log.push(a);
            game.apply(&mut s, &a).unwrap();
        }
        let original_days = s.campaign.as_ref().unwrap().run.day;

        // Round-trip the log through the save-file format, then replay from new_game.
        let text = ron::ser::to_string(&log).expect("the action log serializes");
        let restored: Vec<Action> = ron::from_str(&text).expect("and deserializes");
        assert_eq!(restored, log, "RON round-trips the action log exactly");
        let mut replay = game.new_game(1, 1);
        for a in &restored {
            game.apply(&mut replay, a)
                .expect("every logged action replays");
        }
        assert_eq!(
            replay.campaign.as_ref().unwrap().run.day,
            original_days,
            "replaying the log reconstructs the same state"
        );
        assert!(matches!(
            replay.campaign.as_ref().unwrap().outcome,
            Some(Outcome::Win(PlayerId(0)))
        ));
    }

    /// The card catalog is reachable from the menu, cards open to a detail page showing both the
    /// card and its rules, and Back climbs detail → catalog → top.
    #[test]
    fn card_catalog_navigates() {
        let game = Deckbound;
        let mut s = game.new_game(1, 1);
        assert!(game.legal_actions(&s).contains(&Action::OpenCatalog));
        game.apply(&mut s, &Action::OpenCatalog).unwrap();
        assert_eq!(s.phase, Phase::Menu(Menu::Catalog));

        // The catalog is a grid of clickable cards (one open-action each).
        assert!(game.legal_actions(&s).contains(&Action::OpenCard(0)));
        let grid = game.view(&s, None);
        assert!(
            grid.zones
                .iter()
                .any(|z| z.cards.iter().any(|c| c.action.is_some())),
            "the catalog shows clickable cards"
        );

        // Opening a card shows the card *and* a prose rules description.
        game.apply(&mut s, &Action::OpenCard(0)).unwrap();
        assert_eq!(s.phase, Phase::Menu(Menu::CardDetail(0)));
        let detail = game.view(&s, None);
        assert!(!detail.zones.is_empty(), "the detail shows the card");
        assert!(!detail.prose.is_empty(), "the detail shows the rules");

        // Back climbs the hierarchy.
        game.apply(&mut s, &Action::Back).unwrap();
        assert_eq!(s.phase, Phase::Menu(Menu::Catalog));
        game.apply(&mut s, &Action::Back).unwrap();
        assert_eq!(s.phase, Phase::Menu(Menu::Top));
    }

    /// §4.6 Standoff: a melee unit defaults to the Vanguard (front), a ranged unit to the Rearguard
    /// (back); the human may toggle a unit's position and advance to begin the Fray.
    #[test]
    fn standoff_sets_positions_then_advances_to_the_fray() {
        let game = Deckbound;
        let mut hero = scenarios::build_character("Novice", &[]);
        hero.attack = crate::actor::Attack::Melee;
        let foe = scenarios::build_creature("Husk");
        let mut s = battle_state(vec![hero], vec![foe], false, 1);
        assert_eq!(s.phase, Phase::Standoff);
        assert!(
            s.plan.hero_vanguard[0],
            "a melee unit defaults to the front"
        );
        // Toggle to the Rearguard and back.
        game.apply(&mut s, &Action::SetRearguard(0)).unwrap();
        assert!(!s.plan.hero_vanguard[0]);
        game.apply(&mut s, &Action::SetVanguard(0)).unwrap();
        assert!(s.plan.hero_vanguard[0]);
        // Advance — the Fray begins (or the fight resolves if a side is wiped).
        game.apply(&mut s, &Action::Deploy).unwrap();
        assert!(matches!(s.phase, Phase::Fray | Phase::Standoff) || s.outcome.is_some());
    }

    /// §4.6 cast window **and** §4.4 target-classification position gate. A `cast: Standing` support card
    /// (Wall's Brace) is offered in the Standoff (rank-free); a `cast: Strike` offensive card (Artillery's
    /// Bolt) is not (wrong window) — and once the Fray opens, the ranged offensive Bolt is castable **only
    /// from the Rearguard** (a Vanguard cannot rain ranged spells, §4.2), never the front.
    #[test]
    fn cast_window_and_position_gate_role_cards() {
        use crate::currency::Currency;
        let game = Deckbound;
        // A hero holding both a Standing support card (Iron L1 Brace) and a Strike ranged-offensive card
        // (Brass L1 Bolt).
        let hero = scenarios::build_character(
            "Novice",
            &[
                scenarios::RewardId {
                    track: Currency::Iron,
                    level: 1,
                },
                scenarios::RewardId {
                    track: Currency::Brass,
                    level: 1,
                },
            ],
        );
        let foe = scenarios::build_creature("Husk");
        let mut s = battle_state(vec![hero], vec![foe], false, 1);
        assert_eq!(s.phase, Phase::Standoff);

        // Index the hero's two role cards.
        let brace = s.heroes[0]
            .actions
            .iter()
            .position(|c| c.name == "Brace")
            .expect("Iron L1 grants Brace");
        let bolt = s.heroes[0]
            .actions
            .iter()
            .position(|c| c.name == "Bolt")
            .expect("Brass L1 grants Bolt");

        // In the Standoff: the Standing (support) Brace is playable, the Strike Bolt is not (wrong window).
        let brace_card = s.heroes[0].actions[brace].clone();
        let bolt_card = s.heroes[0].actions[bolt].clone();
        assert!(!brace_card.is_offensive());
        assert!(bolt_card.is_offensive() && bolt_card.is_ranged());
        assert!(
            game.card_playable_now(&s, 0, 0, &brace_card),
            "a cast:Standing support card is offered in the Standoff (rank-free)"
        );
        assert!(
            !game.card_playable_now(&s, 0, 0, &bolt_card),
            "a cast:Strike card is NOT offered in the Standoff (wrong window)"
        );

        // Advance to the Fray; the Standing card is now out of window.
        game.apply(&mut s, &Action::Deploy).unwrap();
        if s.phase == Phase::Fray {
            assert!(
                !game.card_playable_now(&s, 0, 0, &brace_card),
                "a cast:Standing card is not castable in the Fray (wrong window)"
            );
            // §4.4 position gate: the ranged offensive Bolt fires only from the Rearguard.
            s.plan.hero_vanguard[0] = true; // at the front
            assert!(
                !game.card_playable_now(&s, 0, 0, &bolt_card),
                "an offensive ranged spell cannot be cast from the Vanguard (§4.2)"
            );
            s.plan.hero_vanguard[0] = false; // holding the back
            assert!(
                game.card_playable_now(&s, 0, 0, &bolt_card),
                "an offensive ranged spell fires from the Rearguard"
            );
        }
    }

    /// §0 Ruleset: reaching the round cap ends an unfinished fight as a **draw** (which, in PvE, is no
    /// different from a loss). The cap is a pre-game parameter, not a fixed law.
    #[test]
    fn the_round_cap_draws_an_unfinished_fight() {
        let game = Deckbound;
        let hero = scenarios::build_character("Novice", &[]);
        let foe = scenarios::build_creature("Husk");
        let mut s = battle_state_with(
            vec![hero],
            vec![foe],
            false,
            1,
            Ruleset {
                max_rounds: 3,
                max_unique_per_side: 5,
                ..Ruleset::default()
            },
        );
        assert!(s.outcome.is_none());
        s.round = 3; // sitting at the cap, with the fight unfinished
        game.next_round(&mut s);
        assert!(
            matches!(s.outcome, Some(Outcome::Tie(_))),
            "reaching the round cap is a draw"
        );
    }

    /// The optional Clash module runs a four-card duel to an outcome.
    #[test]
    fn clash_module_runs_a_duel() {
        let game = Deckbound;
        let mut s = game.new_game(3, 1);
        game.apply(&mut s, &Action::OpenTutorial).unwrap();
        let idx = scenarios::tutorials().iter().position(|t| t.clash).unwrap();
        game.apply(&mut s, &Action::PickScenario(idx)).unwrap();
        assert_eq!(s.phase, Phase::Clash);
        let _ = autoplay(&game, &mut s);
        assert!(s.outcome.is_some());
    }

    /// Power: Ward gives a melee-less ally (the ranged cannon) a melee guard so it can
    /// self-defend (§4.2 + §4 powers).
    #[test]
    fn ward_grants_a_melee_guard() {
        let game = Deckbound;
        let mut s = game.new_game(1, 1);
        game.apply(&mut s, &Action::OpenCooperation).unwrap();
        game.apply(&mut s, &Action::PickScenario(0)).unwrap(); // Ward the Cannon: Sear, Vow
        let vow = s.heroes.iter().position(|h| h.name == "Vow").unwrap();
        let sear = s.heroes.iter().position(|h| h.name == "Sear").unwrap();
        assert_eq!(s.heroes[sear].attack, crate::actor::Attack::Ranged);
        let idx = s.heroes[vow]
            .actions
            .iter()
            .position(|c| c.name == "Ward")
            .unwrap();
        let card = s.heroes[vow].actions[idx].clone();
        let off = s.heroes[vow].offense;
        let name = s.heroes[vow].name.clone();
        let mut heroes = std::mem::take(&mut s.heroes);
        combat::play_card(
            &card,
            &name,
            off,
            &mut s.creatures,
            &mut heroes,
            Some(vow),
            &mut s.log,
        );
        s.heroes = heroes;
        assert_eq!(
            s.heroes[sear].attack,
            crate::actor::Attack::Both,
            "Ward gives the ranged cannon a melee guard"
        );
    }

    /// Hotseat PvP: each phase is committed by side A, then side B (current_player alternates),
    /// before it resolves — pass-and-play hidden commit (§3.4).
    #[test]
    fn pvp_alternates_sides_per_phase() {
        let game = Deckbound;
        let mut s = game.new_game(1, 1);
        game.apply(&mut s, &Action::OpenVersus).unwrap();
        let idx = scenarios::versus().iter().position(|v| v.pvp).unwrap();
        game.apply(&mut s, &Action::PickScenario(idx)).unwrap();
        assert_eq!(s.phase, Phase::Standoff);
        assert_eq!(
            game.current_player(&s),
            Some(PlayerId(0)),
            "side A takes the Standoff first"
        );
        game.apply(&mut s, &Action::Deploy).unwrap();
        assert_eq!(s.phase, Phase::Standoff, "still in the Standoff");
        assert_eq!(
            game.current_player(&s),
            Some(PlayerId(1)),
            "now side B takes the Standoff"
        );
        game.apply(&mut s, &Action::Deploy).unwrap();
        // Both committed the Standoff → the Fray begins; play it out to an outcome.
        let _ = autoplay(&game, &mut s);
        assert!(s.outcome.is_some());
    }

    /// A base-mode cooperation scenario runs the six-phase round to an outcome.
    #[test]
    fn base_scenario_runs_lanes() {
        let game = Deckbound;
        let mut s = game.new_game(2, 1);
        game.apply(&mut s, &Action::OpenCooperation).unwrap();
        game.apply(&mut s, &Action::PickScenario(0)).unwrap();
        assert_eq!(s.phase, Phase::Standoff);
        let _ = autoplay(&game, &mut s);
        assert!(s.outcome.is_some());
    }

    /// Part 1 — the **per-unit lock** (§4.6, exact). Two heroes front two foes; hero 0 kills its
    /// struck foe in the Fray, hero 1's struck foe survives. At the Fray boundary, `fix_breach_list`
    /// (now fed by the recorded attacked-map through `combat::compute_locks`) must free the killer
    /// while keeping hero 1 pinned — **even though a live enemy Vanguard still stands**. The old
    /// all-or-nothing approximation locked *both* (any live enemy Vanguard ⇒ locked); this proves the
    /// regression is gone.
    #[test]
    fn a_freed_locker_is_free_while_other_enemy_vanguards_stand() {
        use crate::actor::Attack;
        let game = Deckbound;

        // A bare melee fighter with explicit stats and ample Tempo (the test controls who acts).
        fn fighter(name: &str, might: u32, vit: u32, tough: u32) -> Actor {
            let mut a = scenarios::build_character("Novice", &[]);
            a.name = name.into();
            a.attack = Attack::Melee;
            a.offense.might = might;
            a.offense.finesse = a.offense.finesse.max(1);
            a.defense = crate::stats::Defense::new(vit, tough);
            a.weapon = {
                // a 0-power weapon so a blow is exactly `might`
                let mut w = a.weapon.clone();
                w.effects.clear();
                w
            };
            a.tempo = 10;
            a
        }

        let heroes = vec![fighter("Killer", 5, 5, 5), fighter("Pinned", 1, 5, 5)];
        // foe 0 dies (V1/T2 ⇐ Might 5 flips its only card); foe 1 survives (V3/T5 ⇐ Might 1); foe 2 is
        // a Rearguard. All foes Might 0 so the trade-back never kills a hero.
        let foes = vec![
            fighter("Doomed", 0, 1, 2),
            fighter("Survivor", 0, 3, 5),
            fighter("Rear", 0, 2, 2),
        ];
        let mut s = battle_state(heroes, foes, false, 1);
        // Front the two foes; pull the third back to the Rearguard.
        s.plan.foe_vanguard = vec![true, true, false];
        s.plan.hero_vanguard = vec![true, true];
        // Begin the Fray. (PvE: the foe side strikes first — Might 0, harmless.)
        game.apply(&mut s, &Action::Deploy).unwrap();
        assert_eq!(s.phase, Phase::Fray);

        // Drive the hero Fray explicitly: Killer strikes foe 0, Pinned strikes foe 1.
        game.apply(&mut s, &Action::Target(0, 0)).unwrap();
        // After hero 0 acts, hero 1 is the pending unit; strike foe 1.
        game.apply(&mut s, &Action::Target(1, 1)).unwrap();

        // Closing the Fray fixes the breach list. (If hero 1 had nothing left, the phase auto-advanced;
        // otherwise pass it so the Fray closes.)
        if s.phase == Phase::Fray {
            if let Some(&i) = game.pending(&s, 0).first() {
                game.apply(&mut s, &Action::Pass(i)).unwrap();
            }
        }
        assert!(s.creatures[0].fallen, "foe 0 died in the Fray");
        assert!(!s.creatures[1].fallen, "foe 1 survived");
        assert!(
            !s.plan.hero_locked[0],
            "the killer is FREE — its struck foe is dead (per-unit lock)"
        );
        assert!(
            s.plan.hero_locked[1],
            "hero 1 stays LOCKED — its struck foe still stands"
        );
    }

    /// A bare melee fighter with explicit stats and ample Tempo (shared by the Pin/Rout game tests).
    fn fighter(name: &str, might: u32, vit: u32, tough: u32) -> Actor {
        use crate::actor::Attack;
        let mut a = scenarios::build_character("Novice", &[]);
        a.name = name.into();
        a.attack = Attack::Melee;
        a.offense.might = might;
        a.offense.finesse = a.offense.finesse.max(1);
        a.defense = crate::stats::Defense::new(vit, tough);
        a.weapon = {
            let mut w = a.weapon.clone();
            w.effects.clear();
            w
        };
        a.tempo = 10;
        a
    }

    /// §10 **Pin** (Artillery space-control): a free enemy Vanguard pinned by suppressive fire is
    /// **denied its charge** this round. We pin a foe that *would* have charged the hero Rearguard and
    /// confirm the rear is untouched (the pinned foe declares no charge in the foe Volley).
    #[test]
    fn pin_denies_a_charge() {
        let game = Deckbound;
        // A foe Vanguard (would charge) + a hero front-holder so the foe is free, and a hero rear it
        // would gut. Foe Might 5; hero rear V1/T1 so an un-pinned charge would clearly hurt it.
        let heroes = vec![fighter("Front", 0, 8, 5), fighter("Caster", 0, 1, 1)];
        let foes = vec![fighter("Breaker", 5, 8, 5)];
        let mut s = battle_state(heroes, foes, false, 1);
        s.plan.hero_vanguard = vec![true, false]; // Front holds; Caster is the rear
        s.plan.foe_vanguard = vec![true];

        // Pin the foe Vanguard (the round-plan surgery Effect::Pin performs at `do_play_card`).
        s.plan.foe_pinned[0] = true;

        let rear0 = s.heroes[1].defense.health.remaining;
        // Run the round: Standoff → Fray (foe strikes the front, harmless to the rear) → Volley. The
        // foe Volley must skip the pinned Breaker (no charge declared), so the Breach touches no one.
        game.apply(&mut s, &Action::Deploy).unwrap(); // → Fray (foe_fray runs)
        // Close the Fray (front trades; pin survives `fix_breach_list` via the pinned OR).
        while s.phase == Phase::Fray {
            if let Some(&i) = game.pending(&s, 0).first() {
                game.apply(&mut s, &Action::Pass(i)).unwrap();
            } else {
                break;
            }
        }
        assert!(
            s.plan.foe_locked[0],
            "the pinned foe is locked across the Fray boundary (Pin's lock survives fix_breach_list)"
        );
        assert!(
            !s.plan.charges.iter().any(|c| c.side == 1),
            "the pinned foe declared no charge"
        );
        // Carry the Volley through to the Breach.
        while matches!(s.phase, Phase::Volley) {
            if let Some(&i) = game.pending(&s, 0).first() {
                game.apply(&mut s, &Action::Pass(i)).unwrap();
            } else {
                game.apply(&mut s, &Action::Deploy).unwrap();
            }
        }
        assert_eq!(
            s.heroes[1].defense.health.remaining, rear0,
            "the rear is untouched — the pinned charge never crossed (Pin denied it)"
        );
    }

    /// §10 **Rout** (the area-CC rider): a **routed** foe Vanguard is driven off the line and **cannot
    /// charge** — the foe Volley skips it, so the hero rear is spared.
    #[test]
    fn a_routed_foe_cannot_charge() {
        let game = Deckbound;
        let heroes = vec![fighter("Front", 0, 8, 5), fighter("Caster", 0, 1, 1)];
        let foes = vec![fighter("Breaker", 5, 8, 5)];
        let mut s = battle_state(heroes, foes, false, 1);
        s.plan.hero_vanguard = vec![true, false];
        s.plan.foe_vanguard = vec![true];
        s.creatures[0].routed = true; // displaced off the line (a Rout)

        let rear0 = s.heroes[1].defense.health.remaining;
        game.apply(&mut s, &Action::Deploy).unwrap(); // → Fray
        while s.phase == Phase::Fray {
            if let Some(&i) = game.pending(&s, 0).first() {
                game.apply(&mut s, &Action::Pass(i)).unwrap();
            } else {
                break;
            }
        }
        assert!(
            !s.plan.charges.iter().any(|c| c.side == 1),
            "a routed foe declares no charge (it neither holds the front nor crosses)"
        );
        while matches!(s.phase, Phase::Volley) {
            if let Some(&i) = game.pending(&s, 0).first() {
                game.apply(&mut s, &Action::Pass(i)).unwrap();
            } else {
                game.apply(&mut s, &Action::Deploy).unwrap();
            }
        }
        assert_eq!(
            s.heroes[1].defense.health.remaining, rear0,
            "the routed foe could not charge — the hero rear is spared"
        );
    }

    #[test]
    fn every_scenario_terminates() {
        let game = Deckbound;
        for open in [
            Action::OpenCooperation,
            Action::OpenGod,
            Action::OpenTutorial,
            Action::OpenVersus,
        ] {
            let count = match open {
                Action::OpenCooperation => scenarios::campaign().len(),
                Action::OpenGod => scenarios::god().len(),
                Action::OpenVersus => scenarios::versus().len(),
                _ => scenarios::tutorials().len(),
            };
            for i in 0..count {
                let mut s = game.new_game(7 + i as u64, 1);
                launch(&game, &mut s, open, i);
                let _ = autoplay(&game, &mut s);
            }
        }
    }
}
