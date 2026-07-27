//! **Board <-> the step machine.** The card-table fight surface over the CANON combat model
//! (`rules::combat`, the eight-step round - see `docs/games/deckbound/canon/2-spec/combat.md`), with the
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
//!   `target`/`catch`/`strike`/`resolve` minor steps), told by the SHARED formatter
//!   ([`rules::combat::narrate`]) from the engine's recorded transcripts - the fight simulator's log and
//!   this journal cannot tell the story differently.

use cardtable_model::{Board, CardId, CardKind, Choice, PileId};
use rules::combat::narrate;
use rules::combat::regions::{
    Board as Battlefield, MAX_ROUNDS, Rank, catch_reach, reach_cards, strike_report,
};
use rules::combat::resolve::{Combatant, Side};
use rules::combat::step_game::{
    STEPS, Step, StepChoice, StepCombat, StepState, step_coord, step_policy, step_pours,
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

/// The max HP written on a combatant CARD's detail (`Health hp/max`) - the display total, robust to a
/// disambiguated Combatant name (which would miss the catalog lookup). This is the one to use once names may
/// carry a `1`/`2` suffix; it reads the total that was stamped when the card was seated.
pub(crate) fn max_health_on(board: &Board, card: CardId) -> u32 {
    board
        .card(card)
        .and_then(|c| c.detail().first().cloned())
        .map(|l| {
            l.rsplit('/')
                .next()
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0)
        })
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
    /// Strike a target with a chosen commitment: `catch` tempo cards for the reach, `pour` more as extra
    /// blows. Both are part of the order, so an aim is not complete until they are chosen.
    Aim(CardId, u32, u32),
    Hold,
    Go,
    Stay,
}

/// A body's marker lines, decoded: the round's commitments, the selection marks, and the staged order. The
/// gesture beats after the WHAT are: `aiming` (waiting on the WHOM), then `bidding` (target chosen, waiting on
/// the catch), then `striking` (target+catch chosen, waiting on the pour). None is an answer; Commit counts
/// each as owed.
#[derive(Clone, Copy, Default)]
struct Flags {
    struck: bool,
    arrived: bool,
    active: bool,
    aiming: bool,
    bidding: Option<CardId>,
    striking: Option<(CardId, u32)>,
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
                if let Some(rest) = l.strip_prefix("aim ") {
                    // "aim <target-cardid> <catch> <pour>"
                    let mut it = rest.split_whitespace();
                    if let (Some(id), Some(catch), Some(pour)) = (
                        it.next().and_then(|s| s.parse().ok()),
                        it.next().and_then(|s| s.parse().ok()),
                        it.next().and_then(|s| s.parse().ok()),
                    ) {
                        f.staged = Some(Staged::Aim(CardId(id), catch, pour));
                    }
                } else if let Some(id) = l.strip_prefix("bidding ") {
                    f.bidding = id.trim().parse().ok().map(CardId);
                } else if let Some(rest) = l.strip_prefix("striking ") {
                    // "striking <target-cardid> <catch>"
                    let mut it = rest.split_whitespace();
                    if let (Some(id), Some(catch)) = (
                        it.next().and_then(|s| s.parse().ok()),
                        it.next().and_then(|s| s.parse().ok()),
                    ) {
                        f.striking = Some((CardId(id), catch));
                    }
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
    if let Some(t) = f.bidding {
        lines.push(format!("bidding {}", t.0));
    }
    if let Some((t, catch)) = f.striking {
        lines.push(format!("striking {} {}", t.0, catch));
    }
    match f.staged {
        Some(Staged::Aim(t, catch, pour)) => lines.push(format!("aim {} {} {}", t.0, catch, pour)),
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

/// The target a body has chosen and is now bidding on (the beat between aiming and choosing the catch).
fn bidding_of(board: &Board, card: CardId) -> Option<CardId> {
    board
        .card(card)
        .and_then(|c| read_flags(c.detail()).bidding)
}

/// The (target, catch) a body has settled and is now choosing the pour for (the beat between the catch and a
/// staged `Aim`).
fn striking_of(board: &Board, card: CardId) -> Option<(CardId, u32)> {
    board
        .card(card)
        .and_then(|c| read_flags(c.detail()).striking)
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
    disambiguate_names(&mut units);
    (cards, units, regions, ranks)
}

/// Give duplicate bodies distinct DISPLAY names ("The Wall 1", "The Wall 2") so the tiles, the arrows, and
/// the journal never read two bodies as one. Purely cosmetic and applied AFTER the per-card stat lookup
/// (which keys off the card's catalog title, untouched here) - the engine uses these names only to narrate.
/// Numbered per side, in seat order, only when a name actually repeats.
fn disambiguate_names(units: &mut [Combatant]) {
    for side in [Side::Party, Side::Foe] {
        let idxs: Vec<usize> = (0..units.len())
            .filter(|&i| units[i].side == side)
            .collect();
        let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for &i in &idxs {
            *seen.entry(units[i].name.clone()).or_insert(0) += 1;
        }
        let dup: std::collections::HashSet<String> = seen
            .into_iter()
            .filter(|(_, n)| *n > 1)
            .map(|(name, _)| name)
            .collect();
        let mut counter: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for &i in &idxs {
            if dup.contains(&units[i].name) {
                let n = counter.entry(units[i].name.clone()).or_insert(0);
                *n += 1;
                units[i].name = format!("{} {}", units[i].name, n);
            }
        }
    }
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
/// happened). Used for the post-fight RECORD (one card per round); the live in-fight panel shows
/// [`recent_log`] instead.
pub(crate) fn round_log(board: &Board, arena: PileId, round: u32) -> Vec<String> {
    let want = format!("{round}|");
    read_events(board, arena)
        .into_iter()
        .filter_map(|e| e.strip_prefix(&want).map(|s| s.to_string()))
        .collect()
}

/// **The journal since the player's last action** - every line the engine appended during the most recent
/// automatic run (the last Commit's resolution, or the fight's opening), regardless of round or step
/// boundary. This is what the live panel shows: "here is what happened, including the steps that resolved
/// automatically (skipped waves, foe moves) since you last did something." The mark is set at the start of
/// each [`run_engine`] call, so it captures exactly that run's output.
pub(crate) fn recent_log(board: &Board, arena: PileId) -> Vec<String> {
    let events = read_events(board, arena);
    let mark = read_log_mark(board, arena).min(events.len());
    events[mark..]
        .iter()
        .map(|e| e.split_once('|').map(|(_, l)| l).unwrap_or(e).to_string())
        .collect()
}

/// The journal length at the start of the most recent automatic run - everything after it is "since your
/// last action". Kept on the meta card, like the wave mark.
fn read_log_mark(board: &Board, arena: PileId) -> usize {
    meta_card(board, arena)
        .and_then(|m| board.card(m))
        .and_then(|c| {
            c.detail().iter().find_map(|l| {
                l.strip_prefix("logmark ")
                    .and_then(|n| n.trim().parse().ok())
            })
        })
        .unwrap_or(0)
}

fn write_log_mark(board: &mut Board, arena: PileId, mark: usize) {
    let Some(meta) = meta_card(board, arena) else {
        return;
    };
    let mut d = board
        .card(meta)
        .map(|k| k.detail().to_vec())
        .unwrap_or_default();
    d.retain(|l| !l.starts_with("logmark "));
    d.push(format!("logmark {mark}"));
    let _ = board.set_card_detail(meta, d);
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

/// **Why a step had no decision** - a short parenthetical for its `- skipped` line. A step is skipped
/// exactly when the thing it does was not possible, so each reason names the missing prerequisite. Havoc and
/// Withdraw are the clean cases: both act only on outriders, so their absence is the whole reason.
fn skip_reason(step: Step) -> &'static str {
    match step {
        Step::Havoc => "no outriders",
        Step::Withdraw => "no outriders",
        Step::Skirmish => "no vanguard trade",
        Step::Cross => "no crossing available",
        Step::Volley => "no enemy outriders",
        Step::Raid => "no fresh arrivals",
        Step::Assault => "no vanguard in reach",
        Step::Advance => "the rear is screened",
    }
}

/// Print the wave header for `(round, step)`, filling in everything since the last header: a `round N`
/// marker when the round advanced, and a `- skipped (reason)` line for every wave nobody could act in. The
/// same shape as the fight simulator's log, via the same `step_coord` naming.
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
        note(
            board,
            arena,
            r,
            format!("  step {k}/8: {name} - skipped ({})", skip_reason(STEPS[i])),
        );
        i += 1;
    }
    let (k, name) = step_coord(step);
    note(board, arena, round, format!("  step {k}/8: {name}"));
    write_wave_mark(board, arena, round, target);
}

/// Journal the auto-skipped waves BEFORE `step` (and any round marker), but NOT `step`'s own header - used
/// when the engine STOPS at a player decision, so the panel shows which steps skipped on the way here while
/// `step`'s header waits until it actually resolves (keeping header and resolution together). No-op when
/// `step` is the round's first (nothing skipped before it).
fn log_skipped_before(board: &mut Board, arena: PileId, round: usize, step: Step) {
    let target = step_idx(step);
    if target == 0 {
        return; // Havoc: nothing before it this round
    }
    let (mut r, mut i) = match read_wave_mark(board, arena) {
        Some((r0, i0)) => (r0, i0 + 1),
        None => (0, STEPS.len()),
    };
    let mut wrote = false;
    while (r, i) < (round, target) {
        if i >= STEPS.len() {
            r += 1;
            i = 0;
            note(board, arena, r, format!("round {r}"));
            wrote = true;
            continue;
        }
        let (k, name) = step_coord(STEPS[i]);
        note(
            board,
            arena,
            r,
            format!("  step {k}/8: {name} - skipped ({})", skip_reason(STEPS[i])),
        );
        wrote = true;
        i += 1;
    }
    if wrote {
        // Mark the last skipped step, so `step`'s header (written when it resolves) fills nothing.
        write_wave_mark(board, arena, round, target - 1);
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

    // Who fights: the heroes ASSIGNED to this encounter (inside its assignment area). As a fallback for a
    // direct open (a headless launch or a test that did not assign), every hero present at the cell fields.
    // Each fielded hero seats straight into its weapon rank - ranged-only at the back, everything else at the
    // front. Nothing to declare; the fight opens on the first decision.
    let assigned = crate::board_game::assigned_heroes(board, place);
    let heroes: Vec<CardId> = if assigned.is_empty() {
        board
            .content_cards(place)
            .into_iter()
            .filter(|&c| board.card(c).map(|k| k.card_type()) == Some("hero"))
            .collect()
    } else {
        assigned
    };
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
    /// The focused body's chosen target (index into `cards`) while it is on the CATCH beat - target picked,
    /// catch still owed. `Some` only for a non-area striker between the WHOM tap and picking a catch.
    pub(crate) bidding: Option<usize>,
    /// The focused body's (target index, catch) while it is on the STRIKE beat - target+catch picked, pour
    /// still owed.
    pub(crate) striking: Option<(usize, u32)>,
    /// The focused body's strike CHOICES (empty on movement steps): every reachable enemy for a single
    /// striker, one representative per slice for an area striker - one action card each.
    pub(crate) targets: Vec<usize>,
    /// Each asked body's strike **footprint** - the enemies its strike would actually reach this step (an
    /// area striker's whole slice, a single striker's individual targets). Non-strikers get an empty list.
    /// This, not `targets`, is what lights up and is tappable on the board, so an area strike shows its whole
    /// extent rather than just the representative it collapsed to.
    pub(crate) footprints: Vec<Vec<usize>>,
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
    // The PLAYER-facing target list is undeduped: the screen shows every reachable enemy as its own choice,
    // so no legal target looks mysteriously unpickable. The twin dedup stays a search optimisation, hidden in
    // the solver's own `targets`; picking a twin resolves fine (the resolver validates by rank, not the list).
    let targets = focus.map(|i| state.targets_all(i)).unwrap_or_default();
    // Each asked body's footprint - what its strike actually reaches. The board lights and taps these, so an
    // area strike shows its whole slice; `targets` (collapsed for area) is only the action-card menu.
    let footprints: Vec<Vec<usize>> = (0..units.len())
        .map(|j| {
            if asked[j] {
                state.reachable(j)
            } else {
                Vec::new()
            }
        })
        .collect();
    let aiming = focus.is_some_and(|i| aiming_of(board, cards[i]));
    // The catch beat: the focus has a chosen target awaiting its catch. Resolve the stored target CardId to
    // its index; a stale target (no longer present) drops the state.
    let bidding = focus
        .and_then(|i| bidding_of(board, cards[i]))
        .and_then(|t| cards.iter().position(|&c| c == t));
    // The strike beat: target+catch settled, awaiting the pour.
    let striking = focus
        .and_then(|i| striking_of(board, cards[i]))
        .and_then(|(t, catch)| cards.iter().position(|&c| c == t).map(|ti| (ti, catch)));
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
        bidding,
        striking,
        targets,
        footprints,
    })
}

/// A decision the player owes before this wave can be committed, named for the Commit control. `None` means
/// Commit is live. **Nothing is decided by default**: an asked body that has not answered has not chosen to
/// pass - it has chosen nothing, and committing would silently choose for it.
pub fn pending_decision(board: &Board, arena: PileId) -> Option<String> {
    let w = wave(board, arena)?;
    let i = (0..w.units.len()).find(|&i| w.asked[i] && w.staged[i].is_none())?;
    Some(if w.focus == Some(i) && w.striking.is_some() {
        format!("{} is choosing how hard to strike", w.units[i].name)
    } else if w.focus == Some(i) && w.bidding.is_some() {
        format!("{} is choosing how hard to catch", w.units[i].name)
    } else if w.focus == Some(i) && w.aiming {
        format!("{} is targeting", w.units[i].name)
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
    /// Complete an order (a hold, a movement answer, or a full `Aim`). Aiming completes by tapping a lit
    /// enemy, not a card.
    Stage(Staged),
    /// Choose the targeted action (the WHAT) - enter targeting; a tap on a lit enemy completes it, and a tap
    /// on the commanding body drops the gesture (there is no Cancel card).
    BeginAim,
    /// Settle the catch (target, catch) and advance to the STRIKE beat, where the pour is chosen.
    PickCatch(CardId, u32),
}

/// The synthetic [`CardId`] base for **action tiles**. Real card ids are a small monotonic counter, so a
/// base this high can never collide: an action tile is a tracked, tappable card (it rings, it can anchor an
/// arrow), and its id encodes the choice index it stands for.
pub(crate) const ACTION_CARD_ID_BASE: u64 = 1 << 48;

/// The tile id for the choice at `index`.
pub(crate) fn action_card_id(index: usize) -> CardId {
    CardId(ACTION_CARD_ID_BASE + index as u64)
}

/// If `card` is one of the current wave's action tiles, the choice index it stands for - so a tap on it can
/// be read back as `Intention::Choose { index }`. Validated against the live choice list so a stale id from a
/// previous wave means nothing.
pub fn action_choice_index(board: &Board, arena: PileId, card: CardId) -> Option<usize> {
    if card.0 < ACTION_CARD_ID_BASE {
        return None;
    }
    let index = (card.0 - ACTION_CARD_ID_BASE) as usize;
    (index < step_choices(board, arena).len()).then_some(index)
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
        _ if w.bidding.is_some() => {
            // The CATCH beat: the target is chosen, now pick how many tempo cards buy the reach (1..=tempo).
            // Each shows the opening blow it lands (or "slipped"); the oracle stamps its own winnable/doomed.
            // Picking a catch opens the STRIKE beat when there is spare tempo to pour, else stages complete.
            // There is no zero tile - a catch of nothing is spelled Hold. Back out by re-tapping the body.
            let t = w.bidding.expect("catch arm");
            let target = w.cards[t];
            let tempo = w.units[i].tempo;
            let can_pour = step_pours(w.step) && !w.units[i].aoe && !w.units[i].horde;
            for c in 1..=tempo {
                // The reach this catch generates, then the RANGE of damage the Strike beat will let you deal
                // with it: each pour you could still afford, minus any the target dodges. A weak catch lets the
                // target slip your bigger pours (so the range shrinks); a stronger catch resists the slip (so
                // the full pour lands) - the reach-vs-strikes trade the player is here to learn.
                let reach = catch_reach(&w.units[i], c);
                let hi_pour = if can_pour { tempo - c } else { 0 };
                let landed: Vec<u32> = (0..=hi_pour)
                    .filter_map(|p| strike_report(&w.units, i, t, c, p).map(|r| r.damage))
                    .collect();
                let consequence = match (landed.iter().min(), landed.iter().max()) {
                    (Some(&lo), Some(&hi)) if lo == hi => format!("{reach} reach, {hi} damage"),
                    (Some(&lo), Some(&hi)) => format!("{reach} reach, {lo}-{hi} damage"),
                    _ => format!("{reach} reach, slipped"),
                };
                let action = if can_pour && tempo > c {
                    ChoiceAction::PickCatch(target, c)
                } else {
                    ChoiceAction::Stage(Staged::Aim(target, c, 0))
                };
                out.push((Choice::new(format!("Catch {c}"), consequence), action));
            }
        }
        _ if w.striking.is_some() => {
            // The STRIKE beat: target and catch settled, pick how many MORE tempo cards to pour into extra
            // blows - 0 (the opening blow only) up to all remaining. Each shows the total it lands; a bigger
            // pour lands more when the target stands, but the extra harm can tip a sensible target into
            // dodging the whole strike, read here as "slipped".
            let (t, catch) = w.striking.expect("strike arm");
            let target = w.cards[t];
            let tempo = w.units[i].tempo;
            for p in 0..=tempo.saturating_sub(catch) {
                // The total damage this pour banks (blows x Might), against the target's Grit - or "slipped".
                let consequence = match strike_report(&w.units, i, t, catch, p) {
                    None => "slipped".to_string(),
                    Some(r) => format!("{} dmg", r.damage),
                };
                let label = if p == 0 {
                    "Strike (opening blow)".to_string()
                } else {
                    format!("Strike +{p}")
                };
                out.push((
                    Choice::new(label, consequence)
                        .chosen(w.staged[i] == Some(Staged::Aim(target, catch, p))),
                    ChoiceAction::Stage(Staged::Aim(target, catch, p)),
                ));
            }
        }
        _ if w.aiming => {
            // The WHOM has NO cards: the lit enemies on the board ARE the menu - each carries its own
            // winnable/doomed and completes the strike when tapped. To back out, tap the commanding body
            // again (it drops the gesture), so there is no Cancel card either.
        }
        _ => {
            // The WHAT: begin the strike (entering targeting - the ringed tiles complete it), or hold. Just
            // the reach COUNT on the strike card - the ringed tiles already show which, and the log carries
            // the damage math once it happens; Hold needs no gloss, the word says it and the outlook scores
            // it. (The "a line strike bars crossing" rule the old text carried lives in the step prompt.)
            let n = w.targets.len();
            out.push((
                Choice::new("Strike...", format!("{n} in reach"))
                    .chosen(matches!(w.staged[i], Some(Staged::Aim(..)))),
                ChoiceAction::BeginAim,
            ));
            out.push((
                Choice::new("Hold", "").chosen(w.staged[i] == Some(Staged::Hold)),
                ChoiceAction::Stage(Staged::Hold),
            ));
        }
    }
    out
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
            f.bidding = None;
            f.striking = None;
            f.active = false;
        }),
        ChoiceAction::BeginAim => edit_flags(board, card, |f| {
            f.staged = None;
            f.aiming = true;
            f.bidding = None;
            f.striking = None;
        }),
        ChoiceAction::PickCatch(target, catch) => edit_flags(board, card, |f| {
            // The catch is settled; advance to the STRIKE beat to choose the pour.
            f.bidding = None;
            f.striking = Some((*target, *catch));
        }),
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
                f.bidding = None;
                f.striking = None;
                f.active = true;
            });
        } else if w.focus == Some(i) {
            // Tapping the selected hero again puts it back - deselected, gesture (aim/catch/pour) dropped.
            edit_flags(board, card, |f| {
                f.active = false;
                f.aiming = false;
                f.bidding = None;
                f.striking = None;
            });
        } else {
            // Select: move the active mark here, and clear any in-progress gesture everywhere - switching
            // bodies mid-gesture must not strand the abandoned one in a half-aimed/half-caught state (it would
            // re-enter that beat the next time it was selected). The freshly selected body starts at the WHAT.
            for (j, &c) in w.cards.iter().enumerate() {
                if w.units[j].side != Side::Party {
                    continue;
                }
                edit_flags(board, c, |f| {
                    f.active = j == i;
                    f.aiming = false;
                    f.bidding = None;
                    f.striking = None;
                });
            }
        }
        return;
    }
    // A foe tap completes the gesture in progress: it is the WHOM of a chosen action, so it only means
    // something while the focused body is AIMING. Any body in the strike's footprint completes it - for an
    // area strike that is the whole lit slice, not just the representative the choices collapsed to.
    if let Some(f) = w.focus
        && w.aiming
        && w.footprints[f].contains(&i)
    {
        let target = w.cards[i];
        if w.units[f].aoe {
            // An area strike is unevadable, forms no contact and never pours - it commits a single card, so
            // there is no catch or pour to choose: stage it complete (catch 1, pour 0).
            edit_flags(board, w.cards[f], |flags| {
                flags.staged = Some(Staged::Aim(target, 1, 0));
                flags.aiming = false;
                flags.active = false;
            });
        } else {
            // A single strike now owes a BID: advance to the bid beat (target chosen, bid pending). The bid
            // tiles become the menu; nothing is staged until one is picked.
            edit_flags(board, w.cards[f], |flags| {
                flags.aiming = false;
                flags.bidding = Some(target);
            });
        }
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
    // Mark where the journal stands NOW: everything this run appends (the resolution, the auto-skipped
    // waves, the foe moves up to the next decision) is "what happened since the player's last action", and
    // that is what the live panel shows via `recent_log`.
    write_log_mark(board, arena, read_events(board, arena).len());

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
                Some(Staged::Aim(t, catch, pour)) => StepChoice::Strike(
                    cards
                        .iter()
                        .position(|&c| c == t)
                        .map(|ti| (ti, catch, pour)),
                ),
                Some(Staged::Hold) => StepChoice::Strike(None),
                Some(Staged::Go) => StepChoice::Move(true),
                Some(Staged::Stay) => StepChoice::Move(false),
                None => unreachable!(),
            }
        } else {
            // A party decision with nothing staged: the player's turn. Journal the steps that auto-skipped
            // on the way here (so they show in "what happened since your last move"), then hand back control.
            log_skipped_before(board, arena, state.round(), state.step());
            break;
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
        let max_hp = max_health_on(board, card).max(u.health);
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

/// The solver **base** for scoring the focused body's candidates: the seated state with every OTHER staged
/// party order pinned and the cursor resting on the focus, ready for `StepCombat::apply(candidate)`. Returns
/// the wave and focus alongside it. `None` when the fight is not in a scorable state (no focus, or a staged
/// body dropped out of eligibility). Pinning EXACTLY the staged orders + this hero, and leaving every other
/// body to the solver, is what makes the verdict exact for any selection order (the wave declares order-free).
fn outlook_base(board: &Board, arena: PileId) -> Option<(StepState, Wave, usize)> {
    let w = wave(board, arena)?;
    let seated = seat(board, arena)?;
    let focus = w.focus?;
    let mut base = seated.state;
    let staged_first: Vec<usize> = (0..w.units.len())
        .filter(|&i| i != focus && w.units[i].side == Side::Party && w.staged[i].is_some())
        .collect();
    let mut prefix = staged_first.clone();
    prefix.push(focus);
    base.prioritize(&prefix);
    for &i in &staged_first {
        if base.deciding()? != i {
            return None; // a staged body dropped out of eligibility mid-wave; bail rather than mislead
        }
        let c = match w.staged[i] {
            Some(Staged::Aim(t, catch, pour)) => StepChoice::Strike(
                w.cards
                    .iter()
                    .position(|&c| c == t)
                    .map(|ti| (ti, catch, pour)),
            ),
            Some(Staged::Hold) => StepChoice::Strike(None),
            Some(Staged::Go) => StepChoice::Move(true),
            Some(Staged::Stay) => StepChoice::Move(false),
            None => return None,
        };
        base = StepCombat::apply(&base, &c);
    }
    (base.deciding() == Some(focus)).then_some((base, w, focus))
}

/// Ground one candidate to an outlook against `base`, spending from the shared node allowance `left`.
fn score_candidate(
    base: &StepState,
    solver: &mut Solver<StepCombat>,
    left: &mut u64,
    candidate: &StepChoice,
) -> cardtable_model::Outlook {
    use cardtable_model::Outlook;
    let next = StepCombat::apply(base, candidate);
    let before = solver.nodes();
    solver.grant(*left);
    let v = solver.verdict(&next);
    *left = left.saturating_sub(solver.nodes().saturating_sub(before));
    match v {
        Verdict::Winnable => Outlook::Winnable,
        Verdict::Doomed => Outlook::Doomed,
        Verdict::Evaluating => Outlook::Evaluating,
    }
}

/// The best outlook of striking `target` over every bid the attacker could commit (`1..=tempo`), short-circuit
/// on the first `Winnable`. An area strike has no bid (one card, unevadable), so it scores a single candidate.
/// This is what a lit enemy tile carries while aiming: "if I pick this target, does ANY bid keep a win alive?"
fn best_over_bids(
    base: &StepState,
    solver: &mut Solver<StepCombat>,
    left: &mut u64,
    target: usize,
    tempo: u32,
    aoe: bool,
    pours: bool,
) -> cardtable_model::Outlook {
    use cardtable_model::Outlook;
    let hi = if aoe { 1 } else { tempo.max(1) };
    let mut best = Outlook::Doomed;
    for c in 1..=hi {
        // Each catch at its current-behavior pour (all-in) - the same commitment the solver branches on.
        let pour = if pours && !aoe {
            tempo.saturating_sub(c)
        } else {
            0
        };
        let o = score_candidate(
            base,
            solver,
            left,
            &StepChoice::Strike(Some((target, c, pour))),
        );
        best = best_outlook(best, o);
        if best == Outlook::Winnable {
            break;
        }
    }
    best
}

/// While AIMING, the `Winnable`/`Doomed` of striking each **lit enemy**, keyed by its card - so the foe tiles
/// carry their own outlook now that the per-target cards are gone. Each tile scores the BEST bid over the
/// attacker's tempo (picking the target still leaves the bid to choose). Empty when not aiming, or when the
/// fight is not in a scorable state.
pub fn aim_outlook_by_foe(
    board: &Board,
    arena: PileId,
    solver: &mut Solver<StepCombat>,
    budget: u64,
) -> Vec<(CardId, cardtable_model::Outlook)> {
    let Some((base, w, focus)) = outlook_base(board, arena) else {
        return Vec::new();
    };
    if !w.aiming {
        return Vec::new();
    }
    let tempo = w.units[focus].tempo;
    let aoe = w.units[focus].aoe;
    let pours = step_pours(w.step);
    let mut left = budget;
    w.footprints[focus]
        .clone()
        .into_iter()
        .map(|m| {
            let o = best_over_bids(&base, solver, &mut left, m, tempo, aoe, pours);
            (w.cards[m], o)
        })
        .collect()
}

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
    let Some((base, w, focus)) = outlook_base(board, arena) else {
        return vec![Outlook::Unknown; choices.len()];
    };

    let mut left = budget;
    // Ground one candidate out to an outlook, spending from the shared allowance.
    let mut score = |solver: &mut Solver<StepCombat>, candidate: &StepChoice| -> Outlook {
        let next = StepCombat::apply(&base, candidate);
        let before = solver.nodes();
        solver.grant(left);
        let v = solver.verdict(&next);
        left = left.saturating_sub(solver.nodes().saturating_sub(before));
        match v {
            Verdict::Winnable => Outlook::Winnable,
            Verdict::Doomed => Outlook::Doomed,
            Verdict::Evaluating => Outlook::Evaluating,
        }
    };
    let mut out = Vec::with_capacity(choices.len());
    for (_, action) in &choices {
        let outlook = match action {
            // A Strike tile: score exactly that (target, catch, pour). This is where a suboptimal commitment
            // earns its honest verdict.
            ChoiceAction::Stage(Staged::Aim(t, catch, pour)) => score(
                solver,
                &StepChoice::Strike(
                    w.cards
                        .iter()
                        .position(|&x| x == *t)
                        .map(|ti| (ti, *catch, *pour)),
                ),
            ),
            ChoiceAction::Stage(Staged::Hold) => score(solver, &StepChoice::Strike(None)),
            ChoiceAction::Stage(Staged::Go) => score(solver, &StepChoice::Move(true)),
            ChoiceAction::Stage(Staged::Stay) => score(solver, &StepChoice::Move(false)),
            // A Catch tile leads to the Strike beat - its outlook is the BEST pour for that catch (winnable if
            // any pour keeps a win alive).
            ChoiceAction::PickCatch(t, catch) => match w.cards.iter().position(|&x| x == *t) {
                None => Outlook::Unknown,
                Some(ti) => {
                    let mut best = Outlook::Doomed;
                    for pour in 0..=w.units[focus].tempo.saturating_sub(*catch) {
                        let o = score(solver, &StepChoice::Strike(Some((ti, *catch, pour))));
                        best = best_outlook(best, o);
                        if best == Outlook::Winnable {
                            break;
                        }
                    }
                    best
                }
            },
            // The verb button leads to a target then a catch then a pour - so its outlook is the BEST of the
            // completions it opens (winnable if ANY target keeps a win alive), each at its optimal catch and
            // the current-behavior pour, matching the solver's own branching. Otherwise it would sit blank
            // beside a Hold that carries a verdict, reading as if only Hold were endorsed.
            ChoiceAction::BeginAim => {
                let pours = step_pours(w.step);
                let aoe = w.units[focus].aoe;
                let mut best = Outlook::Doomed;
                for &t in &w.targets {
                    let catch = reach_cards(&w.units, focus, t, pours);
                    let pour = if pours && !aoe {
                        w.units[focus].tempo.saturating_sub(catch)
                    } else {
                        0
                    };
                    let o = score(solver, &StepChoice::Strike(Some((t, catch, pour))));
                    best = best_outlook(best, o);
                    if best == Outlook::Winnable {
                        break; // a winnable completion is the best possible - no need to grind the rest
                    }
                }
                best
            }
        };
        out.push(outlook);
    }
    out
}

/// The better of two outlooks for a "best of these" fold: Winnable beats Evaluating beats Doomed; Unknown
/// only wins if that is all there is.
fn best_outlook(
    a: cardtable_model::Outlook,
    b: cardtable_model::Outlook,
) -> cardtable_model::Outlook {
    use cardtable_model::Outlook::*;
    let rank = |o: cardtable_model::Outlook| match o {
        Winnable => 3,
        Evaluating => 2,
        Doomed => 1,
        Unknown => 0,
    };
    if rank(a) >= rank(b) { a } else { b }
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
/// removed. `spend_day` advances the day clock. The **encounter is left in place** so the fight can be
/// fought again - a beaten encounter is marked by its [`record_outcome`] deck (a "Victory" pile that stays
/// at the location), never by removing the encounter.
fn teardown(board: &mut Board, arena: PileId, spend_day: bool) {
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
    }

    // Return to the screen the fight was entered from - the place - falling back to the root felt only
    // if that place no longer exists.
    let return_to = place
        .filter(|p| board.pile(*p).is_some())
        .unwrap_or_else(|| board.root_id());
    let _ = board.focus(return_to);
    let _ = board.remove_pile(arena);
    if spend_day
        && let (Some(p), Some(e)) = (top_deck(board, "Progress"), top_deck(board, "Events"))
    {
        let _ = board.advance_day(p, e);
    }
}

/// **Fold the fight back** after a decision: the fight spends a day, and the record deck goes down at the
/// place before the arena is torn down. The encounter is **left standing** so the fight can be repeated -
/// win or lose, the location stays combat-ready; the record deck ("Victory" / "Defeat" / "Draw") is how you
/// tell what happened here and whether it has been beaten.
pub fn fold_back(board: &mut Board, arena: PileId) {
    let result = outcome(board, arena);
    if let (Some(place), Some(result)) = (place_of(board, arena), result) {
        record_outcome(board, arena, place, result);
    }
    teardown(board, arena, true);
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
    teardown(board, arena, false);
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
    use rules::combat::step_game::pour_default;
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
        // Heroes are marched but not assigned here; `open_fight` falls back to fielding every hero present at
        // the cell, so the fight opens with exactly the kits this helper marched in.
        open_fight(board, place).expect("a fight opens")
    }

    /// Convert an engine choice into the staged order that produces it.
    fn as_staged(cards: &[CardId], c: &StepChoice) -> Staged {
        match c {
            StepChoice::Strike(Some((t, catch, pour))) => Staged::Aim(cards[*t], *catch, *pour),
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
                        Some(Staged::Aim(t, catch, pour)) => StepChoice::Strike(
                            w.cards
                                .iter()
                                .position(|&c| c == t)
                                .map(|ti| (ti, catch, pour)),
                        ),
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
                    let n = scene_choices(&board, arena).len();
                    assert!(
                        n >= 2,
                        "a selected hero always has its order cards on offer"
                    );
                    // The last card is always a pass (Hold) or a Stay - a Stage that completes the order
                    // without entering targeting, so every hero gets an order and the loop terminates.
                    choose(&mut board, n - 1);
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

    /// A decided fight leaves its encounter **standing**, so the same location can be fought again. The
    /// "Victory" record deck, not a vanished encounter, is what marks a place as beaten - which is exactly
    /// what makes a re-fight possible.
    #[test]
    fn a_beaten_fight_can_be_repeated() {
        let mut board = sample_table();
        let arena = open_a_fight_at(&mut board, &["Raider"], Some("The Sundered Vault"));
        auto_play(&mut board, arena);
        assert_eq!(
            outcome(&board, arena),
            Some(Outcome::Victory),
            "the Raider solos The Sundered Vault"
        );
        let place = place_of(&board, arena).unwrap();
        fold_back(&mut board, arena);

        let has_type = |t: &str| {
            board
                .content_cards(place)
                .into_iter()
                .any(|c| board.card(c).map(|k| k.card_type()) == Some(t))
        };
        assert!(
            has_type("encounter"),
            "the encounter is left standing after a win - the location can be re-fought"
        );
        assert!(has_type("hero"), "the hero returned to the place");

        // And a fresh fight really opens at the same place.
        open_fight(&mut board, place).expect("the location is combat-ready again");
        assert!(find_arena(&board).is_some(), "a second fight stands up");
    }

    /// **The five-beat gesture: WHO -> WHAT -> WHOM -> CATCH -> STRIKE.** The verb button enters targeting; a
    /// tap on a lit enemy picks the WHOM and advances to the catch beat; a catch tile settles the reach and
    /// (in a pouring step with spare tempo) advances to the strike beat; a strike tile picks the pour and
    /// stages the complete order. Each beat leaves Commit owed; Cancel puts the action back.
    #[test]
    fn the_five_beat_gesture() {
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

        // Beat 2 - the WHAT: the first card is Strike..., taking it enters targeting.
        let labels: Vec<String> = scene_choices(&board, arena)
            .iter()
            .map(|c| c.label.clone())
            .collect();
        assert!(
            labels[0].ends_with("..."),
            "the strike card leads the menu: {labels:?}"
        );
        choose(&mut board, 0);
        let w = wave(&board, arena).unwrap();
        assert!(w.aiming, "the gesture is in progress");
        assert!(
            pending_decision(&board, arena).is_some_and(|m| m.contains("targeting")),
            "Commit still counts the body as owed, mid-gesture"
        );
        assert!(
            scene_choices(&board, arena).is_empty(),
            "no cards while aiming - the lit enemies on the board are the menu"
        );
        assert!(!w.footprints[i].is_empty(), "the lit targets are on offer");

        // Cancel by tapping the commanding body again: it drops the gesture and backs out to WHO.
        handle_tap(&mut board, w.cards[i]);
        let w = wave(&board, arena).unwrap();
        assert!(!w.aiming, "cancelled - gesture dropped");
        assert!(
            w.focus.is_none(),
            "tapping the source backs all the way out"
        );

        // Re-select, re-enter targeting, and pick the WHOM by tapping a lit enemy.
        handle_tap(&mut board, w.cards[i]);
        choose(&mut board, 0);
        let w = wave(&board, arena).unwrap();
        let target = w.footprints[i][0];
        handle_tap(&mut board, w.cards[target]);
        let w = wave(&board, arena).unwrap();

        // Beat 4 - the CATCH: the target is chosen but nothing is staged yet; the menu is now catch tiles, and
        // Commit still counts the body as owed (choosing a catch).
        assert_eq!(
            w.bidding,
            Some(target),
            "the tap advanced to the catch beat"
        );
        assert!(
            w.staged[i].is_none(),
            "no order staged until a catch is picked"
        );
        assert!(
            pending_decision(&board, arena).is_some_and(|m| m.contains("catch")),
            "an aimed-but-uncaught strike is still owed - it never commits by itself"
        );
        let catch_labels: Vec<String> = scene_choices(&board, arena)
            .iter()
            .map(|c| c.label.clone())
            .collect();
        assert!(
            !catch_labels.is_empty() && catch_labels.iter().all(|l| l.starts_with("Catch ")),
            "the catch tiles are the menu now: {catch_labels:?}"
        );

        // Pick the first catch tile (Catch 1). This is a pouring step with spare tempo, so it opens the
        // STRIKE beat rather than staging.
        choose(&mut board, 0);
        let w = wave(&board, arena).unwrap();
        assert_eq!(
            w.striking,
            Some((target, 1)),
            "the catch advanced to the strike beat"
        );
        assert!(
            w.staged[i].is_none(),
            "still nothing staged - the pour is owed"
        );
        assert!(
            pending_decision(&board, arena).is_some_and(|m| m.contains("strike")),
            "an aimed-and-caught strike still owes its pour"
        );
        let strike_labels: Vec<String> = scene_choices(&board, arena)
            .iter()
            .map(|c| c.label.clone())
            .collect();
        assert!(
            strike_labels.iter().all(|l| l.starts_with("Strike")),
            "the strike tiles are the menu now: {strike_labels:?}"
        );

        // Beat 5 - the STRIKE: pick the first pour tile (pour 0, opening blow only) - that stages the order.
        choose(&mut board, 0);
        let w = wave(&board, arena).unwrap();
        assert_eq!(
            w.staged[i],
            Some(Staged::Aim(w.cards[target], 1, 0)),
            "the strike tile completed the order at catch 1, pour 0"
        );
        assert!(
            !w.aiming && w.bidding.is_none() && w.striking.is_none(),
            "the gesture is done"
        );
    }

    /// **An area strike has no catch or strike beat.** An AOE striker forms no contact, cannot be evaded and
    /// never pours (it commits a single card), so tapping a lit enemy stages the complete `Aim(target, 1, 0)`
    /// at once - it never stops on the catch or strike beat the way a single striker does.
    #[test]
    fn an_area_strike_skips_the_catch_beat() {
        let mut board = sample_table();
        // Bastion carries Sweep (a melee AREA strike).
        let arena = open_a_fight_at(&mut board, &["Bastion"], Some("The Sundered Vault"));
        let w = wave(&board, arena).expect("a wave is pending");
        let i = (0..w.units.len())
            .find(|&i| w.asked[i])
            .expect("Bastion is asked");
        assert!(w.units[i].aoe, "Bastion strikes an area");
        handle_tap(&mut board, w.cards[i]); // select
        choose(&mut board, 0); // Strike... -> aiming
        let w = wave(&board, arena).unwrap();
        let target = w.footprints[i][0];
        handle_tap(&mut board, w.cards[target]); // tap a lit enemy
        let w = wave(&board, arena).unwrap();
        assert_eq!(
            w.staged[i],
            Some(Staged::Aim(w.cards[target], 1, 0)),
            "the area strike staged straight to a one-card commit"
        );
        assert!(
            w.bidding.is_none() && w.striking.is_none() && !w.aiming,
            "no catch or strike beat for an area strike"
        );
        assert!(
            pending_decision(&board, arena).is_none() || w.staged[i].is_some(),
            "the order is complete - nothing owed for this body"
        );
    }

    /// **The oracle scores each catch.** On the catch beat every catch tile gets its own verdict (never blank),
    /// so a suboptimal catch can be SEEN to lose - the teaching the feature exists for.
    #[test]
    fn the_oracle_scores_each_catch() {
        let mut board = sample_table();
        let arena = open_a_fight_at(&mut board, &["Raider"], Some("The Sundered Vault"));
        let w = wave(&board, arena).unwrap();
        let i = (0..w.units.len()).find(|&i| w.asked[i]).unwrap();
        handle_tap(&mut board, w.cards[i]); // select
        choose(&mut board, 0); // Strike... -> aiming
        let w = wave(&board, arena).unwrap();
        let target = w.footprints[i][0];
        handle_tap(&mut board, w.cards[target]); // -> the catch beat
        let w = wave(&board, arena).unwrap();
        assert!(w.bidding.is_some(), "on the catch beat");
        let tiles = scene_choices(&board, arena);
        assert_eq!(
            tiles.len() as u32,
            w.units[i].tempo,
            "one tile per catch 1..=tempo"
        );
        let mut solver: Solver<StepCombat> = Solver::default();
        let outlooks = choice_outlooks(&board, arena, &mut solver, 5_000_000);
        assert_eq!(outlooks.len(), tiles.len(), "an outlook per catch tile");
        assert!(
            outlooks
                .iter()
                .all(|o| *o != cardtable_model::Outlook::Unknown),
            "every catch carries a real verdict, so a losing catch can be seen to lose"
        );
    }

    /// **The strike beat offers every pour, and a held-back pour stages as chosen.** After the catch, the
    /// menu is one tile per pour `0..=(tempo-catch)`; picking a non-zero pour stages exactly that commitment.
    #[test]
    fn the_strike_beat_offers_every_pour() {
        let mut board = sample_table();
        let arena = open_a_fight_at(&mut board, &["Raider"], Some("The Sundered Vault"));
        let w = wave(&board, arena).unwrap();
        let i = (0..w.units.len()).find(|&i| w.asked[i]).unwrap();
        let tempo = w.units[i].tempo;
        handle_tap(&mut board, w.cards[i]); // select
        choose(&mut board, 0); // Strike... -> aiming
        let w = wave(&board, arena).unwrap();
        let target = w.footprints[i][0];
        handle_tap(&mut board, w.cards[target]); // -> catch beat
        choose(&mut board, 0); // Catch 1 -> strike beat
        let w = wave(&board, arena).unwrap();
        assert_eq!(
            w.striking,
            Some((target, 1)),
            "on the strike beat at catch 1"
        );
        let tiles = scene_choices(&board, arena);
        assert_eq!(
            tiles.len() as u32,
            tempo, // pour 0..=(tempo - 1) is `tempo` options
            "one tile per pour 0..=(tempo-catch)"
        );
        // Pick the last (max pour) tile and confirm it stages with that pour held nothing back.
        let last = tiles.len() - 1;
        choose(&mut board, last);
        let w = wave(&board, arena).unwrap();
        assert_eq!(
            w.staged[i],
            Some(Staged::Aim(w.cards[target], 1, tempo - 1)),
            "the max-pour tile staged catch 1 + pour (tempo-1)"
        );
    }

    /// A Catch tile reads its **reach** and the **range of damage** the Strike beat will then offer (each pour
    /// that lands) - a stronger catch trades reach for fewer strikes, the lesson the beat teaches.
    #[test]
    fn catch_tiles_show_reach_and_damage_range() {
        let mut board = sample_table();
        let arena = open_a_fight_at(&mut board, &["Raider"], Some("The Sundered Vault"));
        let w = wave(&board, arena).unwrap();
        let i = (0..w.units.len()).find(|&i| w.asked[i]).unwrap();
        handle_tap(&mut board, w.cards[i]); // select
        choose(&mut board, 0); // Strike... -> aiming
        let w = wave(&board, arena).unwrap();
        let target = w.footprints[i][0];
        handle_tap(&mut board, w.cards[target]); // -> the catch beat
        let cons: Vec<String> = scene_choices(&board, arena)
            .iter()
            .map(|c| c.consequence.clone())
            .collect();
        // Raider M6 / Finesse 2, tempo 2, vs the Wall (melee - it answers, so it never slips). Catch 1: reach
        // 2, pours 0..1 both land -> 6-12 damage. Catch 2: reach 4, only pour 0 -> 6 damage.
        assert_eq!(cons, vec!["2 reach, 6-12 damage", "4 reach, 6 damage"]);
    }

    /// The WHAT beat is a **card**: an action tile's synthetic id reads back to the choice index it stands
    /// for (so a tap on it is `Intention::Choose`), it never collides with a real combatant, and a stale id
    /// past the current menu means nothing.
    #[test]
    fn action_cards_route_taps_to_choices() {
        let mut board = sample_table();
        let arena = open_a_fight_at(&mut board, &["Raider"], Some("The Sundered Vault"));
        // Nothing selected: no action cards, so even index 0 is not live.
        assert_eq!(action_choice_index(&board, arena, action_card_id(0)), None);

        // Select the asked hero: now the WHAT cards exist.
        let w = wave(&board, arena).unwrap();
        let i = (0..w.units.len()).find(|&i| w.asked[i]).unwrap();
        handle_tap(&mut board, w.cards[i]);

        let n = scene_choices(&board, arena).len();
        assert!(n >= 2, "the verb and the pass, at least");
        for k in 0..n {
            assert_eq!(
                action_choice_index(&board, arena, action_card_id(k)),
                Some(k),
                "action id {k} reads back to choice {k}"
            );
        }
        // One past the menu is not a live action.
        assert_eq!(action_choice_index(&board, arena, action_card_id(n)), None);
        // A real combatant is never mistaken for an action card.
        assert!(
            action_card_id(0).0 > w.cards[i].0,
            "synthetic ids sit above real ones"
        );
        assert_eq!(action_choice_index(&board, arena, w.cards[i]), None);
    }

    /// Selecting a hero OUT of cursor order still yields real outlooks (the wave is reordered so the choice
    /// is pinned exactly, not stood in for) - it must not blank the verdict row.
    #[test]
    fn out_of_order_selection_still_scores() {
        use rules::combat::step_game::StepCombat;
        use rules::core::Solver;
        let mut board = sample_table();
        let arena = open_a_fight_at(
            &mut board,
            &["Raider", "Marksman", "Bastion", "Bombardier"],
            Some("Ashfen Crossing"),
        );
        // Pick the SECOND asked hero (skip the first in cursor order).
        let w = wave(&board, arena).unwrap();
        let asked: Vec<usize> = (0..w.units.len()).filter(|&i| w.asked[i]).collect();
        assert!(
            asked.len() >= 2,
            "the capstone asks several heroes at the front"
        );
        handle_tap(&mut board, w.cards[asked[1]]);
        // Take the verb so the target choices (which carry verdicts) are on offer.
        let n = scene_choices(&board, arena).len();
        let _ = n;
        let mut solver: Solver<StepCombat> = Solver::default();
        let outlooks = choice_outlooks(&board, arena, &mut solver, 5_000_000);
        assert!(
            outlooks
                .iter()
                .any(|o| *o != cardtable_model::Outlook::Unknown),
            "an out-of-order selection must still produce a real verdict, not all-blank"
        );
    }

    /// Duplicate foes get distinct DISPLAY names, so the tiles and the journal never read two bodies as one -
    /// while the cards keep their catalog title (stat lookup and the bestiary merge are untouched).
    #[test]
    fn duplicate_foes_read_apart() {
        let mut board = sample_table();
        // Ashfen fields The Wall x2 - the case that used to render as two identical "The Wall".
        let arena = open_a_fight_at(
            &mut board,
            &["Raider", "Marksman", "Bastion", "Bombardier"],
            Some("Ashfen Crossing"),
        );
        let (cards, units, _, _) = read_units(&board, arena);
        let walls: Vec<&Combatant> = units
            .iter()
            .filter(|u| u.name.starts_with("The Wall"))
            .collect();
        assert_eq!(walls.len(), 2, "Ashfen fields two Walls");
        assert_ne!(walls[0].name, walls[1].name, "the two Walls read apart");
        assert!(
            walls.iter().all(|u| u.name != "The Wall"),
            "each duplicate is numbered: {:?}",
            walls.iter().map(|u| &u.name).collect::<Vec<_>>()
        );
        // The cards themselves keep the catalog title, so the max-HP total is still legible from the card.
        for (&c, u) in cards.iter().zip(&units) {
            if u.name.starts_with("The Wall") {
                assert!(
                    max_health_on(&board, c) > 0,
                    "max HP reads off the card, not the display name"
                );
            }
        }
    }

    /// **Outlooks CONDITION on what is staged - inter-hero dependencies are real.** A choice's verdict is
    /// "given everything staged so far, does SOME completion still win". So staging one hero's order can
    /// change another hero's outlook set (Q3: a path is winnable/doomed BECAUSE of a different hero's
    /// choice), and two heroes' badges are only JOINTLY guaranteed once you stage one and let the other
    /// re-condition. This proves the conditioning is live rather than each hero being scored in isolation.
    #[test]
    fn outlooks_condition_on_staged_orders() {
        use rules::combat::step_game::StepCombat;
        use rules::core::Solver;

        let outlooks_for =
            |board: &Board, arena: PileId, hero: usize| -> Vec<cardtable_model::Outlook> {
                // Select `hero`, then read its choice outlooks (a fresh solver each call - we are testing the
                // question the badges answer, not memo warmth).
                let mut b = board.clone();
                let w = wave(&b, arena).unwrap();
                for (j, &c) in w.cards.iter().enumerate() {
                    if w.units[j].side == Side::Party {
                        edit_flags(&mut b, c, |f| f.active = j == hero);
                    }
                }
                let mut solver: Solver<StepCombat> = Solver::default();
                choice_outlooks(&b, arena, &mut solver, 20_000_000)
            };

        // The Hollow Rampart (Raid) is Insight-class - only a real read wins - so hero choices genuinely
        // depend on one another there.
        let mut board = sample_table();
        let arena = open_a_fight_at(
            &mut board,
            &["Raider", "Marksman", "Bastion", "Bombardier"],
            Some("The Hollow Rampart"),
        );
        let w = wave(&board, arena).unwrap();
        let asked: Vec<usize> = (0..w.units.len()).filter(|&i| w.asked[i]).collect();
        assert!(asked.len() >= 2, "several heroes are asked at the opening");
        let (a, b_hero) = (asked[0], asked[1]);

        // b_hero's outlooks with A unstaged (marginal - A is free).
        let marginal = outlooks_for(&board, arena, b_hero);

        // A's legal orders: select A, read its menu.
        let a_targets = {
            let mut b = board.clone();
            for (j, &c) in w.cards.iter().enumerate() {
                if w.units[j].side == Side::Party {
                    edit_flags(&mut b, c, |f| f.active = j == a);
                }
            }
            wave(&b, arena).unwrap().targets
        };
        // Stage EACH of A's legal orders and see whether b_hero's outlook set ever changes. If it does, the
        // badges condition on A's choice (the property under test). Across an Insight corner it must.
        let mut conditioned = false;
        for staged in a_targets
            .iter()
            .map(|&t| {
                let pours = step_pours(w.step);
                let catch = reach_cards(&w.units, a, t, pours);
                Staged::Aim(w.cards[t], catch, pour_default(&w.units[a], catch, pours))
            })
            .chain(std::iter::once(Staged::Hold))
        {
            let mut staged_board = board.clone();
            edit_flags(&mut staged_board, w.cards[a], |f| f.staged = Some(staged));
            let with_a = outlooks_for(&staged_board, arena, b_hero);
            if with_a != marginal {
                conditioned = true;
                break;
            }
        }
        assert!(
            conditioned,
            "staging a hero's order must be able to change another hero's outlook - the badges are conditional, not per-hero-in-isolation"
        );
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
