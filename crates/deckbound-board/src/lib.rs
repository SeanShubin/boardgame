//! **Deckbound as a card-table game** — the deckbound-side implementation of the [`BoardGame`] seam over
//! the *persistent physical board* (plan §17/§18). [`CardTableGame`] operates directly on the conserved
//! card `Board`: recruit / march / advance-day as conservation-clean transitions (PC.2 — cards are
//! moved / split / merged / flipped, never minted). Combat folds in here at stretch A.
//!
//! This replaces the reunification's intermediate `CardTableWorld` view-emitter (a `contract::Game` that
//! rebuilt a `Board` from an abstract snapshot each frame) — the very inversion the re-aiming discarded
//! (plan §0.4): the cards are now the single source of truth and the game mutates them in place.
//!
//! [`BoardGame`]: cardtable_model::BoardGame

pub mod arena;
pub mod battle;
mod board_game;
pub mod combat;
pub mod fixtures;
pub mod solver;

pub use board_game::{CardTableGame, Intention};
pub use fixtures::sample_table;
