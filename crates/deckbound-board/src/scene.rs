//! **Board -> Scene.** The game's own rendering of a running fight, expressed in the renderer's *rules-blind*
//! [`Scene`] vocabulary (tracks / lanes / tiles / arrows / text). Every combat meaning is decided here and
//! flattened into generic possibilities: a ground pile becomes a lane, "asked this wave" becomes a
//! [`Highlight`], a staged aim becomes a [`Link`]. The renderer draws the result without learning any of it.
//!
//! All text is ASCII (it is drawn to the screen and written to the debug log): `->` not an arrow glyph, `x`
//! not a multiply sign, `*` not a bullet.

use cardtable_model::{
    Badge, Board, Highlight, Lane, Link, Outlook, PileId, Scene, SceneBody, Team, Tile, Tone,
    Track, TrackItem,
};
use rules::combat::regions::Rank;
use rules::combat::resolve::{Combatant, Side};
use rules::combat::step_game::{STEPS, Step, step_coord};

use crate::arena::{self, Staged};

/// Build the modal fight [`Scene`], or `None` when no fight is up (the renderer then draws the felt).
pub fn scene(board: &Board, _focus: PileId) -> Option<Scene> {
    let arena = arena::find_arena(board)?;
    let w = arena::wave(board, arena)?;
    let maxes: Vec<u32> = w
        .cards
        .iter()
        .zip(&w.units)
        .map(|(&c, u)| arena::max_health_on(board, c).max(u.health))
        .collect();

    let (k, name) = step_coord(w.step);
    let over = arena::outcome(board, arena).is_some();
    let tracks = build_tracks(w.step);
    let heading = format!("Round {}", w.round);
    // The text under the heading DESCRIBES THE STEP (name + what it is), then names the one thing to do right
    // now. The step description replaces the old "eight steps" reference card: you are told about the step you
    // are on, not the whole schedule.
    let step_desc = format!("Step {k}/8 - {name}: {}", prompt_for(w.step));
    let action = if over {
        "The fight is decided - read the record, then leave.".to_string()
    } else if w.focus.is_none() && arena::pending_decision(board, arena).is_some() {
        "CHOOSE A HERO - click a ringed hero to give it orders.".to_string()
    } else if let Some(f) = w.focus {
        let name = &w.units[f].name;
        if w.aiming {
            format!("{name}: TARGETING - click a lit enemy, or tap {name} again to back out.")
        } else {
            format!("{name}: CHOOSE AN ORDER - one of the cards below.")
        }
    } else if arena::pending_decision(board, arena).is_none() {
        "All orders given - COMMIT resolves the step.".to_string()
    } else {
        String::new()
    };
    let prompt = if action.is_empty() {
        step_desc
    } else {
        format!("{step_desc}\n>> {action}")
    };

    let (lanes, links) = build_lanes(board, arena, &w, &maxes);

    // The panel shows what happened SINCE THE PLAYER'S LAST ACTION - the resolution of their last commit
    // plus any steps that resolved automatically (skipped waves, foe moves) on the way to this decision -
    // not the whole round. The lines carry their own round/step headers, so the title just frames it.
    let log_title = "What happened (since your last move)".to_string();
    let log = arena::recent_log(board, arena);
    let _ = (k, name); // the step is named in the prompt + the log's own headers

    // The Commit control (index 0) is inert while the player still owes an order; its label names whose.
    let disabled_controls = if !over && arena::pending_decision(board, arena).is_some() {
        vec![0]
    } else {
        Vec::new()
    };

    // The decision is drawn as cards (tiles) in the strip, not a button rail: an action tile takes the ring
    // and can be an arrow endpoint, so the WHAT beat is the same card vocabulary as the WHO and WHOM beats.
    let actions = build_actions(board, arena);

    Some(Scene {
        tracks,
        heading,
        prompt,
        body: SceneBody::Lanes(lanes),
        links,
        choices: Vec::new(),
        actions,
        log_title,
        log,
        legend: stat_legend(),
        // No reference card: the step is described under the heading now (`step_desc`), so the whole-schedule
        // card is gone. (`targets::schedule_card` survives for the reference doc and its test.)
        reference: Vec::new(),
        disabled_controls,
    })
}

/// **What the letters on the tiles mean.** Grouped to show the shape of the game: health is Vitality-many
/// cards, each Grit strong; tempo is Cadence-many cards, each Finesse strong. Might stands alone because it
/// is the only stat that is damage.
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

/// The one progress track: the eight steps of the round in schedule order, the current one lit, and the
/// Reset at the end - the deadline that closes every unfinished wound. (The step deck is the physical
/// truth; this is its fixed-order display.)
fn build_tracks(step: Step) -> Vec<Track> {
    let mut items: Vec<TrackItem> = STEPS
        .into_iter()
        .map(|s| TrackItem {
            label: step_coord(s).1.to_string(),
            current: s == step,
        })
        .collect();
    items.push(TrackItem {
        label: "Reset".to_string(),
        current: false,
    });
    vec![Track {
        title: "Step".to_string(),
        items,
    }]
}

fn prompt_for(step: Step) -> &'static str {
    match step {
        Step::Havoc => {
            "Havoc - point-blank: outriders and their hosts trade, both tiers, no screen. Declare a target or hold."
        }
        Step::Withdraw => {
            "Withdraw - each outrider may rejoin its own line, free; standing the havoc was the price."
        }
        Step::Skirmish => {
            "Skirmish - the fast early trade, and the interception window: strike the body you predict will run. A line strike bars your own crossing."
        }
        Step::Cross => {
            "Crossing - a vanguard that declared no line strike may walk into their line, uncontested, landing as an Outrider."
        }
        Step::Volley => {
            "Defensive Volley - your rearguards fire on enemy outriders, one way, the opening blow only."
        }
        Step::Raid => {
            "Raid - this round's arrivals strike a back-line target: the opening blow only, and it may still dodge."
        }
        Step::Assault => {
            "Assault - all firepower to bear: rearguard fire, and every vanguard that held back swings here."
        }
        Step::Advance => {
            "Advance - a rearguard whose screen has fallen BY NOW is reachable: the same-round advance on a collapsed front."
        }
    }
}

// ---- the battlefield: three rank rows, heroes left of the divider, foes right --------------------------

/// The three rank rows, top to bottom - the battlefield as the two lines meeting, with the outriders shown on
/// the side they crossed INTO:
///
/// | left column        | (divider)      | right column       |
/// |--------------------|----------------|--------------------|
/// | **your Vanguard**  | the front line | **their Vanguard** |
/// | *their* Outriders  |                | *your* Outriders   |
/// | **your Rearguard** | the back edges | **their Rearguard**|
///
/// So the front lines face off at the top, the back lines sit at the outer edges, and an outrider reads as a
/// body loose in the enemy's column - your crosser over on their side, their intruder over on yours. The six
/// ground piles fold into three rows: wide, not tall, and the map matches the fiction.
const RANK_ROWS: [(&str, Rank); 3] = [
    ("Vanguard", Rank::Vanguard),
    ("Outrider", Rank::Outrider),
    ("Rearguard", Rank::Rearguard),
];

fn build_lanes(
    board: &Board,
    _arena: PileId,
    w: &arena::Wave,
    maxes: &[u32],
) -> (Vec<Lane>, Vec<Link>) {
    let mut lanes = Vec::new();
    for (label, rank) in RANK_ROWS {
        let mut left = Vec::new();
        let mut right = Vec::new();
        for i in 0..w.units.len() {
            if w.ranks[i] != rank {
                continue;
            }
            let sel = sel_of(w, i);
            let tile = lane_tile(board, w, maxes, i, sel);
            let party = w.units[i].side == Side::Party;
            // Your bodies sit in the left column, theirs in the right - EXCEPT outriders, which show on the
            // side they crossed into: your crosser is loose in their column (right), their intruder in yours
            // (left). The tile keeps its own team colour; only its column moves.
            let in_left = if rank == Rank::Outrider {
                !party
            } else {
                party
            };
            if in_left {
                left.push(tile);
            } else {
                right.push(tile);
            }
        }
        // An empty rank row still draws: the battlefield's shape is the map the player reads position from,
        // and a row that vanishes when empty makes a crossing look like a teleport to nowhere.
        lanes.push(Lane {
            label: label.to_string(),
            left,
            right,
        });
    }

    // Targeting arrows. A committed aim is a solid arrow, an offered one dotted; while a body aims, every
    // other body's committed arrow is muted so the gesture in progress owns the screen. A SINGLE strike is
    // one arrow to its target; an AREA strike fans one arrow to every body in its footprint - the whole slice
    // it sweeps - so the extent shows on the board, not just the representative the choices collapsed to.
    let aiming_focus = if w.aiming { w.focus } else { None };
    let mut links = Vec::new();
    let push_strike = |links: &mut Vec<Link>, from: usize, to: usize, confirmed: bool| {
        links.push(Link {
            from: w.cards[from],
            to: w.cards[to],
            confirmed,
            broad: false,
            muted: aiming_focus.is_some_and(|f| f != from),
        });
    };
    // Committed aims.
    for i in 0..w.units.len() {
        let Some(Staged::Aim(t)) = w.staged[i] else {
            continue;
        };
        if w.units[i].aoe {
            for m in w.footprints[i].clone() {
                push_strike(&mut links, i, m, true);
            }
        } else if let Some(t) = w.cards.iter().position(|&c| c == t) {
            push_strike(&mut links, i, t, true);
        }
    }
    // The aiming body's offer: one arrow per body its strike would reach (its whole footprint). Capped so a
    // wide reach never turns the board into a hairball (past the cap the lit tiles and cards carry the offer).
    if let Some(f) = aiming_focus
        && w.footprints[f].len() <= OFFERED_ARROW_CAP
    {
        for m in w.footprints[f].clone() {
            push_strike(&mut links, f, m, false);
        }
    }
    (lanes, links)
}

/// The most candidate arrows worth drawing at once. Past this, the offered arrows would read as a tangle, so
/// the ringed cards carry the offer instead.
const OFFERED_ARROW_CAP: usize = 8;

/// The attention state of combatant `i` this wave. A body reads in one of a few states, the same at every
/// step: **Dim** = nothing is being asked of it; **Available** = it owes an order (or is a legal target);
/// **Active** = the one being commanded (or the aimed target); **Settled** = ordered, revisable; **Spent** =
/// fallen.
fn sel_of(w: &arena::Wave, i: usize) -> Highlight {
    let u = &w.units[i];
    if u.fallen {
        return Highlight::Spent;
    }
    if u.side == Side::Party {
        if !w.asked[i] {
            return Highlight::Dim;
        }
        if w.focus == Some(i) {
            return Highlight::Active;
        }
        if w.staged[i].is_some() {
            return Highlight::Settled;
        }
        // The ring is always THE click that advances the gesture: with nothing selected, the unordered
        // heroes carry it (choose WHO); once a hero is selected the buttons are the move, so the others
        // fall back to the steady cue (still tappable to switch).
        return if w.focus.is_none() {
            Highlight::Targeted
        } else {
            Highlight::Available
        };
    }
    // A foe lights up when a strike will reach it. A COMMITTED strike (a staged aim) that catches it reads
    // Active - its fate is decided. The AIMING body's footprint reads Available (lit, tappable - the live
    // offer; the ring lives on the action card now). "Catches it" is the whole slice for an area strike, just
    // the named body for a single one - so an area strike lights its whole extent, not just the representative.
    let committed = (0..w.units.len()).any(|j| match w.staged[j] {
        Some(Staged::Aim(t)) => {
            if w.units[j].aoe {
                w.footprints[j].contains(&i)
            } else {
                w.cards[i] == t
            }
        }
        _ => false,
    });
    if committed {
        return Highlight::Active;
    }
    if let Some(f) = w.focus
        && w.aiming
        && w.footprints[f].contains(&i)
    {
        return Highlight::Available;
    }
    Highlight::Idle
}

fn lane_tile(board: &Board, w: &arena::Wave, maxes: &[u32], i: usize, sel: Highlight) -> Tile {
    let u = &w.units[i];
    let team = if u.side == Side::Party {
        Team::Left
    } else {
        Team::Right
    };
    let bar_tone = if u.fallen { Tone::Faded } else { Tone::Muted };
    let mut badges = vec![Badge {
        text: format!(
            "Health {}/{}  Tempo {}/{}",
            u.health, maxes[i], u.tempo, u.cadence
        ),
        tone: bar_tone,
    }];
    badges.push(stats_badge(u));
    // A fallen body says so in words, like the other "why can't I use this" markers - a downed tile reads
    // downed at a glance, not just faded.
    if u.fallen {
        badges.push(Badge {
            text: if u.horde {
                "x wiped out".to_string()
            } else {
                "x downed".to_string()
            },
            tone: Tone::Warn,
        });
    }
    // Why a party body is not in this wave, said on the card - drawing it dim answers "can I use this?",
    // only the words answer "why not?".
    if !u.fallen && u.side == Side::Party && sel == Highlight::Dim {
        let reason = if u.tempo == 0 {
            Some("x no tempo left".to_string())
        } else {
            Some(format!("x not in the {}", step_coord(w.step).1))
        };
        if let Some(text) = reason {
            badges.push(Badge {
                text,
                tone: Tone::Warn,
            });
        }
    }
    // The attention badge SAYS the state in words, so "which one is selected?" never rides on a border
    // color alone: the commanded body says so, a body still waiting its turn says so, an ordered body
    // names its order.
    if u.side == Side::Party && !u.fallen {
        match sel {
            Highlight::Active => badges.push(Badge {
                text: if w.aiming {
                    "* TARGETING *".to_string()
                } else {
                    "* YOUR ORDERS NOW *".to_string()
                },
                tone: Tone::Good,
            }),
            Highlight::Available => badges.push(Badge {
                text: "waits for orders".to_string(),
                tone: Tone::Warn,
            }),
            _ => {}
        }
    }
    let plan = plan_text(board, w, i);
    if !plan.is_empty() {
        badges.push(Badge {
            text: format!("ordered: {plan}"),
            tone: Tone::Good,
        });
    }
    Tile {
        card: w.cards[i],
        title: u.name.clone(),
        team,
        highlight: sel,
        badges,
        draggable: false,
        // Only tiles a tap will do something to: an asked party body (select / re-ask), or a legal target of
        // the focused one (the aiming shortcut) - mirrors `arena::handle_tap`.
        tappable: tap_is_live(w, i),
        // A combatant carries no foresight badge; that is the action tiles' job.
        outlook: Outlook::Unknown,
    }
}

/// **The decision, as cards.** The focused body's legal declarations for this wave (the WHAT: a verb / hold /
/// crossing answer, or - once aiming - the WHOM targets and the way back), each a real [`Tile`] in the strip
/// above the log rather than a button on a separate rail. Because each is a card it takes the rotating ring
/// when it is a live pick and routes its tap through `tap_intention` (which reads the action id back to a
/// choice index) - so the whole WHO -> WHAT -> WHOM gesture is one card vocabulary. The outlook badge is
/// filled in later, in `board_game::scene`, where the solver runs. Empty when no body is selected (the board
/// is the menu then - the unordered heroes carry the ring).
fn build_actions(board: &Board, arena: PileId) -> Vec<Tile> {
    arena::step_choices(board, arena)
        .into_iter()
        .enumerate()
        .map(|(i, (choice, action))| {
            let _ = action;
            let barred = !choice.enabled();
            let highlight = if barred {
                Highlight::Dim
            } else if choice.chosen {
                // The staged pick reads "decided" - the same settled cue an ordered body wears.
                Highlight::Settled
            } else {
                // A live pick that advances the gesture wears the ring - the same invitation the WHO heroes
                // and the WHOM targets carry.
                Highlight::Targeted
            };
            let mut badges = Vec::new();
            let line = if barred {
                &choice.why_not
            } else {
                &choice.consequence
            };
            if !line.is_empty() {
                badges.push(Badge {
                    text: line.clone(),
                    tone: if barred { Tone::Warn } else { Tone::Muted },
                });
            }
            Tile {
                card: arena::action_card_id(i),
                title: choice.label.clone(),
                team: Team::Left,
                highlight,
                badges,
                draggable: false,
                tappable: !barred,
                outlook: Outlook::Unknown,
            }
        })
        .collect()
}

/// Whether tapping unit `i` right now does anything - mirrors `arena::handle_tap`, so what the screen offers
/// and what the game accepts cannot drift apart.
fn tap_is_live(w: &arena::Wave, i: usize) -> bool {
    let u = &w.units[i];
    if u.fallen {
        return false;
    }
    if u.side == Side::Party {
        return w.asked[i];
    }
    // A foe tap completes the gesture - live for any body in the aiming striker's footprint (the whole lit
    // slice for an area strike), mirroring `arena::handle_tap`.
    w.aiming && w.focus.is_some_and(|f| w.footprints[f].contains(&i))
}

/// The staged-order line: what this body will do when the wave commits.
fn plan_text(board: &Board, w: &arena::Wave, i: usize) -> String {
    match w.staged[i] {
        Some(Staged::Aim(t)) => {
            let name = board
                .card(t)
                .map(|c| c.front_title().to_string())
                .unwrap_or_default();
            format!("-> {name}")
        }
        Some(Staged::Hold) => "Hold".to_string(),
        Some(Staged::Go) => match w.step {
            Step::Withdraw => "Withdraw".to_string(),
            _ => "Cross".to_string(),
        },
        Some(Staged::Stay) => "Stay".to_string(),
        None => String::new(),
    }
}

/// The three **constant** stats a fight turns on, compact enough for a tile: **M**ight, **G**rit,
/// **F**inesse. Vitality and Cadence are already on the tile as the totals of the two pools.
fn stats_badge(u: &Combatant) -> Badge {
    // The strike shape rides the stat line so you can tell at a glance whether a body hits ONE target or
    // sweeps an AREA (a whole enemy slice) - it changes how you read its threat and how you aim it.
    let shape = if u.aoe { "area" } else { "single" };
    Badge {
        text: format!("M {}   G {}   F {}   {shape}", u.might, u.grit, u.finesse),
        tone: Tone::Muted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::sample_table;

    /// The fight scene builds from a live arena: the battlefield lanes, a lit step track, choice cards for
    /// the pending decision, and the Commit control barred while an order is owed.
    #[test]
    fn the_fight_scene_builds() {
        let mut board = sample_table();
        let locations = crate::arena::find_arena(&board);
        assert!(locations.is_none(), "no fight yet, no scene");
        assert!(scene(&board, board.root_id()).is_none());

        // Station the Raider at the Wall solo and open the fight.
        let locs = board
            .pile(board.root_id())
            .unwrap()
            .subpiles()
            .into_iter()
            .find(|&p| board.pile(p).map(|q| q.label.as_str()) == Some("Locations"))
            .unwrap();
        let ashfen = board.pile(locs).unwrap().subpiles()[4];
        let vault = board
            .pile(locs)
            .unwrap()
            .subpiles()
            .into_iter()
            .find(|&p| board.pile(p).map(|q| q.label.as_str()) == Some("The Sundered Vault"))
            .unwrap();
        let hero = board
            .content_cards(ashfen)
            .into_iter()
            .find(|&c| {
                board.card(c).map(|k| (k.card_type(), k.front_title())) == Some(("hero", "Raider"))
            })
            .unwrap();
        let progress = board
            .pile(board.root_id())
            .unwrap()
            .subpiles()
            .into_iter()
            .find(|&p| board.pile(p).map(|q| q.label.as_str()) == Some("Progress"))
            .unwrap();
        let _ = board.move_character(hero, vault, progress);
        let arena = crate::arena::open_fight(&mut board, vault).expect("opens");

        let s = scene(&board, arena).expect("a fight scene");
        assert_eq!(s.tracks.len(), 1, "one step track");
        assert!(
            s.tracks[0].items.iter().any(|i| i.current),
            "the current step is lit"
        );
        let SceneBody::Lanes(lanes) = &s.body else {
            panic!("the battlefield is lanes");
        };
        assert_eq!(lanes.len(), 3, "one row per rank - wide, not tall");
        assert!(
            s.actions.is_empty(),
            "no action cards until a hero is selected - the ringed heroes are the menu"
        );
        // Select the asked hero: the order cards appear.
        let w = crate::arena::wave(&board, arena).unwrap();
        let i = (0..w.units.len()).find(|&i| w.asked[i]).unwrap();
        crate::arena::handle_tap(&mut board, w.cards[i]);
        let s = scene(&board, arena).expect("a fight scene");
        assert!(
            !s.actions.is_empty(),
            "the selected hero has its order cards"
        );
        assert!(
            s.actions.iter().any(|t| t.highlight == Highlight::Targeted),
            "a live order card carries the ring, like the WHO and WHOM beats"
        );
        assert_eq!(
            s.disabled_controls,
            vec![0],
            "Commit is barred while an order is owed"
        );
    }
}
