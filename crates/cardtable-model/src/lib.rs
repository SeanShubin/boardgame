//! The **card-table interaction model** — a pure, Bevy-free, game-free representation of cards and
//! piles and the behaviors performed on them: selecting a card, reordering cards within a pile,
//! moving a card from one pile to another, and drilling focus in and out (the recursive zoom that
//! collapses the unattended into piles).
//!
//! This is presentation *state*, not game state and not rendering. Keeping it here means the
//! behaviors unit-test in isolation — no `deckbound`, no `bevy`. The [`model`] core depends on
//! nothing; only [`binding`] touches the [`contract`] crate, to turn a
//! [`TableView`](contract::TableView) into a [`Board`]. The eventual Bevy `cardtable` renderer
//! becomes a thin shell that drives this model and draws it.
//!
//! See `docs/games/deckbound/presentation/card-table-ui.md` for the design this realizes.

pub mod binding;
pub mod model;
pub mod scene;
pub mod seam;

pub use binding::from_table_view;
pub use model::layout;
pub use model::{
    Arrangement, Board, Card, CardId, CardKind, Face, Layout, Node, Pile, PileId, Pos, Recipe,
    Size, TableauError, Utility,
};
pub use scene::{
    Badge, Choice, Highlight, Lane, Link, Row, Scene, SceneBody, Team, Tile, Tone, Track, TrackItem,
};
pub use seam::{BoardGame, DropTarget};
