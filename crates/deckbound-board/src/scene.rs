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
use deckbound::actor::Intention as Rank;
use deckbound::combat::{SCHEDULE, SUB_PHASE_NAMES};

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

    let (body, links, log) = if marshal {
        (build_formation(board, arena), Vec::new(), Vec::new())
    } else {
        let contacts = arena::read_contacts(board, arena, &cards);
        let active = (0..units.len())
            .find(|&i| units[i].side == Side::Party && staged[i].active && !units[i].fallen);
        let (lanes, links) = build_lanes(
            board, &cards, &units, &maxes, &staged, &contacts, active, sub, step,
        );
        let log = build_log(
            board,
            &cards,
            &units,
            &staged,
            &contacts,
            sub,
            step,
            &read_skips(board, arena),
        );
        (SceneBody::Lanes(lanes), links, log)
    };

    // The Start / Commit control (index 0) is inert during Marshal until every hero is ranked.
    let disabled_controls = if marshal && !arena::formation_complete(board, arena) {
        vec![0]
    } else {
        Vec::new()
    };

    Some(Scene {
        tracks,
        heading,
        prompt,
        body,
        links,
        log,
        disabled_controls,
    })
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
    let mut tracks = vec![Track {
        title: "Phase".to_string(),
        items: phase_items,
    }];
    if !marshal {
        let cur = step_name(step);
        let items = ["Catch", "React", "Extra"]
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
        Step::Catch => "Catch",
        Step::React => "React",
        Step::Extra => "Extra",
    }
}

fn prompt_for(step: Step) -> &'static str {
    match step {
        Step::Catch => {
            "Catch - tap a hero to select it, tap a foe to aim, tap the hero again to raise its bid."
        }
        Step::React => {
            "React - tap a struck hero to cycle Eat / Evade / Strike Back (Strike Back answers a melee blow only)."
        }
        Step::Extra => "Extra strikes - tap a hero in contact to spend its remaining tempo.",
        Step::Marshal => {
            "Formation - drag each hero into a rank row (or tap to cycle), then Start."
        }
    }
}

// ---- the formation (Marshal): assignment rows of party tiles -------------------------------------------

fn build_formation(board: &Board, arena: PileId) -> SceneBody {
    let mut rows = Vec::new();
    for (label, rank) in arena::RANK_PILES {
        if let Some(pile) = arena::sub_pile(board, arena, label) {
            rows.push(formation_row(board, pile, label, Some(rank)));
        }
    }
    // The Pool of unranked heroes sits at the bottom, where they are dragged up from.
    if let Some(pool) = arena::sub_pile(board, arena, arena::POOL) {
        rows.push(formation_row(board, pool, "Heroes", None));
    }
    SceneBody::Rows(rows)
}

/// One assignment row: its party heroes as draggable tiles (foes, pre-ranked, are not shown in formation).
fn formation_row(board: &Board, pile: PileId, label: &str, rank: Option<Rank>) -> Row {
    let mut tiles = Vec::new();
    for card in board.content_cards(pile) {
        if board.card(card).map(|k| k.card_type()) != Some("unit") {
            continue;
        }
        // The rank is only needed to flag an ineffective placement; the Pool (unranked) never flags.
        let Some(u) = arena::read_combatant(board, card, rank.unwrap_or(Rank::Vanguard)) else {
            continue;
        };
        let max = arena::max_health(board, &u.name, Side::Party).max(u.health);
        tiles.push(formation_tile(&u, card, max, rank));
    }
    Row {
        label: label.to_string(),
        drop_pile: pile,
        tiles,
    }
}

fn formation_tile(u: &Combatant, card: CardId, max: u32, rank: Option<Rank>) -> Tile {
    let effective = rank.is_none_or(|r| combat::effective_in_rank(r, u.melee, u.ranged));
    let mut badges = vec![Badge {
        text: format!("HP {}/{}  T{}", u.health, max, u.tempo),
        tone: Tone::Muted,
    }];
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
        team: Team::Left,
        highlight: Highlight::Idle,
        badges,
        draggable: true,
        tappable: true,
    }
}

// ---- the combat lanes (Catch / React / Extra): two-sided rows + attention arrows -----------------------

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
            let tile = lane_tile(board, cards, units, staged, maxes, i, sel);
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

    // Catch targeting arrows: from the armed hero to each foe it lights up (confirmed = its aimed target).
    let mut links = Vec::new();
    if step == Step::Catch
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
    match step {
        // A party unit is selectable only if it has a foe it can legally reach AND afford to land on.
        Step::Catch if party => {
            if staged[i].active {
                Highlight::Active
            } else if u.tempo > 0
                && combat::effective_in_rank(u.rank, u.melee, u.ranged)
                && units.iter().any(|f| {
                    f.side == Side::Foe
                        && !f.fallen
                        && combat::legal_catch(sub, u.rank, f.rank)
                        && can_land(u, f)
                })
            {
                Highlight::Available
            } else {
                Highlight::Dim
            }
        }
        // A foe is a target only when the active attacker is effective and can legally reach + afford it.
        Step::Catch => match active {
            Some(a) if staged[a].aim == Some(cards[i]) => Highlight::Active,
            Some(a)
                if combat::effective_in_rank(units[a].rank, units[a].melee, units[a].ranged)
                    && combat::legal_catch(sub, units[a].rank, u.rank)
                    && can_land(&units[a], u) =>
            {
                Highlight::Available
            }
            _ => Highlight::Dim,
        },
        Step::React if party => {
            let (evade_ok, strikeback_ok) = arena::react_options(units, contacts, i);
            if evade_ok || strikeback_ok {
                if staged[i].react.is_some() {
                    Highlight::Active
                } else {
                    Highlight::Available
                }
            } else {
                Highlight::Dim
            }
        }
        Step::Extra if party && u.tempo > 0 && contacts.iter().any(|c| c.attacker == i) => {
            if staged[i].bid > 0 {
                Highlight::Active
            } else {
                Highlight::Available
            }
        }
        _ => Highlight::Dim,
    }
}

fn lane_tile(
    board: &Board,
    cards: &[CardId],
    units: &[Combatant],
    staged: &[Staged],
    maxes: &[u32],
    i: usize,
    sel: Highlight,
) -> Tile {
    let u = &units[i];
    let team = if u.side == Side::Party {
        Team::Left
    } else {
        Team::Right
    };
    let letter = rank_letter(u.rank);
    let bar_tone = if u.fallen { Tone::Faded } else { Tone::Muted };
    let mut badges = vec![Badge {
        text: format!("{letter}  HP {}/{}  T{}", u.health, maxes[i], u.tempo),
        tone: bar_tone,
    }];
    // Off-range: a living body in a position whose attack type it does not carry lands nothing (spec 4.2).
    if !u.fallen && !combat::effective_in_rank(u.rank, u.melee, u.ranged) {
        badges.push(Badge {
            text: format!("x off-range ({})", reach_word(u.melee, u.ranged)),
            tone: Tone::Warn,
        });
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
        tappable: true,
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
    if let Some(r) = s.react {
        plan.push_str(r.label());
    }
    plan.trim().to_string()
}

// ---- the combat log ------------------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn build_log(
    board: &Board,
    _cards: &[CardId],
    units: &[Combatant],
    staged: &[Staged],
    contacts: &[Contact],
    sub: usize,
    step: Step,
    skips: &[String],
) -> Vec<String> {
    let mut log = condense_skips(skips);
    if let Some(pairs) = SCHEDULE.get(sub)
        && !pairs.is_empty()
    {
        let pretty = pairs
            .iter()
            .map(|(a, t)| {
                format!(
                    "{} -> {} ({})",
                    rank_word(*a),
                    rank_word(*t),
                    rank_range_word(*a)
                )
            })
            .collect::<Vec<_>>()
            .join(",  ");
        log.push(format!("This phase, may strike:  {pretty}"));
    }
    match step {
        Step::Catch => {
            log.push("Targets".to_string());
            let mut any = false;
            for i in 0..units.len() {
                if units[i].side == Side::Party
                    && let Some(aim) = staged[i].aim
                {
                    let aim_name = board
                        .card(aim)
                        .map(|c| c.front_title().to_string())
                        .unwrap_or_default();
                    log.push(format!(
                        "  {} -> {}  (bid {})",
                        units[i].name, aim_name, staged[i].bid
                    ));
                    any = true;
                }
            }
            if !any {
                log.push("  (no targets chosen yet)".to_string());
            }
        }
        Step::React => {
            log.push("Strikes landed & reactions".to_string());
            if contacts.is_empty() {
                log.push("  (nobody was caught)".to_string());
            }
            for c in contacts {
                let react = if units[c.target].side == Side::Party {
                    staged[c.target]
                        .react
                        .map(|r| r.label().to_string())
                        .unwrap_or_else(|| "Eat".to_string())
                } else {
                    "Eat".to_string()
                };
                log.push(format!(
                    "  {} struck {}  (bid {}) - {}",
                    units[c.attacker].name, units[c.target].name, c.bid, react
                ));
            }
        }
        Step::Extra => {
            log.push("Surviving contacts & extra strikes".to_string());
            if contacts.is_empty() {
                log.push("  (no contacts survived)".to_string());
            }
            for c in contacts {
                let act = if units[c.attacker].side != Side::Party {
                    "extra strike (foe)".to_string()
                } else if staged[c.attacker].bid > 0 {
                    format!("extra strike x{}", staged[c.attacker].bid)
                } else {
                    "holding".to_string()
                };
                log.push(format!(
                    "  {} on {} - {}",
                    units[c.attacker].name, units[c.target].name, act
                ));
            }
        }
        Step::Marshal => {}
    }
    log
}

/// Condense the auto-skip notes (`"phase|step|reason|pairs"`) into one `Skipped: {phase} ({targeting})` line
/// per auto-passed sub-phase (the targeting we did not get to, spelled out).
fn condense_skips(skips: &[String]) -> Vec<String> {
    let mut seen: Vec<&str> = Vec::new();
    let mut out = Vec::new();
    for s in skips {
        let mut it = s.split('|');
        let phase = it.next().unwrap_or("");
        let (_step, _reason) = (it.next(), it.next());
        let pairs = it.next().unwrap_or("");
        if phase.is_empty() || seen.contains(&phase) {
            continue;
        }
        seen.push(phase);
        let targeting = pairs
            .split(',')
            .filter(|p| !p.is_empty())
            .filter_map(|p| {
                let mut c = p.split('>');
                let a = c.next()?.chars().next()?;
                let t = c.next()?.chars().next()?;
                Some(format!("{} -> {}", rank_word_of(a), rank_word_of(t)))
            })
            .collect::<Vec<_>>()
            .join(", ");
        out.push(format!("Skipped: {phase} ({targeting})"));
    }
    out
}

// ---- rank / reach words --------------------------------------------------------------------------------

fn rank_letter(rank: Rank) -> char {
    rank.label().chars().next().unwrap_or('?')
}

fn rank_word(rank: Rank) -> &'static str {
    rank.label()
}

/// The full rank name for a one-letter code (from the skiplog's `V>O` codes).
fn rank_word_of(c: char) -> &'static str {
    match c {
        'O' => "Outrider",
        'R' => "Rearguard",
        _ => "Vanguard",
    }
}

fn rank_range_word(rank: Rank) -> &'static str {
    if combat::rank_is_ranged(rank) {
        "ranged"
    } else {
        "melee"
    }
}

fn reach_badge(melee: bool, ranged: bool) -> Badge {
    let (text, tone) = match (melee, ranged) {
        (true, true) => ("melee + ranged", Tone::Muted),
        (true, false) => ("melee", Tone::Cool),
        (false, true) => ("ranged", Tone::Warm),
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

fn read_skips(board: &Board, arena: PileId) -> Vec<String> {
    board
        .content_cards(arena)
        .into_iter()
        .find(|&c| board.card(c).map(|k| k.card_type()) == Some("skiplog"))
        .and_then(|c| board.card(c))
        .map(|c| c.detail().to_vec())
        .unwrap_or_default()
}
