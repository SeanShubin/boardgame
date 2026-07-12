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
use deckbound_content::schedule::{SCHEDULE, SUB_PHASE_NAMES};

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

    // The Start / Commit control (index 0) is inert while the player still owes a decision - an unranked hero
    // at Marshal, an unanswered blow at React. Its label names what is missing (see `arena::commit_label`).
    let disabled_controls = if arena::pending_decision(board, arena).is_some() {
        vec![0]
    } else {
        Vec::new()
    };

    // Every decision on offer right now, as cards: React's answers, Catch's targets and bids, Extra's follow-up
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
            "Catch - tap a hero, then give it an order below. Every hero that can strike must aim or Hold."
        }
        Step::React => {
            "React - the log says what struck you and how. Pick a hero's answer below; each card says what it costs."
        }
        Step::Extra => {
            "Extra strikes - tap a hero that landed a blow, then press it or Hold. Its blows bank into the same pile."
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
    // The Pool of unranked heroes sits at the bottom, where they are dragged up from.
    if let Some(pool) = arena::sub_pile(board, arena, arena::POOL) {
        rows.push(formation_row(board, pool, "Heroes", None, Side::Party));
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
        drop_pile: pile,
        tiles,
    }
}

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
        highlight: Highlight::Idle,
        badges,
        // A mustered foe is there to be read, not handled: you cannot drag it, and tapping it does nothing.
        draggable: party,
        tappable: party,
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

    // A hero reads in one of four states, the same in every step: **Active** = the one you are commanding (the
    // choice cards below are its orders); **Available** = it still owes you an order; **Idle** = it has given
    // one; **Dim** = there is nothing being asked of it. That is exactly the state the Commit gate reads, so
    // the board and the barred Commit can never disagree about who is holding things up.
    if party && step != Step::Marshal {
        if staged[i].active {
            return Highlight::Active;
        }
        if arena::owes_order(units, contacts, staged, sub, step, i) {
            return Highlight::Available;
        }
        let asked = match step {
            Step::Catch => u.tempo > 0 && combat::effective_in_rank(u.rank, u.melee, u.ranged),
            Step::React => contacts.iter().any(|c| c.target == i),
            _ => contacts.iter().any(|c| c.attacker == i),
        };
        return if asked {
            Highlight::Idle
        } else {
            Highlight::Dim
        };
    }

    match step {
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
    // and its silence is what made 7 + 7 against Toughness 9 look like a bug. It wipes at the phase boundary.
    if u.pending > 0 && !u.fallen {
        badges.push(Badge {
            text: format!("Pile {}/{}", u.pending, u.toughness),
            tone: Tone::Warn,
        });
    }
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
        Step::Catch => {
            if party {
                // Select it - but only a body that carries a usable strike here. The order itself comes from
                // a choice card; the tap only says which hero we are giving it to.
                u.tempo > 0 && combat::effective_in_rank(u.rank, u.melee, u.ranged)
            } else {
                // A foe is tappable only as something the *armed* attacker can legally aim at.
                active.is_some_and(|a| {
                    combat::legal_catch(sub, units[a].rank, u.rank)
                        && combat::back_access_ok(units, units[a].rank, i)
                })
            }
        }
        // Only a hero actually struck has an answer to choose.
        Step::React => party && contacts.iter().any(|c| c.target == i),
        Step::Extra => party && u.tempo > 0 && contacts.iter().any(|c| c.attacker == i),
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
            // Say what each hero **may** strike, not merely what it has already chosen. "(no targets chosen
            // yet)" on its own left a player looking at a step that appeared to offer nothing, unable to tell
            // whether that was the rules or a bug - and it read as a flat contradiction: "no targets chosen"
            // when there seemed to be none to choose. Now the line either names what is reachable, or says
            // why that hero cannot strike.
            log.push("Targets".to_string());
            let mut any_actor = false;
            for i in 0..units.len() {
                let u = &units[i];
                if u.side != Side::Party || u.fallen {
                    continue;
                }
                let reachable: Vec<&str> = units
                    .iter()
                    .enumerate()
                    .filter(|(j, v)| {
                        v.side == Side::Foe
                            && !v.fallen
                            && combat::legal_catch(sub, u.rank, v.rank)
                            && combat::back_access_ok(units, u.rank, *j)
                    })
                    .map(|(_, v)| v.name.as_str())
                    .collect();

                let barred = if !combat::effective_in_rank(u.rank, u.melee, u.ranged) {
                    Some(format!(
                        "carries no {} strike here",
                        rank_range_word(u.rank)
                    ))
                } else if u.tempo == 0 {
                    Some("no tempo left".to_string())
                } else if reachable.is_empty() {
                    Some("nothing it may strike this phase".to_string())
                } else {
                    None
                };

                match (barred, staged[i].aim) {
                    (Some(reason), _) => {
                        log.push(format!("  {} - cannot strike: {reason}", u.name));
                    }
                    (None, Some(aim)) => {
                        let aim_name = board
                            .card(aim)
                            .map(|c| c.front_title().to_string())
                            .unwrap_or_default();
                        log.push(format!(
                            "  {} -> {}  (bid {})",
                            u.name, aim_name, staged[i].bid
                        ));
                        any_actor = true;
                    }
                    (None, None) => {
                        log.push(format!(
                            "  {} may strike: {}  (not aimed yet)",
                            u.name,
                            reachable.join(", ")
                        ));
                        any_actor = true;
                    }
                }
            }
            // If this fires, the step should never have stopped here at all - `step_needs_input` gates on the
            // same conditions, so "nobody can strike" and "you are being asked to strike" cannot both be true.
            if !any_actor {
                log.push(
                    "  (nobody can strike this phase - this step should have auto-resolved)"
                        .to_string(),
                );
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
                // Say whether the blow was **melee or ranged**. It decides whether Strike Back is even legal
                // (you answer a foe that approached you; there is nothing to answer a shot with), and it used
                // to be nowhere on the screen - so "may I strike back?" was unanswerable by looking. Reach is
                // position-determined: a Rearguard fires, a Vanguard or Outrider strikes.
                let reach = if combat::rank_is_ranged(units[c.attacker].rank) {
                    "ranged"
                } else {
                    "melee"
                };
                log.push(format!(
                    "  {} struck {}  ({reach}, bid {}, {} might) - {}",
                    units[c.attacker].name,
                    units[c.target].name,
                    c.bid,
                    units[c.attacker].might,
                    react
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

/// Report the auto-passed **steps** (notes of the form `"phase|step|reason|pairs"`).
///
/// It used to collapse these to `Skipped: Intercept`, which was wrong in a way that read as a contradiction:
/// it named the **phase** when what was passed was a **step**, dropped the reason, and implied nothing had
/// happened. But a step is only auto-passed when *you* have no decision to make in it — the enemy still acts.
/// So the screen could say "Skipped: Intercept" while the phase card sat on Intercept and the schedule line
/// said Vanguards may strike Outriders, and all three were true at once: your Catch had no legal target, the
/// enemy's Catch landed a blow, and you are now answering it in that same phase's React.
///
/// So say exactly that: which **step**, why *you* had nothing to decide, and that it resolved on its own.
fn condense_skips(skips: &[String]) -> Vec<String> {
    let mut seen: Vec<(String, String)> = Vec::new();
    let mut out = Vec::new();
    for s in skips {
        let mut it = s.split('|');
        let phase = it.next().unwrap_or("").to_string();
        let step = it.next().unwrap_or("").to_string();
        let reason = it.next().unwrap_or("");
        let pairs = it.next().unwrap_or("");
        let key = (phase.clone(), step.clone());
        if phase.is_empty() || seen.contains(&key) {
            continue;
        }
        seen.push(key);
        let targeting = pairs
            .split(',')
            .filter(|p| !p.is_empty())
            .filter_map(|p| {
                let mut c = p.split('>');
                let a = c.next()?.chars().next()?;
                // A target is a priority list now (`O>R|V|O`); the first is what it wants most.
                let t = c.next()?.chars().next()?;
                Some(format!("{} -> {}", rank_word_of(a), rank_word_of(t)))
            })
            .collect::<Vec<_>>()
            .join(", ");
        if out.is_empty() {
            out.push("Resolved without you (no decision to make)".to_string());
        }
        out.push(format!(
            "  {phase} - {step}: you had {reason} ({targeting}). The enemy still acted."
        ));
    }
    out
}

// ---- rank / reach words --------------------------------------------------------------------------------

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

/// The three **constant** stats a fight actually turns on, compact enough for a tile only a card wide:
/// **M**ight (the damage each strike deals), **F**inesse (the bid multiplier — one tempo card is worth this
/// much toward reaching a target, and toward slipping one), **T**oughness (the gate accumulated damage must
/// cross to flip a health card). Abbreviated because the tile is 120px; the hero's own card spells all five
/// out in full, and so does the log.
///
/// **Vitality and Cadence are deliberately absent** — they are already on the tile, as the *totals* of the two
/// pools: `Health 4/6` is Vitality 6, `Tempo 2/3` is Cadence 3. Naming them again would say nothing new, and a
/// badge has to earn its space.
fn stats_badge(u: &Combatant) -> Badge {
    Badge {
        text: format!("M {}   F {}   T {}", u.might, u.finesse, u.toughness),
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

fn read_skips(board: &Board, arena: PileId) -> Vec<String> {
    board
        .content_cards(arena)
        .into_iter()
        .find(|&c| board.card(c).map(|k| k.card_type()) == Some("skiplog"))
        .and_then(|c| board.card(c))
        .map(|c| c.detail().to_vec())
        .unwrap_or_default()
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

    /// **A tile that will not answer a tap must not invite one.** At React only the heroes actually under
    /// fire have an answer to choose; tapping the rest used to do nothing at all, silently — which is the same
    /// lie as an option with no reason attached, and it is exactly what made "only Raider is selectable, but I
    /// see no options" so baffling.
    #[test]
    fn at_react_only_a_struck_hero_is_tappable() {
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
        let live = |i| tap_is_live(&units, &contacts, None, 0, Step::React, i);

        assert!(live(0), "the struck hero picks its answer");
        assert!(!live(1), "a hero nobody struck has nothing to answer");
        assert!(!live(2), "you do not tap the foe that hit you");
    }

    /// A fallen body answers nothing and aims nothing, whatever the step.
    #[test]
    fn a_fallen_body_is_never_tappable() {
        let mut units = vec![unit("Raider", Side::Party, Rank::Outrider)];
        units[0].fallen = true;
        let contacts = vec![];
        for step in [Step::Marshal, Step::Catch, Step::React, Step::Extra] {
            assert!(!tap_is_live(&units, &contacts, None, 0, step, 0));
        }
    }
}
