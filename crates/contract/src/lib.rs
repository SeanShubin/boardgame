//! The **contract** between a game's rules and its presentation — the pure interface, with no
//! concrete logic and no Bevy.
//!
//! A concrete game implements [`Game`] over its own `State` and `Action` types; a presentation layer
//! consumes that trait and the renderer-agnostic [`TableView`] it produces. Neither side knows the
//! other's internals — this crate is the only thing they must agree on. It contains no game rules and
//! no rendering, only the shape of the conversation between them. Reusable *implementation* helpers
//! (`Zone`, `Rng`) live in the separate `engine` toolkit, which the contract deliberately does not
//! reference.
//!
//! Everything here is pure: a game is a deterministic fold of `Action`s over a seed, which keeps
//! games reproducible and unit-testable.

pub mod game;
pub mod player;
pub mod view;

pub use game::{Game, GameError, Outcome, RefEntry};
pub use player::PlayerId;
pub use view::{
    Accent, Arrangement, CardFace, CardView, Grid, GridCell, GridRow, Layout, MapTile, MapView,
    Pairing, ProseLine, TableView, ZoneView,
};
