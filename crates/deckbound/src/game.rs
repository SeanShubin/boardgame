//! Deckbound as a [`contract::Game`] — the §4 sub-phase-schedule battle.
//!
//! A scenario is either **base mode** (deterministic: same-range = trade, mismatch = auto-hit,
//! §4.2) run through the round (Marshal → Reveal → Ready → Engage → Refresh, §4 / §4.6), or a
//! **Clash-module** 1v1 duel (the optional four-card mix-up, [`crate::duel`]). All
//! numbers live in `data/booklet.ron`.

use contract::{Accent, CardView, Game, GameError, Layout, Outcome, PlayerId, TableView, ZoneView};
use engine::Rng;

use crate::actor::{Actor, Intention, Range};
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
    /// Marshal (§4): set this unit's **intention** for the round — Vanguard (hold the front),
    /// Outrider (break the line), or Rearguard (deal from the back).
    SetVanguard(usize),
    SetOutrider(usize),
    SetRearguard(usize),
    /// Advance: finish declaring → resolve the round's sub-phase schedule (§4.6).
    Deploy,
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
        resolution: None,
        pending: Vec::new(),
        cycle_work: None,
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
fn entries_in(category: &str) -> Vec<contract::RefEntry> {
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
        state.phase = Phase::Marshal;
        default_formation(state);
        state.log = vec![scenario.blurb.clone(), "-- Round 1: Marshal --".into()];
    }
    state.scenario = Some(scenario);
}

/// §4 — seed each unit's default **intention**. A unit with an **authored preferred** position (a
/// creature behavior, `Actor::preferred`) holds it; otherwise the position is derived from its stats (the
/// policy of `sub_phase::intention_for`): a ranged unit deals from the Rearguard; an **aggressive** melee
/// unit (Might ≥ Toughness — a glassy striker) breaks the line as an Outrider; a **durable** melee unit
/// (Toughness > Might — a wall) holds as a Vanguard. Marshal lets the human (or the AI) override this per
/// unit each round. (Might-vs-Toughness, not Finesse — the re-tuned tank carries C2F2 to run down the
/// Outrider, so Finesse no longer separates the wall from the skirmisher; §4 amendment, `5e396fc`.)
fn default_intentions(state: &mut State) {
    for side in 0u8..2 {
        let n = state.s_len(side);
        for i in 0..n {
            let a = &state.s_pool(side)[i];
            let intent = a.preferred.unwrap_or_else(|| {
                let ranged = a.can_contest(Range::Ranged) && !a.can_contest(Range::Melee);
                if ranged {
                    Intention::Rearguard
                } else if a.eff_might() >= a.eff_toughness() {
                    Intention::Outrider
                } else {
                    Intention::Vanguard
                }
            });
            state.s_intent_mut(side)[i] = intent;
        }
    }
}

/// §4 / §4.5 — the round's **default formation**: the stat-/behavior-derived intentions
/// ([`default_intentions`]) **plus** the group layout derived from each body's **pack tag**
/// (`Actor::pack`) so a **Hoard**'s one-Health bodies re-bind into one swarm every round (the round reset
/// rebuilds the plan to singletons; this re-imposes the formation). Same-side bodies sharing a tag
/// collapse to their first member's index; an untagged body is its own singleton. Player-side grouping is
/// deferred (§4.5 integration status), so heroes stay singletons; only creatures form packs today.
fn default_formation(state: &mut State) {
    default_intentions(state);
    state.plan.hero_group = pack_groups(&state.heroes);
    state.plan.foe_group = pack_groups(&state.creatures);
}

/// The per-unit group ids for one side, from the bodies' [`Actor::pack`] tags: an untagged body is its own
/// singleton (its index); tagged bodies collapse to the **first** index carrying that tag (so `group_of`,
/// which keys off equal ids, binds them). Mirrors `sub_phase::group_ids`.
fn pack_groups(pool: &[Actor]) -> Vec<usize> {
    pool.iter()
        .enumerate()
        .map(|(i, a)| match a.pack {
            None => i,
            Some(tag) => pool.iter().position(|b| b.pack == Some(tag)).unwrap_or(i),
        })
        .collect()
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
        state.phase = Phase::Marshal;
        default_formation(&mut state);
        state.log = vec!["-- Round 1: Marshal --".into()];
    }
    state
}

/// Whether the heroes (side 0) won this resolved battle — `false` for a loss or a draw. A convenience for
/// callers (e.g. the card-table bridge) that only need the win/lose bit off a finished [`State`].
pub fn hero_won(state: &State) -> bool {
    matches!(state.outcome, Some(Outcome::Win(PlayerId(0))))
}

pub(crate) fn check_outcome(state: &mut State) {
    // A win requires **survivors**: you do not win by dying. A **simultaneous wipe** — both sides
    // emptied in the same resolution step (§1.9: deaths finalize together at a sub-phase boundary) — is a
    // **draw**, not a hero victory. So test the mutual case first; only a side that stands while the other
    // is gone wins.
    let foes_down = state.living_creatures() == 0;
    let party_down = state.living_heroes() == 0;
    if foes_down && party_down {
        state.outcome = Some(Outcome::Tie(vec![PlayerId(0), PlayerId(1)]));
        state
            .log
            .push("Both sides annihilate each other — a draw.".into());
    } else if foes_down {
        state.outcome = Some(Outcome::Win(PlayerId(0)));
        state.log.push("Every foe is down — victory!".into());
    } else if party_down {
        state.outcome = Some(Outcome::Win(PlayerId(1)));
        state.log.push("The party has fallen.".into());
    }
}

impl Deckbound {
    // ---- §4 sub-phase-schedule round -----------------------------------------

    /// Units of `side` that may still **declare an intention** this round (alive, not staggered, not yet
    /// acted). The only interactive combat choice is the declaration; the schedule then resolves (§4.6).
    fn pending(&self, state: &State, side: u8) -> Vec<usize> {
        (0..state.s_len(side))
            .filter(|&i| {
                let a = &state.s_pool(side)[i];
                !a.fallen && !a.is_down() && !a.stunned && !state.s_acted(side)[i]
            })
            .collect()
    }

    /// All declarations are in → resolve the round over the **sub-phase schedule** (§4.6), finalize the
    /// outcome, then advance to the next round's Marshal (via Refresh, the Lull). The foe keeps its
    /// stat-defaulted intentions in PvE.
    fn resolve_and_advance(&self, state: &mut State) {
        state.log.push("-- Engage --".into());
        combat::resolve_round(state);
        check_outcome(state);
        if state.outcome.is_some() {
            return;
        }
        self.next_round(state);
    }

    /// Resolve a **full battle** resumably — the same round loop as the auto path
    /// ([`resolve_and_advance`](Self::resolve_and_advance) + [`next_round`](Self::next_round)), but the
    /// Engage resolution rests at each decision so an external driver can observe and answer it. `on_step`
    /// is called after **every** [`step_manual`](combat::step_manual) transition: on
    /// [`Resting`](combat::StepOutcome::Resting) it must fill [`State::pending`] (a player, or
    /// [`answer_pending_greedily`](combat::answer_pending_greedily)); on `Advanced`/`Done` it may diff the
    /// state to animate what changed. With a greedy answerer this reproduces
    /// [`auto_resolve`](crate::solver::auto_resolve) exactly for a card-free `DuelUnit` battle — Marshal is
    /// then a silent Deploy, so driving Engage directly loses nothing. (Synchronous: for a live UI that
    /// answers across frames, pump [`step_manual`](combat::step_manual) directly instead.)
    pub fn resolve_battle_manual(
        &self,
        state: &mut State,
        mut on_step: impl FnMut(&mut State, combat::StepOutcome),
    ) {
        while state.outcome.is_none() {
            state.log.push("-- Engage --".into());
            combat::log_round_intro(state);
            state.resolution = Some(combat::Resolution::start());
            loop {
                let outcome = combat::step_manual(state);
                on_step(state, outcome);
                if matches!(outcome, combat::StepOutcome::Done) {
                    break;
                }
            }
            check_outcome(state);
            if state.outcome.is_some() {
                break;
            }
            self.next_round(state);
        }
    }

    fn next_round(&self, state: &mut State) {
        // Round cap (§0 Ruleset): a fight not closed within `max_rounds` is a **draw** (PvE: no
        // different from a loss). Refresh (§4.6, the Lull): Tempo resets, Health persists, round++.
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
        default_formation(state);
        state.phase = Phase::Marshal;
        state
            .log
            .push(format!("-- Round {}: Marshal --", state.round));
    }

    /// §4.4 — may actor `i` of `side` play this `card` right now? There is **no per-suit/per-side cap**
    /// (casting is bounded only by Tempo + evade). It enforces Disarm, the §4.6 cast window, and the
    /// **target-classification position rule**: an **offensive** (foe-targeting) card is positioned by
    /// reach (§4.2) — a **ranged** one needs the **Rearguard**, a **melee** one the **Vanguard**;
    /// **support** (ally/self) cards are rank-free. A `cast: Standing` card is only legal at the Ready
    /// sub-step of Marshal; a `cast: Strike` card resolves in the sub-phase schedule (§4.6).
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
        // §4.6 one-shot: a `one_shot` card flips face-down for the rest of the combat once used — it is
        // no longer playable (the tempo-gated replacement for `zone: Spend`). Without this a net-positive
        // one-shot (e.g. Sanctuary, which Hastes its own caster) could be replayed forever.
        if card.one_shot && state.s_pool(side)[i].spent_one_shots.contains(&card.name) {
            return false;
        }
        // §4.4 — casting spends a Tempo card; with no per-suit/per-side cap, **Tempo is the limiter**.
        // A unit with no Tempo cannot cast (consistent with strikes, which also need `tempo > 0`).
        if state.s_pool(side)[i].tempo <= 0 {
            return false;
        }
        // §4 cast window: in the sub-phase-schedule engine the interactive phase is Marshal,
        // where **Standing** (ally/self) casts go up. Offensive abilities are resolved by the sub-phase
        // schedule, not cast interactively — wiring ability-strikes into `resolve_round` is a follow-on
        // (see needs-merge/engine-migration-to-sub-phase-model.md), so only Standing casts are playable.
        if !matches!((state.phase, card.cast), (Phase::Marshal, Cast::Standing)) {
            return false;
        }
        if card.is_offensive() {
            return false;
        }
        true
    }

    /// Play `card` from actor `i` of `side`. A `resolve: Reckoning` card is **wound up** (deferred to
    /// resolve in the last sub-phase, the Breach — disruptable); everything else resolves immediately (`resolve: OnCast`).
    fn do_play_card(&self, state: &mut State, side: u8, i: usize, card: crate::cards::Card) {
        let off = state.s_pool(side)[i].offense;
        let name = state.s_pool(side)[i].name.clone();
        // Casting spends a Tempo card (§4.4) — pay-after. A `one_shot` card additionally flips
        // face-down for the rest of the combat (recorded here; see `card_playable_now`) so a
        // net-positive one-shot (e.g. Sanctuary, which Hastes its own caster) cannot be re-cast.
        let caster = if side == 0 {
            &mut state.heroes[i]
        } else {
            &mut state.creatures[i]
        };
        caster.tempo -= 1;
        if card.one_shot {
            caster.spent_one_shots.push(card.name.clone());
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
                "{name} winds up a held effect (resolves in the last sub-phase, its `Reckoning` resolve)."
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
            (None, Phase::Marshal) => format!(
                "Round {} — Marshal: set intentions (Vanguard / Outrider / Rearguard), then advance. (Esc: menu)",
                state.round
            ),
            (None, Phase::Engage) => "Engage — resolving the sub-phase schedule…".to_string(),
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
        if state.pvp && state.outcome.is_none() && matches!(state.phase, Phase::Marshal) {
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
            // §4 Marshal: the next pending unit picks its **intention** (Vanguard / Outrider /
            // Rearguard) and may cast a `Standing` buff; advancing resolves the round's sub-phase
            // schedule (§4.6). Declaration is sequential (one unit at a time) so the solver branches on
            // intention; Pass accepts the unit's current (defaulted) intention.
            Phase::Marshal => {
                use crate::actor::Intention;
                let side = state.plan.committing;
                let mut a = Vec::new();
                if let Some(&i) = self.pending(state, side).first() {
                    let cur = state.s_intent(side)[i];
                    if cur != Intention::Vanguard {
                        a.push(Action::SetVanguard(i));
                    }
                    if cur != Intention::Outrider {
                        a.push(Action::SetOutrider(i));
                    }
                    if cur != Intention::Rearguard {
                        a.push(Action::SetRearguard(i));
                    }
                    // Standing casts (buffs/braces) for this unit before it locks its intention.
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
                    a.push(Action::Pass(i)); // accept the current intention, no change
                }
                a.push(Action::Deploy);
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
            // Engage is transient — the round resolves synchronously inside Deploy, so it is never a
            // resting state; surface only an escape if ever reached.
            Phase::Engage => vec![Action::ToMenu],
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
            Action::SetVanguard(h) => format!("{} holds the front (Vanguard)", hname(*h)),
            Action::SetOutrider(h) => format!("{} breaks the line (Outrider)", hname(*h)),
            Action::SetRearguard(h) => format!("{} deals from the back (Rearguard)", hname(*h)),
            Action::Deploy => "Advance — Engage (resolve the sub-phase schedule)".into(),
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

            // ---- §4 Marshal: set intentions + Standing buffs ----
            (Phase::Marshal, Action::SetVanguard(i)) => {
                let side = state.plan.committing;
                state.s_intent_mut(side)[*i] = Intention::Vanguard;
                state.s_acted_mut(side)[*i] = true;
            }
            (Phase::Marshal, Action::SetOutrider(i)) => {
                let side = state.plan.committing;
                state.s_intent_mut(side)[*i] = Intention::Outrider;
                state.s_acted_mut(side)[*i] = true;
            }
            (Phase::Marshal, Action::SetRearguard(i)) => {
                let side = state.plan.committing;
                state.s_intent_mut(side)[*i] = Intention::Rearguard;
                state.s_acted_mut(side)[*i] = true;
            }
            (Phase::Marshal, Action::PlayCard(i, idx)) => {
                let side = state.plan.committing;
                let card = state.s_pool(side)[*i]
                    .actions
                    .get(*idx)
                    .cloned()
                    .ok_or_else(|| GameError::new("no such card"))?;
                if !self.card_playable_now(state, side, *i, &card) {
                    return Err(GameError::new("that card can't be cast now"));
                }
                self.do_play_card(state, side, *i, card);
                // A Standing cast does not lock the intention; the unit still declares it.
            }
            (Phase::Marshal, Action::Pass(i)) => {
                let side = state.plan.committing;
                state.s_acted_mut(side)[*i] = true; // accept the current (defaulted) intention
            }
            (Phase::Marshal, Action::Deploy) => {
                if state.pvp && state.plan.committing == 0 {
                    state.plan.committing = 1;
                    state.log.push("-- side B: Marshal --".into());
                } else {
                    state.plan.committing = 0;
                    self.resolve_and_advance(state);
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

    fn reference(&self) -> Vec<contract::RefEntry> {
        scenarios::glossary()
    }

    fn view(&self, state: &State, perspective: Option<PlayerId>) -> TableView {
        if let Some(camp) = &state.campaign {
            return CAMPAIGN.view(camp, perspective);
        }
        let mut zones = Vec::new();
        let mut prose: Vec<contract::ProseLine> = Vec::new();
        match &state.phase {
            Phase::Menu(Menu::Top) => zones.push(menu_zone()),
            // Categories are just names → clickable cards; the *content* of a category is the
            // reading pane (prose), since long rules text doesn't belong on a card.
            Phase::Menu(Menu::Rules) => zones.push(category_zone()),
            Phase::Menu(Menu::Category(i)) => {
                let cat = categories().into_iter().nth(*i).unwrap_or_default();
                prose.push(contract::ProseLine::Heading(cat.clone()));
                for e in entries_in(&cat) {
                    prose.push(contract::ProseLine::Term(e.term));
                    prose.push(contract::ProseLine::Body(e.text));
                    prose.push(contract::ProseLine::Gap);
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
            // §4 Marshal reads as **card placement**: the enemy on top, then your party, each
            // character card clickable to **cycle** its intention (Vanguard → Outrider → Rearguard).
            Phase::Marshal => {
                let side = state.plan.committing;
                zones.push(if side == 0 {
                    creature_zone(state, None)
                } else {
                    hero_zone(state, None)
                });
                let acts = self.legal_actions(state);
                let idx_of = |want: &Action| acts.iter().position(|a| a == want);
                let mut cards = Vec::new();
                for i in 0..state.s_len(side) {
                    let actor = &state.s_pool(side)[i];
                    if actor.fallen {
                        continue;
                    }
                    // Clicking a card cycles it to the next intention.
                    let next = match state.s_intent(side)[i] {
                        Intention::Vanguard => Action::SetOutrider(i),
                        Intention::Outrider => Action::SetRearguard(i),
                        Intention::Rearguard => Action::SetVanguard(i),
                    };
                    let mut card = actor_card(actor, Accent::Ally);
                    if let Some(idx) = idx_of(&next) {
                        card = card.action(idx);
                    }
                    cards.push(card);
                }
                zones.push(ZoneView {
                    label: "Marshal (click to cycle: Vanguard / Outrider / Rearguard)".into(),
                    layout: Layout::Row,
                    owner: None,
                    cards,
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
            format!("HP [{}]", pips(d.health.remaining(), d.health.max())),
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
fn append_triangle_chart(prose: &mut Vec<contract::ProseLine>) {
    prose.push(contract::ProseLine::Gap);
    prose.push(contract::ProseLine::Heading("The triangle".into()));
    for line in [
        "Aggressor (Infiltrator) ▸ beats Glass-Cannon — cracks the thin shield before the cannons win",
        "Glass-Cannon (Artillery) ▸ beats Turtle — out-guns a passive defender it never has to reach",
        "Turtle (Wall) ▸ beats Aggressor — drains the pusher dry, so it reaches the back empty",
    ] {
        prose.push(contract::ProseLine::Body(line.into()));
    }
}

/// The Clash four-card counter-grid ("what beats what"): row vs column, from the row's view.
fn append_clash_chart(prose: &mut Vec<contract::ProseLine>) {
    let win = |t: &str| contract::GridCell::new(t, Accent::Good);
    let lose = contract::GridCell::new("lose", Accent::Foe);
    let trade = contract::GridCell::new("trade", Accent::Warn);
    let none = contract::GridCell::new("—", Accent::Neutral);
    let row = |label: &str, cells: Vec<contract::GridCell>| contract::GridRow {
        label: label.into(),
        cells,
    };
    prose.push(contract::ProseLine::Gap);
    prose.push(contract::ProseLine::Heading("What beats what".into()));
    prose.push(contract::ProseLine::Grid(contract::Grid {
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
    prose.push(contract::ProseLine::Body(
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

    /// Drive a scenario to an outcome: accept the stat-defaulted intentions and resolve each round.
    fn autoplay(game: &Deckbound, s: &mut State) -> Outcome {
        let mut guard = 0;
        while game.current_player(s).is_some() {
            let action = match s.phase {
                Phase::Clash => {
                    let beat = s.clash.map(|c| c.beat).unwrap_or(0);
                    if beat.is_multiple_of(2) {
                        Action::Play(Move::Strike)
                    } else {
                        Action::Play(Move::Anticipate)
                    }
                }
                // Marshal intentions are stat-defaulted; Deploy resolves the sub-phase schedule.
                Phase::Marshal | Phase::Engage => Action::Deploy,
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

    /// Phase 1 of the combat-engine observability refactor: the live combat [`State`] serializes to
    /// RON and back, preserving the combat fields (round, heroes' Health/Tempo, phase, plan
    /// intentions). `scenario` / `campaign` are `#[serde(skip)]` (presentation/campaign context) and
    /// come back `None` — fine for the combat-state use case the `sim` CLI loads.
    #[test]
    fn state_round_trips_through_ron() {
        let game = Deckbound;
        let mut hero = scenarios::build_character("Novice", &[]);
        hero.attack = crate::actor::Attack::Melee;
        let foe = scenarios::build_creature("Husk");
        let mut s = battle_state(vec![hero], vec![foe], false, 7);
        assert_eq!(s.phase, Phase::Marshal);

        // Declare an intention or two before serializing.
        game.apply(&mut s, &Action::SetVanguard(0)).unwrap();
        assert_eq!(s.plan.hero_intent[0], Intention::Vanguard);

        // Serialize → deserialize through RON.
        let text = ron::ser::to_string(&s).expect("the combat state serializes");
        let restored: State = ron::from_str(&text).expect("and deserializes");

        // Key combat fields survive the round-trip.
        assert_eq!(restored.round, s.round);
        assert_eq!(restored.phase, s.phase);
        assert_eq!(restored.seed, s.seed);
        assert_eq!(restored.plan.hero_intent, s.plan.hero_intent);
        assert_eq!(restored.plan.foe_intent, s.plan.foe_intent);
        assert_eq!(restored.heroes.len(), s.heroes.len());
        for (a, b) in restored.heroes.iter().zip(s.heroes.iter()) {
            assert_eq!(a.name, b.name);
            assert_eq!(a.tempo, b.tempo);
            assert_eq!(a.defense.health.remaining(), b.defense.health.remaining());
            assert_eq!(a.defense.health.toughness(), b.defense.health.toughness());
        }
        for (a, b) in restored.creatures.iter().zip(s.creatures.iter()) {
            assert_eq!(a.name, b.name);
            assert_eq!(a.defense.health.remaining(), b.defense.health.remaining());
        }
        // The skipped presentation/campaign context comes back empty (by design).
        assert!(restored.scenario.is_none());
        assert!(restored.campaign.is_none());
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

    /// §4 Marshal: a unit defaults to a stat-based intention; the human may re-declare it and
    /// advance, which resolves the round's sub-phase schedule.
    #[test]
    fn declare_intentions_then_resolve() {
        let game = Deckbound;
        let mut hero = scenarios::build_character("Novice", &[]);
        hero.attack = crate::actor::Attack::Melee;
        let foe = scenarios::build_creature("Husk");
        let mut s = battle_state(vec![hero], vec![foe], false, 1);
        assert_eq!(s.phase, Phase::Marshal);
        // A melee unit defaults to a front intention (Vanguard or Outrider, by Finesse).
        assert!(matches!(
            s.plan.hero_intent[0],
            Intention::Vanguard | Intention::Outrider
        ));
        // Re-declare explicitly, then advance — the round resolves.
        game.apply(&mut s, &Action::SetVanguard(0)).unwrap();
        assert_eq!(s.plan.hero_intent[0], Intention::Vanguard);
        game.apply(&mut s, &Action::Deploy).unwrap();
        // After Deploy the round resolved → a fresh Marshal, or an outcome.
        assert!(matches!(s.phase, Phase::Marshal) || s.outcome.is_some());
    }

    /// §4 cast window: a `cast: Standing` support card (Wall's Brace) is offered at Marshal
    /// (rank-free); an **offensive** ability (Artillery's Bolt) is not — offensive casting is resolved by
    /// the sub-phase schedule, not cast interactively (a deferred follow-on, §4.6).
    #[test]
    fn standing_support_casts_at_declare_offensive_deferred() {
        use crate::currency::Currency;
        let game = Deckbound;
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
        let s = battle_state(vec![hero], vec![foe], false, 1);
        assert_eq!(s.phase, Phase::Marshal);

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
        let brace_card = s.heroes[0].actions[brace].clone();
        let bolt_card = s.heroes[0].actions[bolt].clone();
        assert!(!brace_card.is_offensive());
        assert!(bolt_card.is_offensive());
        assert!(
            game.card_playable_now(&s, 0, 0, &brace_card),
            "a cast:Standing support card is offered at Marshal"
        );
        assert!(
            !game.card_playable_now(&s, 0, 0, &bolt_card),
            "offensive abilities are not interactively cast (resolved by the schedule)"
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
        assert_eq!(s.phase, Phase::Marshal);
        assert_eq!(
            game.current_player(&s),
            Some(PlayerId(0)),
            "side A declares first"
        );
        game.apply(&mut s, &Action::Deploy).unwrap();
        assert_eq!(s.phase, Phase::Marshal, "still declaring (side B now)");
        assert_eq!(
            game.current_player(&s),
            Some(PlayerId(1)),
            "now side B declares"
        );
        game.apply(&mut s, &Action::Deploy).unwrap();
        // Both sides declared → the sub-phase resolves; play it out to an outcome.
        let _ = autoplay(&game, &mut s);
        assert!(s.outcome.is_some());
    }

    /// A base-mode cooperation scenario runs the sub-phase-schedule round to an outcome.
    #[test]
    fn base_scenario_runs_to_outcome() {
        let game = Deckbound;
        let mut s = game.new_game(2, 1);
        game.apply(&mut s, &Action::OpenCooperation).unwrap();
        game.apply(&mut s, &Action::PickScenario(0)).unwrap();
        assert_eq!(s.phase, Phase::Marshal);
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
