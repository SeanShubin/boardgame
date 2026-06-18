//! A presentation-agnostic framework for turn-based tabletop board games.
//!
//! The engine knows nothing about Bevy, rendering, or input. It defines the
//! shape of a game ([`Game`]), reusable building blocks for card games
//! ([`Zone`], [`Rng`]), and a renderer-agnostic snapshot of the table
//! ([`TableView`]) that a presentation layer can draw without understanding
//! any specific game's rules.
//!
//! A concrete game implements [`Game`] over its own `State` and `Action`
//! types. Everything in this crate is pure and deterministic given a seed,
//! which keeps games unit-testable and reproducible.

pub mod game;
pub mod player;
pub mod rng;
pub mod view;
pub mod zone;

pub use game::{Game, GameError, Outcome, RefEntry};
pub use player::PlayerId;
pub use rng::Rng;
pub use view::{Accent, CardFace, CardView, Layout, ProseLine, TableView, ZoneView};
pub use zone::Zone;
