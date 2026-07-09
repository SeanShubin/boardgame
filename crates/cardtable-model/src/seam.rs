//! The **Board-based game seam** — how a game declares itself to the card-table renderer while the
//! physical board stays the single source of truth. The renderer is generic over [`BoardGame`]: it turns
//! a player gesture into an [`Intention`](BoardGame::Intention) the game recognizes, then asks the game to
//! [`apply`](BoardGame::apply) it to the persistent [`Tableau`]. No game logic lives in the renderer; no
//! renderer/Bevy type leaks into the game.
//!
//! Contrast `contract::Game` — the flat `TableView` *snapshot* seam the button sample renders through.
//! This seam instead **mutates one persistent board in place**: the cards are the only state (plan §0),
//! so there is no snapshot to derive and nothing resets between moves.

use crate::model::{CardId, PileId, Tableau};

/// What a dragged card was dropped onto — the target the game interprets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropTarget {
    /// Dropped onto another card.
    Card(CardId),
    /// Dropped into a pile (its felt), not onto a specific card.
    Pile(PileId),
}

/// A game played on a persistent physical card [`Tableau`]. The renderer provides all game-agnostic
/// services (draw, drill, drag, focus); the game provides only *meaning* — which gestures are legal moves
/// (**intentions**) and how applying them transforms the board. Every transform is conservation-preserving
/// (cards are moved / split / merged / flipped, never minted), because the physical cards are the sole
/// source of truth.
pub trait BoardGame {
    /// A legal move the game recognizes — the transient value the UI submits (physical cards are the
    /// state; intentions are transient, plan §0.3). Applying a set of them is the only way the board
    /// changes.
    type Intention;

    /// The opening board — every card in its starting place.
    fn opening(&self) -> Tableau;

    /// Apply a set of intentions to the board as **one order-free batch** (plan §0.2). A single move is a
    /// one-element set; combat stages several and commits them together.
    fn apply(&self, board: &mut Tableau, intentions: &[Self::Intention]);

    /// Interpret dropping `dragged` onto `onto`: `Some(intention)` if that is a legal move, `None` if not
    /// (the renderer then settles the card back). Subsumes the renderer's old `can_drop_on_*` predicates.
    fn drop_intention(
        &self,
        board: &Tableau,
        dragged: CardId,
        onto: DropTarget,
    ) -> Option<Self::Intention>;

    /// The contextual game actions offered in the currently-focused zone — each `(label, intention)`
    /// becomes a clickable **control card** the renderer draws (e.g. "Fight" when a location is ready,
    /// "Advance Day" in the day track). These are *not* board cards the game recognizes on click; the game
    /// *declares* them, so the renderer stays game-agnostic. Replaces the renderer's hardcoded affordance
    /// injection + `location_ready_for_combat`-style predicates. Empty = the zone offers no game action.
    fn affordances(&self, board: &Tableau, focus: PileId) -> Vec<(String, Self::Intention)>;
}
