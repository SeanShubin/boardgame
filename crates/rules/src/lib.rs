//! **The pure game rules** - and nothing else.
//!
//! This crate has no dependencies (see `Cargo.toml`), knows nothing about physical cards or rendering, and
//! keeps each category of rules in its own module that does not reach into the others. It exists so that a
//! single file can be read and the rules held whole in the head - and so that the application, solvers, and
//! demo UIs can all drive the same machine through one small interface ([`core::Game`]).
//!
//! - [`core`] - the generic `Game` state-machine interface, the runner, and the tree-walk helpers.
//! - [`combat`] - the combat rules (the regions model), as a `Game`. Knows nothing of any other system.

pub mod combat;
pub mod core;

pub use core::{Game, Outcome, decisions_within, run};
