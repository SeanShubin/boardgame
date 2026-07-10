//! **cardtable-combat** — the bridge that resolves an encounter on the card table using `deckbound`'s
//! battle-tested combat, then folds the result back onto the [`Tableau`].
//!
//! This is the one place the product reaches into the reference game: the pure [`cardtable_model`] never
//! learns about `deckbound`, and the Bevy renderer only records *that* a fight was asked for. Here we read
//! the location's heroes and foes off their cards, translate each into a combat unit (a hero's stats come
//! from its deck via [`Tableau::character_recipe`]; a foe's from [`catalog::creature`]), decide the fight
//! with the headless resolver, and apply the consequences. No Bevy — a plain function over the model, so it
//! unit-tests in isolation.
//!
//! The combatants carry only a 5-stat line and a strike shape (reach / area / hoard / stance); the ability
//! *name* is flavour, ignored by the resolver — mechanics are all numbers, matching the duel-locks proof.

use cardtable_model::{CardId, CardKind, Face, PileId, Tableau, catalog};
use deckbound::actor::Intention;
use deckbound::balance::{DuelUnit, Stat5, build_duel_unit};
use deckbound::combat::{
    PendingDecision, StepOutcome, TargetAnswer, answer_pending_greedily,
    answer_pending_greedily_side,
};
use deckbound::game::{battle_state_with, hero_won};
use deckbound::ruleset::Ruleset;
use deckbound::{Actor, Deckbound, ManualStatus, State};

/// The result of a resolved encounter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatOutcome {
    Win,
    Loss,
}

/// Resolve the encounter at `place`: gather every hero stationed there and the encounter's foes, **play out**
/// the fight with `deckbound`'s seeded, deterministic resolver ([`resolve_logged`](deckbound::resolve_logged),
/// which also returns the turn-by-turn log), and apply the consequences to `table`.
///
/// Combat is a resolved event that leaves a single **virtual combat-log card** at the location (replacing any
/// prior one — one log per location). Foes are **virtual**, read from the catalog rather than dealt as cards:
/// on a **win** the encounter header is cleared (nothing returns to the Bestiary); on a **loss** it stays, so
/// the fight can be retried. Either way the heroes remain where they are and a day passes. A place with no
/// heroes or no encounter is a no-op reporting [`CombatOutcome::Loss`].
pub fn resolve_encounter(table: &mut Tableau, place: PileId, seed: u64) -> CombatOutcome {
    let heroes = hero_units(table, place);
    let foes = foe_units(table, place);
    if heroes.is_empty() || foes.is_empty() {
        return CombatOutcome::Loss;
    }
    let hero_actors: Vec<Actor> = heroes.iter().map(build_duel_unit).collect();
    let foe_actors: Vec<Actor> = foes.iter().map(build_duel_unit).collect();
    let (won, log) = deckbound::resolve_logged(hero_actors, foe_actors, seed);
    let outcome = if won == Some(true) {
        CombatOutcome::Win
    } else {
        CombatOutcome::Loss
    };
    apply_consequences(table, place, outcome, log);
    outcome
}

/// `[u8; 5]` → the resolver's `(Might, Vitality, Toughness, Cadence, Finesse)` tuple.
fn stat5(s: [u8; 5]) -> Stat5 {
    (
        s[0] as u32,
        s[1] as u32,
        s[2] as u32,
        s[3] as u32,
        s[4] as u32,
    )
}

/// The heroes stationed at `place` as combat units — each `hero` position copy mapped to its character
/// deck (by hero name), whose [`Recipe`](cardtable_model::Recipe) gives the stats + ability. The ability
/// sets the strike shape via [`catalog::ability_shape`]; a kit leaves its stance stat-derived (`pos: None`).
fn hero_units(table: &Tableau, place: PileId) -> Vec<DuelUnit> {
    table
        .content_cards(place)
        .into_iter()
        .filter_map(|c| hero_card_unit(table, c))
        .collect()
}

/// One `hero` position card as a [`DuelUnit`] — the per-card kernel of [`hero_units`], shared with the
/// manual path (which also needs the card's id). `None` for a non-hero card or one whose character deck
/// can't be read back into a recipe.
fn hero_card_unit(table: &Tableau, card: CardId) -> Option<DuelUnit> {
    let c = table.card(card)?;
    if c.card_type() != "hero" {
        return None;
    }
    let name = c.front_title().to_string();
    let recipe = table.character_recipe(character_deck(table, &name)?)?;
    let (ranged, aoe) = catalog::ability_shape(&recipe.ability);
    Some(DuelUnit {
        name,
        ability: recipe.ability,
        stats: stat5(recipe.stats),
        ranged,
        aoe,
        count: 1,
        hoard: false,
        pos: None,
    })
}

/// One instantiated **foe** card as a [`DuelUnit`] — its name resolves to a [`catalog::Creature`]. The
/// manual counterpart to [`foe_units`]'s per-creature kernel, but reading a *real* card rather than the
/// virtual catalog roster (so it carries a `CardId` the arena can track).
fn foe_card_unit(table: &Tableau, card: CardId) -> Option<DuelUnit> {
    let creature = catalog::creature(table.card(card)?.front_title())?;
    Some(DuelUnit {
        name: creature.name.to_string(),
        ability: creature.ability.to_string(),
        stats: stat5(creature.stats),
        ranged: creature.ranged,
        aoe: creature.aoe,
        count: 1,
        hoard: creature.hoard,
        pos: creature.pos.map(str::to_string),
    })
}

/// The encounter's foes at `place` as combat units. Foes are **virtual** — the location holds only an
/// encounter card, so the roster comes from the catalog: the place's label names the encounter, and
/// [`catalog::encounter_foes`] gives its `(creature, quantity)` list, expanded so a `×2` keystone fields two.
fn foe_units(table: &Tableau, place: PileId) -> Vec<DuelUnit> {
    let Some(name) = table.pile(place).map(|p| p.label.clone()) else {
        return Vec::new();
    };
    let Some(encounter) = catalog::encounter_for(&name) else {
        return Vec::new();
    };
    let mut units = Vec::new();
    for (creature, qty) in catalog::encounter_foes(encounter) {
        for _ in 0..qty {
            units.push(DuelUnit {
                name: creature.name.to_string(),
                ability: creature.ability.to_string(),
                stats: stat5(creature.stats),
                ranged: creature.ranged,
                aoe: creature.aoe,
                count: 1,
                hoard: creature.hoard,
                pos: creature.pos.map(str::to_string),
            });
        }
    }
    units
}

// ===== Manual combat (headless) =========================================================================
//
// Where [`resolve_encounter`] plays the whole fight out in one call, manual combat instantiates the foes as
// **real cards** and drives `deckbound`'s resumable resolver one decision at a time, so the player makes
// every choice on the table. This module is still headless (no Bevy): it owns the battle [`State`], the
// card↔actor identity map, and the [`CardMutation`] diff the renderer will animate — everything provable
// with a scripted "player" in tests.

/// A change to one combatant's card that a resolution step produced — the input to the arena's animation
/// layer. Keyed by the arena `CardId` via the combat's card↔actor map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardMutation {
    /// The unit's remaining Health changed (`from` → `to` face-up Health cards).
    HealthChanged { card: CardId, from: u32, to: u32 },
    /// The unit fell (all Health down) on this step.
    Fell { card: CardId },
}

impl CardMutation {
    /// The arena card this mutation applies to.
    pub fn card(&self) -> CardId {
        match *self {
            CardMutation::HealthChanged { card, .. } | CardMutation::Fell { card } => card,
        }
    }
}

/// A manual combat in progress: the `deckbound` battle [`State`] plus the **card↔actor identity map** — a
/// hero actor `i` is `hero_cards[i]`, a foe actor `j` is `foe_cards[j]`. The resolver rests at every
/// decision (drive it with [`ManualCombat::run_to_end`], answering `state.pending`), and
/// [`ManualCombat::diff`] turns an `Actor` delta into the [`CardMutation`]s the arena animates.
pub struct ManualCombat {
    /// The live battle state. Its `pending` list holds the decisions the player is currently being asked.
    pub state: State,
    hero_cards: Vec<CardId>,
    foe_cards: Vec<CardId>,
}

/// Begin a manual combat at `place`: map its stationed heroes to units, **instantiate** the encounter's
/// foes as real cards in `arena` (split off the `bestiary` stacks, PC.2), and build the seeded battle
/// [`State`]. Returns `None` (dealing no foes) if the place has no stationed hero or no encounter. The foe
/// cards live in `arena` until the fight is torn down, when they return to the Bestiary.
pub fn begin_manual_combat(
    table: &mut Tableau,
    place: PileId,
    arena: PileId,
    bestiary: PileId,
    seed: u64,
) -> Option<ManualCombat> {
    // Heroes: the stationed `hero` cards at the place, paired with their units (keep both in lockstep).
    let hero_pairs: Vec<(CardId, DuelUnit)> = table
        .content_cards(place)
        .into_iter()
        .filter_map(|c| Some((c, hero_card_unit(table, c)?)))
        .collect();
    if hero_pairs.is_empty() {
        return None;
    }
    // Foes: instantiate the encounter's roster as real cards (nothing dealt if the place has no encounter).
    let label = table.pile(place)?.label.clone();
    let foe_cards = table
        .instantiate_encounter_foes(bestiary, arena, &label)
        .ok()?;
    if foe_cards.is_empty() {
        return None;
    }
    let foe_units: Vec<DuelUnit> = foe_cards
        .iter()
        .filter_map(|&c| foe_card_unit(table, c))
        .collect();

    let hero_cards: Vec<CardId> = hero_pairs.iter().map(|(c, _)| *c).collect();
    let hero_actors: Vec<Actor> = hero_pairs.iter().map(|(_, u)| build_duel_unit(u)).collect();
    let foe_actors: Vec<Actor> = foe_units.iter().map(build_duel_unit).collect();
    let state = battle_state_with(hero_actors, foe_actors, false, seed, Ruleset::analysis());
    Some(ManualCombat {
        state,
        hero_cards,
        foe_cards,
    })
}

impl ManualCombat {
    /// The arena card standing in for actor `idx` on `side` (0 = heroes, 1 = foes), if mapped.
    pub fn card_of(&self, side: u8, idx: usize) -> Option<CardId> {
        let cards = if side == 0 {
            &self.hero_cards
        } else {
            &self.foe_cards
        };
        cards.get(idx).copied()
    }

    /// The instantiated foe cards (in the Bestiary supply) — needed to return them on teardown.
    pub fn foe_cards(&self) -> &[CardId] {
        &self.foe_cards
    }

    /// The [`CardMutation`]s from `prev` to the current [`state`](Self::state) — per unit, a Health change
    /// and/or a fall, keyed to its arena card.
    pub fn diff(&self, prev: &State) -> Vec<CardMutation> {
        diff_states(prev, &self.state, &self.hero_cards, &self.foe_cards)
    }

    /// Drive the fight to its end, answering each decision via `decide` and reporting the [`CardMutation`]s
    /// of every transition via `observe`. With `decide = `[`answer_pending_greedily`] this is the auto
    /// fight; a player driver answers `state.pending` instead. Returns the final [`CombatOutcome`].
    pub fn run_to_end(
        &mut self,
        mut decide: impl FnMut(&mut State),
        mut observe: impl FnMut(&[CardMutation]),
    ) -> CombatOutcome {
        let hero_cards = self.hero_cards.clone();
        let foe_cards = self.foe_cards.clone();
        let mut prev = self.state.clone();
        Deckbound.resolve_battle_manual(&mut self.state, |s, out| {
            if matches!(out, StepOutcome::Resting) {
                decide(s);
            }
            let muts = diff_states(&prev, s, &hero_cards, &foe_cards);
            if !muts.is_empty() {
                observe(&muts);
            }
            prev = s.clone();
        });
        if hero_won(&self.state) {
            CombatOutcome::Win
        } else {
            CombatOutcome::Loss
        }
    }

    /// Resolve the rest of the fight automatically (greedy), still reporting mutations to `observe` — the
    /// "let the game finish it" path from within a manual arena.
    pub fn run_to_end_auto(&mut self, observe: impl FnMut(&[CardMutation])) -> CombatOutcome {
        self.run_to_end(answer_pending_greedily, observe)
    }
}

/// The card mutations from `prev` to `next` given the card↔actor map — per unit on both sides, a Health
/// change and/or a fall. A free function so the resumable driver can call it without borrowing the combat.
fn diff_states(
    prev: &State,
    next: &State,
    hero_cards: &[CardId],
    foe_cards: &[CardId],
) -> Vec<CardMutation> {
    let mut out = Vec::new();
    for side in 0u8..2 {
        let (p, n, cards) = if side == 0 {
            (&prev.heroes, &next.heroes, hero_cards)
        } else {
            (&prev.creatures, &next.creatures, foe_cards)
        };
        for (i, (pr, nx)) in p.iter().zip(n.iter()).enumerate() {
            let Some(&card) = cards.get(i) else {
                continue;
            };
            let (from, to) = (pr.defense.health.remaining(), nx.defense.health.remaining());
            if from != to {
                out.push(CardMutation::HealthChanged { card, from, to });
            }
            if !pr.fallen && nx.fallen {
                out.push(CardMutation::Fell { card });
            }
        }
    }
    out
}

// ===== Arena driver + render view (the interactive front-end for a manual fight) ========================
//
// A live UI can't use the synchronous `run_to_end` (it answers decisions across frames), so `ManualCombat`
// also exposes a **frame-steppable** driver (`advance`) plus a plain-data render API (`view` /
// `current_decision` / `answer_current`) that the renderer reads without touching any `deckbound` type.

/// One combatant as the arena shows it, read from the battle state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitView {
    pub name: String,
    /// Rank letter — `V`/`O`/`R` (Vanguard/Outrider/Rearguard), which governs reach.
    pub rank: char,
    pub health_remaining: u32,
    pub health_max: u32,
    /// Current Tempo (face-up Tempo cards) — the budget a strike / evade / strike-back spends.
    pub tempo: i32,
    pub fallen: bool,
    /// 0 = hero, 1 = foe.
    pub side: u8,
    /// Index within its side's pool (the key the arena's answers use).
    pub idx: usize,
}

/// Both sides' combatants for the rank-lane layout.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ArenaView {
    pub heroes: Vec<UnitView>,
    pub foes: Vec<UnitView>,
}

/// The current phase as the top card of the **phase pile** (Intercept / Volley / Raid / Clash / Breach, or
/// `Marshal` between rounds). `index`/`total` say how far through the pile the top card is (for the stack
/// look); `pairs` is the rank→rank pairs this phase resolves — who may strike whom — one per entry, spelled
/// out (e.g. `"Vanguard -> Outrider"`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhaseView {
    pub round: u32,
    pub index: usize,
    pub total: usize,
    pub name: String,
    pub pairs: Vec<String>,
    /// `true` while resolving the schedule; `false` between rounds (Marshal).
    pub resolving: bool,
}

/// One legal target offered for a hero's strike (a foe at `ti`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetChoice {
    pub ti: usize,
    pub name: String,
}

/// The current **hero** decision the arena must present, as plain data. `attacker` in `Evade`/`StrikeBack`
/// is the **foe** that struck; `soaker` is the party unit deciding, and `tempo` is what it has to spend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecisionView {
    /// Which foe should the party unit `attacker` strike this sub-phase (or hold)?
    Target {
        attacker: String,
        candidates: Vec<TargetChoice>,
    },
    /// Does `soaker` evade `attacker`'s incoming blow, for `cost` Tempo (it has `tempo`)?
    Evade {
        soaker: String,
        attacker: String,
        cost: i32,
        tempo: i32,
    },
    /// Does `soaker` strike back (1 Tempo, it has `tempo`) at the melee foe `attacker` that just hit it?
    StrikeBack {
        soaker: String,
        attacker: String,
        tempo: i32,
    },
}

/// The player's answer to the current hero decision (see [`ManualCombat::answer_current`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArenaAnswer {
    /// Strike the foe at this `ti` (a [`DecisionView::Target`] choice).
    Strike(usize),
    /// Hold — commit nothing (a [`DecisionView::Target`] choice).
    Hold,
    /// Evade / endure (a [`DecisionView::Evade`] answer).
    Evade(bool),
    /// Strike back / hold (a [`DecisionView::StrikeBack`] answer).
    StrikeBack(bool),
}

/// The rank letter for an intention (`V`/`O`/`R`) — used on the compact unit tiles.
fn rank_char(r: &Intention) -> char {
    match r {
        Intention::Vanguard => 'V',
        Intention::Outrider => 'O',
        Intention::Rearguard => 'R',
    }
}

/// The full rank name for an intention — used where there's room to spell it out (the phase pairs).
fn rank_name(r: &Intention) -> &'static str {
    match r {
        Intention::Vanguard => "Vanguard",
        Intention::Outrider => "Outrider",
        Intention::Rearguard => "Rearguard",
    }
}

/// Whether `pd` is an **unanswered hero** (side 0) decision — the ones the player must answer.
fn hero_undecided(pd: &PendingDecision) -> bool {
    matches!(
        pd,
        PendingDecision::Target {
            side: 0,
            answer: None,
            ..
        } | PendingDecision::Evade {
            side: 0,
            answer: None,
            ..
        } | PendingDecision::StrikeBack {
            side: 0,
            answer: None,
            ..
        }
    )
}

impl ManualCombat {
    /// Advance the fight one **frame-steppable** unit (see
    /// [`Deckbound::advance_manual_battle`](deckbound::Deckbound::advance_manual_battle)), returning the
    /// status and the [`CardMutation`]s that transition produced (for the arena to reflect). When it
    /// returns [`ManualStatus::Deciding`], answer [`current_decision`](Self::current_decision) (hero) and/or
    /// [`answer_foe_side`](Self::answer_foe_side) (foe AI), then call again.
    pub fn advance(&mut self) -> (ManualStatus, Vec<CardMutation>) {
        let prev = self.state.clone();
        let status = Deckbound.advance_manual_battle(&mut self.state);
        let muts = self.diff(&prev);
        (status, muts)
    }

    /// Auto-answer the **foe** side's pending decisions greedily (the AI opponent), leaving the player's
    /// hero decisions for [`answer_current`](Self::answer_current).
    pub fn answer_foe_side(&mut self) {
        answer_pending_greedily_side(&mut self.state, 1);
    }

    /// Whether the battle is over (a side won or it was a draw).
    pub fn is_finished(&self) -> bool {
        self.state.outcome.is_some()
    }

    /// Both sides' combatants, in pool order, for the rank-lane layout.
    pub fn view(&self) -> ArenaView {
        ArenaView {
            heroes: self.side_view(0),
            foes: self.side_view(1),
        }
    }

    fn side_view(&self, side: u8) -> Vec<UnitView> {
        let (pool, intents) = if side == 0 {
            (&self.state.heroes, &self.state.plan.hero_intent)
        } else {
            (&self.state.creatures, &self.state.plan.foe_intent)
        };
        pool.iter()
            .enumerate()
            .map(|(idx, a)| UnitView {
                name: a.name.clone(),
                rank: intents.get(idx).map(rank_char).unwrap_or('?'),
                health_remaining: a.defense.health.remaining(),
                health_max: a.defense.health.max(),
                tempo: a.tempo,
                fallen: a.fallen,
                side,
                idx,
            })
            .collect()
    }

    /// The current phase as the top of the tabletop **phase pile** — its round, name, and the rank→rank
    /// pairs it resolves (who may strike whom), plus how many phases sit under it (the deck the top card
    /// rotates through). Between rounds it reads as `Marshal`. Answers "what phase am I in".
    pub fn phase(&self) -> PhaseView {
        let round = self.state.round;
        let total = deckbound::combat::SUB_PHASE_NAMES.len();
        match self.state.resolution {
            Some(r) => {
                let name = deckbound::combat::SUB_PHASE_NAMES
                    .get(r.step)
                    .copied()
                    .unwrap_or("-")
                    .to_string();
                let pairs = deckbound::combat::SCHEDULE
                    .get(r.step)
                    .map(|ps| {
                        ps.iter()
                            .map(|(a, t)| format!("{} -> {}", rank_name(a), rank_name(t)))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                PhaseView {
                    round,
                    index: r.step,
                    total,
                    name,
                    pairs,
                    resolving: true,
                }
            }
            None => PhaseView {
                round,
                index: 0,
                total,
                name: "Marshal".to_string(),
                pairs: vec!["declare & deploy".to_string()],
                resolving: false,
            },
        }
    }

    /// The running battle log (round intros, sub-phase headers, each strike/evade/fall) — so the play can be
    /// read back and followed. Grows as the fight advances.
    pub fn log(&self) -> &[String] {
        &self.state.log
    }

    /// The **first unanswered hero decision** the arena must present, or `None` if the party isn't currently
    /// on the clock (a foe decision or an auto step is next).
    pub fn current_decision(&self) -> Option<DecisionView> {
        self.state
            .pending
            .iter()
            .find(|pd| hero_undecided(pd))
            .map(|pd| match pd {
                PendingDecision::Target {
                    attacker,
                    candidates,
                    ..
                } => DecisionView::Target {
                    attacker: attacker.clone(),
                    candidates: candidates
                        .iter()
                        .map(|c| TargetChoice {
                            ti: c.ti,
                            name: c.name.clone(),
                        })
                        .collect(),
                },
                PendingDecision::Evade {
                    soaker,
                    attacker,
                    cost,
                    ..
                } => DecisionView::Evade {
                    soaker: self.hero_name(*soaker),
                    attacker: attacker.clone(),
                    cost: *cost,
                    tempo: self.hero_tempo(*soaker),
                },
                PendingDecision::StrikeBack {
                    soaker, attacker, ..
                } => DecisionView::StrikeBack {
                    soaker: self.hero_name(*soaker),
                    attacker: attacker.clone(),
                    tempo: self.hero_tempo(*soaker),
                },
            })
    }

    /// The name of the party unit at `idx` (the deciding soaker for an Evade / Strike-back), or `"?"`.
    fn hero_name(&self, idx: usize) -> String {
        self.state
            .heroes
            .get(idx)
            .map(|a| a.name.clone())
            .unwrap_or_else(|| "?".to_string())
    }

    /// The current Tempo (face-up Tempo cards) of the party unit at `idx` — what a strike-back / evade is
    /// paid from.
    fn hero_tempo(&self, idx: usize) -> i32 {
        self.state.heroes.get(idx).map(|a| a.tempo).unwrap_or(0)
    }

    /// Record the player's `answer` to the current hero decision (the one [`current_decision`] reports). A
    /// mismatched answer kind is ignored.
    ///
    /// [`current_decision`]: Self::current_decision
    pub fn answer_current(&mut self, answer: ArenaAnswer) {
        let Some(pd) = self.state.pending.iter_mut().find(|pd| hero_undecided(pd)) else {
            return;
        };
        match (pd, answer) {
            (PendingDecision::Target { answer, .. }, ArenaAnswer::Strike(ti)) => {
                *answer = Some(TargetAnswer::Strike(ti));
            }
            (PendingDecision::Target { answer, .. }, ArenaAnswer::Hold) => {
                *answer = Some(TargetAnswer::Hold);
            }
            (PendingDecision::Evade { answer, .. }, ArenaAnswer::Evade(b)) => *answer = Some(b),
            (PendingDecision::StrikeBack { answer, .. }, ArenaAnswer::StrikeBack(b)) => {
                *answer = Some(b);
            }
            _ => {} // the answer kind doesn't match the current decision - ignore
        }
    }
}

/// Fold a **finished** manual combat back onto the table — the manual counterpart of the auto path's
/// [`apply_consequences`]. The instantiated foe cards return to the `bestiary` (merged back into their `×N`
/// stacks, PC.2, whatever state they ended in), then the shared consequence applies: on a **win** the
/// `encounter` header at `place` is cleared (consumed), a single virtual combat-log card is left, and the
/// day advances once. `combat` must be finished (its `state.outcome` set); it is consumed here.
pub fn finish_manual_combat(
    table: &mut Tableau,
    place: PileId,
    bestiary: PileId,
    combat: ManualCombat,
) -> CombatOutcome {
    let outcome = if hero_won(&combat.state) {
        CombatOutcome::Win
    } else {
        CombatOutcome::Loss
    };
    // The real foes go home to the Bestiary (win or lose — a loss leaves the *virtual* encounter to retry).
    let _ = table.return_foes_to_bestiary(&combat.foe_cards, bestiary);
    apply_consequences(table, place, outcome, combat.state.log);
    outcome
}

/// Fold the result onto the table. A location holds at most **one** combat-log card, so any prior one is
/// removed first. Foes are **virtual** (the Bestiary supply never left home), so a **win** just clears the
/// `encounter` header — nothing to send back; on a **loss** it stays, so the fight can be retried. Then the
/// new virtual log card is left at the place and [`advance_day`](Tableau::advance_day) runs once — the fight
/// costs a day. The heroes are not moved either way.
fn apply_consequences(
    table: &mut Tableau,
    place: PileId,
    outcome: CombatOutcome,
    log: Vec<String>,
) {
    // One log per location: clear the previous combat's card before recording this one.
    for c in cards_of_types(table, place, &["log"]) {
        let _ = table.remove_card(c);
    }
    if outcome == CombatOutcome::Win {
        // The encounter is defeated: remove its header. The foes were virtual, so there is nothing to
        // return to the Bestiary — its stacked supply was only ever *read*, never moved.
        for c in cards_of_types(table, place, &["encounter"]) {
            let _ = table.remove_card(c);
        }
    }
    add_log_card(table, place, outcome, log);
    if let (Some(progress), Some(events)) =
        (find_pile(table, "Progress"), find_pile(table, "Events"))
    {
        let _ = table.advance_day(progress, events);
    }
}

/// Leave a **virtual** combat-log card at `place`: a non-physical [`CardKind::Virtual`] card typed `log`,
/// titled by the outcome, with a one-line summary (Medium) over the full turn-by-turn narration (its Large
/// panel). It renders and expands like any card but is excluded from the physical tally.
fn add_log_card(table: &mut Tableau, place: PileId, outcome: CombatOutcome, log: Vec<String>) {
    let (title, summary) = match outcome {
        CombatOutcome::Win => ("Victory", "The party cleared the encounter."),
        CombatOutcome::Loss => ("Defeat", "The party was beaten back."),
    };
    let Ok(id) = table.add_card(
        place,
        Face::Up {
            title: title.into(),
        },
        None,
    ) else {
        return;
    };
    let _ = table.set_card_type(id, "log");
    let _ = table.set_card_kind(id, CardKind::Virtual);
    let _ = table.set_card_detail(id, vec![summary.to_string()]);
    let _ = table.set_card_panel(id, log);
}

/// The content cards of `place` whose `card_type` is one of `types`.
fn cards_of_types(table: &Tableau, place: PileId, types: &[&str]) -> Vec<CardId> {
    table
        .content_cards(place)
        .into_iter()
        .filter(|&c| {
            table
                .card(c)
                .is_some_and(|k| types.contains(&k.card_type()))
        })
        .collect()
}

/// The character deck for hero `name` — the root sub-pile that [`reflects`](cardtable_model::Pile::reflects)
/// an identity card with that title.
fn character_deck(table: &Tableau, name: &str) -> Option<PileId> {
    subpiles(table, table.root_id()).into_iter().find(|&p| {
        table
            .pile(p)
            .and_then(|pile| pile.reflects())
            .and_then(|id| table.card(id))
            .is_some_and(|c| c.front_title() == name)
    })
}

/// A top-level deck by label.
fn find_pile(table: &Tableau, label: &str) -> Option<PileId> {
    subpiles(table, table.root_id())
        .into_iter()
        .find(|&p| table.pile(p).is_some_and(|pile| pile.label == label))
}

/// The sub-pile ids of `pile` (empty if unknown).
fn subpiles(table: &Tableau, pile: PileId) -> Vec<PileId> {
    table.pile(pile).map(|p| p.subpiles()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cardtable_model::{Recipe, sample_table};

    /// The four inn-adjacent solo encounters and the kit that answers each (the duel-locks diagonal).
    fn marksman() -> Recipe {
        Recipe {
            stats: [4, 4, 1, 2, 2],
            ability: "Stand-Off".into(),
        }
    }
    fn executioner() -> Recipe {
        Recipe {
            stats: [6, 3, 1, 1, 1],
            ability: "Alpha Strike".into(),
        }
    }

    fn deck(t: &Tableau, label: &str) -> PileId {
        find_pile(t, label).expect("a top-level deck")
    }

    /// The inn — Ashfen Crossing, the centre place inside the Locations grid.
    fn ashfen(t: &Tableau) -> PileId {
        subpiles(t, deck(t, "Locations"))
            .into_iter()
            .find(|&p| t.pile(p).unwrap().label == "Ashfen Crossing")
            .expect("the inn")
    }

    /// Recruit Heroes deck hero #0 with `recipe` (dealing its four copies, including a position copy at
    /// the inn and a Progress move marker), then march that position copy to the place `dest`. Returns the
    /// destination place id.
    fn station_at(t: &mut Tableau, recipe: Recipe, dest: &str) -> PileId {
        let (heroes, stats, numbers, abilities, progress) = (
            deck(t, "Heroes"),
            deck(t, "Stats"),
            deck(t, "Numbers"),
            deck(t, "Abilities"),
            deck(t, "Progress"),
        );
        let ashfen = ashfen(t);
        let name = t
            .card(t.content_cards(heroes)[0])
            .unwrap()
            .name()
            .to_string();
        t.equip_character(
            &name, &recipe, heroes, stats, numbers, abilities, ashfen, progress,
        )
        .unwrap();
        // The hero's position copy now rests at the inn; march it to `dest`.
        let position = t
            .content_cards(ashfen)
            .into_iter()
            .find(|&c| {
                let k = t.card(c).unwrap();
                k.card_type() == "hero" && k.front_title() == name
            })
            .expect("the stationed hero's position copy");
        let place = t
            .pile(deck(t, "Locations"))
            .unwrap()
            .subpiles()
            .into_iter()
            .find(|&p| t.pile(p).unwrap().label == dest)
            .unwrap();
        t.move_character(position, place, progress).unwrap();
        place
    }

    /// Whether an unresolved encounter still sits at `place`. Foes are virtual (no physical foe cards), so
    /// the encounter header is the marker of a pending fight — present at setup, removed on a win.
    fn has_encounter(t: &Tableau, place: PileId) -> bool {
        !cards_of_types(t, place, &["encounter"]).is_empty()
    }

    /// The one combat-log card at `place` (its title), if any.
    fn log_title(t: &Tableau, place: PileId) -> Option<String> {
        cards_of_types(t, place, &["log"])
            .first()
            .map(|&c| t.card(c).unwrap().name().to_string())
    }

    /// The right kit wins: a Marksman clears Cinderwatch Keep's Coil — the encounter is cleared (foes are
    /// virtual, so nothing returns to the Bestiary), a non-physical "Victory" log card is left behind (with
    /// a real narration panel), and a day passes.
    #[test]
    fn answering_kit_wins_clears_foes_and_leaves_a_log() {
        let mut t = sample_table();
        let day_before = t.current_day(deck(&t, "Progress"));
        let place = station_at(&mut t, marksman(), "Cinderwatch Keep");
        assert!(has_encounter(&t, place), "the Coil's encounter waits here");

        let outcome = resolve_encounter(&mut t, place, 1);
        assert_eq!(outcome, CombatOutcome::Win);
        assert!(!has_encounter(&t, place), "the cleared encounter is gone");
        assert_eq!(log_title(&t, place).as_deref(), Some("Victory"));
        // The place now holds only its "Location" label + the hero token; the log card is virtual, so it
        // does not raise the physical tally (the encounter header is gone; foes were never physical).
        assert_eq!(t.physical_card_count(place), 2);
        assert_eq!(
            t.current_day(deck(&t, "Progress")),
            day_before + 1,
            "the fight cost a day"
        );
    }

    /// The wrong kit loses: an Executioner cannot crack the Coil. The foes REMAIN (retriable), the hero
    /// stays at the location (no retreat), a "Defeat" log card is left, and a day passes.
    #[test]
    fn wrong_kit_loses_foes_remain_hero_stays() {
        let mut t = sample_table();
        let place = station_at(&mut t, executioner(), "Cinderwatch Keep");

        let outcome = resolve_encounter(&mut t, place, 1);
        assert_eq!(outcome, CombatOutcome::Loss);
        assert!(has_encounter(&t, place), "the foes still hold the place");
        assert_eq!(log_title(&t, place).as_deref(), Some("Defeat"));
        let stayed = cards_of_types(&t, place, &["hero"])
            .iter()
            .any(|&c| t.card(c).unwrap().front_title() == "Vael Thornbrand");
        assert!(stayed, "the hero stays put on a loss");
        let at_inn = cards_of_types(&t, ashfen(&t), &["hero"])
            .iter()
            .any(|&c| t.card(c).unwrap().front_title() == "Vael Thornbrand");
        assert!(!at_inn, "the hero did not retreat to the inn");
    }

    /// One combat-log card per location: a second fight replaces the first's log rather than stacking.
    #[test]
    fn a_new_fight_replaces_the_previous_log() {
        let mut t = sample_table();
        let place = station_at(&mut t, executioner(), "Cinderwatch Keep");
        resolve_encounter(&mut t, place, 1); // loss - foes remain, "Defeat" logged
        resolve_encounter(&mut t, place, 1); // fight again
        assert_eq!(
            cards_of_types(&t, place, &["log"]).len(),
            1,
            "only the latest combat's log remains"
        );
    }

    /// Manual combat instantiates the foes as **real cards**, drives the resumable resolver, and — answered
    /// greedily — reaches the same outcome as the auto path, emitting card mutations for the blows that land.
    #[test]
    fn manual_combat_matches_auto_and_diffs_the_flips() {
        // Auto reference: the Marksman clears Cinderwatch Keep at this seed.
        let mut auto = sample_table();
        let auto_place = station_at(&mut auto, marksman(), "Cinderwatch Keep");
        let auto_outcome = resolve_encounter(&mut auto, auto_place, 7);
        assert_eq!(auto_outcome, CombatOutcome::Win);

        // Manual, same station + seed.
        let mut t = sample_table();
        let place = station_at(&mut t, marksman(), "Cinderwatch Keep");
        let bestiary = deck(&t, "Bestiary");
        let arena = t.add_pile(t.root_id(), "Arena").unwrap();
        let mut combat =
            begin_manual_combat(&mut t, place, arena, bestiary, 7).expect("a fight begins");

        // The foes are now real cards sitting in the arena.
        assert!(!combat.foe_cards().is_empty(), "foes instantiated as cards");
        assert_eq!(t.content_cards(arena).len(), combat.foe_cards().len());

        let mut muts: Vec<CardMutation> = Vec::new();
        let manual_outcome = combat.run_to_end_auto(|m| muts.extend_from_slice(m));

        assert_eq!(
            manual_outcome, auto_outcome,
            "manual greedy reproduces the auto outcome"
        );
        assert!(!muts.is_empty(), "a real fight flips cards");

        // Every mutation keys a known combatant card (a stationed hero, or an instantiated foe).
        let known: std::collections::HashSet<CardId> = combat
            .foe_cards()
            .iter()
            .copied()
            .chain(cards_of_types(&t, place, &["hero"]))
            .collect();
        assert!(
            muts.iter().all(|m| known.contains(&m.card())),
            "mutations key real combatant cards"
        );
        // The win means at least one foe fell — a Fell mutation should name a foe card.
        let foes: std::collections::HashSet<CardId> = combat.foe_cards().iter().copied().collect();
        assert!(
            muts.iter()
                .any(|m| matches!(m, CardMutation::Fell { card } if foes.contains(card))),
            "a foe fell on the way to victory"
        );
    }

    /// No hero stationed → no fight begins, and (crucially) no foes are dealt out of the Bestiary.
    #[test]
    fn manual_combat_needs_a_stationed_hero() {
        let mut t = sample_table();
        let place = subpiles(&t, deck(&t, "Locations"))
            .into_iter()
            .find(|&p| t.pile(p).unwrap().label == "Cinderwatch Keep")
            .unwrap();
        let bestiary = deck(&t, "Bestiary");
        let bestiary_before = t.physical_card_count(bestiary);
        let arena = t.add_pile(t.root_id(), "Arena").unwrap();

        assert!(begin_manual_combat(&mut t, place, arena, bestiary, 7).is_none());
        assert!(
            t.content_cards(arena).is_empty(),
            "no foes dealt without a hero"
        );
        assert_eq!(
            t.physical_card_count(bestiary),
            bestiary_before,
            "the Bestiary supply is untouched"
        );
    }

    /// End-to-end capstone: a full manual fight from the sample table — instantiate foes → drive the
    /// resolver to a decision → run it out → fold back — conserving cards the whole way. The foes round-trip
    /// through the Bestiary; the only net change is the encounter header a win consumes.
    #[test]
    fn a_full_manual_fight_conserves_cards_and_folds_back() {
        let mut t = sample_table();
        let place = station_at(&mut t, marksman(), "Cinderwatch Keep");
        let bestiary = deck(&t, "Bestiary");
        let arena = t.add_pile(t.root_id(), "Arena").unwrap();

        let total_before = t.card_count();
        let phys_before = t.physical_card_count(t.root_id());
        let bestiary_before = t.physical_card_count(bestiary);
        let day_before = t.current_day(deck(&t, "Progress"));

        let mut combat =
            begin_manual_combat(&mut t, place, arena, bestiary, 7).expect("a fight begins");
        let foe_count = combat.foe_cards().len();
        // Instantiation split real foes off the Bestiary — nothing minted.
        assert_eq!(
            t.card_count(),
            total_before,
            "instantiation minted nothing (PC.2)"
        );
        assert_eq!(t.physical_card_count(bestiary), bestiary_before - foe_count);

        // Drive the fight out (greedily, so the capstone is self-contained), then fold it back.
        let outcome = combat.run_to_end_auto(|_| {});
        assert_eq!(outcome, CombatOutcome::Win);
        let outcome = finish_manual_combat(&mut t, place, bestiary, combat);
        assert_eq!(outcome, CombatOutcome::Win);

        // The foes are home (Bestiary whole), the arena is clear.
        assert_eq!(
            t.physical_card_count(bestiary),
            bestiary_before,
            "foes returned to the Bestiary"
        );
        assert!(t.content_cards(arena).is_empty(), "the arena is emptied");
        // A win consumed the encounter header, left a virtual "Victory" log, and cost a day.
        assert!(!has_encounter(&t, place), "the cleared encounter is gone");
        assert_eq!(log_title(&t, place).as_deref(), Some("Victory"));
        assert_eq!(
            t.current_day(deck(&t, "Progress")),
            day_before + 1,
            "the fight cost a day"
        );
        // Conservation: every foe round-trips and the day-card is a move, so the only *physical* change is
        // the one encounter header a win consumes. `card_count` itself is unchanged — the virtual Victory
        // log card takes the consumed encounter's place 1:1.
        assert_eq!(
            t.physical_card_count(t.root_id()),
            phys_before - 1,
            "only the cleared encounter left the physical tally"
        );
        assert_eq!(
            t.card_count(),
            total_before,
            "card_count conserved across the whole round-trip"
        );
    }

    /// The frame-steppable arena driver (`advance`), answered greedily on both sides, reproduces the auto
    /// outcome — the interactive front-end resolves the same fight as the one-shot when the player defers.
    #[test]
    fn arena_driver_greedy_reproduces_auto() {
        let mut auto = sample_table();
        let ap = station_at(&mut auto, marksman(), "Cinderwatch Keep");
        let auto_outcome = resolve_encounter(&mut auto, ap, 7);

        let mut t = sample_table();
        let place = station_at(&mut t, marksman(), "Cinderwatch Keep");
        let bestiary = deck(&t, "Bestiary");
        let arena = t.add_pile(t.root_id(), "Arena").unwrap();
        let mut combat = begin_manual_combat(&mut t, place, arena, bestiary, 7).unwrap();

        let mut guard = 0;
        while !combat.is_finished() {
            deckbound::combat::answer_pending_greedily(&mut combat.state); // both sides greedy
            combat.advance();
            guard += 1;
            assert!(guard < 100_000, "arena driver failed to terminate");
        }
        let outcome = finish_manual_combat(&mut t, place, bestiary, combat);
        assert_eq!(
            outcome, auto_outcome,
            "arena driver reproduces the auto outcome"
        );
    }

    /// Playing a full fight through the render API — `view` / `current_decision` / `answer_current` for the
    /// party, `answer_foe_side` for the AI — drives it to a decisive end, with the foes present in the view
    /// and Health falling as blows land. Proves the whole arena front-end without any Bevy.
    #[test]
    fn arena_view_and_answers_drive_a_full_fight() {
        let mut t = sample_table();
        let place = station_at(&mut t, marksman(), "Cinderwatch Keep");
        let bestiary = deck(&t, "Bestiary");
        let arena = t.add_pile(t.root_id(), "Arena").unwrap();
        let mut combat = begin_manual_combat(&mut t, place, arena, bestiary, 7).unwrap();

        // The opening view: both sides present, full Health, ranks assigned.
        let v0 = combat.view();
        assert!(!v0.heroes.is_empty() && !v0.foes.is_empty());
        assert!(
            v0.foes
                .iter()
                .all(|u| u.health_remaining == u.health_max && !u.fallen)
        );
        assert!(v0.heroes.iter().all(|u| "VOR".contains(u.rank)));

        let mut hero_decisions = 0;
        let mut guard = 0;
        while !combat.is_finished() {
            if let Some(dec) = combat.current_decision() {
                hero_decisions += 1;
                match dec {
                    DecisionView::Target { candidates, .. } => match candidates.first() {
                        Some(c) => combat.answer_current(ArenaAnswer::Strike(c.ti)),
                        None => combat.answer_current(ArenaAnswer::Hold),
                    },
                    DecisionView::Evade { .. } => combat.answer_current(ArenaAnswer::Evade(true)),
                    DecisionView::StrikeBack { .. } => {
                        combat.answer_current(ArenaAnswer::StrikeBack(true))
                    }
                }
            } else {
                combat.answer_foe_side();
                combat.advance();
            }
            guard += 1;
            assert!(guard < 100_000, "arena playthrough failed to terminate");
        }
        assert!(hero_decisions > 0, "the party actually made decisions");
        let vend = combat.view();
        assert!(
            vend.foes
                .iter()
                .any(|u| u.fallen || u.health_remaining < u.health_max),
            "the foes took damage over the fight"
        );
        let _ = finish_manual_combat(&mut t, place, bestiary, combat);
    }
}
