//! Deckbound as a [`BoardGame`] — the game operating on the **persistent physical board** through the
//! seam (plan §17/§18), the cards-as-truth successor to the retired `CardTableWorld` view-emitter. Equip /
//! march / advance-day are conservation-clean transitions straight over the `Board` (PC.2: cards are
//! moved / split / merged / flipped, never minted). Combat stays on the renderer's request path for now;
//! stretch A folds it in here as rank×phase intentions.

use std::sync::{Arc, Mutex};

use cardtable_model::{
    Arrangement, Board, BoardGame, CardId, CardKind, DropTarget, Outlook, PileId, Scene, SceneBody,
    Utility, oracle_toggle_on,
};

use crate::sample_table;
use rules::combat::step_game::StepCombat;
use rules::core::Solver;

/// Nodes the oracle may visit per frame.
///
/// **Derived, not guessed.** The probe measured the worst full search at 27,270 nodes in 117 ms - about 4.3 us
/// a node - so a frame's 16 ms buys roughly 3,700. Take 2,500 and leave headroom for everything else the frame
/// has to do. The worst position in the game therefore settles in about a dozen frames: a fifth of a second of
/// "evaluating", once, and then the memo is warm and every later look is free.
///
/// This is the *whole* safety mechanism. Raise it and the app hitches; lower it and the badge takes longer to
/// land. Nothing is ever **wrong**, only later - which is exactly what `Evaluating` is for.
const NODE_BUDGET: u64 = 2_500;

/// The doom oracle's per-fight state: the memo (so the first look walks the tree and every later one is a
/// lookup) and the timing it has cost so far.
///
/// It hangs off the game rather than the board because it is **not part of the game state**: it is a *belief
/// about* the state, derived and disposable. Putting it on the board would make Back rewind it, snapshot it,
/// and log it as a physical change - none of which it is.
#[derive(Default)]
pub struct DoomOracle {
    solver: Solver<StepCombat>,
    /// The fight it is about. A new arena is a new tree, and a stale memo would be an answer about a battle
    /// that is not happening.
    fight: Option<PileId>,
    /// Wall-clock spent searching, and the frames it took to settle - the timing you asked to be able to see.
    pub elapsed_ms: f64,
    pub frames: u32,
    /// Whether the row has settled (nothing left Evaluating). Once true, the cost is paid and every later look
    /// is free.
    pub settled: bool,
}

/// The deckbound card-table game, played on a persistent physical board.
///
/// Holds the doom oracle behind a `Mutex` because the seam hands the game to the renderer by shared reference
/// (`scene(&self, ..)`) - the verdict is *derived from* the board, not part of it, so it cannot ride along on
/// the board and must live here instead. A Mutex rather than a RefCell because Bevy requires the game to be
/// `Send + Sync`; it is uncontended (one thread on native, and WebAssembly has only one anyway).
#[derive(Clone, Default)]
pub struct CardTableGame(pub Arc<Mutex<DoomOracle>>);

/// A legal move over the board — transient (the cards are the state, plan §0.3). Drop-borne:
/// `Unequip`/`March`. Affordance-borne: `AdvanceDay`. (`Fight`/`Arena` join at stretch A.)
///
/// There is no `Equip`: the party starts assembled (a hero *is* its kit) and there is no recruit view to
/// pair a hero with a kit in. See the Inn-removal commit — `Equip { identity, kit }` was the drop of a hero
/// identity onto a kit card inside the Inn's projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intention {
    /// Disband the character deck whose Zone label is `label` — return every borrowed card to its bank and
    /// the hero's four copies to the Heroes reserve.
    Unequip { label: CardId },
    /// March the hero map-position `position` to the adjacent place `to`.
    March { position: CardId, to: PileId },
    /// **Enlist** `hero` into `place`'s encounter assignment area (send it to fight), if there is room. See
    /// [`enlist`].
    Enlist { hero: CardId, place: PileId },
    /// **Dismiss** `hero` from `place`'s encounter back to standing at the cell. The inverse of
    /// [`Intention::Enlist`]; see [`dismiss`].
    Dismiss { hero: CardId, place: PileId },
    /// Advance the day clock (stand the move-markers back up, lay a new Day Passed).
    AdvanceDay,
    /// Open a v2 fight at the combat-ready `place` (a stationed hero + an encounter).
    Fight { place: PileId },
    /// Stage `unit`'s movement (Cross / Withdraw) by dropping it on the ground pile the move lands in.
    Assign { unit: CardId, to: PileId },
    /// Take the scene's choice at `index` - the decision the arena is asking for (a struck hero's
    /// reaction). Staging, like `Tap`: nothing is revealed until Commit.
    Choose { index: usize },
    /// Tap a combatant in the arena — edit the staged plan for the current step (cycle rank / select /
    /// bid / aim / react). What it does is read from the fight's step; see [`crate::arena::handle_tap`].
    Tap { card: CardId },
    /// Commit the current fight's step (resolve it), or — once the fight is over — leave the arena.
    Commit,
    /// Cancel the current fight (retreat): tear the arena down with nothing resolved, encounter intact.
    CancelFight,
    /// Restart the current fight: reset combatants and phase to round 1 · Marshal, keeping the formation.
    RestartFight,
}

impl BoardGame for CardTableGame {
    type Intention = Intention;

    fn opening(&self) -> Board {
        sample_table()
    }

    fn apply(&self, board: &mut Board, intentions: &[Intention]) {
        for intention in intentions {
            match *intention {
                Intention::Unequip { label } => unequip(board, label),
                Intention::March { position, to } => march(board, position, to),
                Intention::Enlist { hero, place } => enlist(board, hero, place),
                Intention::Dismiss { hero, place } => dismiss(board, hero, place),
                Intention::AdvanceDay => advance_day(board),
                Intention::Fight { place } => {
                    crate::arena::open_fight(board, place);
                }
                Intention::Assign { unit, to } => crate::arena::assign(board, unit, to),
                Intention::Tap { card } => crate::arena::handle_tap(board, card),
                Intention::Choose { index } => crate::arena::choose(board, index),
                Intention::Commit => {
                    if let Some(a) = crate::arena::find_arena(board) {
                        if crate::arena::outcome(board, a).is_some() {
                            crate::arena::fold_back(board, a);
                        } else {
                            // One commit reveals the staged wave, resolves every step through the next real
                            // party decision (scripted foes and empty waves auto-advance inside), and stops
                            // there - or at the fight's end.
                            crate::arena::commit(board, a);
                        }
                    }
                }
                Intention::CancelFight => {
                    if let Some(a) = crate::arena::find_arena(board) {
                        crate::arena::cancel_fight(board, a);
                    }
                }
                Intention::RestartFight => {
                    if let Some(a) = crate::arena::find_arena(board) {
                        crate::arena::restart_fight(board, a);
                    }
                }
            }
        }
    }

    fn drop_intention(
        &self,
        board: &Board,
        dragged: CardId,
        onto: DropTarget,
    ) -> Option<Intention> {
        match onto {
            // No card-onto-card drops: assignment is a hero dropped into a PILE (the encounter area or the
            // cell), handled below.
            DropTarget::Card(_) => None,
            DropTarget::Pile(dest) => {
                // In the arena, a hero dropped onto the ground pile its move would land in stages the move
                // (Cross / Withdraw) - position is EARNED, so the card walks at resolution, not at the drop.
                if let Some(arena) = crate::arena::find_arena(board)
                    && crate::arena::is_ground_pile(board, arena, dest)
                    && board.card(dragged).map(|c| c.card_type()) == Some("unit")
                {
                    return Some(Intention::Assign {
                        unit: dragged,
                        to: dest,
                    });
                }
                // A character deck's label dropped back on the Heroes deck un-equips.
                if top_deck(board, "Heroes") == Some(dest) && is_character_label(board, dragged) {
                    return Some(Intention::Unequip { label: dragged });
                }
                let hero_home = board
                    .card(dragged)
                    .filter(|c| c.card_type() == "hero")
                    .map(|c| c.home());
                // A **bench** hero (unassigned, standing at the cell) dropped into that cell's encounter area
                // enlists it, if there is room. `dest` is the cell's Assigned marker pile.
                if let Some(place) = board.pile(dest).and_then(|p| p.parent())
                    && assigned_pile(board, place) == Some(dest)
                    && hero_home == Some(place)
                    && !board.is_selected(dragged)
                    && assigned_heroes(board, place).len() < capacity_of(board, place)
                {
                    return Some(Intention::Enlist {
                        hero: dragged,
                        place,
                    });
                }
                // An **assigned** hero dropped back onto its cell (out of the encounter area) is dismissed.
                // `dest` is the cell it stands on and the hero is currently assigned (selected).
                if hero_home == Some(dest) && board.is_selected(dragged) {
                    return Some(Intention::Dismiss {
                        hero: dragged,
                        place: dest,
                    });
                }
                // A hero's map position dropped onto an orthogonally-adjacent place marches there.
                if can_march(board, dragged, dest) {
                    return Some(Intention::March {
                        position: dragged,
                        to: dest,
                    });
                }
                None
            }
        }
    }

    /// Which steps **Back** can return you to: the points of no return, not every fiddle.
    ///
    /// `Assign`, `Tap` and `Choose` are **staging** — arranging the formation, selecting a hero, picking a
    /// reaction, moving an aim, raising a bid. None of it has been revealed, and all of it is already
    /// revisable in place by simply choosing again, so there is nothing an undo could give you. Recording it
    /// would only make Back walk you back through your own indecision one tap at a time.
    ///
    /// Everything else is a **commit** — `Commit` above all, the moment a staged plan is revealed and
    /// resolved, and after which there are no take-backs. Going Back to one puts you at that decision again
    /// with your plan still staged, ready to change. Rewinding past the `Fight` that opened the arena walks
    /// you out of the fight entirely, onto the location you started it from.
    fn is_checkpoint(&self, intention: &Intention) -> bool {
        !matches!(
            intention,
            Intention::Assign { .. }
                | Intention::Tap { .. }
                | Intention::Choose { .. }
                | Intention::Enlist { .. }
                | Intention::Dismiss { .. }
        )
    }

    /// The arena's choice cards (a struck hero's reaction). Staging — it changes what will happen on Commit,
    /// and nothing is revealed until then.
    fn choice_intention(&self, board: &Board, index: usize) -> Option<Intention> {
        crate::arena::find_arena(board)?;
        Some(Intention::Choose { index })
    }

    fn tap_intention(&self, board: &Board, card: CardId) -> Option<Intention> {
        // Off a fight there are no per-card taps: assigning heroes to an encounter is a drag, not a tap.
        let arena = crate::arena::find_arena(board)?;
        // While a fight is up, tapping a combatant edits the staged plan for the current step.
        // An action tile IS a card: tapping one is taking that choice. Its id reads back to the choice index,
        // so the WHAT beat flows through the same tap path as selecting a hero or a target - no button rail.
        if let Some(index) = crate::arena::action_choice_index(board, arena, card) {
            return Some(Intention::Choose { index });
        }
        crate::arena::is_combatant(board, arena, card).then_some(Intention::Tap { card })
    }

    fn affordances(&self, board: &Board, focus: PileId) -> Vec<(String, Intention)> {
        // While a fight is up it is modal: offer its controls (Commit index 0, Cancel index 1) no matter
        // which sub-zone focus has drilled into (e.g. a rank pile clicked during formation).
        if let Some(arena) = crate::arena::find_arena(board) {
            return vec![
                (
                    crate::arena::commit_label(board, arena).to_string(),
                    Intention::Commit,
                ),
                ("Restart battle".to_string(), Intention::RestartFight),
                ("Cancel battle".to_string(), Intention::CancelFight),
            ];
        }
        // A place whose encounter has at least one hero assigned offers to fight, its label the party size.
        if let Some(n) = fightable(board, focus) {
            let plural = if n == 1 { "hero" } else { "heroes" };
            return vec![(
                format!("Fight with {n} {plural}"),
                Intention::Fight { place: focus },
            )];
        }
        // The day track (Progress zone) offers Advance Day.
        if top_deck(board, "Progress") == Some(focus) {
            return vec![("Advance Day".to_string(), Intention::AdvanceDay)];
        }
        Vec::new()
    }

    /// While a fight is up, the game draws it as a modal [`Scene`] (the arena); otherwise the felt.
    fn scene(&self, board: &Board, focus: PileId) -> Option<Scene> {
        let mut scene = crate::scene::scene(board, focus)?;
        // The doom oracle runs only when its System-deck toggle is on. Off (the default), the solver is never
        // touched - no per-frame search, and every tile keeps its `Unknown` outlook, so no foresight badges.
        // A stuck player flips it on to read the winnable lines, then off again; nothing else changes.
        if oracle_enabled(board)
            && let Some(arena) = crate::arena::find_arena(board)
            && let Ok(mut doom) = self.0.lock()
        {
            // A new fight is a new tree. A memo carried over from the last one would be an answer about a
            // battle that is not happening.
            if doom.fight != Some(arena) {
                *doom = DoomOracle {
                    fight: Some(arena),
                    ..Default::default()
                };
            }
            // **Not `Instant` unguarded**: on `wasm32-unknown-unknown` - which is where this ships - it
            // compiles and then *panics* at run time. Time it on native, and simply do not on the web; the
            // number is telemetry, and telemetry may never be the thing that takes the app down.
            let t0 = (!cfg!(target_arch = "wasm32")).then(std::time::Instant::now);
            let outlooks =
                crate::arena::choice_outlooks(board, arena, &mut doom.solver, NODE_BUDGET);
            // While aiming there are no action cards; the choices are the lit enemies, so score each and
            // badge its tile. (One of these two is always empty - action cards exist except during aiming.)
            let foe_outlooks =
                crate::arena::aim_outlook_by_foe(board, arena, &mut doom.solver, NODE_BUDGET);
            if let Some(t0) = t0 {
                doom.elapsed_ms += t0.elapsed().as_secs_f64() * 1000.0;
            }
            doom.frames += 1;

            let was_settled = doom.settled;
            doom.settled = !outlooks.contains(&Outlook::Evaluating)
                && !foe_outlooks.iter().any(|(_, o)| *o == Outlook::Evaluating);
            // Log the cost the moment it settles, once - the honest answer to "how long did exploring this
            // formation take?", which is a number you only get to see if something writes it down.
            if doom.settled && !was_settled {
                let (ms, frames, nodes, known) = (
                    doom.elapsed_ms,
                    doom.frames,
                    doom.solver.nodes(),
                    doom.solver.states(),
                );
                eprintln!(
                    "doom oracle settled: {ms:.1} ms over {frames} frame(s), {nodes} nodes, {known} positions known"
                );
            }
            // The outlooks are index-aligned with `step_choices`, which is the order `build_actions` laid the
            // action tiles in - so this drops each foresight onto the card it belongs to.
            for (t, o) in scene.actions.iter_mut().zip(outlooks) {
                t.outlook = o;
            }
            // Drop each aiming foe's foresight onto its board tile, found by card id.
            for (card, o) in foe_outlooks {
                if let Some(tile) = body_tile_mut(&mut scene, card) {
                    tile.outlook = o;
                }
            }
        }
        Some(scene)
    }
}

/// A mutable handle to the scene body tile for `card`, if it is on the board (a lane tile).
fn body_tile_mut(scene: &mut Scene, card: CardId) -> Option<&mut cardtable_model::Tile> {
    match &mut scene.body {
        SceneBody::Lanes(lanes) => lanes
            .iter_mut()
            .flat_map(|l| l.left.iter_mut().chain(l.right.iter_mut()))
            .find(|t| t.card == card),
        SceneBody::Rows(rows) => rows
            .iter_mut()
            .flat_map(|r| r.tiles.iter_mut())
            .find(|t| t.card == card),
    }
}

// ---- intention application — conservation-clean ops over the board ---------------------------------

fn unequip(board: &mut Board, label: CardId) {
    if !is_character_label(board, label) {
        return;
    }
    let Some(deck) = board.card(label).map(|c| c.home()) else {
        return;
    };
    // Drop the app-only kit manifest (a Virtual readout) first, so the reconcile doesn't mistake it for a
    // borrowed bank card and shove it into the Heroes stack (its default for an unrecognized card type).
    for c in board.content_cards(deck) {
        if board.card(c).map(|k| k.kind()) == Some(CardKind::Virtual) {
            let _ = board.remove_card(c);
        }
    }
    let (Some(heroes), Some(stats), Some(numbers), Some(abilities)) = (
        top_deck(board, "Heroes"),
        top_deck(board, "Stats"),
        top_deck(board, "Numbers"),
        top_deck(board, "Abilities"),
    ) else {
        return;
    };
    let _ = board.unequip_character(deck, heroes, stats, numbers, abilities);
}

fn march(board: &mut Board, position: CardId, to: PileId) {
    // Locations are UNCAPPED: any number of heroes may stand on, pass through, or gather at a cell.
    if let Some(progress) = top_deck(board, "Progress") {
        let _ = board.move_character(position, to, progress);
    }
    // The marched hero arrives on the destination's **bench** (un-assigned): its assignment is re-decided
    // here, not carried from wherever it came. Then auto-fill sends everyone in if they all fit, so a hero
    // marched to a fight it fits into is already assigned when you look at the location.
    board.deselect(position);
    auto_fill(board, to);
}

/// The encounter's **assignment area** on `place` - an (empty) marker sub-pile that is the encounter's drop
/// zone. Assignment itself is tracked by card **selection** (a hero stays standing at the cell whether sent
/// in or not), so this pile holds nothing; it exists only as the target the renderer glows and a drop lands
/// on. Created with the cell (see fixtures).
pub(crate) fn assigned_pile(board: &Board, place: PileId) -> Option<PileId> {
    board
        .pile(place)?
        .subpiles()
        .into_iter()
        .find(|&s| board.pile(s).map(|p| p.label.as_str()) == Some("Assigned"))
}

/// Every hero standing at `place` (its map-position cards - assigned or on the bench alike).
fn heroes_at(board: &Board, place: PileId) -> Vec<CardId> {
    board
        .content_cards(place)
        .into_iter()
        .filter(|&c| board.card(c).map(|k| k.card_type()) == Some("hero"))
        .collect()
}

/// The heroes **assigned** to `place`'s encounter - the ones selected (sent in). They keep standing at the
/// cell; selection is what marks them for the fight.
pub(crate) fn assigned_heroes(board: &Board, place: PileId) -> Vec<CardId> {
    heroes_at(board, place)
        .into_iter()
        .filter(|&c| board.is_selected(c))
        .collect()
}

/// How many heroes `place`'s encounter has **room** for (its capacity), or 0 if it has no encounter.
pub(crate) fn capacity_of(board: &Board, place: PileId) -> usize {
    board
        .pile(place)
        .map(|p| p.label.clone())
        .and_then(|label| deckbound_content::catalog::encounter_for(&label))
        .map(|e| e.capacity)
        .unwrap_or(0)
}

/// **Enlist** `hero` into `place`'s encounter (mark it assigned), if there is still room.
fn enlist(board: &mut Board, hero: CardId, place: PileId) {
    if assigned_heroes(board, place).len() < capacity_of(board, place) {
        let _ = board.select(hero);
    }
}

/// **Dismiss** `hero` from `place`'s encounter back to the bench (un-assign it).
fn dismiss(board: &mut Board, hero: CardId, _place: PileId) {
    board.deselect(hero);
}

/// **Auto-fill** `place`'s encounter: if everyone present fits in its capacity, send them all in. If they do
/// not all fit, leave them on the bench - the player drags heroes in one at a time. Only ever *adds* (never
/// dismisses), so it never fights a manual choice.
pub(crate) fn auto_fill(board: &mut Board, place: PileId) {
    let capacity = capacity_of(board, place);
    let heroes = heroes_at(board, place);
    if capacity > 0 && heroes.len() <= capacity {
        for hero in heroes {
            let _ = board.select(hero);
        }
    }
}

fn advance_day(board: &mut Board) {
    if let (Some(progress), Some(events)) = (top_deck(board, "Progress"), top_deck(board, "Events"))
    {
        let _ = board.advance_day(progress, events);
    }
}

// ---- legality — ported from the renderer's predicates ---------------------------------------------

/// Whether `card` is a character deck's Zone label (its home pile `reflects` it).
fn is_character_label(board: &Board, card: CardId) -> bool {
    let Some(deck) = board.card(card).map(|c| c.home()) else {
        return false;
    };
    board.pile(deck).and_then(|p| p.reflects()) == Some(card)
}

/// A hero map-position card dropped onto an orthogonally-adjacent place, while drilled into the map.
fn can_march(board: &Board, dragged: CardId, dest: PileId) -> bool {
    if top_deck(board, "Locations") != Some(board.focus_id()) {
        return false;
    }
    let Some(card) = board.card(dragged).filter(|c| c.card_type() == "hero") else {
        return false;
    };
    places_adjacent(board, card.home(), dest)
}

/// Orthogonal (Manhattan-1) adjacency of two place piles on the Locations grid.
fn places_adjacent(board: &Board, a: PileId, b: PileId) -> bool {
    let Some(locations) = top_deck(board, "Locations") else {
        return false;
    };
    let Some(Arrangement::Grid { columns }) = board.pile(locations).map(|p| p.layout().arrangement)
    else {
        return false;
    };
    let places = board
        .pile(locations)
        .map(|p| p.subpiles())
        .unwrap_or_default();
    let (Some(ia), Some(ib)) = (
        places.iter().position(|&p| p == a),
        places.iter().position(|&p| p == b),
    ) else {
        return false;
    };
    let (ra, ca) = (ia / columns, ia % columns);
    let (rb, cb) = (ib / columns, ib % columns);
    ra.abs_diff(rb) + ca.abs_diff(cb) == 1
}

// ---- fixed-zone lookups by label ------------------------------------------------------------------

/// The number of heroes **assigned** to `place`'s encounter if a fight can start (at least one is in the
/// assignment area), else `None`. This gates the Fight control and titles it with the party size.
fn fightable(board: &Board, place: PileId) -> Option<usize> {
    let n = assigned_heroes(board, place).len();
    (n >= 1).then_some(n)
}

/// Whether the doom oracle's System-deck toggle is currently on. The state lives nowhere but the toggle
/// card's own title (written by the renderer), so this reads it straight back - no belief carried on the
/// board, no setting duplicated. Absent card (e.g. an old save, or a headless board) reads as off.
fn oracle_enabled(board: &Board) -> bool {
    let Some(system) = top_deck(board, "System") else {
        return false;
    };
    board
        .pile(system)
        .map(|p| p.cards())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|c| board.card(c))
        .any(|c| {
            c.kind() == CardKind::Utility(Utility::ToggleOracle)
                && oracle_toggle_on(c.front_title())
        })
}

fn top_deck(board: &Board, label: &str) -> Option<PileId> {
    board
        .pile(board.root_id())?
        .subpiles()
        .into_iter()
        .find(|&s| board.pile(s).map(|p| p.label.as_str()) == Some(label))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Back returns you to commits, not to every fiddle.** Staging a plan — cycling a rank, toggling a
    /// reaction, moving an aim — is already revisable in place by choosing again, so it is not a step to come
    /// back to; recording it would make Back walk you back through your own indecision one tap at a time.
    #[test]
    fn staging_is_not_a_step_you_can_go_back_to_but_committing_is() {
        let game = CardTableGame::default();
        let card = CardId(1);
        let pile = PileId(1);

        // Staging: freely revisable in place, nothing revealed. Enlisting / dismissing a hero into the
        // encounter is staging too - the fight has not begun, and it is undone by dragging the hero back.
        assert!(!game.is_checkpoint(&Intention::Tap { card }));
        assert!(!game.is_checkpoint(&Intention::Assign {
            unit: card,
            to: pile
        }));
        assert!(!game.is_checkpoint(&Intention::Enlist {
            hero: card,
            place: pile
        }));
        assert!(!game.is_checkpoint(&Intention::Dismiss {
            hero: card,
            place: pile
        }));

        // Commits: the points of no return. Fight opens the arena.
        assert!(game.is_checkpoint(&Intention::Commit));
        assert!(game.is_checkpoint(&Intention::Fight { place: pile }));
        assert!(game.is_checkpoint(&Intention::March {
            position: card,
            to: pile
        }));
        assert!(game.is_checkpoint(&Intention::AdvanceDay));
    }

    #[test]
    fn advance_day_is_offered_in_the_day_track_and_advances_it() {
        let game = CardTableGame::default();
        let mut board = game.opening();
        let progress = top_deck(&board, "Progress").expect("the opening board has a Progress zone");

        assert_eq!(board.current_day(progress), 0, "opens on day 0");
        assert_eq!(
            game.affordances(&board, progress),
            vec![("Advance Day".to_string(), Intention::AdvanceDay)],
            "the day track offers Advance Day"
        );

        game.apply(&mut board, &[Intention::AdvanceDay]);
        assert_eq!(board.current_day(progress), 1, "the day advanced");
    }

    /// A card by front-title within a pile or any of its sub-piles (test helper) - so a hero counts as "at a
    /// cell" whether it stands on the bench or sits in the cell's encounter (assignment) area.
    fn card_in(board: &Board, pile: PileId, name: &str) -> Option<CardId> {
        let p = board.pile(pile)?;
        if let Some(c) = p
            .cards()
            .into_iter()
            .find(|&c| board.card(c).map(|c| c.front_title()) == Some(name))
        {
            return Some(c);
        }
        p.subpiles()
            .into_iter()
            .find_map(|s| card_in(board, s, name))
    }

    /// The party starts assembled, so the only half of the round-trip left is the teardown: **un-equipping**
    /// disbands a character deck conservation-clean — every borrowed stat / number / ability card returns to
    /// its bank, and the hero's four copies re-form the `×4` stack in the Heroes reserve.
    #[test]
    fn unequip_disbands_a_character_deck_conservation_clean() {
        let game = CardTableGame::default();
        let mut board = game.opening();
        let root = board.root_id();
        let before = board.physical_card_count(root);

        let deck = top_deck(&board, "Marksman").expect("the Marksman starts in the party");
        let label = board
            .pile(deck)
            .unwrap()
            .reflects()
            .expect("the deck is labelled by the hero's own card");

        game.apply(&mut board, &[Intention::Unequip { label }]);

        assert!(
            top_deck(&board, "Marksman").is_none(),
            "the character deck was torn down"
        );
        let heroes = top_deck(&board, "Heroes").unwrap();
        assert_eq!(
            card_in(&board, heroes, "Marksman")
                .and_then(|c| board.card(c))
                .map(|c| c.quantity()),
            Some(4),
            "the hero's four copies re-formed the x4 stack in the reserve"
        );
        assert_eq!(
            board.physical_card_count(root),
            before,
            "unequip is conservation-clean (PC.2)"
        );
    }

    #[test]
    fn march_moves_the_map_position() {
        let game = CardTableGame::default();
        let mut board = game.opening();

        // The party starts stationed at the home cell (Ashfen Crossing, the grid centre); march one hero to
        // an orthogonally-adjacent place (index 1 is directly above the centre index 4).
        let locations = top_deck(&board, "Locations").unwrap();
        let home = board.pile(locations).unwrap().subpiles()[4];
        let dest = board.pile(locations).unwrap().subpiles()[1];
        let position =
            card_in(&board, home, "Marksman").expect("the Marksman is stationed at home");

        game.apply(&mut board, &[Intention::March { position, to: dest }]);

        assert!(
            card_in(&board, home, "Marksman").is_none(),
            "the position left the home cell"
        );
        assert!(
            card_in(&board, dest, "Marksman").is_some(),
            "the position arrived at the destination"
        );
    }

    /// **A solo encounter has room for one.** Marching one hero in auto-fills it (it fits); a second stays on
    /// the bench (no room). Fight fields exactly the assigned hero.
    #[test]
    fn a_solo_has_room_for_one_hero() {
        let game = CardTableGame::default();
        let mut board = game.opening();
        let locations = top_deck(&board, "Locations").unwrap();
        let home = board.pile(locations).unwrap().subpiles()[4]; // Ashfen (centre)
        let solo = board.pile(locations).unwrap().subpiles()[1]; // Cinderwatch Keep, a solo (room for 1)

        // Capacity comes from the encounter catalog.
        assert_eq!(capacity_of(&board, solo), 1, "a solo has room for one");

        // March the Raider in: it fits (1 <= 1), so it auto-fills the encounter.
        let raider = card_in(&board, home, "Raider").unwrap();
        game.apply(
            &mut board,
            &[Intention::March {
                position: raider,
                to: solo,
            }],
        );
        assert_eq!(
            assigned_heroes(&board, solo),
            vec![raider],
            "the lone hero auto-fills"
        );
        assert_eq!(
            game.affordances(&board, solo)
                .first()
                .map(|(l, _)| l.clone()),
            Some("Fight with 1 hero".to_string()),
            "Fight is offered with the party size"
        );

        // March the Marksman in: now two present, over capacity, so it stays on the bench.
        let marksman = card_in(&board, home, "Marksman").unwrap();
        game.apply(
            &mut board,
            &[Intention::March {
                position: marksman,
                to: solo,
            }],
        );
        assert_eq!(
            assigned_heroes(&board, solo),
            vec![raider],
            "the second hero does not auto-fill"
        );

        // Fight fields exactly the assigned Raider.
        game.apply(&mut board, &[Intention::Fight { place: solo }]);
        let arena = crate::arena::find_arena(&board).expect("a fight opened");
        let fielded: Vec<String> = crate::arena::wave(&board, arena)
            .unwrap()
            .units
            .iter()
            .filter(|u| u.side == rules::combat::resolve::Side::Party)
            .map(|u| u.name.clone())
            .collect();
        assert_eq!(
            fielded,
            vec!["Raider".to_string()],
            "the solo is fought by the assigned hero"
        );
    }

    /// **Dragging enlists and dismisses a hero**: dropping a bench hero into the encounter area assigns it (up
    /// to capacity), and dropping an assigned hero back on the cell dismisses it. No Fight is offered with an
    /// empty encounter.
    #[test]
    fn dragging_enlists_and_dismisses_a_hero() {
        use cardtable_model::{BoardGame, DropTarget};
        let game = CardTableGame::default();
        let mut board = game.opening();
        let locations = top_deck(&board, "Locations").unwrap();
        let home = board.pile(locations).unwrap().subpiles()[4];
        let solo = board.pile(locations).unwrap().subpiles()[1];
        let area = assigned_pile(&board, solo).expect("the cell has an encounter area");

        // March both heroes in; the first to arrive auto-fills the solo (room for one), the second benches.
        for name in ["Marksman", "Raider"] {
            let h = card_in(&board, home, name).unwrap();
            game.apply(
                &mut board,
                &[Intention::March {
                    position: h,
                    to: solo,
                }],
            );
        }
        // Dismiss whoever auto-filled, to start from an empty encounter.
        for hero in assigned_heroes(&board, solo) {
            game.apply(&mut board, &[Intention::Dismiss { hero, place: solo }]);
        }
        assert!(
            assigned_heroes(&board, solo).is_empty(),
            "encounter starts empty"
        );
        assert!(
            game.affordances(&board, solo).is_empty(),
            "no Fight with an empty encounter"
        );

        let marksman = card_in(&board, solo, "Marksman").unwrap();
        let raider = card_in(&board, solo, "Raider").unwrap();

        // Drop a bench hero into the encounter area: enlist.
        let enlist = game
            .drop_intention(&board, marksman, DropTarget::Pile(area))
            .expect("dropping a bench hero into the encounter enlists it");
        game.apply(&mut board, &[enlist]);
        assert_eq!(
            assigned_heroes(&board, solo),
            vec![marksman],
            "the hero is enlisted"
        );

        // The solo is full (room for one): a second bench hero cannot be enlisted.
        assert!(
            game.drop_intention(&board, raider, DropTarget::Pile(area))
                .is_none(),
            "no room for a second in a solo"
        );

        // Drop the assigned hero back onto the cell: dismiss.
        let dismiss = game
            .drop_intention(&board, marksman, DropTarget::Pile(solo))
            .expect("dropping an assigned hero on the cell dismisses it");
        game.apply(&mut board, &[dismiss]);
        assert!(
            assigned_heroes(&board, solo).is_empty(),
            "the hero is dismissed"
        );
    }

    #[test]
    /// A **party** encounter (a corner) fields everyone standing at the cell - no seating, no cap.
    fn a_party_fights_everyone_present() {
        let game = CardTableGame::default();
        let mut board = game.opening();
        let locations = top_deck(&board, "Locations").unwrap();
        let home = board.pile(locations).unwrap().subpiles()[4]; // Ashfen (centre)
        let corner = board.pile(locations).unwrap().subpiles()[0]; // The Hollow Rampart, a party corner

        for name in ["Marksman", "Raider"] {
            let h = card_in(&board, home, name).unwrap();
            game.apply(
                &mut board,
                &[Intention::March {
                    position: h,
                    to: corner,
                }],
            );
        }
        let arena =
            crate::arena::open_fight(&mut board, corner).expect("a party corner is combat-ready");
        let fielded: Vec<String> = crate::arena::wave(&board, arena)
            .unwrap()
            .units
            .iter()
            .filter(|u| u.side == rules::combat::resolve::Side::Party)
            .map(|u| u.name.clone())
            .collect();
        assert!(
            fielded.contains(&"Marksman".to_string()) && fielded.contains(&"Raider".to_string()),
            "both present heroes field for a party fight: {fielded:?}"
        );
    }
}
