//! Deckbound — the cooperative card-combat game, as an [`engine::Game`].
//!
//! This is the **duel sandbox**: the playable realization of the Edge duel
//! (`docs/games/deckbound/design/the-duel.md`). Combat is a sequence of
//! one-on-one duels — Marshal / Unleash / Overwhelm / Parry, with a public,
//! per-duel **Edge** bank that builds and is stolen. Creatures duel through
//! stance-policies. No Bevy dependency, so the whole thing is unit-testable and
//! reproducible from a seed; the `tabletop` plugin renders it.
//!
//! The earlier Strike/Block/Evade/Scheme combat (formation, gauntlet, fear,
//! multi-target) is parked while we tune the duel.

pub mod actors;
pub mod duel;
pub mod game;
pub mod scenarios;
pub mod state;
pub mod stats;

pub use actors::{Creature, Hero, StancePolicy};
pub use duel::{Side, Stance, resolve};
pub use game::{Action, Deckbound};
pub use scenarios::{Scenario, campaign, tutorials};
pub use state::{Duel, Menu, Phase, State};
pub use stats::Body;
