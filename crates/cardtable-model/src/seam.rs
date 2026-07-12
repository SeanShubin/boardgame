//! The **Board-based game seam** — how a game declares itself to the card-table renderer while the
//! physical board stays the single source of truth. The renderer is generic over [`BoardGame`]: it turns
//! a player gesture into an [`Intention`](BoardGame::Intention) the game recognizes, then asks the game to
//! [`apply`](BoardGame::apply) it to the persistent [`Board`]. No game logic lives in the renderer; no
//! renderer/Bevy type leaks into the game.
//!
//! Contrast `contract::Game` — the flat `TableView` *snapshot* seam the button sample renders through.
//! This seam instead **mutates one persistent board in place**: the cards are the only state (plan §0),
//! so there is no snapshot to derive and nothing resets between moves.

use crate::model::{Board, CardId, PileId};
use crate::scene::Scene;

/// What a dragged card was dropped onto — the target the game interprets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropTarget {
    /// Dropped onto another card.
    Card(CardId),
    /// Dropped into a pile (its felt), not onto a specific card.
    Pile(PileId),
}

/// A game played on a persistent physical card [`Board`]. The renderer provides all game-agnostic
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
    fn opening(&self) -> Board;

    /// Apply a set of intentions to the board as **one order-free batch** (plan §0.2). A single move is a
    /// one-element set; combat stages several and commits them together.
    fn apply(&self, board: &mut Board, intentions: &[Self::Intention]);

    /// Interpret dropping `dragged` onto `onto`: `Some(intention)` if that is a legal move, `None` if not
    /// (the renderer then settles the card back). Subsumes the renderer's old `can_drop_on_*` predicates.
    fn drop_intention(
        &self,
        board: &Board,
        dragged: CardId,
        onto: DropTarget,
    ) -> Option<Self::Intention>;

    /// The contextual game actions offered in the currently-focused zone — each `(label, intention)`
    /// becomes a clickable **control card** the renderer draws (e.g. "Fight" when a location is ready,
    /// "Advance Day" in the day track). These are *not* board cards the game recognizes on click; the game
    /// *declares* them, so the renderer stays game-agnostic. Replaces the renderer's hardcoded affordance
    /// injection + `location_ready_for_combat`-style predicates. Empty = the zone offers no game action.
    fn affordances(&self, board: &Board, focus: PileId) -> Vec<(String, Self::Intention)>;

    /// Whether applying this intention is a **point of no return** — a step the player should be able to come
    /// **Back** to. This is what the rewind history records; everything else is passed over.
    ///
    /// The distinction is not "big move vs small move", it is **committed vs staged**. A staged decision (a
    /// reaction toggled between Eat / Evade / Strike Back, a rank cycled, an aim moved) is *already* freely
    /// revisable — you simply choose again, in place — so there is nothing for an undo to give you, and
    /// recording it would only force the player to walk back through their own indecision one tap at a time.
    /// A **commit** is different: it is the moment a decision stops being private and is revealed, and after
    /// it there are no take-backs. Those are exactly the steps worth being able to return to — and returning
    /// to one puts you back at the decision with your plan still staged, ready to change it.
    ///
    /// Default: **every** intention is a checkpoint, which is right for a game with no staging layer.
    fn is_checkpoint(&self, intention: &Self::Intention) -> bool {
        let _ = intention;
        true
    }

    /// Interpret a **tap** (single click) on the board card `card`: `Some(intention)` if tapping it is a
    /// legal move, `None` to let the renderer handle the click normally (focus / zoom). This is the third
    /// input verb beside drag ([`drop_intention`](BoardGame::drop_intention)) and zone control
    /// ([`affordances`](BoardGame::affordances)) — for in-place per-card actions the "clicks/drags/piles
    /// only" UI needs, e.g. cycling a combatant's tempo bid or picking a reaction during a fight. Most games
    /// want no tap actions, so the default is `None`.
    fn tap_intention(&self, board: &Board, card: CardId) -> Option<Self::Intention> {
        let _ = (board, card);
        None
    }

    /// A **full-screen modal scene** to draw in place of the felt for the current board, or `None` for the
    /// ordinary table. This is how the game keeps all *presentation of a special mode* (a combat arena) on its
    /// own side of the seam: it returns a pure [`Scene`] (tracks / tiles / rows / arrows / text) that the
    /// renderer draws without knowing any game concept. Interaction still flows through
    /// [`tap_intention`](BoardGame::tap_intention) / [`drop_intention`](BoardGame::drop_intention) /
    /// [`affordances`](BoardGame::affordances). Games with no modal (the default) return `None` and render as
    /// a normal table. `focus` is the currently-focused pile, in case the scene depends on it.
    fn scene(&self, board: &Board, focus: PileId) -> Option<Scene> {
        let _ = (board, focus);
        None
    }
}
