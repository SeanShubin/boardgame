//! The **Board-game driver** — drives the game-agnostic renderer from a [`BoardGame`] over the
//! *persistent* board (plan §17/§18), the cards-as-truth successor to the `contract::Game` [`GamePlugin`]
//! (which rebuilt a `Board` from a snapshot each frame). Here the board is the single source of truth:
//! the observers **record** a player's gesture into a request resource, and this driver interprets it
//! through the game and mutates the board in place. The renderer never mentions the game type.
//!
//! Record-in-renderer / apply-in-driver (the same pattern as `CombatRequest`): the core observers write
//! [`DropRequest`] / [`AffordanceClick`]; [`apply_drop`] / [`apply_affordance`] drain them; and
//! [`sync_affordances`] fills [`AffordanceLabels`] so the non-generic `redraw` can draw the game's
//! contextual actions as control cards without knowing the game.

use bevy::prelude::*;
use cardtable_model::{Board, BoardGame, CardId, DropTarget, Scene};

use crate::{CardTablePlugin, CardTableSet, NeedsRebuild, Table};

// ---- core request/affordance state (non-generic — the observers record into these) ---------------

/// A drop the renderer recorded for the driver to interpret: the dragged card and what it landed on.
/// Drained by [`apply_drop`]; `None` when idle. Core-owned so the observers record without a game type.
#[derive(Resource, Default)]
pub struct DropRequest(pub Option<(CardId, DropTarget)>);

/// A recorded click on the affordance control card at this index (into [`AffordanceLabels`]).
#[derive(Resource, Default)]
pub struct AffordanceClick(pub Option<usize>);

/// **The rewind history** — a stack of past [`Board`]s, one pushed before each move that changes the board.
/// Popping one restores the table exactly as it was.
///
/// This is trivially possible *because the cards are the state*: the whole game — a fight's formation, every
/// bid, every wound — is cards on the board, so a snapshot of the board is a snapshot of everything, and
/// there is nothing else to wind back. Nothing here knows what a move meant.
///
/// Because a fight is opened by an ordinary move, rewinding past it restores the board from before the fight
/// existed — which lands you back on the location screen with the encounter intact. So one Back card walks
/// all the way out of a combat, decision by decision.
///
/// **Single-player only, and the reason is exact.** Commit is an *information boundary*: it is the moment a
/// decision stops being private and is revealed. Undo is harmless right up to that line — nothing has been
/// disclosed, so taking it back tells nobody anything. Past it, there are no take-backs, because you would be
/// undoing with knowledge you only have *because* the other side revealed themselves: learn what they
/// declared, rewind, re-declare against it. That is precisely what this Back card does — it crosses commits —
/// so a competitive mode must not offer it. Against the AI there is no one to leak to, so it is simply a
/// take-back.
///
/// This is why Commit stays even though it costs a click: it is not friction, it is the line.
#[derive(Resource, Default)]
pub struct BoardHistory(Vec<Board>);

/// How many moves back you can go. Bounded so a long session can't grow without limit; a board is small
/// (a few hundred cards) and a whole fight is far fewer moves than this.
const MAX_UNDO: usize = 250;

impl BoardHistory {
    /// Remember `board` as a step you can come back to. Called *before* the board is changed.
    pub fn push(&mut self, board: &Board) {
        if self.0.len() == MAX_UNDO {
            self.0.remove(0); // drop the oldest step rather than grow without bound
        }
        self.0.push(board.clone());
    }

    /// Step back one move, returning the board as it was, or `None` when there is nothing to undo.
    pub fn pop(&mut self) -> Option<Board> {
        self.0.pop()
    }

    /// Whether there is anything to rewind (the renderer only offers Back when there is).
    pub fn can_undo(&self) -> bool {
        !self.0.is_empty()
    }

    /// Forget everything — the board was replaced wholesale (Start Over), so the old steps lead nowhere.
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

/// A recorded click on the **Back** (rewind) control — pop one step off [`BoardHistory`].
#[derive(Resource, Default)]
pub struct UndoClick(pub bool);

/// Marks the **Back** control card that rewinds one move.
#[derive(Component, Clone, Copy)]
pub struct UndoControl;

/// A recorded **tap** on a board card the driver should interpret through the game's
/// [`tap_intention`](cardtable_model::BoardGame::tap_intention) — the third input verb (beside drop and
/// affordance) for in-place per-card actions (cycling a combatant's bid, picking a reaction). Drained by
/// [`apply_tap`]; `None` when idle. Core-owned so the observers record without a game type.
#[derive(Resource, Default)]
pub struct TapRequest(pub Option<CardId>);

/// The labels of the game actions offered in the current zone — `redraw` draws one control card each,
/// tagged [`AffordanceControl`] with its index. Filled by [`sync_affordances`]; empty with no game/actions.
#[derive(Resource, Default)]
pub struct AffordanceLabels(pub Vec<String>);

/// The **modal scene** the game wants drawn in place of the felt (a combat arena), or `None` for the ordinary
/// table. Filled by [`sync_affordances`] from [`BoardGame::scene`](cardtable_model::BoardGame::scene); the
/// renderer draws it without knowing what it means. Core-owned so `redraw` / the arrow overlay read it
/// without a game type.
#[derive(Resource, Default)]
pub struct SceneState(pub Option<Scene>);

/// A human-readable trace of each resolved drop — the dragged card, what it landed on (the *resolved*
/// [`DropTarget`], not the raw pick-hit), and the outcome. Pushed by [`apply_drop`] (the one place that sees
/// every drop's resolution), drained by the UI debug log so a session is reconstructable from that log alone.
#[derive(Resource, Default)]
pub struct DropTrace(pub Vec<String>);

/// Marks a control card as the affordance at this index (into [`AffordanceLabels`]); clicking it records
/// [`AffordanceClick`].
#[derive(Component, Clone, Copy)]
pub struct AffordanceControl(pub usize);

// ---- the game, and the per-game affordance intentions (generic) ----------------------------------

/// Wraps the game (a bevy-free [`BoardGame`]) so the driver systems can reach it as a resource.
#[derive(Resource)]
pub struct GameRes<G>(pub G);

/// The intentions behind the current zone's affordances, index-aligned with [`AffordanceLabels`].
#[derive(Resource)]
struct Affordances<G: BoardGame>(Vec<G::Intention>);

impl<G: BoardGame> Default for Affordances<G> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

// ---- systems -------------------------------------------------------------------------------------

/// Interpret a recorded drop through the game: a legal move is applied; otherwise, a drop into a pile is
/// the default card move (a drop onto a card that isn't a move does nothing — it settles back).
fn apply_drop<G>(
    mut request: ResMut<DropRequest>,
    mut table: ResMut<Table>,
    game: Res<GameRes<G>>,
    mut rebuild: ResMut<NeedsRebuild>,
    mut history: ResMut<BoardHistory>,
    mut trace: ResMut<DropTrace>,
) where
    G: BoardGame + Send + Sync + 'static,
{
    let Some((dragged, onto)) = request.0.take() else {
        return;
    };
    history.push(&table.0); // remember where we were, so Back can come here
    // Describe the drop before applying it (the cards move on apply). The target is the *resolved*
    // DropTarget (e.g. the map place a march landed on), which the raw pointer event can't report.
    let dragged_name = table
        .0
        .card(dragged)
        .map(|c| c.front_title().to_string())
        .unwrap_or_default();
    let onto_desc = match onto {
        DropTarget::Card(t) => format!(
            "card {}",
            table.0.card(t).map(|c| c.front_title()).unwrap_or_default()
        ),
        DropTarget::Pile(p) => format!(
            "pile [{}]",
            table
                .0
                .pile(p)
                .map(|p| p.label.as_str())
                .unwrap_or_default()
        ),
    };
    let outcome = if let Some(intention) = game.0.drop_intention(&table.0, dragged, onto) {
        game.0.apply(&mut table.0, &[intention]);
        "applied a game move"
    } else if let DropTarget::Pile(dest) = onto {
        let at = table.0.pile(dest).map_or(0, |p| p.cards().len());
        let _ = table.0.move_card(dragged, dest, at);
        "default move into pile"
    } else {
        "no move (settled back)"
    };
    trace.0.push(format!(
        "drop: {dragged_name} onto {onto_desc} -> {outcome}"
    ));
    rebuild.0 = true;
}

/// Interpret a recorded tap through the game: if [`tap_intention`](cardtable_model::BoardGame::tap_intention)
/// recognizes it as a move, apply it. A tap the game ignores is a no-op here (the renderer already handled
/// the click as focus/zoom).
fn apply_tap<G>(
    mut request: ResMut<TapRequest>,
    mut table: ResMut<Table>,
    game: Res<GameRes<G>>,
    mut rebuild: ResMut<NeedsRebuild>,
    mut history: ResMut<BoardHistory>,
) where
    G: BoardGame + Send + Sync + 'static,
{
    let Some(card) = request.0.take() else {
        return;
    };
    if let Some(intention) = game.0.tap_intention(&table.0, card) {
        history.push(&table.0);
        game.0.apply(&mut table.0, &[intention]);
        rebuild.0 = true;
    }
}

/// Apply the game action behind a clicked affordance control card.
fn apply_affordance<G>(
    mut click: ResMut<AffordanceClick>,
    affordances: Res<Affordances<G>>,
    mut table: ResMut<Table>,
    game: Res<GameRes<G>>,
    mut rebuild: ResMut<NeedsRebuild>,
    mut history: ResMut<BoardHistory>,
) where
    G: BoardGame + Send + Sync + 'static,
    G::Intention: Clone + Send + Sync + 'static,
{
    let Some(index) = click.0.take() else {
        return;
    };
    if let Some(intention) = affordances.0.get(index).cloned() {
        history.push(&table.0);
        game.0.apply(&mut table.0, &[intention]);
        rebuild.0 = true;
    }
}

/// **Back** — rewind one move: restore the board exactly as it was before it. Nothing here knows what the
/// move meant; the board *is* the state, so putting the old board back is the entire undo.
fn apply_undo(
    mut click: ResMut<UndoClick>,
    mut history: ResMut<BoardHistory>,
    mut table: ResMut<Table>,
    mut rebuild: ResMut<NeedsRebuild>,
) {
    if !std::mem::take(&mut click.0) {
        return;
    }
    if let Some(previous) = history.pop() {
        table.0 = previous;
        rebuild.0 = true;
    }
}

/// Recompute the current zone's affordances **and** modal scene from the game each frame: labels + intentions
/// for the controls, and the [`SceneState`] the renderer draws. Cheap (small lookups); runs before the Draw
/// set so `redraw` sees fresh state.
fn sync_affordances<G>(
    table: Res<Table>,
    game: Res<GameRes<G>>,
    mut labels: ResMut<AffordanceLabels>,
    mut affordances: ResMut<Affordances<G>>,
    mut scene: ResMut<SceneState>,
) where
    G: BoardGame + Send + Sync + 'static,
    G::Intention: Send + Sync + 'static,
{
    let focus = table.0.focus_id();
    let offered = game.0.affordances(&table.0, focus);
    labels.0 = offered.iter().map(|(label, _)| label.clone()).collect();
    affordances.0 = offered
        .into_iter()
        .map(|(_, intention)| intention)
        .collect();
    scene.0 = game.0.scene(&table.0, focus);
}

// ---- the plugin ----------------------------------------------------------------------------------

/// Drives the [`CardTablePlugin`] core from a [`BoardGame`] `G`: seeds the board from the game's opening
/// position and routes recorded gestures through the game onto the persistent board. Add this instead of a
/// hand-built `Table` to run a game.
pub struct BoardGamePlugin<G>(pub G);

impl<G> Plugin for BoardGamePlugin<G>
where
    G: BoardGame + Clone + Send + Sync + 'static,
    G::Intention: Clone + Send + Sync + 'static,
{
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<CardTablePlugin>() {
            app.add_plugins(CardTablePlugin);
        }
        app.insert_resource(GameRes(self.0.clone()))
            .insert_resource(Table(self.0.opening()))
            .init_resource::<Affordances<G>>()
            .init_resource::<BoardHistory>()
            .init_resource::<UndoClick>()
            .add_systems(
                Update,
                (
                    apply_drop::<G>,
                    apply_tap::<G>,
                    apply_affordance::<G>,
                    apply_undo,
                    sync_affordances::<G>,
                )
                    .chain()
                    .in_set(CardTableSet::Apply),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cardtable_model::Face;

    /// Going Back restores the board **exactly** as it was. This is the whole undo: because the cards are the
    /// state, there is no separate history of "what the move did" to invert - you just put the old board back.
    #[test]
    fn back_restores_the_board_exactly_as_it_was() {
        let mut history = BoardHistory::default();
        let mut board = Board::new();
        let root = board.root_id();
        let deck = board.add_pile(root, "Deck").expect("root exists");
        let before = board.card_count();

        history.push(&board); // remember where we were...
        board
            .add_card(
                deck,
                Face::Up {
                    title: "played".into(),
                },
                None,
            )
            .expect("deck exists"); // ...then make a move
        assert_eq!(board.card_count(), before + 1);

        board = history.pop().expect("a step to go back to");

        assert_eq!(board.card_count(), before, "Back put the old board back");
        assert!(!history.can_undo(), "and there is nothing further back");
    }

    /// The history is bounded, so a long session cannot grow without limit; the oldest step is dropped.
    #[test]
    fn history_is_bounded() {
        let mut history = BoardHistory::default();
        let board = Board::new();
        for _ in 0..(MAX_UNDO + 10) {
            history.push(&board);
        }
        assert_eq!(history.0.len(), MAX_UNDO);
    }

    /// Start Over replaces the board wholesale, so every remembered step leads to a board that no longer
    /// exists. There must be nothing to go Back to.
    #[test]
    fn start_over_leaves_nothing_to_go_back_to() {
        let mut history = BoardHistory::default();
        history.push(&Board::new());
        assert!(history.can_undo());

        history.clear();

        assert!(!history.can_undo());
    }
}
