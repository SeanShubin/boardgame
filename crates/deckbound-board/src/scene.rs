//! **Board -> Scene.** The game's own rendering of a running fight, expressed in the renderer's *rules-blind*
//! [`Scene`] vocabulary (tracks / lanes / tiles / arrows / text). Every combat meaning is decided here and
//! flattened into generic possibilities: a rank becomes a lane, "effective in this position" becomes a
//! [`Highlight`], "carries a melee blow" becomes a [`Tone`]. The renderer draws the result without learning
//! any of it. This is the game side of the A1 boundary: the `cardtable` renderer no longer knows combat;
//! it just draws the `Scene` this module builds.
//!
//! All text is ASCII (it is drawn to the screen and written to the debug log): `->` not an arrow glyph, `x`
//! not a multiply sign, `*` not a bullet. The renderer supplies the typography via its own palette if it
//! wants; the game stays ASCII.

use cardtable_model::{
    Badge, Board, CardId, Highlight, Lane, Link, PileId, Row, Scene, SceneBody, Team, Tile, Tone,
    Track, TrackItem,
};
use deckbound_content::rank::Intention as Rank;
use deckbound_content::schedule::SUB_PHASE_NAMES;

use crate::arena::{self, Staged, Step};
use crate::combat::{self, Combatant, Contact, Side};

/// Build the modal fight [`Scene`], or `None` when no fight is up (the renderer then draws the felt).
pub fn scene(board: &Board, _focus: PileId) -> Option<Scene> {
    let arena = arena::find_arena(board)?;
    let (cards, units, sub, round, step) = arena::arena_state(board, arena);
    let maxes = arena::maxes_of(board, &units);
    let staged: Vec<Staged> = cards.iter().map(|&c| arena::staged_of(board, c)).collect();
    let marshal = step == Step::Marshal;

    let tracks = build_tracks(sub, step, marshal);
    let heading = format!("Round {round}");
    let prompt = prompt_for(step).to_string();

    let (body, links) = if marshal {
        (build_formation(board, arena), Vec::new())
    } else {
        let contacts = arena::read_contacts(board, arena, &cards);
        let active = (0..units.len())
            .find(|&i| units[i].side == Side::Party && staged[i].active && !units[i].fallen);
        let (lanes, links) = build_lanes(
            board, &cards, &units, &maxes, &staged, &contacts, active, sub, step,
        );
        (SceneBody::Lanes(lanes), links)
    };
    // The log is the **record**, not a guide: only what actually changed the table. What you *may* do is the
    // prompt's job and the choice cards' job, and saying it twice made the log a wall of speculation you had to
    // read past to find the one line that mattered.
    // **No log at Marshal.** The record it would show is the round that just ended, and nothing in it bears on
    // the one decision Marshal asks: where your heroes stand. Tempo has stood back up and open wounds have
    // closed - the Reset wiped the state that record described. A panel that cannot inform the decision in
    // front of you is furniture.
    let (log, log_title) = if marshal {
        (Vec::new(), String::new())
    } else {
        (
            build_log(board, arena),
            format!(
                "Round {round} - {} - {}",
                SUB_PHASE_NAMES.get(sub).copied().unwrap_or("?"),
                step_name(step)
            ),
        )
    };

    // The Start / Commit control (index 0) is inert while the player still owes a decision - an unranked hero
    // at Marshal, an unanswered blow at React. Its label names what is missing (see `arena::commit_label`).
    let disabled_controls = if arena::pending_decision(board, arena).is_some() {
        vec![0]
    } else {
        Vec::new()
    };

    // Every decision on offer right now, as cards: React's answers, Strike's targets and bids, Extra's follow-up
    // strikes. Each carries what it costs and does, and a barred one says why - so "may I strike back?" is
    // answerable from the screen rather than from the rules. A tap on the table only says *which* hero or foe
    // we mean; the order itself is always one of these.
    let choices = arena::scene_choices(board, arena);

    Some(Scene {
        tracks,
        heading,
        prompt,
        body,
        links,
        choices,
        log_title,
        log,
        legend: stat_legend(),
        disabled_controls,
    })
}

/// **What the letters on the tiles mean.** A tile has no room for words, so it says "M 7  F 2  T 1" - and an
/// abbreviation the player cannot expand is just noise. The meaning belongs on the table, beside the thing it
/// explains, not in a manual.
///
/// Grouped to show the shape of the game, which is a symmetry worth seeing: **health is Vitality-many cards,
/// each Grit strong; tempo is Cadence-many cards, each Finesse strong.** Might stands alone because it is
/// the only stat that is damage - Finesse buys reach and escape and never touches it.
fn stat_legend() -> Vec<String> {
    vec![
        "Stats".to_string(),
        "  M  Might - how hard you hit".to_string(),
        "Health".to_string(),
        "  V  Vitality - how many health cards".to_string(),
        "  G  Grit - strength of each one".to_string(),
        "Tempo".to_string(),
        "  C  Cadence - how many tempo cards".to_string(),
        "  F  Finesse - strength of each one".to_string(),
    ]
}

// ---- the two progress tracks (fixed display order; the physical decks rotate) --------------------------

fn build_tracks(sub: usize, step: Step, marshal: bool) -> Vec<Track> {
    let mut phase_items = vec![TrackItem {
        label: "Marshal".to_string(),
        current: marshal,
    }];
    for (i, name) in SUB_PHASE_NAMES.iter().enumerate() {
        phase_items.push(TrackItem {
            label: (*name).to_string(),
            current: !marshal && i == sub,
        });
    }
    // The Reset closes the round: tempo stands back up and every unfinished wound closes with it. It is never
    // "current" - you do not stop there - but it has to be ON the track, because a wound you cannot finish
    // before it is a wound you did not inflict. That deadline is the whole shape of a round now, and a
    // deadline the player cannot see is the same bug as damage that vanishes unremarked.
    phase_items.push(TrackItem {
        label: "Reset".to_string(),
        current: false,
    });
    let mut tracks = vec![Track {
        title: "Phase".to_string(),
        items: phase_items,
    }];
    if !marshal {
        let cur = step_name(step);
        let items = ["Engage", "Evade", "Strike"]
            .into_iter()
            .map(|n| TrackItem {
                label: n.to_string(),
                current: n == cur,
            })
            .collect();
        tracks.push(Track {
            title: "Step".to_string(),
            items,
        });
    }
    tracks
}

fn step_name(step: Step) -> &'static str {
    match step {
        Step::Marshal => "Marshal",
        Step::Engage => "Engage",
        Step::Evade => "Evade",
        Step::Strike => "Strike",
    }
}

fn prompt_for(step: Step) -> &'static str {
    match step {
        Step::Engage => {
            "Engage - commit tempo to REACH a foe. More makes you harder to slip, but buys no damage: reaching wins one blow, whatever it cost."
        }
        Step::Evade => {
            "Evade - you can see what they committed, so the price is exact. Pay it, or stand and keep the tempo to hit back."
        }
        Step::Strike => {
            "Strike - contact is made and Finesse is done. One tempo card = one blow of Might."
        }
        Step::Marshal => {
            "Formation - you can read every foe, but not where it will stand. Drag each hero into a rank, then Start."
        }
    }
}

// ---- the formation (Marshal): assignment rows of party tiles -------------------------------------------

fn build_formation(board: &Board, arena: PileId) -> SceneBody {
    // The foes you face, at the top - every stat readable, and **no rank**: you are entitled to know who you
    // are fighting, but their formation is not on the table until you commit yours (see `arena::MUSTER`).
    let mut rows = Vec::new();
    if let Some(muster) = arena::sub_pile(board, arena, arena::MUSTER) {
        let row = formation_row(board, muster, arena::MUSTER, None, Side::Foe);
        if !row.tiles.is_empty() {
            rows.push(row);
        }
    }
    for (label, rank) in arena::RANK_PILES {
        if let Some(pile) = arena::sub_pile(board, arena, label) {
            rows.push(formation_row(board, pile, label, Some(rank), Side::Party));
        }
    }
    // The Pool of unranked heroes, where they are dragged up from - **only while anyone is still in it.** That
    // is the first Marshal and no other: from round two on, a hero holds the rank you last gave it, so the row
    // is empty, and an empty row you can never fill again is just a stripe of nothing to read past.
    if let Some(pool) = arena::sub_pile(board, arena, arena::POOL) {
        let row = formation_row(board, pool, "Heroes", None, Side::Party);
        if !row.tiles.is_empty() {
            rows.push(row);
        }
    }
    SceneBody::Rows(rows)
}

/// One formation row: the party's ranks are assignment rows you drag heroes into; the foes' muster is a
/// reading row - you can study every card in it, but you cannot move it and it has no rank to read off.
fn formation_row(board: &Board, pile: PileId, label: &str, rank: Option<Rank>, side: Side) -> Row {
    let want = if side == Side::Party { "unit" } else { "foe" };
    let mut tiles = Vec::new();
    for card in board.content_cards(pile) {
        if board.card(card).map(|k| k.card_type()) != Some(want) {
            continue;
        }
        // The rank is only needed to flag an ineffective placement; the Pool and the muster never flag.
        let Some(u) = arena::read_combatant(board, card, rank.unwrap_or(Rank::Vanguard)) else {
            continue;
        };
        let max = arena::max_health(board, &u.name, side).max(u.health);
        tiles.push(formation_tile(&u, card, max, rank, side));
    }
    Row {
        label: label.to_string(),
        hint: rank_hint(rank, side),
        drop_pile: pile,
        tiles,
    }
}

/// **What putting a hero in this rank is FOR** — read straight off the schedule, in the second person, so the
/// formation screen teaches the plan instead of merely naming three boxes.
///
/// Marshal is the one decision made blind, and it is the decision the whole round hangs on. A row labelled
/// "Outrider" tells a new player nothing about *why* they would ever want one; these lines say what the rank
/// buys and what it costs, which is exactly the schedule (`deckbound_content::schedule::SCHEDULE`) put into
/// words. If the schedule ever changes, these are wrong and must move with it.
fn rank_hint(rank: Option<Rank>, side: Side) -> String {
    // The muster gets none. A hint is *printed on the drop zone's felt*, to be covered by the cards you place
    // there - and nothing is ever placed in the muster, so text there could only sit in the open forever,
    // saying what the row of foe cards already says.
    if side == Side::Foe {
        return String::new();
    }
    match rank {
        // Raid (3rd) is the Outrider's whole point: it reaches the enemy back line before anyone else can.
        // It pays for that with the Intercept and the Volley, in that order, before it ever lands a blow.
        Some(Rank::Outrider) => {
            "Slip past their front to kill their Rearguard before it fires. You pay for it: their Vanguard \
             screens you first, then their Rearguard shoots you."
                .into()
        }
        // Intercept (1st) + Clash (4th). And while it stands, back_access_ok screens your own Rearguard.
        Some(Rank::Vanguard) => {
            "Screen your Rearguard from their Outriders, and break their Vanguard - a fallen front is what \
             opens their back line to everyone."
                .into()
        }
        // Volley (2nd) + Clash (4th), fired from behind a screen it does not have to earn.
        Some(Rank::Rearguard) => {
            "Kill from range, safe while your own Vanguard stands. Shoot their crossing Outriders, then \
             pound their Vanguard."
                .into()
        }
        // The Pool gets none either. It is the one row whose cards *leave* rather than arrive, so its text
        // would be uncovered exactly while it is full - and by the time the row empties, and the text would be
        // readable, the row is gone. It could only ever be in the way.
        None => String::new(),
    }
}

/// A formation tile speaks the **same three states the combat tiles do** - one vocabulary for the whole
/// fight, so what a ring means never depends on which screen you are looking at:
///
/// - **Available** (waiting on you): a hero still in the Pool. It has no rank, and Start is barred until it
///   does - it is *exactly* the "this one still needs you" case the combat steps use the cue for.
/// - **Settled** (decided, still open): a hero standing in a rank. You told it where to stand, and you may
///   tell it somewhere else.
/// - **Idle**: a mustered foe. Nothing is being asked of you about it and nothing ever will be.
fn formation_tile(u: &Combatant, card: CardId, max: u32, rank: Option<Rank>, side: Side) -> Tile {
    let party = side == Side::Party;
    let effective = rank.is_none_or(|r| combat::effective_in_rank(r, u.melee, u.ranged));
    // Health and Tempo are both card stacks you flip, so both read `up / total` — showing only the tempo
    // remainder hid how much of the pool was already spent, which is the whole decision in a bid.
    let mut badges = vec![Badge {
        text: format!(
            "Health {}/{}  Tempo {}/{}",
            u.health, max, u.tempo, u.cadence
        ),
        tone: Tone::Muted,
    }];
    badges.push(stats_badge(u));
    badges.push(reach_badge(u.melee, u.ranged));
    if !effective {
        badges.push(Badge {
            text: "x ineffective here".to_string(),
            tone: Tone::Warn,
        });
    }
    Tile {
        card,
        title: u.name.clone(),
        team: if party { Team::Left } else { Team::Right },
        highlight: match (party, rank) {
            (true, Some(_)) => Highlight::Settled,
            (true, None) => Highlight::Available,
            (false, _) => Highlight::Idle,
        },
        badges,
        // A mustered foe is there to be read, not handled: you cannot drag it, and tapping it does nothing.
        draggable: party,
        tappable: party,
    }
}

// ---- the combat lanes (Strike / React / Extra): two-sided rows + attention arrows -----------------------

#[allow(clippy::too_many_arguments)]
fn build_lanes(
    board: &Board,
    cards: &[CardId],
    units: &[Combatant],
    maxes: &[u32],
    staged: &[Staged],
    contacts: &[Contact],
    active: Option<usize>,
    sub: usize,
    step: Step,
) -> (Vec<Lane>, Vec<Link>) {
    let mut lanes = Vec::new();
    for (label, rank) in arena::RANK_PILES {
        let mut left = Vec::new();
        let mut right = Vec::new();
        for i in 0..units.len() {
            if units[i].rank != rank {
                continue;
            }
            let sel = sel_of(cards, units, staged, contacts, active, sub, step, i);
            let tile = lane_tile(
                board, cards, units, staged, maxes, contacts, active, sub, step, i, sel,
            );
            if units[i].side == Side::Party {
                left.push(tile);
            } else {
                right.push(tile);
            }
        }
        lanes.push(Lane {
            label: label.to_string(),
            left,
            right,
        });
    }

    // Strike targeting arrows: from the armed hero to each foe it lights up (confirmed = its aimed target).
    let mut links = Vec::new();
    if step == Step::Strike
        && let Some(a) = active
    {
        for i in 0..units.len() {
            if units[i].side != Side::Foe {
                continue;
            }
            let broad = units[a].aoe;
            match sel_of(cards, units, staged, contacts, active, sub, step, i) {
                Highlight::Active => links.push(Link {
                    from: cards[a],
                    to: cards[i],
                    confirmed: true,
                    broad,
                }),
                Highlight::Available => links.push(Link {
                    from: cards[a],
                    to: cards[i],
                    confirmed: false,
                    broad,
                }),
                _ => {}
            }
        }
    }
    (lanes, links)
}

/// Whether attacker `atk` can afford the minimum bid to *land* on `tgt` (ceil(F_tgt / F_atk) <= tempo).
fn can_land(atk: &Combatant, tgt: &Combatant) -> bool {
    tgt.finesse.div_ceil(atk.finesse.max(1)).max(1) <= atk.tempo
}

/// The attention state of combatant `i` this step (fallen recedes; the rest mirror the old renderer's `Sel`).
#[allow(clippy::too_many_arguments)]
fn sel_of(
    cards: &[CardId],
    units: &[Combatant],
    staged: &[Staged],
    contacts: &[Contact],
    active: Option<usize>,
    sub: usize,
    step: Step,
    i: usize,
) -> Highlight {
    let u = &units[i];
    if u.fallen {
        return Highlight::Spent;
    }
    let party = u.side == Side::Party;

    // A hero reads in one of four states, the same in every step: **Dim** = nothing is being asked of it;
    // **Available** = it owes you an order; **Active** = it is the one you are commanding (the choice cards
    // below are its orders); **Idle** = it has given one. All four come off the rules' own `can_act` /
    // `owes_order`, so the board, the barred Commit and the schedule cannot disagree.
    //
    // Dim comes FIRST and is unconditional: a hero with no move here - a Raider in the Outrider rank during
    // the Clash, a pairing the schedule simply does not contain - must read as plainly unusable as a foe you
    // cannot select. A warning border on an otherwise normal-looking card reads as "usable, with a caveat".
    if party && step != Step::Marshal {
        if !arena::can_act(units, contacts, sub, step, i) {
            return Highlight::Dim;
        }
        if staged[i].active {
            return Highlight::Active;
        }
        if arena::owes_order(units, contacts, staged, sub, step, i) {
            return Highlight::Available;
        }
        // It has told you what it will do, and you may tell it something else. This used to be Idle - drawn as
        // an ordinary card, indistinguishable from a hero with nothing to say - so "which of these have I
        // already done?", the one question you ask over and over while working through a phase, could only be
        // answered by reading every tile's small print.
        return Highlight::Settled;
    }

    match step {
        // A foe is a target only when the active attacker is effective and can legally reach + afford it.
        Step::Strike => match active {
            Some(a) if staged[a].aim == Some(cards[i]) => Highlight::Active,
            Some(a)
                if combat::effective_in_rank(units[a].rank, units[a].melee, units[a].ranged)
                    && combat::legal_strike(sub, units[a].rank, u.rank)
                    && can_land(&units[a], u) =>
            {
                Highlight::Available
            }
            _ => Highlight::Dim,
        },
        _ => Highlight::Dim,
    }
}

#[allow(clippy::too_many_arguments)]
fn lane_tile(
    board: &Board,
    cards: &[CardId],
    units: &[Combatant],
    staged: &[Staged],
    maxes: &[u32],
    contacts: &[Contact],
    active: Option<usize>,
    sub: usize,
    step: Step,
    i: usize,
    sel: Highlight,
) -> Tile {
    let u = &units[i];
    let team = if u.side == Side::Party {
        Team::Left
    } else {
        Team::Right
    };
    // No rank letter: the tile is *in* its rank's lane, so printing "O" on an Outrider says nothing the
    // position has not already said. A badge has to earn its space.
    let bar_tone = if u.fallen { Tone::Faded } else { Tone::Muted };
    let mut badges = vec![Badge {
        text: format!(
            "Health {}/{}  Tempo {}/{}",
            u.health, maxes[i], u.tempo, u.cadence
        ),
        tone: bar_tone,
    }];
    badges.push(stats_badge(u));
    // The damage pile, whenever it holds anything: it is what makes a second blow this phase worth striking,
    // and its silence is what made 7 + 7 against Grit 9 look like a bug. It wipes at the phase boundary.
    if u.pending > 0 && !u.fallen {
        badges.push(Badge {
            text: format!("Pile {}/{}", u.pending, u.grit),
            tone: Tone::Warn,
        });
    }
    // Why a hero cannot act, said on the card. Drawing it dim answers "can I use this?"; only the words answer
    // "why not?", and without them a rank with no slot this sub-phase looks like a bug in the schedule.
    if !u.fallen && u.side == Side::Party && sel == Highlight::Dim {
        let reason = if !combat::effective_in_rank(u.rank, u.melee, u.ranged) {
            // Off-range: a body in a position whose attack type it does not carry lands nothing (spec 4.2).
            Some(format!("x off-range ({})", reach_word(u.melee, u.ranged)))
        } else if u.tempo == 0 {
            Some("x no tempo left".to_string())
        } else if step == Step::Strike {
            Some(format!(
                "x nothing to strike in the {}",
                SUB_PHASE_NAMES[sub]
            ))
        } else {
            None // React/Extra: it simply is not in this step's conversation (nobody hit it, it hit nobody).
        };
        if let Some(text) = reason {
            badges.push(Badge {
                text,
                tone: Tone::Warn,
            });
        }
    }
    let plan = plan_text(board, &staged[i]);
    if !plan.is_empty() {
        badges.push(Badge {
            text: plan,
            tone: Tone::Good,
        });
    }
    Tile {
        card: cards[i],
        title: u.name.clone(),
        team,
        highlight: sel,
        badges,
        draggable: false,
        // Only tiles a tap will actually *do* something to are tappable. Every tile used to invite a tap, so
        // clicking the three heroes nobody had struck did nothing at all, silently - and an affordance that
        // does nothing is the same lie as a choice with no reason attached (0.8): it teaches nothing and
        // reads as a bug. At React that means only the heroes actually under fire.
        tappable: tap_is_live(units, contacts, active, sub, step, i),
    }
}

/// Whether tapping unit `i` right now does anything — mirrors `arena::handle_tap`, so what the screen offers
/// and what the game accepts cannot drift apart.
fn tap_is_live(
    units: &[Combatant],
    contacts: &[Contact],
    active: Option<usize>,
    sub: usize,
    step: Step,
    i: usize,
) -> bool {
    let u = &units[i];
    if u.fallen {
        return false;
    }
    let party = u.side == Side::Party;
    match step {
        Step::Marshal => party,
        // A hero is tappable exactly when the step is asking it something - the same `can_act` the choice
        // cards and the Commit gate read, so what the screen offers and what the game accepts cannot drift.
        _ if party => arena::can_act(units, contacts, sub, step, i),
        // A foe is tappable only while reaching, as something the selected attacker may legally reach.
        Step::Engage => active.is_some_and(|a| {
            combat::legal_strike(sub, units[a].rank, u.rank)
                && combat::back_access_ok(units, units[a].rank, i)
        }),
        _ => false,
    }
}

/// The staged-plan line: what this unit will do on Commit (`* ` armed, `-> name` aim, `bid N`, a reaction).
fn plan_text(board: &Board, s: &Staged) -> String {
    let mut plan = String::new();
    if s.active {
        plan.push_str("* ");
    }
    if let Some(aim) = s.aim {
        let name = board
            .card(aim)
            .map(|c| c.front_title().to_string())
            .unwrap_or_default();
        plan.push_str(&format!("-> {name} "));
    }
    if s.bid > 0 {
        plan.push_str(&format!("bid {} ", s.bid));
    }
    if s.hold {
        plan.push_str("Hold ");
    }
    if let Some(d) = s.dodge {
        plan.push_str(match d {
            combat::Dodge::Slip => "Slip",
            combat::Dodge::Stand => "Stand",
        });
    }
    plan.trim().to_string()
}

// ---- the combat log ------------------------------------------------------------------------------------

/// **The combat log: what happened, and nothing else.**
///
/// Every line is a change to the state of the table - a tempo card flipped, damage banked, a Health card
/// turned, a body fallen. It used to be a snapshot of the current step that mostly re-stated what you *may* do
/// ("Raider may reach: The Wall (no target yet)"), which the prompt and the choice cards already say, in more
/// detail, right next to it. So the log said nothing the screen did not, and the one line that mattered - the
/// blow that actually landed - was buried in it.
///
/// The events are recorded on the board as they resolve (`arena::note_event`), which is the only way a log can
/// remember: the scene is rebuilt from state every frame, and the state does not know its own history. Grouped
/// under the sub-phase they happened in; a sub-phase where nothing happened does not appear, because nothing
/// happened.
fn build_log(board: &Board, arena: PileId) -> Vec<String> {
    // **This round only.** The journal spans the battle (a fight ends with a card carrying the whole thing),
    // but the panel shows the round you are in: the Reset closed everything before it - tempo stood back up,
    // open wounds closed - so nothing earlier still bears on what you decide next.
    let round = arena::arena_state(board, arena).3;
    arena::round_log(board, arena, round)
}

/// The three **constant** stats a fight actually turns on, compact enough for a tile only a card wide:
/// **M**ight (the damage each blow deals), **G**rit (the bar accumulated damage must cross to turn one health
/// card), **F**inesse (one tempo card is worth this much toward reaching a target, and toward slipping one).
///
/// **Every stat's initial is unique** — M H V G T C F — so a single letter is never ambiguous. That is the
/// whole reason Toughness became Grit: it collided with Tempo, and a legend that has to disambiguate its own
/// abbreviations is not a legend. The rename also makes the shape audible, because the two pools are the same
/// shape: *health is Vitality cards, each Grit strong; tempo is Cadence cards, each Finesse strong.*
///
/// Abbreviated because the tile is 120px; the legend card in the sidebar spells them out. **Vitality and
/// Cadence are deliberately absent here** — they are already on the tile as the *totals* of the two pools
/// (`Health 4/6` is Vitality 6, `Tempo 2/3` is Cadence 3). Naming them twice would say nothing new, and a
/// badge has to earn its space.
fn stats_badge(u: &Combatant) -> Badge {
    Badge {
        text: format!("M {}   G {}   F {}", u.might, u.grit, u.finesse),
        tone: Tone::Muted,
    }
}

fn reach_badge(melee: bool, ranged: bool) -> Badge {
    // Melee reads as a warm (close, aggressive) accent; ranged as a cool (distant) one.
    let (text, tone) = match (melee, ranged) {
        (true, true) => ("melee + ranged", Tone::Muted),
        (true, false) => ("melee", Tone::Warm),
        (false, true) => ("ranged", Tone::Cool),
        (false, false) => ("no strike", Tone::Muted),
    };
    Badge {
        text: text.to_string(),
        tone,
    }
}

fn reach_word(melee: bool, ranged: bool) -> &'static str {
    match (melee, ranged) {
        (true, true) => "melee + ranged",
        (true, false) => "melee",
        (false, true) => "ranged",
        (false, false) => "no strike",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample_table;

    /// Open a fight from the sample board: recruit Vael (a Marksman), march to an encounter, open it.
    fn open_a_fight() -> (Board, PileId) {
        let mut board = sample_table();
        let top = |b: &Board, label: &str| {
            b.pile(b.root_id())
                .unwrap()
                .subpiles()
                .into_iter()
                .find(|&p| b.pile(p).map(|q| q.label.as_str()) == Some(label))
        };
        // The party starts assembled and stationed at the home cell (a hero *is* its kit), so there is
        // nothing to recruit - march the Marksman out to an encounter.
        let locations = top(&board, "Locations").unwrap();
        let place = board
            .pile(locations)
            .unwrap()
            .subpiles()
            .into_iter()
            .find(|&p| {
                board
                    .content_cards(p)
                    .iter()
                    .any(|&c| board.card(c).map(|k| k.card_type()) == Some("encounter"))
            })
            .unwrap();
        let position = board
            .content_cards(board.pile(locations).unwrap().subpiles()[4])
            .into_iter()
            .find(|&c| {
                board.card(c).map(|k| (k.card_type(), k.front_title()))
                    == Some(("hero", "Marksman"))
            })
            .unwrap();
        let progress = top(&board, "Progress").unwrap();
        let _ = board.move_character(position, place, progress);
        let arena = arena::open_fight(&mut board, place).expect("a fight opens");
        (board, arena)
    }

    #[test]
    fn no_fight_means_no_scene() {
        let board = sample_table();
        assert!(
            scene(&board, board.root_id()).is_none(),
            "with no arena up, the game asks for no modal scene (the renderer draws the felt)"
        );
    }

    #[test]
    fn a_fresh_fight_is_a_formation_scene() {
        let (board, _arena) = open_a_fight();
        let s = scene(&board, board.root_id()).expect("a fight is up, so there is a scene");
        // Marshal: assignment rows, one of which is the Heroes pool holding the unranked party.
        let SceneBody::Rows(rows) = &s.body else {
            panic!("a fresh fight (Marshal) is an assignment-rows scene");
        };
        let heroes = rows
            .iter()
            .find(|r| r.label == "Heroes")
            .expect("a Heroes pool row");
        assert!(
            heroes.tiles.iter().any(|t| t.title == "Marksman"),
            "the hero who marched here is a draggable tile in the pool"
        );
        assert!(
            heroes.tiles.iter().all(|t| t.draggable && t.tappable),
            "formation tiles are both drag- and tap-able"
        );
        // The Phase track exists, currently on Marshal; the Start control (index 0) is disabled until ranked.
        assert!(s.tracks.iter().any(|t| t.title == "Phase"));
        assert_eq!(
            s.disabled_controls,
            vec![0],
            "Start is inert until every hero is ranked"
        );
    }

    #[test]
    fn after_start_the_scene_becomes_lanes() {
        let (mut board, arena) = open_a_fight();
        // Rank the lone hero, then Start (commit Marshal) to enter the schedule.
        let hero = arena::sub_pile(&board, arena, arena::POOL)
            .map(|p| board.content_cards(p))
            .unwrap_or_default()
            .into_iter()
            .find(|&c| board.card(c).map(|k| k.card_type()) == Some("unit"))
            .unwrap();
        let outrider = arena::sub_pile(&board, arena, "Outrider").unwrap();
        arena::assign(&mut board, hero, outrider);
        arena::commit(&mut board, arena); // Marshal -> the first sub-phase

        let s = scene(&board, board.root_id()).expect("the fight is still up");
        assert!(
            matches!(s.body, SceneBody::Lanes(_)),
            "once the schedule runs, the scene is two-sided lanes"
        );
        assert!(
            s.tracks.iter().any(|t| t.title == "Step"),
            "the Step track appears once the schedule is running"
        );
    }
}

#[cfg(test)]
mod tap_tests {
    use super::*;
    use crate::combat::Side;
    use deckbound_content::rank::Intention as Rank;

    fn unit(name: &str, side: Side, rank: Rank) -> Combatant {
        let mut c = Combatant::from_stats(name, side, rank, [3, 5, 1, 2, 2], 0, true, false);
        c.tempo = 2;
        c
    }

    /// **The three states must be three looks.** While you work through a phase you ask one question over and
    /// over - *which of these have I already done?* - and the screen has to answer it without being read. A
    /// hero that had given its orders used to draw as `Idle`: an ordinary card, indistinguishable from one with
    /// nothing to say. So the answer was only in the small print.
    #[test]
    fn a_hero_reads_as_dim_settled_or_waiting_on_you() {
        let mut units = vec![
            unit("Raider", Side::Party, Rank::Outrider), // 0 - has orders
            unit("Bastion", Side::Party, Rank::Vanguard), // 1 - still owes them
            unit("Marksman", Side::Party, Rank::Rearguard), // 2 - melee-only, so nothing is asked of it
            unit("The Wall", Side::Foe, Rank::Vanguard),
        ];
        units[2].melee = true;
        units[2].ranged = false; // melee body in the back line: it can never reach anything
        let contacts = vec![];
        let mut staged = vec![Staged::default(); 4];
        staged[0].hold = true; // an order, and a real one

        // Everyone can reach The Wall at the Clash (Vanguard -> Vanguard is sub-phase 3)... except the
        // Outrider and the ineffective Rearguard.
        let look =
            |i, staged: &[Staged]| sel_of(&[], &units, staged, &contacts, None, 3, Step::Engage, i);
        assert_eq!(look(2, &staged), Highlight::Dim, "nothing is asked of it");
        assert_eq!(
            look(1, &staged),
            Highlight::Available,
            "it still owes you an order - the loudest cue on the screen"
        );

        // Give Bastion an order too, and it settles - a different look, not the same one.
        let mut done = staged.clone();
        done[1].hold = true;
        assert_eq!(look(1, &done), Highlight::Settled);
        assert_ne!(
            look(1, &done),
            look(1, &staged),
            "deciding must CHANGE how a hero looks, or the screen cannot say what you have done"
        );
        assert_ne!(look(1, &done), Highlight::Dim, "settled is not spent");

        // ...and the one you are commanding outranks both.
        let mut sel = done.clone();
        sel[1].active = true;
        assert_eq!(look(1, &sel), Highlight::Active);
    }

    /// **A rank row has to say what putting a hero there is FOR.** Marshal is the one blind decision, and it is
    /// the one the whole round hangs on - but a row labelled "Outrider" tells a new player nothing about why
    /// they would ever want one. The hints are the schedule, in words, in the second person.
    #[test]
    fn every_rank_row_says_what_it_is_for() {
        for (rank, must_mention) in [
            (Rank::Outrider, "Rearguard"), // it crosses to kill the enemy back line - that IS the role
            (Rank::Vanguard, "Outriders"), // it screens your own back line from exactly them
            (Rank::Rearguard, "range"),    // it kills from safety
        ] {
            let hint = rank_hint(Some(rank), Side::Party);
            assert!(
                hint.contains(must_mention),
                "the {rank:?} hint must say what it is for: {hint}"
            );
            assert!(hint.len() > 40, "a hint that says nothing is not a hint");
        }
        // **Only a rank gets a hint.** A hint is printed on the drop zone's felt to be *covered* by the cards
        // placed there - so it belongs only where cards arrive and the row starts empty. The muster is never
        // dropped into, and the Pool is the one row whose cards *leave*: its text would sit uncovered exactly
        // while it is full, and by the time it emptied the row would be gone.
        assert!(rank_hint(None, Side::Party).is_empty());
        // ...but the muster gets none: a hint is printed on the felt to be *covered* by the cards you place
        // there, and nothing is ever placed in the muster.
        assert!(rank_hint(None, Side::Foe).is_empty());
    }

    /// **A tile that will not answer a tap must not invite one.** At Evade only a hero that can actually
    /// afford to escape is being asked anything; tapping the rest used to do nothing at all, silently - which is
    /// the same lie as an option with no reason attached.
    #[test]
    fn at_evade_only_a_hero_that_can_escape_is_tappable() {
        let units = vec![
            unit("Raider", Side::Party, Rank::Outrider), // 0 - struck
            unit("Bastion", Side::Party, Rank::Vanguard), // 1 - untouched
            unit("The Wall", Side::Foe, Rank::Vanguard), // 2 - the attacker
        ];
        let contacts = vec![Contact {
            attacker: 2,
            target: 0,
            bid: 2,
        }];
        let live = |i| tap_is_live(&units, &contacts, None, 0, Step::Evade, i);

        assert!(live(0), "the reached hero picks its answer");
        assert!(!live(1), "a hero nobody reached for has nothing to answer");
        assert!(!live(2), "you do not tap the foe reaching for you");
    }

    /// A fallen body answers nothing and aims nothing, whatever the step.
    #[test]
    fn a_fallen_body_is_never_tappable() {
        let mut units = vec![unit("Raider", Side::Party, Rank::Outrider)];
        units[0].fallen = true;
        let contacts = vec![];
        for step in [Step::Marshal, Step::Engage, Step::Evade, Step::Strike] {
            assert!(!tap_is_live(&units, &contacts, None, 0, step, 0));
        }
    }
}
