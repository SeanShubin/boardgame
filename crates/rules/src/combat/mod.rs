//! **The combat rules** - the regions model, self-contained and knowing nothing of any other system (no
//! overworld, no physical cards, no rendering).
//!
//! - [`resolve`] - the primitives: the [`Combatant`](resolve::Combatant), and the order-free bid/slip/strike
//!   machine a single exchange resolves through.
//! - [`regions`] - the rules of *formation*: regions, front/back posts, the slip, the schedule, and how a
//!   whole round plays out. This is the file to read to know how combat works.

pub mod game;
pub mod regions;
pub mod resolve;
