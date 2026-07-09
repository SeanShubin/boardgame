//! Deckbound as a [`BoardGame`] — the game operating on the **persistent physical board** through the
//! seam (plan §17/§18), the cards-as-truth successor to the [`CardTableWorld`](crate::CardTableWorld)
//! emitter. Equip / march / advance-day are conservation-clean transitions straight over the `Tableau`
//! (PC.2: cards are moved / split / merged / flipped, never minted). Combat stays on the renderer's
//! request path for now; stretch A folds it in here as rank×phase intentions.

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

    fn affordances(&self, board: &Tableau, focus: PileId) -> Vec<(String, Intention)> {
        // The day track (Progress zone) offers Advance Day. (Combat affordances join at stretch A.)
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
}
