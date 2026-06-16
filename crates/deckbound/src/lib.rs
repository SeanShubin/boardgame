//! Deckbound — the cooperative card-combat game, as an [`engine::Game`].
//!
//! Combat is a sequence of **rounds** built on the documented model: three aspects
//! (Body/Mind/Spirit), the cut→bar→pool defense (`stats`), data-driven action/effect
//! cards (`cards`), the card-based **Clash** (Strike/Throw/Parry/Evade + Charge/Recover)
//! as the engagement atom (`duel`), and a round loop with Tempo/Focus budgets, overflow
//! free-hits, the gauntlet, and AoE (`combat`). Actors are Characters (human) or Creatures
//! (scripted). No Bevy dependency, so it's unit-testable and seed-reproducible; the
//! `tabletop` plugin renders it. All numbers live in `data/booklet.ron`.

pub mod actor;
pub mod cards;
pub mod combat;
pub mod duel;
pub mod game;
pub mod scenarios;
pub mod state;
pub mod stats;

pub use actor::{Actor, Behavior, Driver, Instinct, Line, Script, TargetRule};
pub use cards::{Card, Effect, Lifecycle};
pub use duel::{Move, Side, resolve};
pub use game::{Action, Deckbound};
pub use scenarios::{Scenario, campaign, god, tutorials};
pub use state::{Duel, Menu, Phase, State};
pub use stats::{Aspect, DamageType, Defense, Health, Offense};
