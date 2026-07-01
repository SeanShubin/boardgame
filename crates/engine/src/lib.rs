//! The shared **card-game toolkit** — concrete, reusable building blocks an implementation uses
//! internally.
//!
//! This crate holds the pieces that are *helpers*, not *contract*: an ordered pile of cards
//! ([`Zone`]) and a seeded, dependency-free generator ([`Rng`]). The rules↔presentation interface
//! (the `Game` trait and the `TableView` family) lives in the separate `contract` crate; nothing here
//! depends on it. Everything in this crate is pure and deterministic given a seed, which keeps games
//! unit-testable and reproducible.

pub mod markdown;
pub mod rng;
pub mod zone;

pub use rng::Rng;
pub use zone::Zone;
