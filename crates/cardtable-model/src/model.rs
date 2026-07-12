//! The card-table interaction model, split into three sibling submodules:
//!
//! - [`physical`] — the **conserved pile/card types** (`Card` / `Pile` / `Board`, ids, `Layout`,
//!   `Recipe`): the source of truth. Everything the game and save-file are made of.
//! - `ui` — the per-observer **attention** state (focus / selection / arrangement), held inside a `Board`
//!   but knowing nothing the physical layer relies on.
//! - `geometry` — pure box-packing helpers (clamp / separate) shared by the above.
//!
//! `physical` is re-exported here so the crate's public surface (`model::Card`, `model::Board`, ...) is
//! unchanged; `ui` and `geometry` stay private to the model.

mod geometry;
pub mod layout;
mod physical;
mod ui;

pub use physical::*;
