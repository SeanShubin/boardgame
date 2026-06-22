//! Deckbound as an [`engine::Game`] — the §4 lane commitment system.
//!
//! A scenario is either **base mode** (deterministic: same-range = trade, mismatch = auto-hit,
//! §4.2) run through the lane round (Assemble → Slip → Vanguard resolve → Skirmish → Reserve),
//! or a **Clash-module** 1v1 duel (the optional four-card mix-up, [`crate::duel`]). All numbers
//! live in `data/booklet.ron`.

use engine::{
    Accent, CardView, Game, GameError, Layout, Outcome, PlayerId, Rng, TableView, ZoneView,
};

use crate::actor::{Actor, Range};
use crate::campaign::{Campaign, reference_campaign};
use crate::combat;
use crate::duel::{self, Move, Side};
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
    /// Assemble: put this hero in the Vanguard / Reserve, then deploy.
    SetVanguard(usize),
    SetSkirmisher(usize),
    SetReserve(usize),
    Deploy,
    /// Assign phase: place this Vanguard hero into this lane.
    AssignLane(usize, usize),
    /// Slip phase: this Vanguard hero holds its lane / slips past; resolve the front.
    Hold(usize),
    Slip(usize),
    ResolveFront,
    /// Skirmish/Reserve phase: this actor strikes that enemy, plays a card, or passes.
    Target(usize, usize),
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
        state.phase = Phase::Assemble;
        state.log = vec![scenario.blurb.clone(), "-- Round 1: assemble --".into()];
    }
    state.scenario = Some(scenario);
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
        state.phase = Phase::Assemble;
        state.log = vec!["-- Round 1: assemble --".into()];
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

impl Deckbound {
    // ---- base-mode lane round ------------------------------------------------

    /// Deploy: read the declared ranks (human + AI), resolve **the Line** (§4 Tier 1, static-ranks),
    /// then open the **Open** (Skirmish + Reserve). Skirmishers who **crossed** — plus Vanguards who
    /// **pour through** a wiped enemy front — are flagged as `skirmisher` so the Open lets them strike
    /// the backfield.
    fn deploy(&self, state: &mut State) {
        // PvP: side B's ranks are set by hand; PvE: creatures declare by aggression — the bold charge
        // and flank (Skirmisher), the steady charge and hold (Vanguard), the timid hold back (Reserve).
        if !state.pvp {
            for f in 0..state.creatures.len() {
                let aggr = state.creatures[f]
                    .behavior()
                    .map(|b| b.aggression)
                    .unwrap_or(0);
                let up = !state.creatures[f].is_down();
                let melee = state.creatures[f].can_contest(Range::Melee);
                // The bold charge (aggression); ranged/support hold the Reserve. A charger **flanks** as
                // a Skirmisher only if it is *fast* (high Daring — an Infiltrator); steady melee holds the
                // line as a Vanguard.
                state.plan.foe_charging[f] = up && aggr >= 3 && melee;
                state.plan.foe_flank[f] =
                    up && aggr >= 3 && melee && state.creatures[f].offense.daring >= 3;
            }
        }
        let h_charging = state.plan.hero_charging.clone();
        let h_flank = state.plan.hero_flank.clone();
        let f_charging = state.plan.foe_charging.clone();
        let f_flank = state.plan.foe_flank.clone();
        let mut heroes = std::mem::take(&mut state.heroes);
        let mut creatures = std::mem::take(&mut state.creatures);
        let (mut h_skirm, mut f_skirm) = combat::the_line(
            &mut heroes,
            &h_charging,
            &h_flank,
            &mut creatures,
            &f_charging,
            &f_flank,
            &mut state.log,
        );
        // Pour-through: once a side has no living Vanguard, the enemy Vanguards break through to the
        // backfield in the Open — flagged as skirmishers so they may strike anything.
        let f_van_alive =
            (0..creatures.len()).any(|i| f_charging[i] && !f_flank[i] && !creatures[i].is_down());
        if !f_van_alive {
            for i in 0..heroes.len() {
                if h_charging[i] && !h_flank[i] && !heroes[i].is_down() {
                    h_skirm[i] = true;
                }
            }
        }
        let h_van_alive =
            (0..heroes.len()).any(|i| h_charging[i] && !h_flank[i] && !heroes[i].is_down());
        if !h_van_alive {
            for i in 0..creatures.len() {
                if f_charging[i] && !f_flank[i] && !creatures[i].is_down() {
                    f_skirm[i] = true;
                }
            }
        }
        state.heroes = heroes;
        state.creatures = creatures;
        state.plan.hero_skirmisher = h_skirm;
        state.plan.foe_skirmisher = f_skirm;
        combat::tally(&mut state.heroes, &mut state.log);
        combat::tally(&mut state.creatures, &mut state.log);
        check_outcome(state);
        if state.outcome.is_some() {
            return;
        }
        self.begin_skirmish(state);
    }

    fn begin_skirmish(&self, state: &mut State) {
        state.plan.committing = 0;
        state.plan.hero_acted.iter_mut().for_each(|v| *v = false);
        state.plan.foe_acted.iter_mut().for_each(|v| *v = false);
        state.phase = Phase::Skirmish;
        if self.pending_targets(state, 0, false).is_empty() {
            self.skirmish_done(state);
        }
    }

    /// Actors of `side` that must still act this target phase. `reserve = false` → Skirmishers
    /// (slipped a lane); `reserve = true` → Reserves (not in a lane, not a Skirmisher).
    fn pending_targets(&self, state: &State, side: u8, reserve: bool) -> Vec<usize> {
        (0..state.s_len(side))
            .filter(|&i| {
                let a = &state.s_pool(side)[i];
                let role_ok = if reserve {
                    !state.s_charging(side)[i]
                } else {
                    state.s_skirm(side)[i]
                };
                // A Staggered actor loses its action this round (§4 Controller debuff) — skip it.
                role_ok && !a.fallen && !a.is_down() && !a.stunned && !state.s_acted(side)[i]
            })
            .collect()
    }

    /// The committing side's Skirmishers are done. PvP: hand to side B, then advance; PvE: run
    /// the creature-AI Skirmishers, then the Reserve phase.
    fn skirmish_done(&self, state: &mut State) {
        if state.pvp {
            if state.plan.committing == 0 {
                state.plan.committing = 1;
                if self.pending_targets(state, 1, false).is_empty() {
                    self.skirmish_done(state);
                }
            } else {
                self.begin_reserve(state);
            }
            return;
        }
        for f in 0..state.creatures.len() {
            let attacks = state.plan.foe_skirmisher[f] && !state.creatures[f].is_down();
            if !attacks {
                continue;
            }
            if let Some(t) = self.foe_pick(state, f) {
                self.strike(state, false, f, t, Range::Melee);
            }
        }
        combat::tally(&mut state.heroes, &mut state.log);
        combat::tally(&mut state.creatures, &mut state.log);
        check_outcome(state);
        if state.outcome.is_some() {
            return;
        }
        self.begin_reserve(state);
    }

    fn begin_reserve(&self, state: &mut State) {
        state.plan.committing = 0;
        state.plan.hero_acted.iter_mut().for_each(|v| *v = false);
        state.plan.foe_acted.iter_mut().for_each(|v| *v = false);
        state.phase = Phase::Reserve;
        if self.pending_targets(state, 0, true).is_empty() {
            self.reserve_done(state);
        }
    }

    /// The committing side's Reserves are done. PvP: hand to side B, then next round; PvE: run
    /// the creature-AI Reserves (ranged fire), then refresh.
    fn reserve_done(&self, state: &mut State) {
        if state.pvp {
            if state.plan.committing == 0 {
                state.plan.committing = 1;
                if self.pending_targets(state, 1, true).is_empty() {
                    self.reserve_done(state);
                }
            } else {
                self.next_round(state);
            }
            return;
        }
        for f in 0..state.creatures.len() {
            let reserve = !state.plan.foe_charging[f];
            let fires = reserve
                && !state.creatures[f].is_down()
                && state.creatures[f].can_contest(Range::Ranged);
            if !fires {
                continue;
            }
            if let Some(t) = self.foe_pick(state, f) {
                self.strike(state, false, f, t, Range::Ranged);
            }
        }
        combat::tally(&mut state.heroes, &mut state.log);
        combat::tally(&mut state.creatures, &mut state.log);
        check_outcome(state);
        if state.outcome.is_some() {
            return;
        }
        self.next_round(state);
    }

    fn next_round(&self, state: &mut State) {
        // Round cap (§0 Ruleset): a fight not closed within `max_rounds` is a **draw** (PvE: a draw is
        // no different from a loss). Live play caps high (the historical backstop); analysis bounds it
        // low (e.g. 5) so the combat tree is finite and the win/lose question is horizon-terminal.
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
        state.phase = Phase::Assemble;
        state
            .log
            .push(format!("-- Round {}: assemble --", state.round));
    }

    /// One actor strikes a target at `range` — a trade if the target can contest, else an
    /// auto-hit (§4.2). `hero_attacker` selects which pool the attacker is in.
    fn strike(
        &self,
        state: &mut State,
        hero_attacker: bool,
        attacker: usize,
        target: usize,
        range: Range,
    ) {
        let (atk_snap, atk_name, atk_can) = if hero_attacker {
            (
                combat::snapshot(&state.heroes[attacker]),
                state.heroes[attacker].name.clone(),
                state.heroes[attacker].can_contest(range),
            )
        } else {
            (
                combat::snapshot(&state.creatures[attacker]),
                state.creatures[attacker].name.clone(),
                state.creatures[attacker].can_contest(range),
            )
        };
        if !atk_can {
            return; // can't attack at this range
        }
        // Striking is an action: it spends one of the attacker's **Tempo** cards (§3). Pay-after —
        // the strike always lands even if it drives Tempo negative — so a phase never stalemates;
        // the bite is on *defense* (a depleted unit can't strike back).
        if hero_attacker {
            state.heroes[attacker].tempo -= 1;
        } else {
            state.creatures[attacker].tempo -= 1;
        }
        // attacker -> target. The target may **reflexively strike back** (§3) — but striking is an
        // action, so it must spend a **Tempo** card; with none to flip it just takes the hit (a free
        // hit). Ranged is one-sided: only a contesting *melee/both* defender answers (`can_contest`).
        if hero_attacker {
            combat::apply_strike(
                &mut state.creatures[target],
                atk_snap,
                &atk_name,
                &mut state.log,
            );
            if state.creatures[target].can_contest_now(range)
                && !state.creatures[target].is_down()
                && state.creatures[target].tempo > 0
            {
                state.creatures[target].tempo -= 1; // strike-back spends a Tempo card
                let back = combat::snapshot(&state.creatures[target]);
                let name = state.creatures[target].name.clone();
                combat::apply_strike(&mut state.heroes[attacker], back, &name, &mut state.log);
            }
        } else {
            combat::apply_strike(
                &mut state.heroes[target],
                atk_snap,
                &atk_name,
                &mut state.log,
            );
            if state.heroes[target].can_contest_now(range)
                && !state.heroes[target].is_down()
                && state.heroes[target].tempo > 0
            {
                state.heroes[target].tempo -= 1; // strike-back spends a Tempo card
                let back = combat::snapshot(&state.heroes[target]);
                let name = state.heroes[target].name.clone();
                combat::apply_strike(&mut state.creatures[attacker], back, &name, &mut state.log);
            }
        }
    }

    /// Is actor `i` of `side` currently a Reserve (it did not charge)?
    fn is_reserve(&self, state: &State, side: u8, i: usize) -> bool {
        !state.s_charging(side)[i]
    }

    /// Actor `i`'s current §4 position (0 = Vanguard, 1 = Skirmisher, 2 = Reserve). A charger that
    /// broke through is a Skirmisher; a charger that stopped is a Vanguard; a non-charger is Reserve.
    fn position_of(&self, state: &State, side: u8, i: usize) -> u8 {
        if state.s_skirm(side)[i] {
            1
        } else if state.s_charging(side)[i] {
            0
        } else {
            2
        }
    }

    /// §4.4 — may actor `i` of `side` play this role `card` right now? Enforces the **per-role
    /// per-round cap** (one card of each role track per round) and **positional coherence**
    /// (a positional Wall / Infiltrator / Artillery card requires its §4 position). Non-role cards
    /// (the pre-built scenario kits, `role: None`) are unaffected — only `passive` is excluded.
    fn role_card_playable(
        &self,
        state: &State,
        side: u8,
        i: usize,
        card: &crate::cards::Card,
    ) -> bool {
        use crate::currency::Currency;
        if card.passive {
            return false;
        }
        // A Disarmed actor cannot play its role cards this round (§4 Controller debuff).
        if state.s_pool(side)[i].disarmed {
            return false;
        }
        if let Some(track) = card.role {
            let played = if side == 0 {
                &state.plan.hero_roles_played
            } else {
                &state.plan.foe_roles_played
            };
            if played[i].contains(&track) {
                return false; // already played a card of this role this round
            }
        }
        if card.positional {
            let need = match card.role {
                Some(Currency::Iron) => 0u8,   // Wall → Vanguard
                Some(Currency::Silver) => 1u8, // Infiltrator → Skirmisher
                Some(Currency::Brass) => 2u8,  // Artillery → Reserve
                _ => return true,              // positional flag without a positional role
            };
            return self.position_of(state, side, i) == need;
        }
        true
    }

    /// Record that actor `i` of `side` played a card of `card`'s role this round (the §4.4 cap).
    fn note_role_played(&self, state: &mut State, side: u8, i: usize, card: &crate::cards::Card) {
        if let Some(track) = card.role {
            let played = if side == 0 {
                &mut state.plan.hero_roles_played
            } else {
                &mut state.plan.foe_roles_played
            };
            if !played[i].contains(&track) {
                played[i].push(track);
            }
        }
    }

    /// §4 **Muster** — may actor `i` of `side` play this role `card` *now*, during the pre-gauntlet
    /// Muster window? Muster is for cards whose effect **persists through the round**: a charging
    /// Wall's standing defenses (Brace / Rally / Last Stand) and the position-agnostic Controller
    /// (Bone) / Support (Salt) buffs & debuffs — played before the Gauntlet so they bite it. The
    /// positional attack cards (Infiltrator slips, Artillery fire) are **not** mustered; they wait for
    /// their own phase (they need post-gauntlet targets). Respects the per-role cap and Disarm.
    fn muster_card_playable(
        &self,
        state: &State,
        side: u8,
        i: usize,
        card: &crate::cards::Card,
    ) -> bool {
        use crate::currency::Currency;
        if card.passive {
            return false;
        }
        if state.s_pool(side)[i].disarmed {
            return false;
        }
        let Some(track) = card.role else {
            return false; // only role cards muster (weapons / scenario kits do not)
        };
        let played = if side == 0 {
            &state.plan.hero_roles_played
        } else {
            &state.plan.foe_roles_played
        };
        if played[i].contains(&track) {
            return false;
        }
        if card.positional {
            // Only a Wall's Vanguard standing cards muster, and only if it is charging (it intends to
            // hold the front). Infiltrator / Artillery positional cards act in their own phase.
            matches!(track, Currency::Iron) && state.s_charging(side)[i]
        } else {
            true // Controller / Support effect cards persist the round — muster them
        }
    }

    /// The enemies a Reserve of `side` may target (§4 matrix): the enemy front (Vanguard +
    /// Skirmishers). **Longshot** (or an empty front) extends the reach to enemy Reserves.
    fn reserve_targets(&self, state: &State, side: u8, actor: usize) -> Vec<usize> {
        let other = 1 - side;
        let front: Vec<usize> = combat::living(state.s_pool(other))
            .into_iter()
            .filter(|&t| !self.is_reserve(state, other, t))
            .collect();
        if state.s_pool(side)[actor].has("Longshot") || front.is_empty() {
            combat::living(state.s_pool(other))
        } else {
            front
        }
    }

    /// A creature picks a living hero target by its rule.
    fn foe_pick(&self, state: &State, _foe: usize) -> Option<usize> {
        let rule = state.creatures[_foe]
            .behavior()
            .map(|b| b.target_rule)
            .unwrap_or(crate::actor::TargetRule::Front);
        let cands = combat::living(&state.heroes);
        combat::pick_target(&state.heroes, &cands, rule)
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

        let (hp, hd, hpre) = combat::base_strike(&state.heroes[c.hero]);
        let (cp, cd, cpre) = combat::base_strike(&state.creatures[c.foe]);
        let hn = state.heroes[c.hero].name.clone();
        let cn = state.creatures[c.foe].name.clone();
        let a = Side {
            power: hp,
            dtype: hd,
            precision: hpre,
            force: c.hero_force,
            name: &hn,
        };
        let b = Side {
            power: cp,
            dtype: cd,
            precision: cpre,
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
            (None, Phase::Assemble) => format!(
                "Round {} — charge: pick who runs the gauntlet, then Deploy. (Esc: menu)",
                state.round
            ),
            (None, Phase::Skirmish) => "Skirmishers pick targets. (Esc: menu)".to_string(),
            (None, Phase::Reserve) => "Reserve: fire or aid. (Esc: menu)".to_string(),
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
            && matches!(
                state.phase,
                Phase::Assemble | Phase::Skirmish | Phase::Reserve
            )
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
            // Charge selection + the Muster window: each hero charges or holds back (Reserve), and may
            // play standing/persistent cards (Wall defenses, Controller debuffs, Support buffs) before
            // Deploy resolves the gauntlet (§4).
            Phase::Assemble => {
                let side = state.plan.committing;
                let mut a = Vec::new();
                for i in 0..state.s_len(side) {
                    if state.s_pool(side)[i].fallen {
                        continue;
                    }
                    // Offer the ranks this unit is not already in (Skirmisher needs a melee attack).
                    let charging = state.s_charging(side)[i];
                    let flank = state.s_flank(side)[i];
                    let melee = state.s_pool(side)[i].can_contest(Range::Melee);
                    if !charging || flank {
                        a.push(Action::SetVanguard(i));
                    }
                    if melee && (!charging || !flank) {
                        a.push(Action::SetSkirmisher(i));
                    }
                    if charging {
                        a.push(Action::SetReserve(i));
                    }
                    for idx in 0..state.s_pool(side)[i].actions.len() {
                        if self.muster_card_playable(
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
            Phase::Skirmish => {
                let side = state.plan.committing;
                let other = 1 - side;
                let mut a = Vec::new();
                if let Some(&i) = self.pending_targets(state, side, false).first() {
                    for t in combat::living(state.s_pool(other)) {
                        a.push(Action::Target(i, t));
                    }
                    // A Skirmisher may also play its role cards (Infiltrator / effect cards, §4.4).
                    for idx in 0..state.s_pool(side)[i].actions.len() {
                        if self.role_card_playable(
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
            Phase::Reserve => {
                let side = state.plan.committing;
                let mut a = Vec::new();
                if let Some(&i) = self.pending_targets(state, side, true).first() {
                    if state.s_pool(side)[i].can_contest(Range::Ranged) {
                        for t in self.reserve_targets(state, side, i) {
                            a.push(Action::Target(i, t));
                        }
                    }
                    for idx in 0..state.s_pool(side)[i].actions.len() {
                        if self.role_card_playable(
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
            Action::SetVanguard(h) => format!("Send {} to the Vanguard", hname(*h)),
            Action::SetSkirmisher(h) => format!("Send {} to flank as a Skirmisher", hname(*h)),
            Action::SetReserve(h) => format!("Pull {} back to the Reserve", hname(*h)),
            Action::Deploy => "Deploy — start the round".into(),
            Action::AssignLane(h, lane) => format!("Place {} in lane {}", hname(*h), lane + 1),
            Action::Hold(h) => format!("{}: hold the lane", hname(*h)),
            Action::Slip(h) => format!("{}: slip past", hname(*h)),
            Action::ResolveFront => "Resolve the front".into(),
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

            (Phase::Assemble, Action::SetVanguard(i)) => {
                let side = state.plan.committing;
                state.s_charging_mut(side)[*i] = true;
                state.s_flank_mut(side)[*i] = false;
            }
            (Phase::Assemble, Action::SetSkirmisher(i)) => {
                let side = state.plan.committing;
                state.s_charging_mut(side)[*i] = true;
                state.s_flank_mut(side)[*i] = true;
            }
            (Phase::Assemble, Action::SetReserve(i)) => {
                let side = state.plan.committing;
                state.s_charging_mut(side)[*i] = false;
                state.s_flank_mut(side)[*i] = false;
            }
            (Phase::Assemble, Action::Deploy) => {
                if state.pvp && state.plan.committing == 0 {
                    state.plan.committing = 1;
                    state.log.push("-- side B: assemble --".into());
                } else {
                    self.deploy(state);
                }
            }
            (Phase::Assemble, Action::PlayCard(i, idx)) => {
                // The Muster window (§4): play a standing/persistent card before the Gauntlet. Its
                // effect (a Wall's Brace/Last Stand, a Controller's debuff, a Support's buff) holds
                // through the round and shapes the gauntlet that follows.
                let side = state.plan.committing;
                let card = state.s_pool(side)[*i]
                    .actions
                    .get(*idx)
                    .cloned()
                    .ok_or_else(|| GameError::new("no such card"))?;
                if !self.muster_card_playable(state, side, *i, &card) {
                    return Err(GameError::new("that card can't be mustered now"));
                }
                self.note_role_played(state, side, *i, &card);
                let off = state.s_pool(side)[*i].offense;
                let name = state.s_pool(side)[*i].name.clone();
                if side == 0 {
                    let mut allies = std::mem::take(&mut state.heroes);
                    combat::play_card(
                        &card,
                        &name,
                        off,
                        &mut state.creatures,
                        &mut allies,
                        Some(*i),
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
                        Some(*i),
                        &mut state.log,
                    );
                    state.creatures = allies;
                }
                combat::tally(&mut state.heroes, &mut state.log);
                combat::tally(&mut state.creatures, &mut state.log);
                check_outcome(state);
            }

            (Phase::Skirmish, Action::Target(i, t)) => {
                let side = state.plan.committing;
                let other = 1 - side;
                // Backstab: a Skirmisher hits an enemy Reserve harder.
                let backstab =
                    state.s_pool(side)[*i].has("Backstab") && self.is_reserve(state, other, *t);
                if backstab {
                    if side == 0 {
                        state.heroes[*i].offense.power += 3;
                    } else {
                        state.creatures[*i].offense.power += 3;
                    }
                }
                // Assassinate (M4): a killing strike — when an Infiltrator with the capstone hits an
                // enemy Reserve, that foe is downed outright (the §10 execute).
                let execute =
                    state.s_pool(side)[*i].has("Assassinate") && self.is_reserve(state, other, *t);
                self.strike(state, side == 0, *i, *t, Range::Melee);
                if backstab {
                    if side == 0 {
                        state.heroes[*i].offense.power -= 3;
                    } else {
                        state.creatures[*i].offense.power -= 3;
                    }
                }
                if execute {
                    let victim = if side == 0 {
                        &mut state.creatures[*t]
                    } else {
                        &mut state.heroes[*t]
                    };
                    if !victim.is_down() {
                        victim.defense.body.remaining = 0;
                        let vname = victim.name.clone();
                        state.log.push(format!("{vname} is marked and executed!"));
                    }
                }
                // Act while you have Tempo (§4 the Open): a crossed Skirmisher keeps striking the
                // backfield until its cards run out.
                if state.s_pool(side)[*i].tempo <= 0 {
                    state.s_acted_mut(side)[*i] = true;
                }
                combat::tally(&mut state.heroes, &mut state.log);
                combat::tally(&mut state.creatures, &mut state.log);
                check_outcome(state);
                if state.outcome.is_none() && self.pending_targets(state, side, false).is_empty() {
                    self.skirmish_done(state);
                }
            }
            (Phase::Skirmish, Action::PlayCard(i, idx)) => {
                let side = state.plan.committing;
                let card = state.s_pool(side)[*i]
                    .actions
                    .get(*idx)
                    .cloned()
                    .ok_or_else(|| GameError::new("no such card"))?;
                if !self.role_card_playable(state, side, *i, &card) {
                    return Err(GameError::new("that card can't be played from here now"));
                }
                self.note_role_played(state, side, *i, &card);
                let off = state.s_pool(side)[*i].offense;
                let name = state.s_pool(side)[*i].name.clone();
                if side == 0 {
                    let mut allies = std::mem::take(&mut state.heroes);
                    combat::play_card(
                        &card,
                        &name,
                        off,
                        &mut state.creatures,
                        &mut allies,
                        Some(*i),
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
                        Some(*i),
                        &mut state.log,
                    );
                    state.creatures = allies;
                }
                state.s_acted_mut(side)[*i] = true;
                combat::tally(&mut state.heroes, &mut state.log);
                combat::tally(&mut state.creatures, &mut state.log);
                check_outcome(state);
                if state.outcome.is_none() && self.pending_targets(state, side, false).is_empty() {
                    self.skirmish_done(state);
                }
            }
            (Phase::Skirmish, Action::Pass(i)) => {
                let side = state.plan.committing;
                state.s_acted_mut(side)[*i] = true;
                if self.pending_targets(state, side, false).is_empty() {
                    self.skirmish_done(state);
                }
            }
            (Phase::Reserve, Action::Target(i, t)) => {
                let side = state.plan.committing;
                self.strike(state, side == 0, *i, *t, Range::Ranged);
                state.s_acted_mut(side)[*i] = true;
                combat::tally(&mut state.heroes, &mut state.log);
                combat::tally(&mut state.creatures, &mut state.log);
                check_outcome(state);
                if state.outcome.is_none() && self.pending_targets(state, side, true).is_empty() {
                    self.reserve_done(state);
                }
            }
            (Phase::Reserve, Action::PlayCard(i, idx)) => {
                let side = state.plan.committing;
                let card = state.s_pool(side)[*i]
                    .actions
                    .get(*idx)
                    .cloned()
                    .ok_or_else(|| GameError::new("no such card"))?;
                if !self.role_card_playable(state, side, *i, &card) {
                    return Err(GameError::new("that card can't be played from here now"));
                }
                self.note_role_played(state, side, *i, &card);
                let off = state.s_pool(side)[*i].offense;
                let name = state.s_pool(side)[*i].name.clone();
                if side == 0 {
                    let mut allies = std::mem::take(&mut state.heroes);
                    combat::play_card(
                        &card,
                        &name,
                        off,
                        &mut state.creatures,
                        &mut allies,
                        Some(*i),
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
                        Some(*i),
                        &mut state.log,
                    );
                    state.creatures = allies;
                }
                state.s_acted_mut(side)[*i] = true;
                combat::tally(&mut state.heroes, &mut state.log);
                combat::tally(&mut state.creatures, &mut state.log);
                check_outcome(state);
                if state.outcome.is_none() && self.pending_targets(state, side, true).is_empty() {
                    self.reserve_done(state);
                }
            }
            (Phase::Reserve, Action::Pass(i)) => {
                let side = state.plan.committing;
                state.s_acted_mut(side)[*i] = true;
                if self.pending_targets(state, side, true).is_empty() {
                    self.reserve_done(state);
                }
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
            // Charge selection reads as **card placement**: the enemy on top, then your Charge and
            // Reserve zones, each character card clickable to move it between them (§4). The action
            // rides on the card itself, so dropping the card in a zone *is* the choice.
            Phase::Assemble => {
                let side = state.plan.committing;
                zones.push(if side == 0 {
                    creature_zone(state, None)
                } else {
                    hero_zone(state, None)
                });
                let acts = self.legal_actions(state);
                let idx_of = |want: &Action| acts.iter().position(|a| a == want);
                let mut charging = Vec::new();
                let mut reserve = Vec::new();
                for i in 0..state.s_len(side) {
                    let actor = &state.s_pool(side)[i];
                    if actor.fallen {
                        continue;
                    }
                    let is_charging = state.s_charging(side)[i];
                    // Clicking a card sends it to the *other* zone (the charge toggle as placement).
                    let toggle = if is_charging {
                        Action::SetReserve(i)
                    } else {
                        Action::SetVanguard(i)
                    };
                    let mut card = actor_card(actor, Accent::Ally);
                    if let Some(idx) = idx_of(&toggle) {
                        card = card.action(idx);
                    }
                    if is_charging {
                        charging.push(card);
                    } else {
                        reserve.push(card);
                    }
                }
                zones.push(ZoneView {
                    label: "Charge — run the gauntlet".into(),
                    layout: Layout::Row,
                    owner: None,
                    cards: charging,
                });
                zones.push(ZoneView {
                    label: "Reserve — hold back, fire from the rear".into(),
                    layout: Layout::Row,
                    owner: None,
                    cards: reserve,
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
            format!("HP [{}]", pips(d.body.remaining, d.body.max)),
            format!(
                "Spd {} Drv {} Pow {} {}",
                a.offense.speed,
                a.offense.daring.max(1),
                a.offense.power,
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

/// The role triangle (Vanguard ▸ Skirmisher ▸ Reserve ▸ Vanguard) as a small chart.
fn append_triangle_chart(prose: &mut Vec<engine::ProseLine>) {
    prose.push(engine::ProseLine::Gap);
    prose.push(engine::ProseLine::Heading("The triangle".into()));
    for line in [
        "Vanguard ▸ beats Skirmisher (holds the wall, strikes first)",
        "Skirmisher ▸ beats Reserve (slips in to assassinate)",
        "Reserve ▸ beats Vanguard (fires from safety, untouchable in melee)",
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
                Phase::Assemble => Action::Deploy,
                Phase::Skirmish | Phase::Reserve => acts
                    .iter()
                    .find(|a| matches!(a, Action::Target(..)))
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

    fn duel_state() -> State {
        let game = Deckbound;
        let mut s = game.new_game(1, 1);
        game.apply(&mut s, &Action::OpenTutorial).unwrap();
        game.apply(&mut s, &Action::PickScenario(0)).unwrap(); // "1. The Trade" (base mode)
        s
    }

    /// §4.2: a same-range strike is a trade (both take damage); a range mismatch is an auto-hit
    /// (the target takes it and cannot answer).
    #[test]
    fn same_range_trades_mismatch_auto_hits() {
        let game = Deckbound;

        // Mismatch: ranged attacker vs a melee-only target → auto-hit, no trade-back.
        let mut s = duel_state();
        s.heroes[0].attack = crate::actor::Attack::Ranged;
        s.creatures[0].attack = crate::actor::Attack::Melee;
        let h0 = s.heroes[0].defense.body.remaining;
        let f0 = s.creatures[0].defense.body.remaining;
        game.strike(&mut s, true, 0, 0, Range::Ranged);
        assert!(
            s.creatures[0].defense.body.remaining < f0,
            "the foe is auto-hit"
        );
        assert_eq!(
            s.heroes[0].defense.body.remaining, h0,
            "no trade-back on a mismatch"
        );

        // Same range: both contest → a trade, both take damage.
        let mut s2 = duel_state();
        s2.heroes[0].attack = crate::actor::Attack::Melee;
        s2.creatures[0].attack = crate::actor::Attack::Melee;
        s2.creatures[0].defense.body.remaining = 12;
        s2.creatures[0].defense.body.max = 12;
        let h = s2.heroes[0].defense.body.remaining;
        let f = s2.creatures[0].defense.body.remaining;
        game.strike(&mut s2, true, 0, 0, Range::Melee);
        assert!(
            s2.creatures[0].defense.body.remaining < f && s2.heroes[0].defense.body.remaining < h,
            "same-range melee is a trade — both are hit"
        );
    }

    /// §4 Muster: a charging Wall may play a standing card (Brace) before the gauntlet, and only
    /// while charging — its effect (here, the Guard Tempo boost) persists into the round.
    #[test]
    fn muster_window_offers_a_charging_walls_standing_card() {
        use crate::currency::Currency;
        let game = Deckbound;
        let wall = scenarios::build_character("Novice", &scenarios::rewards_for(Currency::Iron));
        let foe = scenarios::build_creature("Husk");
        let mut s = battle_state(vec![wall], vec![foe], false, 1);
        assert_eq!(s.phase, Phase::Assemble);
        let brace = s.heroes[0]
            .actions
            .iter()
            .position(|c| c.name == "Brace")
            .expect("the Iron kit owns Brace");

        // Not charging yet → a positional Wall standing card is not yet musterable.
        assert!(
            !game.legal_actions(&s).contains(&Action::PlayCard(0, brace)),
            "Brace is offered only once the Wall is charging"
        );

        // Charge the Wall, then the Muster window offers Brace.
        game.apply(&mut s, &Action::SetVanguard(0)).unwrap();
        assert!(
            game.legal_actions(&s).contains(&Action::PlayCard(0, brace)),
            "a charging Wall may muster Brace"
        );
        let tempo_before = s.heroes[0].tempo;
        game.apply(&mut s, &Action::PlayCard(0, brace)).unwrap();
        assert_eq!(
            s.phase,
            Phase::Assemble,
            "mustering stays in the Assemble window"
        );
        assert!(
            s.heroes[0].tempo > tempo_before,
            "Brace's Guard Tempo persists into the gauntlet"
        );
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
        assert_eq!(s.phase, Phase::Assemble);
        assert_eq!(
            game.current_player(&s),
            Some(PlayerId(0)),
            "side A assembles first"
        );
        game.apply(&mut s, &Action::Deploy).unwrap();
        assert_eq!(s.phase, Phase::Assemble, "still assembling");
        assert_eq!(
            game.current_player(&s),
            Some(PlayerId(1)),
            "now side B assembles"
        );
        game.apply(&mut s, &Action::Deploy).unwrap();
        // Both assembled (no Vanguard) → deploys; play it out to an outcome.
        let _ = autoplay(&game, &mut s);
        assert!(s.outcome.is_some());
    }

    /// A base-mode cooperation scenario runs the gauntlet round to an outcome.
    #[test]
    fn base_scenario_runs_lanes() {
        let game = Deckbound;
        let mut s = game.new_game(2, 1);
        game.apply(&mut s, &Action::OpenCooperation).unwrap();
        game.apply(&mut s, &Action::PickScenario(0)).unwrap();
        assert_eq!(s.phase, Phase::Assemble);
        let _ = autoplay(&game, &mut s);
        assert!(s.outcome.is_some());
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
