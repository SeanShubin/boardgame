//! Treasure Dive — a small, original push-your-luck card game that exercises
//! the [`engine`] framework.
//!
//! # Rules
//!
//! The deck holds six suits, each with one card of every value from 1 to 6.
//! On your turn you keep two options in front of you:
//!
//! - **Dive** — flip the top card of the deck onto your dive pile. If its suit
//!   is *new* to the pile, it stays and you may keep going. If its suit already
//!   appears in the pile you **bust**: the whole dive pile (and the card you
//!   just flipped) is discarded and your turn ends with nothing banked.
//! - **Surface** — stop, and bank the total value of your dive pile into your
//!   score. Your turn ends.
//!
//! When the deck runs out the active player's pile is banked automatically and
//! the game ends. The highest score wins; equal top scores tie.

pub mod cards;
pub mod game;
pub mod state;

pub use cards::{Card, Suit};
pub use game::{Action, TreasureDive};
pub use state::{PlayerState, State};
