//! The card-table interaction model, split into three sibling submodules:
//!
//! - [`physical`] — the **conserved pile/card types** (`Card` / `Pile` / `Board`, ids, `Layout`,
//!   `Recipe`): the source of truth. Everything the game and save-file are made of.
//! - `ui` — the per-observer **attention** state (focus / selection / arrangement), held inside a `Board`
//!   but knowing nothing the physical layer relies on.
//! - `geometry` — pure box-packing helpers (clamp / separate) shared by the above.
//! - `reconcile` — the load-time seam: rebuild a saved board's *arrangement* on the pristine board the
//!   current code builds, so persisted content can never go stale across a software update.
//!
//! `physical` is re-exported here so the crate's public surface (`model::Card`, `model::Board`, ...) is
//! unchanged; `ui` and `geometry` stay private to the model.

mod geometry;
pub mod layout;
mod physical;
mod reconcile;
mod ui;

pub use physical::*;
pub use reconcile::reconcile;
