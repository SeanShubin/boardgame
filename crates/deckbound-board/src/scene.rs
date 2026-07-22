//! **Board -> Scene.** The game's own rendering of a running fight, expressed in the renderer's *rules-blind*
//! [`Scene`] vocabulary (tracks / lanes / tiles / arrows / text). Every combat meaning is decided here and
//! flattened into generic possibilities: a ground pile becomes a lane, "asked this wave" becomes a
//! [`Highlight`], a staged aim becomes a [`Link`]. The renderer draws the result without learning any of it.
//!
//! All text is ASCII (it is drawn to the screen and written to the debug log): `->` not an arrow glyph, `x`
//! not a multiply sign, `*` not a bullet.

use cardtable_model::{
    Badge, Board, Highlight, Lane, Link, PileId, Scene, SceneBody, Team, Tile, Tone, Track,
    TrackItem,
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
        .units
        .iter()
        .map(|u| arena::max_health(board, &u.name, u.side).max(u.health))
        .collect();

    let (k, name) = step_coord(w.step);
    let over = arena::outcome(board, arena).is_some();
    let tracks = build_tracks(w.step);
    let heading = format!("Round {}", w.round);
    let prompt = if over {
        "The fight is decided - read the record, then leave.".to_string()
    } else {
        prompt_for(w.step).to_string()
    };

    let (lanes, links) = build_lanes(board, arena, &w, &maxes);

    let log_title = format!("Round {} - step {k}/8: {name}", w.round);
    let log = arena::round_log(board, arena, w.round as u32);

    // The Commit control (index 0) is inert while the player still owes an order; its label names whose.
    let disabled_controls = if !over && arena::pending_decision(board, arena).is_some() {
        vec![0]
    } else {
        Vec::new()
    };

    let choices = arena::scene_choices(board, arena);

    Some(Scene {
        tracks,
        heading,
        prompt,
        body: SceneBody::Lanes(lanes),
        links,
        choices,
        log_title,
        log,
        legend: stat_legend(),
        reference: crate::targets::schedule_card(),
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

/// The three rank rows, top to bottom: the outriders (each side's bodies loose in the OTHER side's line -
/// the row of bodies past the front), then the two formation tiers. The symmetry does the work: each side's
/// column shows its own bodies by rank, the divider is the front line, and the six ground piles fold into
/// three rows instead of six stacked lanes - wide, not tall.
const RANK_ROWS: [(&str, Rank); 3] = [
    ("Outrider", Rank::Outrider),
    ("Vanguard", Rank::Vanguard),
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
            if w.units[i].side == Side::Party {
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

    // Targeting arrows: from the focused body to its staged aim (confirmed) and its legal targets (offered).
    let mut links = Vec::new();
    if let Some(f) = w.focus {
        let broad = w.units[f].aoe;
        if let Some(Staged::Aim(t)) = w.staged[f] {
            links.push(Link {
                from: w.cards[f],
                to: t,
                confirmed: true,
                broad,
            });
        } else {
            for &t in &w.targets {
                links.push(Link {
                    from: w.cards[f],
                    to: w.cards[t],
                    confirmed: false,
                    broad,
                });
            }
        }
    }
    (lanes, links)
}

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
        return Highlight::Available;
    }
    // A foe: the focused body's aimed target reads Active, its other legal targets Available.
    match w.focus {
        Some(f) if w.staged[f] == Some(Staged::Aim(w.cards[i])) => Highlight::Active,
        Some(_) if w.targets.contains(&i) => Highlight::Available,
        _ => Highlight::Idle,
    }
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
    let plan = plan_text(board, w, i);
    if !plan.is_empty() {
        badges.push(Badge {
            text: plan,
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
    }
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
    w.focus.is_some() && w.targets.contains(&i)
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
    Badge {
        text: format!("M {}   G {}   F {}", u.might, u.grit, u.finesse),
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
            !s.choices.is_empty(),
            "the pending decision has its choice cards"
        );
        assert_eq!(
            s.disabled_controls,
            vec![0],
            "Commit is barred while an order is owed"
        );
    }
}
