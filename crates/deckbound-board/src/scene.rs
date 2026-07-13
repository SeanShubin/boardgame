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
    // The Lull closes the round: tempo stands back up and every unfinished wound closes with it. It is never
    // "current" - you do not stop there - but it has to be ON the track, because a wound you cannot finish
    // before it is a wound you did not inflict. That deadline is the whole shape of a round now, and a
    // deadline the player cannot see is the same bug as damage that vanishes unremarked.
    phase_items.push(TrackItem {
        label: "Lull".to_string(),
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
        return Highlight::Idle;
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
    // and its silence is what made 7 + 7 against Toughness 9 look like a bug. It wipes at the phase boundary.
    if u.pending > 0 && !u.fallen {
        badges.push(Badge {
            text: format!("Pile {}/{}", u.pending, u.toughness),
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
        log.push(format!("This phase, may reach:  {pretty}"));
    }
    match step {
        // Who may reach whom, and what it would cost - not merely who has already chosen. A hero that cannot
        // act says why, rather than sitting silently unusable (which reads as a bug in the schedule).
        Step::Engage => {
            log.push("Reach".to_string());
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
                            && combat::legal_strike(sub, u.rank, v.rank)
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
                    Some("nothing it may reach this phase".to_string())
                } else {
                    None
                };

                match (barred, staged[i].aim) {
                    (Some(reason), _) => {
                        log.push(format!("  {} - cannot reach: {reason}", u.name));
                    }
                    (None, Some(aim)) => {
                        let aim_name = board
                            .card(aim)
                            .map(|c| c.front_title().to_string())
                            .unwrap_or_default();
                        log.push(format!(
                            "  {} reaches for {}  (committing {} tempo)",
                            u.name, aim_name, staged[i].bid
                        ));
                    }
                    (None, None) if staged[i].hold => {
                        log.push(format!("  {} holds", u.name));
                    }
                    (None, None) => {
                        log.push(format!(
                            "  {} may reach: {}  (no target yet)",
                            u.name,
                            reachable.join(", ")
                        ));
                    }
                }
            }
        }
        // What is reaching for whom, what it committed, and therefore what escaping costs. This is the whole
        // input to the decision: it is on the table precisely so the price is exact.
        Step::Evade => {
            log.push("Reaching you".to_string());
            if contacts.is_empty() {
                log.push("  (nobody was reached)".to_string());
            }
            for c in contacts {
                // Melee or ranged decides whether standing buys you anything: you can answer a body that came
                // to you, but nothing answers a shot from the back line.
                let reach = if combat::rank_is_ranged(units[c.attacker].rank) {
                    "ranged"
                } else {
                    "melee"
                };
                let d = &units[c.target];
                let price = c.bid / d.finesse.max(1) + 1;
                let answer = if d.side == Side::Party {
                    match staged[c.target].dodge {
                        Some(combat::Dodge::Slip) => " - Slip",
                        Some(combat::Dodge::Stand) => " - Stand",
                        None => "",
                    }
                } else {
                    ""
                };
                // Say what they SPENT, not only the value it came to. The player watches tempo cards flip, and
                // the spend is the thing that tells them how heavily this was committed - the value is derived.
                log.push(format!(
                    "  {} reaches {}  ({reach}, spent {} tempo -> value {}, Might {}) - slipping costs {price}{answer}",
                    units[c.attacker].name,
                    d.name,
                    combat::reach_cards(units, c),
                    c.bid,
                    units[c.attacker].might
                ));
            }
        }
        // Contact is made. Each engager's opening blow is already paid for; everything after is one card, one
        // blow - and on a melee edge, the body that was reached may answer along it.
        Step::Strike => {
            log.push("Contact".to_string());
            if contacts.is_empty() {
                log.push("  (nothing was reached - everyone slipped or held)".to_string());
            }
            for c in contacts {
                log.push(format!(
                    "  {} has {} - one opening blow ({} might), already paid for",
                    units[c.attacker].name, units[c.target].name, units[c.attacker].might
                ));
            }
            for i in 0..units.len() {
                let u = &units[i];
                if u.side != Side::Party || u.fallen {
                    continue;
                }
                let Some(t) = combat::strike_target(units, contacts, i) else {
                    continue;
                };
                let answering = !contacts.iter().any(|c| c.attacker == i);
                let plan = if staged[i].bid > 0 {
                    format!(
                        "{} blows ({} damage)",
                        staged[i].bid,
                        staged[i].bid * u.might
                    )
                } else if staged[i].hold {
                    "holding".to_string()
                } else {
                    format!("{} tempo to spend", u.tempo)
                };
                log.push(format!(
                    "  {} {} {} - {plan}",
                    u.name,
                    if answering { "answers" } else { "presses" },
                    units[t].name
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
/// said Vanguards may strike Outriders, and all three were true at once: your Strike had no legal target, the
/// enemy's Strike landed a blow, and you are now answering it in that same phase's React.
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
