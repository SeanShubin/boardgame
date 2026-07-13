//! Deckbound as a [`BoardGame`] — the game operating on the **persistent physical board** through the
//! seam (plan §17/§18), the cards-as-truth successor to the retired `CardTableWorld` view-emitter. Equip /
//! march / advance-day are conservation-clean transitions straight over the `Board` (PC.2: cards are
//! moved / split / merged / flipped, never minted). Combat stays on the renderer's request path for now;
//! stretch A folds it in here as rank×phase intentions.

use cardtable_model::{Arrangement, Board, BoardGame, CardId, CardKind, DropTarget, PileId, Scene};

use crate::sample_table;

/// The deckbound card-table game, played on a persistent physical board.
#[derive(Clone, Copy)]
pub struct CardTableGame;

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
    /// Advance the day clock (stand the move-markers back up, lay a new Day Passed).
    AdvanceDay,
    /// Open a v2 fight at the combat-ready `place` (a stationed hero + an encounter).
    Fight { place: PileId },
    /// Assign `unit` a rank by moving it into rank pile `to` (the formation drag / drop).
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
                            crate::arena::commit(board, a);
                            // Walk straight past any following step the player has no decision in - it
                            // resolves greedily - and stop at the next real choice, or the fight's end.
                            //
                            // Nothing needs recording here any more. A skipped step is not a silent one: the
                            // enemy still acts in it, and everything it *does* lands in the event journal like
                            // any other change. The old skip-notes existed only because the log could not
                            // remember what happened; now it can, so "you had no legal target" is answered by
                            // the log simply not mentioning you.
                            let mut guard = 0;
                            while crate::arena::outcome(board, a).is_none()
                                && !crate::arena::step_needs_input(board, a)
                                && guard < 100
                            {
                                crate::arena::commit(board, a);
                                guard += 1;
                            }
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
            // Dropping a card onto another card means nothing: the only rule that used it was the Inn's
            // equip pairing (a hero identity onto a kit), and the Inn is gone.
            DropTarget::Card(_) => None,
            DropTarget::Pile(dest) => {
                // In the arena during Marshal, a hero dropped into a rank row takes that rank (pile membership).
                if let Some(arena) = crate::arena::find_arena(board)
                    && crate::arena::current_step(board, arena) == crate::arena::Step::Marshal
                    && crate::arena::is_rank_pile(board, arena, dest)
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
            Intention::Assign { .. } | Intention::Tap { .. } | Intention::Choose { .. }
        )
    }

    /// The arena's choice cards (a struck hero's reaction). Staging — it changes what will happen on Commit,
    /// and nothing is revealed until then.
    fn choice_intention(&self, board: &Board, index: usize) -> Option<Intention> {
        crate::arena::find_arena(board)?;
        Some(Intention::Choose { index })
    }

    fn tap_intention(&self, board: &Board, card: CardId) -> Option<Intention> {
        // While a fight is up, tapping a combatant edits the staged plan for the current step.
        let arena = crate::arena::find_arena(board)?;
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
        // A combat-ready place (a stationed hero + an encounter) offers a fight.
        if combat_ready(board, focus) {
            return vec![("Fight".to_string(), Intention::Fight { place: focus })];
        }
        // The day track (Progress zone) offers Advance Day.
        if top_deck(board, "Progress") == Some(focus) {
            return vec![("Advance Day".to_string(), Intention::AdvanceDay)];
        }
        Vec::new()
    }

    /// While a fight is up, the game draws it as a modal [`Scene`] (the arena); otherwise the felt.
    fn scene(&self, board: &Board, focus: PileId) -> Option<Scene> {
        crate::scene::scene(board, focus)
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
    if let Some(progress) = top_deck(board, "Progress") {
        let _ = board.move_character(position, to, progress);
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

/// A place is combat-ready when it holds both a stationed hero and an encounter.
fn combat_ready(board: &Board, place: PileId) -> bool {
    let cards = board.content_cards(place);
    let has = |t: &str| {
        cards
            .iter()
            .any(|&c| board.card(c).map(|k| k.card_type()) == Some(t))
    };
    has("hero") && has("encounter")
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
        let game = CardTableGame;
        let card = CardId(1);
        let pile = PileId(1);

        // Staging: freely revisable in place, nothing revealed.
        assert!(!game.is_checkpoint(&Intention::Tap { card }));
        assert!(!game.is_checkpoint(&Intention::Assign {
            unit: card,
            to: pile
        }));

        // Commits: the points of no return.
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
        let game = CardTableGame;
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

    /// A card by front-title within a pile (test helper).
    fn card_in(board: &Board, pile: PileId, name: &str) -> Option<CardId> {
        board
            .pile(pile)?
            .cards()
            .into_iter()
            .find(|&c| board.card(c).map(|c| c.front_title()) == Some(name))
    }

    /// The party starts assembled, so the only half of the round-trip left is the teardown: **un-equipping**
    /// disbands a character deck conservation-clean — every borrowed stat / number / ability card returns to
    /// its bank, and the hero's four copies re-form the `×4` stack in the Heroes reserve.
    #[test]
    fn unequip_disbands_a_character_deck_conservation_clean() {
        let game = CardTableGame;
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
        let game = CardTableGame;
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
}
