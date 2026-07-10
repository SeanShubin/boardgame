//! Deckbound as a [`BoardGame`] — the game operating on the **persistent physical board** through the
//! seam (plan §17/§18), the cards-as-truth successor to the retired `CardTableWorld` view-emitter. Equip /
//! march / advance-day are conservation-clean transitions straight over the `Tableau` (PC.2: cards are
//! moved / split / merged / flipped, never minted). Combat stays on the renderer's request path for now;
//! stretch A folds it in here as rank×phase intentions.

use cardtable_model::{Arrangement, BoardGame, CardId, DropTarget, PileId, Tableau, sample_table};

/// The deckbound card-table game, played on a persistent physical board.
#[derive(Clone, Copy)]
pub struct CardTableGame;

/// A legal move over the board — transient (the cards are the state, plan §0.3). Drop-borne:
/// `Equip`/`Unequip`/`March`. Affordance-borne: `AdvanceDay`. (`Fight`/`Arena` join at stretch A.)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intention {
    /// Recruit `identity` with `kit`'s recipe — assemble a character deck from the banks.
    Equip { identity: CardId, kit: CardId },
    /// Un-recruit the character deck whose Zone label is `label` — return every borrowed card.
    Unequip { label: CardId },
    /// March the hero map-position `position` to the adjacent place `to`.
    March { position: CardId, to: PileId },
    /// Advance the day clock (stand the move-markers back up, lay a new Day Passed).
    AdvanceDay,
    /// Open a v2 fight at the combat-ready `place` (a stationed hero + an encounter).
    Fight { place: PileId },
    /// Assign `unit` a rank by moving it into rank pile `to` (the formation drag / drop).
    Assign { unit: CardId, to: PileId },
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

    fn opening(&self) -> Tableau {
        sample_table()
    }

    fn apply(&self, board: &mut Tableau, intentions: &[Intention]) {
        for intention in intentions {
            match *intention {
                Intention::Equip { identity, kit } => equip(board, identity, kit),
                Intention::Unequip { label } => unequip(board, label),
                Intention::March { position, to } => march(board, position, to),
                Intention::AdvanceDay => advance_day(board),
                Intention::Fight { place } => {
                    crate::arena::open_fight(board, place);
                }
                Intention::Assign { unit, to } => crate::arena::assign(board, unit, to),
                Intention::Tap { card } => crate::arena::handle_tap(board, card),
                Intention::Commit => {
                    if let Some(a) = crate::arena::find_arena(board) {
                        if crate::arena::outcome(board, a).is_some() {
                            crate::arena::fold_back(board, a);
                        } else {
                            crate::arena::commit(board, a);
                            // Skip straight past any following steps the player has no decision in (they
                            // resolve greedily), stopping at the next real choice or the fight's end.
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
        board: &Tableau,
        dragged: CardId,
        onto: DropTarget,
    ) -> Option<Intention> {
        match onto {
            // Onto a card, inside a projection (the inn): a hero identity + a kit pair up to equip.
            DropTarget::Card(target) => equip_pair(board, dragged, target)
                .map(|(identity, kit)| Intention::Equip { identity, kit }),
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

    fn tap_intention(&self, board: &Tableau, card: CardId) -> Option<Intention> {
        // While a fight is up, tapping a combatant edits the staged plan for the current step.
        let arena = crate::arena::find_arena(board)?;
        crate::arena::is_combatant(board, arena, card).then_some(Intention::Tap { card })
    }

    fn affordances(&self, board: &Tableau, focus: PileId) -> Vec<(String, Intention)> {
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
}

// ---- intention application — conservation-clean ops over the board ---------------------------------

fn equip(board: &mut Tableau, identity: CardId, kit: CardId) {
    let Some(recipe) = board.card(kit).and_then(|c| c.recipe().cloned()) else {
        return;
    };
    let Some(name) = board.card(identity).map(|c| c.front_title().to_string()) else {
        return;
    };
    let (Some(heroes), Some(stats), Some(numbers), Some(abilities), Some(home), Some(progress)) = (
        board.card(identity).map(|c| c.home()),
        top_deck(board, "Stats"),
        top_deck(board, "Numbers"),
        top_deck(board, "Abilities"),
        home_location(board),
        top_deck(board, "Progress"),
    ) else {
        return;
    };
    let _ = board.equip_character(
        &name, &recipe, heroes, stats, numbers, abilities, home, progress,
    );
}

fn unequip(board: &mut Tableau, label: CardId) {
    if !is_character_label(board, label) {
        return;
    }
    let Some(deck) = board.card(label).map(|c| c.home()) else {
        return;
    };
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

fn march(board: &mut Tableau, position: CardId, to: PileId) {
    if let Some(progress) = top_deck(board, "Progress") {
        let _ = board.move_character(position, to, progress);
    }
}

fn advance_day(board: &mut Tableau) {
    if let (Some(progress), Some(events)) = (top_deck(board, "Progress"), top_deck(board, "Events"))
    {
        let _ = board.advance_day(progress, events);
    }
}

// ---- legality — ported from the renderer's predicates ---------------------------------------------

/// In a projection view (the inn), a hero identity and a kit (recipe) card pair to equip. Returns
/// `(identity, kit)`, or `None` if the drop isn't a legal equip.
fn equip_pair(board: &Tableau, a: CardId, b: CardId) -> Option<(CardId, CardId)> {
    if a == b
        || board
            .pile(board.focus_id())
            .is_none_or(|p| p.projection().is_empty())
    {
        return None;
    }
    let a_kit = board.card(a).is_some_and(|c| c.recipe().is_some());
    let b_kit = board.card(b).is_some_and(|c| c.recipe().is_some());
    match (a_kit, b_kit) {
        (true, false) => Some((b, a)), // a is the kit, b the identity
        (false, true) => Some((a, b)), // b is the kit, a the identity
        _ => None,                     // two kits or two heroes
    }
}

/// Whether `card` is a character deck's Zone label (its home pile `reflects` it).
fn is_character_label(board: &Tableau, card: CardId) -> bool {
    let Some(deck) = board.card(card).map(|c| c.home()) else {
        return false;
    };
    board.pile(deck).and_then(|p| p.reflects()) == Some(card)
}

/// A hero map-position card dropped onto an orthogonally-adjacent place, while drilled into the map.
fn can_march(board: &Tableau, dragged: CardId, dest: PileId) -> bool {
    if top_deck(board, "Locations") != Some(board.focus_id()) {
        return false;
    }
    let Some(card) = board.card(dragged).filter(|c| c.card_type() == "hero") else {
        return false;
    };
    places_adjacent(board, card.home(), dest)
}

/// Orthogonal (Manhattan-1) adjacency of two place piles on the Locations grid.
fn places_adjacent(board: &Tableau, a: PileId, b: PileId) -> bool {
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
fn combat_ready(board: &Tableau, place: PileId) -> bool {
    let cards = board.content_cards(place);
    let has = |t: &str| {
        cards
            .iter()
            .any(|&c| board.card(c).map(|k| k.card_type()) == Some(t))
    };
    has("hero") && has("encounter")
}

fn top_deck(board: &Tableau, label: &str) -> Option<PileId> {
    board
        .pile(board.root_id())?
        .subpiles()
        .into_iter()
        .find(|&s| board.pile(s).map(|p| p.label.as_str()) == Some(label))
}

fn home_location(board: &Tableau) -> Option<PileId> {
    let locations = top_deck(board, "Locations")?;
    board
        .pile(locations)?
        .subpiles()
        .into_iter()
        .find(|&s| board.pile(s).map(|p| p.label.as_str()) == Some("Ashfen Crossing"))
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn card_in(board: &Tableau, pile: PileId, name: &str) -> Option<CardId> {
        board
            .pile(pile)?
            .cards()
            .into_iter()
            .find(|&c| board.card(c).map(|c| c.front_title()) == Some(name))
    }

    #[test]
    fn equip_assembles_a_character_deck_conservation_clean() {
        let game = CardTableGame;
        let mut board = game.opening();
        let root = board.root_id();
        let before = board.physical_card_count(root);

        let heroes = top_deck(&board, "Heroes").unwrap();
        let kit = top_deck(&board, "Kit").unwrap();
        let vael = card_in(&board, heroes, "Vael Thornbrand").expect("Vael is in the Heroes deck");
        let marksman = card_in(&board, kit, "Marksman").expect("Marksman is in the Kit deck");

        game.apply(
            &mut board,
            &[Intention::Equip {
                identity: vael,
                kit: marksman,
            }],
        );

        // A character deck named for the hero now exists, its label being the hero's own card.
        let deck =
            top_deck(&board, "Vael Thornbrand").expect("a Vael character deck was assembled");
        assert!(
            board.pile(deck).unwrap().reflects().is_some(),
            "the deck's label is the hero's own card"
        );
        // Conservation (PC.2): every card was moved / dealt from the banks, never minted.
        assert_eq!(
            board.physical_card_count(root),
            before,
            "physical card count is conserved through equip"
        );
        // The kit recipe card is a reusable spec — untouched.
        assert!(
            card_in(&board, kit, "Marksman").is_some(),
            "the kit stays in the Kit deck"
        );
    }

    #[test]
    fn march_moves_the_map_position() {
        let game = CardTableGame;
        let mut board = game.opening();
        let heroes = top_deck(&board, "Heroes").unwrap();
        let kit = top_deck(&board, "Kit").unwrap();
        let vael = card_in(&board, heroes, "Vael Thornbrand").unwrap();
        let marksman = card_in(&board, kit, "Marksman").unwrap();
        game.apply(
            &mut board,
            &[Intention::Equip {
                identity: vael,
                kit: marksman,
            }],
        );

        // The character's map position starts at the home town (Ashfen Crossing, the grid centre); march it
        // to an orthogonally-adjacent place (index 1 is directly above the centre index 4).
        let home = home_location(&board).expect("Ashfen Crossing");
        let position =
            card_in(&board, home, "Vael Thornbrand").expect("Vael's map position at Ashfen");
        let locations = top_deck(&board, "Locations").unwrap();
        let dest = board.pile(locations).unwrap().subpiles()[1];

        game.apply(&mut board, &[Intention::March { position, to: dest }]);

        assert!(
            card_in(&board, home, "Vael Thornbrand").is_none(),
            "the position left the home town"
        );
        assert!(
            card_in(&board, dest, "Vael Thornbrand").is_some(),
            "the position arrived at the destination"
        );
    }
}
