//! **Board <-> the step machine.** The card-table fight surface over the CANON combat model
//! (`rules::combat`, the eight-step round - see `docs/games/deckbound/combat-round-sequence.md`), with the
//! **cards as the source of truth**:
//!
//! - **Rank and region are pile membership - and both are EARNED, never declared.** `[Arena]` holds one pile
//!   per (ground, rank): your `Vanguard`/`Rearguard` and the foes' `Foe Vanguard`/`Foe Rearguard`
//!   (weapon-derived, fixed), plus `Outriders` (your bodies loose in THEIR line) and `Intruders` (their
//!   bodies loose in YOURS) - the one rank you reach by playing. A body's card physically walks into the
//!   enemy line pile when its crossing RESOLVES, and back out when it withdraws. There is no formation
//!   declaration: a fight opens with both lines seated and face up.
//! - **The `[Steps]` deck is the schedule**: eight cards, Havoc through Advance, top = the current step. Each
//!   step is one declare/reveal **wave**: the eligible party bodies stage orders (an aim, a hold, a
//!   go/stay), and **Commit** reveals the wave - scripted foes declare by `step_policy` - and resolves the
//!   step on the spot. Waves nobody can act in auto-advance and journal as `- skipped`.
//! - **The engine is transient; the cards persist.** Every read seats a `StepState` FROM the cards
//!   (`StepState::resume`), and every Commit writes the resolution back: health/tempo on card detail,
//!   position as pile moves, the round's `struck`/`arrived` commitments as marker lines. Between waves there
//!   is nothing off-table to lose - the grit pile closes at each step, so no scratch state survives a wave.
//! - **The journal is the record**, in the canonical log format (`round N` / `  step K/8: Name` / the
//!   `target`/`bid`/`strike`/`resolve` minor steps), told by the SHARED formatter
//!   ([`rules::combat::narrate`]) from the engine's recorded transcripts - the fight simulator's log and
//!   this journal cannot tell the story differently.

use cardtable_model::{Board, CardId, CardKind, Choice, PileId};
use rules::combat::narrate;
use rules::combat::regions::{Board as Battlefield, MAX_ROUNDS, Rank};
use rules::combat::resolve::{Combatant, Side};
use rules::combat::step_game::{
    STEPS, Step, StepChoice, StepCombat, StepState, step_coord, step_policy,
};
use rules::core::{Game, Outcome as FightOutcome, Solver, Verdict};

/// The top-level zone a fight lives in while it runs.
pub const ARENA: &str = "Arena";

/// The six ground piles: `(label, region, rank)`. Region 0 is the party's ground, 1 the foes'. (The scene
/// folds these into three RANK rows - heroes left of the divider, foes right - reading the symmetry; the
/// piles stay the physical truth of who stands where.)
pub(crate) const GROUND_PILES: [(&str, u8, Rank); 6] = [
    ("Foe Rearguard", 1, Rank::Rearguard),
    ("Foe Vanguard", 1, Rank::Vanguard),
    ("Outriders", 1, Rank::Outrider),
    ("Intruders", 0, Rank::Outrider),
    ("Vanguard", 0, Rank::Vanguard),
    ("Rearguard", 0, Rank::Rearguard),
];

/// The rotating step deck (eight cards, top = current) and its label.
const STEPS_DECK: &str = "Steps";

// ---- constant stats, derived from the source ([Might, Vitality, Grit, Cadence, Finesse]) ----------

struct Stats {
    might: u32,
    vitality: u32,
    grit: u32,
    cadence: u32,
    finesse: u32,
}

fn stats_of(s: [u8; 5]) -> Stats {
    Stats {
        might: s[0] as u32,
        vitality: s[1] as u32,
        grit: s[2] as u32,
        cadence: s[3] as u32,
        finesse: s[4] as u32,
    }
}

fn top_deck(board: &Board, label: &str) -> Option<PileId> {
    board
        .pile(board.root_id())?
        .subpiles()
        .into_iter()
        .find(|&p| board.pile(p).map(|p| p.label.as_str()) == Some(label))
}

/// A sub-pile of `arena` by label (a ground pile or the step deck).
pub(crate) fn sub_pile(board: &Board, arena: PileId, label: &str) -> Option<PileId> {
    board
        .pile(arena)?
        .subpiles()
        .into_iter()
        .find(|&p| board.pile(p).map(|p| p.label.as_str()) == Some(label))
}

fn character_deck(board: &Board, name: &str) -> Option<PileId> {
    board
        .pile(board.root_id())?
        .subpiles()
        .into_iter()
        .find(|&p| {
            board
                .pile(p)
                .and_then(|q| q.reflects())
                .and_then(|c| board.card(c))
                .map(|c| c.front_title())
                == Some(name)
        })
}

/// A hero's stats plus its reach `(melee, ranged)` and area flag, re-derived from the character deck (the
/// source) on every read.
fn hero_stats(board: &Board, name: &str) -> Option<(Stats, bool, bool, bool)> {
    let recipe = board.character_recipe(
        character_deck(board, name)?,
        &deckbound_content::catalog::stat_names(),
    )?;
    let (melee, ranged) = deckbound_content::catalog::ability_reach(&recipe.ability);
    let (_ranged, aoe) = deckbound_content::catalog::ability_shape(&recipe.ability);
    Some((stats_of(recipe.stats), melee, ranged, aoe))
}

fn foe_stats(name: &str) -> Option<(Stats, bool, bool, bool)> {
    let c = deckbound_content::catalog::creature(name)?;
    Some((stats_of(c.stats), c.melee, c.ranged, c.aoe))
}

/// The vitality (max HP, or body count for a horde) of a combatant by name and side.
pub(crate) fn max_health(board: &Board, name: &str, side: Side) -> u32 {
    match side {
        Side::Party => hero_stats(board, name).map(|(s, _, _, _)| s.vitality),
        Side::Foe => foe_stats(name).map(|(s, _, _, _)| s.vitality),
    }
    .unwrap_or(0)
}

// ---- combatant card state (HP / tempo / flags on detail; commitments + staging after) -----------------

#[allow(clippy::too_many_arguments)]
fn detail_lines(
    hp: u32,
    max_hp: u32,
    tempo: u32,
    max_tempo: u32,
    finesse: u32,
    melee: bool,
    ranged: bool,
    area: bool,
) -> Vec<String> {
    // Health and Tempo are both **stacks of cards** you flip, so both read `up / total`. Tempo is LIVE across
    // the whole round now (the step model): what a body spent at an early step it does not have at a late one,
    // and the card is where that fact lives between waves.
    vec![
        format!("Health {hp}/{max_hp}"),
        format!("Tempo {tempo}/{max_tempo}"),
        format!(
            "Finesse {finesse}{}{}{}",
            if melee { " Melee" } else { "" },
            if ranged { " Ranged" } else { "" },
            if area { " Area" } else { "" }
        ),
    ]
}

/// The number of leading detail lines that are the unit's *state*. The round's commitments and the staged
/// order follow them.
const BASE_LINES: usize = 3;

fn num_after(line: &str, prefix: &str) -> u32 {
    line.strip_prefix(prefix)
        .and_then(|s| s.split(['/', ' ']).next())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

/// One party body's **staged order** for the current wave - private until Commit reveals it (the commit is
/// the information boundary). `Aim`/`Hold` at a strike step; `Go`/`Stay` at a movement step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Staged {
    Aim(CardId),
    Hold,
    Go,
    Stay,
}

/// A body's marker lines, decoded: the round's commitments, the selection marks, and the staged order.
/// `aiming` is the mid-gesture state - the body chose a targeted action (the WHAT) and is waiting on the
/// target (the WHOM); it is not yet an answer, and Commit still counts it as owed.
#[derive(Clone, Copy, Default)]
struct Flags {
    struck: bool,
    arrived: bool,
    active: bool,
    aiming: bool,
    staged: Option<Staged>,
}

fn read_flags(d: &[String]) -> Flags {
    let mut f = Flags::default();
    for line in d.iter().skip(BASE_LINES) {
        match line.as_str() {
            "struck" => f.struck = true,
            "arrived" => f.arrived = true,
            "active" => f.active = true,
            "aiming" => f.aiming = true,
            "hold" => f.staged = Some(Staged::Hold),
            "go" => f.staged = Some(Staged::Go),
            "stay" => f.staged = Some(Staged::Stay),
            l => {
                if let Some(id) = l.strip_prefix("aim ") {
                    f.staged = id.trim().parse().ok().map(|n| Staged::Aim(CardId(n)));
                }
            }
        }
    }
    f
}

fn write_flags(board: &mut Board, card: CardId, f: Flags) {
    let Some(d) = board.card(card).map(|c| c.detail().to_vec()) else {
        return;
    };
    let mut lines: Vec<String> = d.into_iter().take(BASE_LINES).collect();
    while lines.len() < BASE_LINES {
        lines.push(String::new());
    }
    if f.struck {
        lines.push("struck".into());
    }
    if f.arrived {
        lines.push("arrived".into());
    }
    if f.active {
        lines.push("active".into());
    }
    if f.aiming {
        lines.push("aiming".into());
    }
    match f.staged {
        Some(Staged::Aim(t)) => lines.push(format!("aim {}", t.0)),
        Some(Staged::Hold) => lines.push("hold".into()),
        Some(Staged::Go) => lines.push("go".into()),
        Some(Staged::Stay) => lines.push("stay".into()),
        None => {}
    }
    let _ = board.set_card_detail(card, lines);
}

/// Read a card's flags, apply `edit`, write them back.
fn edit_flags(board: &mut Board, card: CardId, edit: impl FnOnce(&mut Flags)) {
    let mut f = board
        .card(card)
        .map(|c| read_flags(c.detail()))
        .unwrap_or_default();
    edit(&mut f);
    write_flags(board, card, f);
}

pub(crate) fn staged_of(board: &Board, card: CardId) -> Option<Staged> {
    board.card(card).map(|c| read_flags(c.detail()).staged)?
}

fn active_of(board: &Board, card: CardId) -> bool {
    board
        .card(card)
        .map(|c| read_flags(c.detail()).active)
        .unwrap_or(false)
}

fn aiming_of(board: &Board, card: CardId) -> bool {
    board
        .card(card)
        .map(|c| read_flags(c.detail()).aiming)
        .unwrap_or(false)
}

/// Read one combatant card into a rules [`Combatant`] - constant stats from the source, mutable state from
/// detail. Region and rank come from the pile it stands in, supplied by the caller.
pub(crate) fn read_combatant(board: &Board, card: CardId) -> Option<Combatant> {
    let c = board.card(card)?;
    let name = c.front_title().to_string();
    let side = match c.card_type() {
        "unit" => Side::Party,
        "foe" => Side::Foe,
        _ => return None,
    };
    let (stats, melee, ranged, aoe) = match side {
        Side::Party => hero_stats(board, &name)?,
        Side::Foe => foe_stats(&name)?,
    };
    let d = c.detail();
    let hp = d
        .first()
        .map(|l| num_after(l, "Health "))
        .unwrap_or(stats.vitality);
    let tempo = d
        .get(1)
        .map(|l| num_after(l, "Tempo "))
        .unwrap_or(stats.cadence);
    let horde =
        side == Side::Foe && deckbound_content::catalog::creature(&name).is_some_and(|c| c.horde);
    let mut u = Combatant::from_stats(
        &name,
        side,
        [
            stats.might as u8,
            stats.vitality as u8,
            stats.grit as u8,
            stats.cadence as u8,
            stats.finesse as u8,
        ],
        0,
        melee,
        ranged,
    );
    u.aoe = aoe;
    u.horde = horde;
    u.tempo = tempo;
    u.health = hp;
    u.fallen = hp == 0;
    Some(u)
}

// ---- seating the engine from the cards -----------------------------------------------------------------

/// Everything the cards say about the fight, in engine terms - the bridge's read half.
pub(crate) struct Seated {
    pub(crate) cards: Vec<CardId>,
    pub(crate) state: StepState,
}

/// The combatant cards in ground-pile order with their (region, rank) read off pile membership.
fn read_units(board: &Board, arena: PileId) -> (Vec<CardId>, Vec<Combatant>, Vec<u8>, Vec<Rank>) {
    let mut cards = Vec::new();
    let mut units = Vec::new();
    let mut regions = Vec::new();
    let mut ranks = Vec::new();
    for (label, region, rank) in GROUND_PILES {
        let Some(p) = sub_pile(board, arena, label) else {
            continue;
        };
        for c in board.content_cards(p) {
            if let Some(u) = read_combatant(board, c) {
                cards.push(c);
                units.push(u);
                regions.push(region);
                ranks.push(rank);
            }
        }
    }
    (cards, units, regions, ranks)
}

fn read_round(board: &Board, arena: PileId) -> usize {
    board
        .content_cards(arena)
        .into_iter()
        .find(|&c| board.card(c).map(|k| k.card_type()) == Some("round"))
        .and_then(|c| board.card(c))
        .map(|c| num_after(c.front_title(), "Round ") as usize)
        .unwrap_or(1)
}

fn read_step(board: &Board, arena: PileId) -> Step {
    let top = sub_pile(board, arena, STEPS_DECK)
        .and_then(|d| board.pile(d).and_then(|p| p.cards().first().copied()))
        .and_then(|c| board.card(c).map(|k| k.front_title().to_string()));
    STEPS
        .into_iter()
        .find(|&s| Some(step_coord(s).1) == top.as_deref())
        .unwrap_or(Step::Havoc)
}

/// Seat the engine from the cards. The mutating ops keep the invariant that the card step's wave has a
/// pending decision (or the fight is over), so seating on the read path never advances anything.
pub(crate) fn seat(board: &Board, arena: PileId) -> Option<Seated> {
    let (cards, units, regions, ranks) = read_units(board, arena);
    if cards.is_empty() {
        return None;
    }
    let (mut struck, mut arrived) = (vec![false; cards.len()], vec![false; cards.len()]);
    for (i, &c) in cards.iter().enumerate() {
        if let Some(k) = board.card(c) {
            let f = read_flags(k.detail());
            struck[i] = f.struck;
            arrived[i] = f.arrived;
        }
    }
    let state = StepState::resume(
        units,
        regions,
        ranks,
        read_round(board, arena),
        read_step(board, arena),
        struck,
        arrived,
    );
    Some(Seated { cards, state })
}

// ---- the journal ---------------------------------------------------------------------------------------

/// Append one journal line under `round`. Stored on a loose Virtual card so the record is part of the board
/// like everything else (Back rewinds it for free; the outcome pile reads it back).
fn note(board: &mut Board, arena: PileId, round: usize, line: String) {
    let line = format!("{round}|{line}");
    if let Some(c) = board
        .content_cards(arena)
        .into_iter()
        .find(|&c| board.card(c).map(|k| k.card_type()) == Some("eventlog"))
    {
        let mut d = board
            .card(c)
            .map(|k| k.detail().to_vec())
            .unwrap_or_default();
        d.push(line);
        let _ = board.set_card_detail(c, d);
    } else if let Ok(c) = board.add_card(
        arena,
        cardtable_model::Face::Down {
            title: "events".into(),
        },
        None,
    ) {
        let _ = board.set_card_kind(c, CardKind::Virtual);
        let _ = board.set_card_type(c, "eventlog");
        let _ = board.set_card_detail(c, vec![line]);
    }
}

fn read_events(board: &Board, arena: PileId) -> Vec<String> {
    board
        .content_cards(arena)
        .into_iter()
        .filter(|&c| board.card(c).map(|k| k.card_type()) == Some("eventlog"))
        .flat_map(|c| {
            board
                .card(c)
                .map(|k| k.detail().to_vec())
                .unwrap_or_default()
        })
        .collect()
}

/// One round of the journal, already formatted (the lines were written in the canonical format as they
/// happened). The **one** formatter chain - the live panel and the outcome card read the same lines.
pub(crate) fn round_log(board: &Board, arena: PileId, round: u32) -> Vec<String> {
    let want = format!("{round}|");
    read_events(board, arena)
        .into_iter()
        .filter_map(|e| e.strip_prefix(&want).map(|s| s.to_string()))
        .collect()
}

/// The rounds the journal has anything to say about, in order.
fn rounds_logged(board: &Board, arena: PileId) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    for e in read_events(board, arena) {
        if let Some(r) = e.split('|').next().and_then(|r| r.parse::<u32>().ok())
            && !out.contains(&r)
        {
            out.push(r);
        }
    }
    out
}

fn clear_events(board: &mut Board, arena: PileId) {
    let stale: Vec<CardId> = board
        .content_cards(arena)
        .into_iter()
        .filter(|&c| board.card(c).map(|k| k.card_type()) == Some("eventlog"))
        .collect();
    for c in stale {
        let _ = board.remove_card(c);
    }
}

/// The last wave the journal printed a header for, kept on the meta card so headers (and `- skipped` fills)
/// stay correct across commits: `(round, step index)`.
fn read_wave_mark(board: &Board, arena: PileId) -> Option<(usize, usize)> {
    let meta = meta_card(board, arena)?;
    board.card(meta)?.detail().iter().find_map(|l| {
        let rest = l.strip_prefix("wave ")?;
        let mut it = rest.split_whitespace();
        Some((it.next()?.parse().ok()?, it.next()?.parse().ok()?))
    })
}

fn write_wave_mark(board: &mut Board, arena: PileId, round: usize, idx: usize) {
    let Some(meta) = meta_card(board, arena) else {
        return;
    };
    let mut d = board
        .card(meta)
        .map(|k| k.detail().to_vec())
        .unwrap_or_default();
    d.retain(|l| !l.starts_with("wave "));
    d.push(format!("wave {round} {idx}"));
    let _ = board.set_card_detail(meta, d);
}

fn step_idx(s: Step) -> usize {
    STEPS.iter().position(|&x| x == s).unwrap_or(0)
}

/// Print the wave header for `(round, step)`, filling in everything since the last header: a `round N`
/// marker when the round advanced, and a `- skipped` line for every wave nobody could act in. The same shape
/// as the fight simulator's log, via the same `step_coord` naming.
fn log_wave_header(board: &mut Board, arena: PileId, round: usize, step: Step) {
    let target = step_idx(step);
    let (mut r, mut i) = match read_wave_mark(board, arena) {
        Some((r0, i0)) => (r0, i0 + 1),
        None => (0, STEPS.len()),
    };
    while (r, i) < (round, target) {
        if i >= STEPS.len() {
            r += 1;
            i = 0;
            note(board, arena, r, format!("round {r}"));
            continue;
        }
        let (k, name) = step_coord(STEPS[i]);
        note(board, arena, r, format!("  step {k}/8: {name} - skipped"));
        i += 1;
    }
    let (k, name) = step_coord(step);
    note(board, arena, round, format!("  step {k}/8: {name}"));
    write_wave_mark(board, arena, round, target);
}

// ---- the declaration labels (the commit lines' vocabulary) ---------------------------------------------

/// What a declaration does at this step, in words - the commit line's label, matching the fight simulator's.
fn describe(step: Step, units: &[Combatant], choice: &StepChoice) -> String {
    let who = |t: usize| {
        let u = &units[t];
        let kind = if u.horde { "bodies" } else { "hp" };
        format!("{} ({} {kind})", u.name, u.health)
    };
    match (step, choice) {
        (Step::Havoc, StepChoice::Strike(Some(t))) => format!("Melee {}", who(*t)),
        (Step::Skirmish, StepChoice::Strike(Some(t))) => format!("Skirmish {}", who(*t)),
        (Step::Volley, StepChoice::Strike(Some(t))) => format!("Volley the crossing {}", who(*t)),
        (Step::Raid, StepChoice::Strike(Some(t))) => format!("Raid {}", who(*t)),
        (Step::Advance, StepChoice::Strike(Some(t))) => {
            format!("Advance on the exposed {}", who(*t))
        }
        (_, StepChoice::Strike(Some(t))) => format!("Strike {}", who(*t)),
        (_, StepChoice::Strike(None)) => "Hold (pass this step)".to_string(),
        (Step::Withdraw, StepChoice::Move(true)) => "Withdraw to your own line".to_string(),
        (Step::Withdraw, StepChoice::Move(false)) => "Stay loose in their ranks".to_string(),
        (Step::Cross, StepChoice::Move(true)) => "Cross into their line".to_string(),
        (Step::Cross, StepChoice::Move(false)) => "Hold the line (do not cross)".to_string(),
        (_, StepChoice::Move(go)) => if *go { "Go" } else { "Stay" }.to_string(),
    }
}

// ---- opening a fight -----------------------------------------------------------------------------------

/// Open a fight at `place`: build the `[Arena]` with the six ground piles, seat every stationed hero and
/// instantiated foe **directly in its weapon rank** (no formation declaration - rank is derived, position is
/// earned), install the step deck at round 1 - Havoc, journal the opening, and auto-advance to the first
/// party decision.
pub fn open_fight(board: &mut Board, place: PileId) -> Option<PileId> {
    let bestiary = top_deck(board, "Bestiary")?;
    let root = board.root_id();
    let arena = board.add_pile(root, ARENA).ok()?;
    for (label, _, _) in GROUND_PILES {
        let _ = board.add_pile(arena, label);
    }

    // A hidden meta card remembers the originating place (for teardown) and the journal's wave mark.
    if let Ok(meta) = board.add_card(
        arena,
        cardtable_model::Face::Down {
            title: format!("place {}", place.0),
        },
        None,
    ) {
        let _ = board.set_card_kind(meta, CardKind::Virtual);
        let _ = board.set_card_type(meta, "arena-meta");
    }

    // Heroes: each stationed hero seats straight into its weapon rank - ranged-only at the back, everything
    // else at the front. Nothing to declare; the fight opens on the first decision.
    let heroes: Vec<CardId> = board
        .content_cards(place)
        .into_iter()
        .filter(|&c| board.card(c).map(|k| k.card_type()) == Some("hero"))
        .collect();
    for card in heroes {
        let name = board.card(card).map(|c| c.front_title().to_string())?;
        if let Some((stats, melee, ranged, aoe)) = hero_stats(board, &name) {
            let label = home_pile_label(Side::Party, melee, ranged);
            let dest = sub_pile(board, arena, label)?;
            let at = board.pile(dest).map_or(0, |p| p.cards().len());
            let _ = board.move_card(card, dest, at);
            let _ = board.set_card_type(card, "unit");
            let _ = board.set_card_detail(
                card,
                detail_lines(
                    stats.vitality,
                    stats.vitality,
                    stats.cadence,
                    stats.cadence,
                    stats.finesse,
                    melee,
                    ranged,
                    aoe,
                ),
            );
        }
    }

    // Foes: instantiate the encounter roster from the Bestiary straight into their weapon ranks, face up.
    // There is no muster: with no formation to declare, there is nothing to hide and no reveal to stage.
    let label = board.pile(place)?.label.clone();
    let foes = board
        .instantiate_from_bank(
            bestiary,
            arena,
            &deckbound_content::catalog::encounter_roster(&label),
        )
        .ok()?;
    for card in foes {
        let name = board.card(card).map(|c| c.front_title().to_string())?;
        if let Some((stats, melee, ranged, aoe)) = foe_stats(&name) {
            let _ = board.set_card_type(card, "foe");
            let _ = board.set_card_detail(
                card,
                detail_lines(
                    stats.vitality,
                    stats.vitality,
                    stats.cadence,
                    stats.cadence,
                    stats.finesse,
                    melee,
                    ranged,
                    aoe,
                ),
            );
            let dest = sub_pile(board, arena, home_pile_label(Side::Foe, melee, ranged))?;
            let at = board.pile(dest).map_or(0, |p| p.cards().len());
            let _ = board.move_card(card, dest, at);
        }
    }

    install_step_deck(board, arena);
    set_round_card(board, arena, 1);
    let _ = board.focus(arena);
    // Round 1 opens at Havoc, and nobody is an outrider yet: advance (journalling the skipped waves) to the
    // first wave with a party decision in it.
    run_engine(board, arena, false);
    Some(arena)
}

/// The home pile for a body of `side` with this reach - its weapon rank on its own ground.
fn home_pile_label(side: Side, melee: bool, ranged: bool) -> &'static str {
    let back = ranged && !melee;
    match (side, back) {
        (Side::Party, false) => "Vanguard",
        (Side::Party, true) => "Rearguard",
        (Side::Foe, false) => "Foe Vanguard",
        (Side::Foe, true) => "Foe Rearguard",
    }
}

/// (Re)install the step deck at round 1 - Havoc on top. The top card of the deck IS the current step; a
/// step transition moves the top card to the bottom, and a full cycle is a round.
fn install_step_deck(board: &mut Board, arena: PileId) {
    if let Some(p) = sub_pile(board, arena, STEPS_DECK) {
        let _ = board.remove_pile(p);
    }
    if let Ok(deck) = board.add_pile(arena, STEPS_DECK) {
        for s in STEPS {
            let (k, name) = step_coord(s);
            if let Ok(card) = board.add_card(
                deck,
                cardtable_model::Face::Up {
                    title: name.to_string(),
                },
                None,
            ) {
                let _ = board.set_card_kind(card, CardKind::Virtual);
                let _ = board.set_card_type(card, "step");
                let _ = board.set_card_detail(card, vec![format!("Step {k} of 8")]);
            }
        }
    }
}

/// Rotate the step deck until `step` is on top (a transition per move; a wrap is a new round).
fn set_step_deck(board: &mut Board, arena: PileId, step: Step) {
    for _ in 0..STEPS.len() {
        if read_step(board, arena) == step {
            return;
        }
        let Some(deck) = sub_pile(board, arena, STEPS_DECK) else {
            return;
        };
        let cards = board.pile(deck).map(|p| p.cards()).unwrap_or_default();
        if let Some(&top) = cards.first() {
            let _ = board.move_card(top, deck, cards.len());
        }
    }
}

fn set_round_card(board: &mut Board, arena: PileId, round: usize) {
    let title = format!("Round {round}");
    if let Some(c) = board
        .content_cards(arena)
        .into_iter()
        .find(|&c| board.card(c).map(|k| k.card_type()) == Some("round"))
    {
        let _ = board.set_face(c, cardtable_model::Face::Up { title });
    } else if let Ok(c) = board.add_card(arena, cardtable_model::Face::Up { title }, None) {
        let _ = board.set_card_kind(c, CardKind::Virtual);
        let _ = board.set_card_type(c, "round");
    }
}

// ---- the outcome ---------------------------------------------------------------------------------------

/// How a fight ended. A battle is decided by breaking a line - or, failing that, by the clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Every foe is fallen.
    Victory,
    /// Every hero is fallen.
    Defeat,
    /// Neither line broke within the round cap: the two sides disengage.
    Draw,
}

/// Whether the fight is over, and how - read straight off the cards (a side with no standing body has
/// lost; a round counter past the cap is the draw).
pub fn outcome(board: &Board, arena: PileId) -> Option<Outcome> {
    let (_, units, _, _) = read_units(board, arena);
    if units.is_empty() {
        return None;
    }
    let party_alive = units.iter().any(|u| u.side == Side::Party && !u.fallen);
    let foes_alive = units.iter().any(|u| u.side == Side::Foe && !u.fallen);
    match (party_alive, foes_alive) {
        (false, _) => Some(Outcome::Defeat),
        (true, false) => Some(Outcome::Victory),
        (true, true) if read_round(board, arena) > MAX_ROUNDS => Some(Outcome::Draw),
        (true, true) => None,
    }
}

// ---- the wave: who is asked, what is staged, what is owed ----------------------------------------------

/// The current wave from the party's point of view: which bodies are asked, and what each has staged.
pub(crate) struct Wave {
    pub(crate) cards: Vec<CardId>,
    pub(crate) units: Vec<Combatant>,
    /// Each body's rank, for the scene's rank rows (region is legible from side + rank).
    pub(crate) ranks: Vec<Rank>,
    pub(crate) round: usize,
    pub(crate) step: Step,
    /// Asked this wave (eligible party bodies).
    pub(crate) asked: Vec<bool>,
    /// The staged order per body (party only; `None` = not yet answered).
    pub(crate) staged: Vec<Option<Staged>>,
    /// The body the choice cards address: the selected one, else the first asked-and-unanswered.
    pub(crate) focus: Option<usize>,
    /// Whether the focused body is mid-gesture: it chose a targeted action (the WHAT) and is waiting on the
    /// WHOM. While true, its legal targets carry the animated invitation cue.
    pub(crate) aiming: bool,
    /// The focused body's legal strike targets (empty on movement steps).
    pub(crate) targets: Vec<usize>,
}

pub(crate) fn wave(board: &Board, arena: PileId) -> Option<Wave> {
    let seated = seat(board, arena)?;
    let state = &seated.state;
    let cards = seated.cards;
    let units: Vec<Combatant> = state.board().units.clone();
    let ranks: Vec<Rank> = state.board().ranks.clone();
    let asked: Vec<bool> = (0..units.len())
        .map(|i| units[i].side == Side::Party && state.is_eligible(i))
        .collect();
    let staged: Vec<Option<Staged>> = cards.iter().map(|&c| staged_of(board, c)).collect();
    // The focus is ONLY ever the body the player explicitly selected - there is no auto-fallback. The
    // selection click is informational by design: the player knows who is being commanded because the
    // player did the selecting.
    let focus = (0..units.len()).find(|&i| asked[i] && active_of(board, cards[i]));
    let targets = focus.map(|i| state.targets(i)).unwrap_or_default();
    let aiming = focus.is_some_and(|i| aiming_of(board, cards[i]));
    Some(Wave {
        cards,
        units,
        ranks,
        round: state.round(),
        step: state.step(),
        asked,
        staged,
        focus,
        aiming,
        targets,
    })
}

/// A decision the player owes before this wave can be committed, named for the Commit control. `None` means
/// Commit is live. **Nothing is decided by default**: an asked body that has not answered has not chosen to
/// pass - it has chosen nothing, and committing would silently choose for it.
pub fn pending_decision(board: &Board, arena: PileId) -> Option<String> {
    let w = wave(board, arena)?;
    let i = (0..w.units.len()).find(|&i| w.asked[i] && w.staged[i].is_none())?;
    Some(if w.focus == Some(i) && w.aiming {
        format!("{} is picking a target", w.units[i].name)
    } else {
        format!("{} has no orders", w.units[i].name)
    })
}

/// The label for the Commit control: the outcome once decided, the owed order while one is missing, else
/// Commit - the step deck already names the wave.
pub fn commit_label(board: &Board, arena: PileId) -> String {
    match outcome(board, arena) {
        Some(Outcome::Victory) => "Victory - leave".to_string(),
        Some(Outcome::Defeat) => "Defeat - leave".to_string(),
        Some(Outcome::Draw) => "Draw - leave".to_string(),
        None => match pending_decision(board, arena) {
            Some(owed) => owed,
            None => "Commit".to_string(),
        },
    }
}

// ---- the decision surface: choice cards, taps, drags ---------------------------------------------------

/// What taking a choice card does to the focused body's order-in-progress.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChoiceAction {
    /// Complete an order (a target picked, a hold, a movement answer).
    Stage(Staged),
    /// Choose the targeted action (the WHAT) - enter targeting; the WHOM completes it.
    BeginAim,
    /// Drop the chosen action and return to the order menu.
    CancelAim,
}

/// The step's strike verb, for the WHAT button and the target buttons.
fn step_verb(step: Step) -> &'static str {
    match step {
        Step::Havoc => "Melee",
        Step::Skirmish => "Skirmish",
        Step::Volley => "Volley",
        Step::Raid => "Raid",
        Step::Advance => "Advance on",
        _ => "Strike",
    }
}

/// **Every decision on offer right now, as cards** - the focused body's legal declarations for this wave,
/// each carrying what it does. A tap on the table only says *which* body; the order itself is one of these.
pub(crate) fn step_choices(board: &Board, arena: PileId) -> Vec<(Choice, ChoiceAction)> {
    let Some(w) = wave(board, arena) else {
        return Vec::new();
    };
    // Nothing selected: no buttons - the board is the menu (the unordered heroes carry the ring), and
    // the selection click is the player's own act of choosing WHO.
    let Some(i) = w.focus else {
        return Vec::new();
    };
    let mut out = Vec::new();
    match w.step {
        Step::Withdraw | Step::Cross => {
            let (go, stay) = if w.step == Step::Withdraw {
                (
                    (
                        "Withdraw to your own line",
                        "rejoin your line at weapon rank - free; standing the havoc was the price",
                    ),
                    (
                        "Stay loose in their ranks",
                        "keep wreaking havoc inside their formation",
                    ),
                )
            } else {
                (
                    (
                        "Cross into their line",
                        "walk uncontested - you declared no line strike; you land as an Outrider past their screen",
                    ),
                    (
                        "Hold the line (do not cross)",
                        "stay in formation; you can still swing at the Assault",
                    ),
                )
            };
            out.push((
                Choice::new(go.0, go.1).chosen(w.staged[i] == Some(Staged::Go)),
                ChoiceAction::Stage(Staged::Go),
            ));
            out.push((
                Choice::new(stay.0, stay.1).chosen(w.staged[i] == Some(Staged::Stay)),
                ChoiceAction::Stage(Staged::Stay),
            ));
        }
        _ if w.aiming => {
            // The WHOM: one button per ringed target (the board's rings and these buttons are the same
            // menu), plus the way back out of the gesture.
            for &t in &w.targets {
                let label = describe(w.step, &w.units, &StepChoice::Strike(Some(t)));
                let text = strike_consequence(&w.units, i, t);
                out.push((
                    Choice::new(label, text),
                    ChoiceAction::Stage(Staged::Aim(w.cards[t])),
                ));
            }
            out.push((
                Choice::new("Cancel", "put the action back - choose again"),
                ChoiceAction::CancelAim,
            ));
        }
        _ => {
            // The WHAT: the step's verb (entering targeting - the ringed targets complete it), or the pass.
            let verb = step_verb(w.step);
            let n = w.targets.len();
            out.push((
                Choice::new(
                    format!("{verb}..."),
                    format!("pick a target: {n} in reach - they light up on the board",),
                )
                .chosen(matches!(w.staged[i], Some(Staged::Aim(_)))),
                ChoiceAction::BeginAim,
            ));
            out.push((
                Choice::new(
                    "Hold (pass this step)",
                    "keep your tempo for a later step - or for the crossing (a line strike bars it)",
                )
                .chosen(w.staged[i] == Some(Staged::Hold)),
                ChoiceAction::Stage(Staged::Hold),
            ));
        }
    }
    out
}

/// What aiming at `t` buys, in one line: the bid the reach will make, the dodge it must clear, and the
/// damage the blows would bank - both compared numbers on the page, same as the log will say.
fn strike_consequence(units: &[Combatant], a: usize, t: usize) -> String {
    let (u, foe) = (&units[a], &units[t]);
    if u.aoe {
        return format!(
            "an area strike: one tempo, unevadable, every enemy in {}'s tier",
            foe.name
        );
    }
    let reach = rules::combat::regions::reach_cards(units, a, t);
    let strikes = 1 + u.tempo.saturating_sub(reach);
    let dmg = u.might.saturating_sub(foe.armor) * strikes;
    format!(
        "flip {reach} tempo to reach (dodge floor {}), then up to {strikes} blows = {dmg} damage vs Grit {}",
        foe.tempo * foe.finesse.max(1),
        foe.grit
    )
}

/// The choice cards for the current wave, for the renderer.
pub fn scene_choices(board: &Board, arena: PileId) -> Vec<Choice> {
    step_choices(board, arena)
        .into_iter()
        .map(|(c, _)| c)
        .collect()
}

/// Take the scene's choice at `index` - stage the order on the focused body's card. Staging only; nothing
/// resolves (or is revealed) until Commit.
pub fn choose(board: &mut Board, index: usize) {
    let Some(arena) = find_arena(board) else {
        return;
    };
    let Some(w) = wave(board, arena) else {
        return;
    };
    let Some(i) = w.focus else {
        return;
    };
    let actions = step_choices(board, arena);
    let Some((_, action)) = actions.get(index) else {
        return;
    };
    let card = w.cards[i];
    match action {
        ChoiceAction::Stage(s) => edit_flags(board, card, |f| {
            f.staged = Some(*s);
            f.aiming = false;
            f.active = false;
        }),
        ChoiceAction::BeginAim => edit_flags(board, card, |f| {
            f.staged = None;
            f.aiming = true;
        }),
        ChoiceAction::CancelAim => edit_flags(board, card, |f| f.aiming = false),
    }
}

/// **A tap says which, never what.** Tapping an asked party body selects it (the choice cards become its
/// orders); tapping one that already answered clears its order (re-asks it); tapping a legal enemy target
/// while a body is focused is the shortcut for aiming at it.
pub fn handle_tap(board: &mut Board, card: CardId) {
    let Some(arena) = find_arena(board) else {
        return;
    };
    let Some(w) = wave(board, arena) else {
        return;
    };
    let Some(i) = w.cards.iter().position(|&c| c == card) else {
        return;
    };
    if w.units[i].side == Side::Party {
        if !w.asked[i] {
            return;
        }
        if w.staged[i].is_some() {
            // Re-ask: clear the staged order, and make it the focus.
            edit_flags(board, card, |f| {
                f.staged = None;
                f.active = true;
            });
        } else if w.focus == Some(i) {
            // Tapping the selected hero again puts it back - deselected, gesture dropped.
            edit_flags(board, card, |f| {
                f.active = false;
                f.aiming = false;
            });
        } else {
            // Select: move the active mark here.
            for (j, &c) in w.cards.iter().enumerate() {
                if w.units[j].side != Side::Party {
                    continue;
                }
                edit_flags(board, c, |f| f.active = j == i);
            }
        }
        return;
    }
    // A foe tap completes the gesture in progress: it is the WHOM of a chosen action, so it only means
    // something while the focused body is AIMING (the ring is the tap's affordance).
    if let Some(f) = w.focus
        && w.aiming
        && w.targets.contains(&i)
    {
        let target = w.cards[i];
        edit_flags(board, w.cards[f], |flags| {
            flags.staged = Some(Staged::Aim(target));
            flags.aiming = false;
            flags.active = false;
        });
    }
}

/// The movement-step drag: dropping an asked body onto the pile its move would land it in stages `go`
/// (position is EARNED - the card walks at resolution, so the drop stages the intent and the card settles
/// back). Any other drop is a no-op.
pub fn assign(board: &mut Board, unit: CardId, to: PileId) {
    let Some(arena) = find_arena(board) else {
        return;
    };
    let Some(w) = wave(board, arena) else {
        return;
    };
    let Some(i) = w.cards.iter().position(|&c| c == unit) else {
        return;
    };
    if !w.asked[i] {
        return;
    }
    let dest_label = match w.step {
        Step::Cross => "Outriders",
        Step::Withdraw => home_pile_label(Side::Party, w.units[i].melee, w.units[i].ranged),
        _ => return,
    };
    if sub_pile(board, arena, dest_label) == Some(to) {
        edit_flags(board, unit, |f| {
            f.staged = Some(Staged::Go);
            f.aiming = false;
        });
    }
}

/// Whether `card` is a combatant in the arena (a legal tap target).
pub fn is_combatant(board: &Board, arena: PileId, card: CardId) -> bool {
    matches!(
        board.card(card).map(|k| k.card_type()),
        Some("unit") | Some("foe")
    ) && GROUND_PILES.iter().any(|(label, _, _)| {
        sub_pile(board, arena, label)
            .and_then(|p| board.pile(p))
            .is_some_and(|p| p.cards().contains(&card))
    })
}

/// Whether `pile` is one of the arena's ground piles (a legal movement-stage drop target).
pub fn is_ground_pile(board: &Board, arena: PileId, pile: PileId) -> bool {
    GROUND_PILES
        .iter()
        .any(|(label, _, _)| sub_pile(board, arena, label) == Some(pile))
}

// ---- committing a wave ---------------------------------------------------------------------------------

/// **Commit the current wave**: reveal the staged party orders to the engine in cursor order, let the
/// scripted foes declare (`step_policy`), resolve every step that completes, journal the whole thing (commit
/// lines, then the shared narration), write the results back to the cards, and stop at the next party
/// decision (or the fight's end). Returns whether the fight is over.
pub fn commit(board: &mut Board, arena: PileId) -> bool {
    run_engine(board, arena, true);
    outcome(board, arena).is_some()
}

/// The engine loop behind [`commit`] and the auto-advance: feed declarations until a party body needs an
/// order that is not staged (`use_staged` = whether staged orders are consumed; the auto path has none).
fn run_engine(board: &mut Board, arena: PileId, use_staged: bool) {
    let Some(seated) = seat(board, arena) else {
        return;
    };
    let cards = seated.cards;
    let mut state = seated.state;
    state.set_record(true);
    let mut prev_board: Battlefield = state.board().clone();
    let staged: Vec<Option<Staged>> = cards.iter().map(|&c| staged_of(board, c)).collect();
    let mut consumed = vec![false; cards.len()];

    loop {
        // Drain and journal any steps that resolved since the last declaration.
        let logs = state.take_transcript();
        if !logs.is_empty() {
            let round = read_wave_round(board, arena);
            for line in narrate::narrate(&prev_board, &logs) {
                // The wave header was already printed with its commit lines; narrate's own header would
                // double it. Its event lines land under ours.
                if !line.trim_start().starts_with("step ") {
                    note(board, arena, round, line);
                }
            }
            prev_board = state.board().clone();
        }
        if StepCombat::outcome(&state).is_some() {
            break;
        }
        let Some(i) = state.deciding() else {
            break;
        };
        let choice = if state.board().units[i].side == Side::Foe {
            step_policy(&state, i)
        } else if use_staged && !consumed[i] && staged[i].is_some() {
            consumed[i] = true;
            match staged[i] {
                Some(Staged::Aim(t)) => StepChoice::Strike(cards.iter().position(|&c| c == t)),
                Some(Staged::Hold) => StepChoice::Strike(None),
                Some(Staged::Go) => StepChoice::Move(true),
                Some(Staged::Stay) => StepChoice::Move(false),
                None => unreachable!(),
            }
        } else {
            break; // a party decision with nothing staged: the player's turn
        };
        // The wave header (with its round marker and skipped-wave fills). The app's journal is the
        // MECHANICAL record only - the commit lines are the simulator's; here the staged orders are
        // legible on the cards before Commit, and the narration says what they became.
        let (round, step) = (state.round(), state.step());
        if read_wave_mark(board, arena) != Some((round, step_idx(step))) {
            log_wave_header(board, arena, round, step);
        }
        state = StepCombat::apply(&state, &choice);
    }

    write_back(board, arena, &cards, &state);
    if let Some(o) = StepCombat::outcome(&state) {
        let label = match o {
            FightOutcome::Win => "Victory",
            FightOutcome::Loss => "Defeat",
            FightOutcome::Draw => "Draw",
        };
        note(
            board,
            arena,
            state.round().min(MAX_ROUNDS),
            format!("========== {label} =========="),
        );
    }
}

/// The round the journal is currently writing into (the wave mark's round, else the round card).
fn read_wave_round(board: &Board, arena: PileId) -> usize {
    read_wave_mark(board, arena)
        .map(|(r, _)| r)
        .unwrap_or_else(|| read_round(board, arena))
}

/// Write the engine's state back to the cards: health/tempo on detail, position as pile moves, the round's
/// commitments as marker lines, staged orders cleared, the step deck and round card advanced.
fn write_back(board: &mut Board, arena: PileId, cards: &[CardId], state: &StepState) {
    let b = state.board();
    for (i, &card) in cards.iter().enumerate() {
        let u = &b.units[i];
        let max_hp = max_health(board, &u.name, u.side).max(u.health);
        let _ = board.set_card_detail(
            card,
            detail_lines(
                u.health, max_hp, u.tempo, u.cadence, u.finesse, u.melee, u.ranged, u.aoe,
            ),
        );
        write_flags(
            board,
            card,
            Flags {
                struck: state.struck_flag(i),
                arrived: state.arrived_flag(i),
                ..Flags::default()
            },
        );
        // Position is earned: the card walks to the pile its (region, rank) says.
        let label = GROUND_PILES
            .iter()
            .find(|&&(_, region, rank)| region == b.regions[i] && rank == b.ranks[i])
            .map(|&(l, _, _)| l);
        if let Some(label) = label
            && let Some(dest) = sub_pile(board, arena, label)
            && !board.pile(dest).is_some_and(|p| p.cards().contains(&card))
        {
            let at = board.pile(dest).map_or(0, |p| p.cards().len());
            let _ = board.move_card(card, dest, at);
        }
    }
    set_step_deck(board, arena, state.step());
    set_round_card(board, arena, state.round());
}

// ---- the outlooks (where each choice leads) ------------------------------------------------------------

/// Where each choice on offer **leads** - `Winnable` / `Evaluating` / `Doomed` - index-aligned with
/// [`scene_choices`], computed with the generic solver over the SAME game the balance gate asserts. The
/// candidate is applied on top of the orders already staged (in cursor order), and everything undecided is
/// left free for the search. It **marks, never bars**: a doomed move stays fully playable. `budget` is a
/// node allowance per call; running out is `Evaluating`, and the next frame picks up with the memo warm.
pub fn choice_outlooks(
    board: &Board,
    arena: PileId,
    solver: &mut Solver<StepCombat>,
    budget: u64,
) -> Vec<cardtable_model::Outlook> {
    use cardtable_model::Outlook;
    let choices = step_choices(board, arena);
    if choices.is_empty() {
        return Vec::new();
    }
    let Some(w) = wave(board, arena) else {
        return vec![Outlook::Unknown; choices.len()];
    };
    let Some(seated) = seat(board, arena) else {
        return vec![Outlook::Unknown; choices.len()];
    };
    let Some(focus) = w.focus else {
        return vec![Outlook::Unknown; choices.len()];
    };

    // Apply the already-staged orders that come BEFORE the focus in cursor order; stop at the first gap (the
    // engine cannot skip a declaration, so a candidate past a gap is unanswerable this frame).
    let mut base = seated.state;
    loop {
        let Some(i) = base.deciding() else {
            return vec![Outlook::Unknown; choices.len()];
        };
        if i == focus {
            break;
        }
        let c = if base.board().units[i].side == Side::Foe {
            step_policy(&base, i)
        } else {
            match w.staged[i] {
                Some(Staged::Aim(t)) => StepChoice::Strike(w.cards.iter().position(|&c| c == t)),
                Some(Staged::Hold) => StepChoice::Strike(None),
                Some(Staged::Go) => StepChoice::Move(true),
                Some(Staged::Stay) => StepChoice::Move(false),
                None => return vec![Outlook::Unknown; choices.len()],
            }
        };
        base = StepCombat::apply(&base, &c);
    }

    let mut left = budget;
    let mut out = Vec::with_capacity(choices.len());
    for (_, action) in &choices {
        let candidate = match action {
            ChoiceAction::Stage(Staged::Aim(t)) => {
                StepChoice::Strike(w.cards.iter().position(|&c| c == *t))
            }
            ChoiceAction::Stage(Staged::Hold) => StepChoice::Strike(None),
            ChoiceAction::Stage(Staged::Go) => StepChoice::Move(true),
            ChoiceAction::Stage(Staged::Stay) => StepChoice::Move(false),
            // The gesture beats (choose the verb / put it back) decide nothing by themselves - the verdicts
            // belong to the completions they lead to.
            ChoiceAction::BeginAim | ChoiceAction::CancelAim => {
                out.push(Outlook::Unknown);
                continue;
            }
        };
        let next = StepCombat::apply(&base, &candidate);
        let before = solver.nodes();
        solver.grant(left);
        let v = solver.verdict(&next);
        left = left.saturating_sub(solver.nodes().saturating_sub(before));
        out.push(match v {
            Verdict::Winnable => Outlook::Winnable,
            Verdict::Doomed => Outlook::Doomed,
            Verdict::Evaluating => Outlook::Evaluating,
        });
    }
    out
}

// ---- teardown ------------------------------------------------------------------------------------------

/// **The game-side authority for "is a fight modal right now".** The arena is active whenever it exists.
pub fn find_arena(board: &Board) -> Option<PileId> {
    top_deck(board, ARENA)
}

fn meta_card(board: &Board, arena: PileId) -> Option<CardId> {
    board
        .content_cards(arena)
        .into_iter()
        .find(|&c| board.card(c).map(|k| k.card_type()) == Some("arena-meta"))
}

/// The place a fight was opened from, remembered in the hidden meta card.
fn place_of(board: &Board, arena: PileId) -> Option<PileId> {
    let meta = meta_card(board, arena)?;
    board
        .card(meta)
        .map(|k| PileId(num_after(k.front_title(), "place ") as u64))
}

/// Every combatant card in the arena, by side-type.
fn all_of_type(board: &Board, arena: PileId, card_type: &str) -> Vec<CardId> {
    let mut out = Vec::new();
    for (label, _, _) in GROUND_PILES {
        if let Some(p) = sub_pile(board, arena, label) {
            out.extend(
                board
                    .content_cards(p)
                    .into_iter()
                    .filter(|&c| board.card(c).map(|k| k.card_type()) == Some(card_type)),
            );
        }
    }
    out
}

/// Tear the arena down: foes back to the Bestiary, heroes back to the place as position cards, the arena
/// removed. `clear_encounter` removes a beaten encounter; `spend_day` advances the day clock.
fn teardown(board: &mut Board, arena: PileId, clear_encounter: bool, spend_day: bool) {
    let place = place_of(board, arena);
    let bestiary = top_deck(board, "Bestiary");

    let foes = all_of_type(board, arena, "foe");
    if let Some(b) = bestiary {
        let _ = board.return_foes_to_bestiary(&foes, b);
    }

    let units = all_of_type(board, arena, "unit");
    if let Some(place) = place {
        for u in units {
            let _ = board.set_card_type(u, "hero");
            let _ = board.set_card_detail(u, Vec::new());
            let at = board.pile(place).map_or(0, |p| p.cards().len());
            let _ = board.move_card(u, place, at);
        }
        if clear_encounter
            && let Some(enc) = board
                .content_cards(place)
                .into_iter()
                .find(|&c| board.card(c).map(|k| k.card_type()) == Some("encounter"))
        {
            let _ = board.remove_card(enc);
        }
    }

    let root = board.root_id();
    let _ = board.focus(root);
    let _ = board.remove_pile(arena);
    if spend_day
        && let (Some(p), Some(e)) = (top_deck(board, "Progress"), top_deck(board, "Events"))
    {
        let _ = board.advance_day(p, e);
    }
}

/// **Fold the fight back** after a decision: on a win the encounter is cleared; the fight spends a day. The
/// record goes down at the place before the arena is torn down.
pub fn fold_back(board: &mut Board, arena: PileId) {
    let result = outcome(board, arena);
    let won = result == Some(Outcome::Victory);
    if let (Some(place), Some(result)) = (place_of(board, arena), result) {
        record_outcome(board, arena, place, result);
    }
    teardown(board, arena, won, true);
}

/// **What happened here**, left at the place as a pile: a named result, and the whole battle inside it, one
/// card per round - a stack you drill into, bounded by the round cap.
fn record_outcome(board: &mut Board, arena: PileId, place: PileId, result: Outcome) {
    let label = match result {
        Outcome::Victory => "Victory",
        Outcome::Defeat => "Defeat",
        Outcome::Draw => "Draw",
    };
    let stale: Vec<PileId> = board
        .pile(place)
        .map(|p| p.subpiles())
        .unwrap_or_default()
        .into_iter()
        .filter(|&sp| {
            board
                .pile(sp)
                .is_some_and(|p| matches!(p.label.as_str(), "Victory" | "Defeat" | "Draw"))
        })
        .collect();
    for sp in stale {
        let _ = board.remove_pile(sp);
    }

    let Ok(record) = board.add_pile(place, label) else {
        return;
    };
    for round in rounds_logged(board, arena) {
        let Ok(card) = board.add_card(
            record,
            cardtable_model::Face::Up {
                title: format!("Round {round}"),
            },
            None,
        ) else {
            continue;
        };
        let _ = board.set_card_kind(card, CardKind::Virtual);
        let _ = board.set_card_type(card, "log");
        let _ = board.set_card_panel(card, round_log(board, arena, round));
    }
}

/// **Cancel the fight** (retreat): tear the arena down with nothing resolved - encounter intact, no day
/// spent.
pub fn cancel_fight(board: &mut Board, arena: PileId) {
    teardown(board, arena, false, false);
}

/// **Restart the fight**: every combatant back to full health, fresh tempo, and its weapon rank on its own
/// ground; the step deck back to round 1 - Havoc; the journal wiped (the record of a battle that no longer
/// happened); then auto-advance to the first decision, as at open.
pub fn restart_fight(board: &mut Board, arena: PileId) {
    clear_events(board, arena);
    let all: Vec<CardId> = all_of_type(board, arena, "unit")
        .into_iter()
        .chain(all_of_type(board, arena, "foe"))
        .collect();
    for card in all {
        let Some((name, ctype)) = board
            .card(card)
            .map(|c| (c.front_title().to_string(), c.card_type().to_string()))
        else {
            continue;
        };
        let (side, stats) = match ctype.as_str() {
            "unit" => (Side::Party, hero_stats(board, &name)),
            "foe" => (Side::Foe, foe_stats(&name)),
            _ => continue,
        };
        let Some((s, melee, ranged, aoe)) = stats else {
            continue;
        };
        let _ = board.set_card_detail(
            card,
            detail_lines(
                s.vitality, s.vitality, s.cadence, s.cadence, s.finesse, melee, ranged, aoe,
            ),
        );
        if let Some(dest) = sub_pile(board, arena, home_pile_label(side, melee, ranged))
            && !board.pile(dest).is_some_and(|p| p.cards().contains(&card))
        {
            let at = board.pile(dest).map_or(0, |p| p.cards().len());
            let _ = board.move_card(card, dest, at);
        }
    }
    install_step_deck(board, arena);
    set_round_card(board, arena, 1);
    clear_wave_mark(board, arena);
    run_engine(board, arena, false);
}

fn clear_wave_mark(board: &mut Board, arena: PileId) {
    let Some(meta) = meta_card(board, arena) else {
        return;
    };
    let mut d = board
        .card(meta)
        .map(|k| k.detail().to_vec())
        .unwrap_or_default();
    d.retain(|l| !l.starts_with("wave "));
    let _ = board.set_card_detail(meta, d);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::sample_table;
    use rules::core::Outcome as EngineOutcome;

    /// Move each kit's map position from the home cell to `place_name` (or the first encounter place) and
    /// open the fight there. Pass several kits to field a party.
    fn open_a_fight_at(board: &mut Board, kits: &[&str], place_name: Option<&str>) -> PileId {
        let locations = top_deck(board, "Locations").unwrap();
        let ashfen = board.pile(locations).unwrap().subpiles()[4];
        let place = board
            .pile(locations)
            .unwrap()
            .subpiles()
            .into_iter()
            .find(|&p| {
                let named = place_name
                    .is_none_or(|want| board.pile(p).map(|k| k.label.as_str()) == Some(want));
                named
                    && board
                        .content_cards(p)
                        .iter()
                        .any(|&c| board.card(c).map(|k| k.card_type()) == Some("encounter"))
            })
            .unwrap();
        for kit in kits {
            let position = board
                .content_cards(ashfen)
                .into_iter()
                .find(|&c| {
                    board.card(c).map(|k| (k.card_type(), k.front_title())) == Some(("hero", *kit))
                })
                .unwrap_or_else(|| panic!("{kit} is stationed at Ashfen"));
            let progress = top_deck(board, "Progress").unwrap();
            let _ = board.move_character(position, place, progress);
        }
        open_fight(board, place).expect("a fight opens")
    }

    /// Convert an engine choice into the staged order that produces it.
    fn as_staged(cards: &[CardId], c: &StepChoice) -> Staged {
        match c {
            StepChoice::Strike(Some(t)) => Staged::Aim(cards[*t]),
            StepChoice::Strike(None) => Staged::Hold,
            StepChoice::Move(true) => Staged::Go,
            StepChoice::Move(false) => Staged::Stay,
        }
    }

    /// Self-play THROUGH THE CARDS: stage the side-agnostic policy for every asked body (computed against
    /// the state with all earlier staged orders applied, exactly as commit will apply them), then commit;
    /// repeat to the end. This is the production path end to end - seat, stage, commit, write back.
    fn auto_play(board: &mut Board, arena: PileId) {
        let mut guard = 0;
        while outcome(board, arena).is_none() && guard < 500 {
            guard += 1;
            let Some(w) = wave(board, arena) else { break };
            let next_unstaged = (0..w.units.len()).find(|&i| w.asked[i] && w.staged[i].is_none());
            let Some(i) = next_unstaged else {
                commit(board, arena);
                continue;
            };
            // The policy choice for this body, honest about everything already staged before it.
            let seated = seat(board, arena).unwrap();
            let mut st = seated.state;
            while let Some(j) = st.deciding() {
                if j == i {
                    break;
                }
                let c = if st.board().units[j].side == Side::Foe {
                    step_policy(&st, j)
                } else {
                    match w.staged[j] {
                        Some(Staged::Aim(t)) => {
                            StepChoice::Strike(w.cards.iter().position(|&c| c == t))
                        }
                        Some(Staged::Hold) => StepChoice::Strike(None),
                        Some(Staged::Go) => StepChoice::Move(true),
                        Some(Staged::Stay) => StepChoice::Move(false),
                        None => break,
                    }
                };
                st = StepCombat::apply(&st, &c);
            }
            let choice = step_policy(&st, i);
            let staged = as_staged(&w.cards, &choice);
            edit_flags(board, w.cards[i], |f| f.staged = Some(staged));
        }
        assert!(guard < 500, "the self-play must terminate");
    }

    /// The reference: the same fight played straight through the engine, policy for both sides - the exact
    /// playout the balance machinery trusts.
    fn engine_play(mut state: StepState) -> (EngineOutcome, Vec<u32>) {
        let mut guard = 0;
        while StepCombat::outcome(&state).is_none() && guard < 4000 {
            guard += 1;
            let i = state.deciding().unwrap();
            let c = step_policy(&state, i);
            state = StepCombat::apply(&state, &c);
        }
        let healths = state.board().units.iter().map(|u| u.health).collect();
        (StepCombat::outcome(&state).unwrap(), healths)
    }

    /// **The no-drift gate.** The card path (seat -> stage -> commit -> write back, every wave) must
    /// reproduce the pure engine playout EXACTLY - same outcome, same final healths, on a solo and on the
    /// full-party capstone. If these ever disagree, the arena is playing a different game than the one the
    /// balance gate asserts.
    #[test]
    fn the_card_path_reproduces_the_engine_exactly() {
        let solo: &[&str] = &["Raider"];
        let party: &[&str] = &["Raider", "Marksman", "Bastion", "Bombardier"];
        for (kits, place) in [
            (solo, Some("The Sundered Vault")),
            (party, Some("Ashfen Crossing")),
        ] {
            let mut board = sample_table();
            let arena = open_a_fight_at(&mut board, kits, place);

            // The reference playout, from the exact state the cards opened at.
            let opened = seat(&board, arena).expect("the fight seats");
            let (want_outcome, want_healths) = engine_play(opened.state);

            auto_play(&mut board, arena);

            let got = outcome(&board, arena).expect("the fight ends");
            let want = match want_outcome {
                EngineOutcome::Win => Outcome::Victory,
                EngineOutcome::Loss => Outcome::Defeat,
                EngineOutcome::Draw => Outcome::Draw,
            };
            assert_eq!(got, want, "{place:?}: the card path changed the outcome");

            let (cards, units, _, _) = read_units(&board, arena);
            assert_eq!(cards.len(), want_healths.len(), "no body lost or minted");
            for (u, want_hp) in units.iter().zip(&want_healths) {
                assert_eq!(
                    u.health, *want_hp,
                    "{place:?}: {} final health drifted from the engine",
                    u.name
                );
            }
        }
    }

    /// A fight opens with both lines seated in their weapon ranks - ranged-only at the back, everything else
    /// at the front - and the schedule advanced to the first party decision (round 1 has no outriders, so
    /// Havoc and Withdraw are skipped and say so on the record).
    #[test]
    fn a_fight_opens_seated_at_weapon_ranks() {
        let mut board = sample_table();
        let arena = open_a_fight_at(
            &mut board,
            &["Raider", "Marksman"],
            Some("The Hollow Rampart"),
        );
        let in_pile = |label: &str, name: &str| {
            sub_pile(&board, arena, label)
                .map(|p| board.content_cards(p))
                .unwrap_or_default()
                .iter()
                .any(|&c| board.card(c).map(|k| k.front_title()) == Some(name))
        };
        assert!(in_pile("Vanguard", "Raider"), "melee at the front");
        assert!(in_pile("Rearguard", "Marksman"), "ranged at the back");
        assert!(in_pile("Foe Rearguard", "The Sniper"), "their back seated");
        let w = wave(&board, arena).expect("a wave is pending");
        assert!(
            w.asked.iter().any(|&a| a),
            "the fight opens at the first party decision"
        );
        assert!(
            w.focus.is_none(),
            "nothing is selected until the player selects - the click is the information"
        );
    }

    /// Staging is the whole pre-commit surface: choices stage orders, the Commit gate names who still owes
    /// one, and nothing resolves until the commit.
    #[test]
    fn staging_gates_the_commit() {
        let mut board = sample_table();
        let arena = open_a_fight_at(
            &mut board,
            &["Raider", "Bastion"],
            Some("The Hollow Rampart"),
        );
        let owed = pending_decision(&board, arena).expect("orders are owed at the first wave");
        assert!(owed.contains("has no orders"), "{owed}");
        let hp_before: Vec<u32> = read_units(&board, arena)
            .1
            .iter()
            .map(|u| u.health)
            .collect();

        let mut guard = 0;
        while pending_decision(&board, arena).is_some() && guard < 20 {
            guard += 1;
            let w = wave(&board, arena).unwrap();
            match w.focus {
                None => {
                    // Choose WHO: click a ringed (asked, unordered) hero.
                    let i = (0..w.units.len())
                        .find(|&i| w.asked[i] && w.staged[i].is_none())
                        .expect("an order is owed, so a hero is ringed");
                    handle_tap(&mut board, w.cards[i]);
                }
                Some(_) => {
                    assert!(
                        !scene_choices(&board, arena).is_empty(),
                        "a selected hero always has its order buttons on offer"
                    );
                    choose(&mut board, 0);
                }
            }
        }
        assert!(
            pending_decision(&board, arena).is_none(),
            "every asked body has an order"
        );
        let hp_now: Vec<u32> = read_units(&board, arena)
            .1
            .iter()
            .map(|u| u.health)
            .collect();
        assert_eq!(hp_before, hp_now, "staging must not resolve anything");
    }

    /// Cancel is conservation-clean: the arena tears down, the foes merge back into the Bestiary, the heroes
    /// stand at the place again, and the total card count is exactly what it was before the fight opened.
    #[test]
    fn cancel_restores_the_table() {
        let mut board = sample_table();
        let total = board.card_count();
        let arena = open_a_fight_at(&mut board, &["Raider"], Some("The Sundered Vault"));
        cancel_fight(&mut board, arena);
        assert_eq!(board.card_count(), total, "conservation (PC.2)");
        assert!(find_arena(&board).is_none(), "the arena is gone");
    }

    /// Folding back after a decided fight leaves the record at the place: the named result pile with one log
    /// card per round.
    #[test]
    fn fold_back_leaves_the_record() {
        let mut board = sample_table();
        let arena = open_a_fight_at(&mut board, &["Raider"], Some("The Sundered Vault"));
        auto_play(&mut board, arena);
        let result = outcome(&board, arena).expect("decided");
        let place = place_of(&board, arena).unwrap();
        fold_back(&mut board, arena);
        let label = match result {
            Outcome::Victory => "Victory",
            Outcome::Defeat => "Defeat",
            Outcome::Draw => "Draw",
        };
        let record = board
            .pile(place)
            .unwrap()
            .subpiles()
            .into_iter()
            .find(|&sp| board.pile(sp).map(|p| p.label.as_str()) == Some(label))
            .expect("the result pile stands at the place");
        assert!(
            !board.content_cards(record).is_empty(),
            "the record holds the battle, one card per round"
        );
    }

    /// **The three-beat gesture: WHO -> WHAT -> WHOM.** The verb button enters targeting (the mid-gesture
    /// state - Commit still counts the body as owed, phrased as picking a target); while aiming, the legal
    /// targets invite completion and a tap on one stages the order; Cancel puts the action back.
    #[test]
    fn the_three_beat_gesture() {
        let mut board = sample_table();
        let arena = open_a_fight_at(&mut board, &["Raider"], Some("The Sundered Vault"));

        // Beat 1 - the WHO: nothing selected, no buttons; the unordered hero carries the ring, and the
        // player's own click selects it.
        let w = wave(&board, arena).expect("a wave is pending");
        assert!(w.focus.is_none(), "nothing selected at open");
        assert!(
            scene_choices(&board, arena).is_empty(),
            "no buttons until a hero is chosen - the board is the menu"
        );
        let i = (0..w.units.len())
            .find(|&i| w.asked[i])
            .expect("the Raider is asked");
        handle_tap(&mut board, w.cards[i]);
        let w = wave(&board, arena).unwrap();
        assert_eq!(w.focus, Some(i), "the click selected the hero");
        assert!(!w.aiming, "no gesture yet");

        // Beat 2 - the WHAT: the first button is the step's verb; taking it enters targeting.
        let labels: Vec<String> = scene_choices(&board, arena)
            .iter()
            .map(|c| c.label.clone())
            .collect();
        assert!(
            labels[0].ends_with("..."),
            "the verb button leads the menu: {labels:?}"
        );
        choose(&mut board, 0);
        let w = wave(&board, arena).unwrap();
        assert!(w.aiming, "the gesture is in progress");
        assert!(
            pending_decision(&board, arena).is_some_and(|m| m.contains("picking a target")),
            "Commit still counts the body as owed, mid-gesture"
        );
        assert!(!w.targets.is_empty(), "the targets are on offer");

        // Cancel puts the action back...
        let n = scene_choices(&board, arena).len();
        choose(&mut board, n - 1);
        let w = wave(&board, arena).unwrap();
        assert!(!w.aiming, "cancelled - back to the order menu");

        // ...and the full gesture completes by tapping the ringed target.
        choose(&mut board, 0);
        let w = wave(&board, arena).unwrap();
        let target = w.targets[0];
        handle_tap(&mut board, w.cards[target]);
        let w = wave(&board, arena).unwrap();
        assert_eq!(
            w.staged[i],
            Some(Staged::Aim(w.cards[target])),
            "the tap completed the order"
        );
        assert!(!w.aiming, "the gesture is done");
    }
    /// The journal speaks the canonical log language: wave headers, commit lines, and the minor steps.
    #[test]
    fn the_journal_speaks_the_canonical_format() {
        let mut board = sample_table();
        let arena = open_a_fight_at(&mut board, &["Raider"], Some("The Sundered Vault"));
        auto_play(&mut board, arena);
        let all: Vec<String> = rounds_logged(&board, arena)
            .into_iter()
            .flat_map(|r| round_log(&board, arena, r))
            .collect();
        let has = |needle: &str| all.iter().any(|l| l.contains(needle));
        assert!(has("step "), "wave headers: {all:?}");
        assert!(
            has("- skipped"),
            "the opening waves nobody could act in are on the record"
        );
        assert!(
            !has("commit"),
            "the app journal is the mechanical record only - no commit lines"
        );
        assert!(has("strike"), "the strike minor step");
        assert!(has("resolve"), "the resolve minor step");
    }
}
