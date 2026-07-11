//! **Deckbound shared content** — the stable, dependency-light layer both the card-table product
//! (`deckbound-board`) and the reference sample (`deckbound`) build on, so the product no longer depends on
//! the sample crate. Three pieces:
//!
//! - [`catalog`] — the card content (kits / creatures / encounters / rumors, stat names, ability shapes).
//! - [`rank`] — the Vanguard / Outrider / Rearguard [`Intention`](rank::Intention) (the "rank" the product
//!   sees, the "declared intention" the sample sees).
//! - [`schedule`] — the §4.6 sub-phase [`SCHEDULE`](schedule::SCHEDULE) both combat models walk.
//!
//! No Bevy; the card content and rank derive `serde` so saves round-trip. Nothing here depends on either
//! consumer, so it sits below both in the dependency graph.

pub mod catalog;
pub mod rank;
pub mod schedule;
