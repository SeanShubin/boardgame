//! The **UI model** — attention plus renderer-fed transient state, kept distinct from the physical
//! card tree (plan §0/§16). The physical model knows nothing of what is focused, what is selected, or
//! how the renderer has sized the felt; those live here. Held by [`Board`](super::Board) during the
//! transition (its public methods delegate); a later step promotes this into the standalone UI-model
//! layer the renderer talks to directly.

use super::{CardId, PileId, Pos};

/// Attention + transient presentation state for a [`Board`](super::Board): which pile is focused,
/// which cards are selected, and the renderer-reported bounds + pinned fixtures.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(super) struct UiModel {
    /// The currently focused (drilled-into) pile.
    pub(super) focus: PileId,
    /// The selected cards.
    pub(super) selection: Vec<CardId>,
    // Renderer-fed, transient: not persisted — re-reported every frame, so a save round-trips without them.
    #[serde(skip, default = "super::physical::default_bounds")]
    pub(super) bounds: Pos,
    /// **Pinned** rectangles `(top-left, size)` — the fixed felt fixtures (the centered zone title, the
    /// Back card) that freely-placed content must settle clear of. In `separate` they take top priority:
    /// placed first, so nothing overrides them; they never move for a card. Fed by the renderer each frame.
    #[serde(skip)]
    pub(super) pinned: Vec<(Pos, Pos)>,
}
