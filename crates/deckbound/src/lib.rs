//! Deckbound — the cooperative card-combat game, as an [`engine::Game`].
//!
//! This crate is the pure ruleset for the **sample combat**
//! (`docs/games/deckbound/design/sample-round.md`): four heroes — Aldric, Vera,
//! Sefa, Bram — against a warband of an armored bluffer, a Runner, a
//! fear-caster, and a husk swarm. It has no Bevy dependency, so the whole fight
//! is unit-testable and reproducible from a seed; a presentation layer (the
//! `tabletop` Bevy plugin) renders it without knowing any of the rules.
//!
//! # The round
//!
//! A combat is a sequence of rounds, each with the same beats:
//!
//! 1. **Declare** — each living hero secretly commits a [`Play`] (a read, or a
//!    signature card) and a target. The fight is simultaneous, but the
//!    turn-based [`Game`] contract collects these one hero at a time.
//! 2. **Resolve** — once the last hero declares, the round settles at once:
//!    support (Rally) → morale (Dread, the Howl) → the gauntlet (drag stops the
//!    Runner) → the exchange (reads, Strikes, Firestorm) → recover. See
//!    [`resolve`].
//!
//! Win by downing every foe with a hero still standing; lose if the last hero
//! falls. The whole thing is data-light and deterministic — exactly the seam
//! the engine is built around.
//!
//! See the crate's `game` tests for the documented winning line and the
//! coordination failures it guards against.

pub mod actors;
pub mod game;
pub mod read;
pub mod resolve;
pub mod state;
pub mod stats;

pub use actors::{Behavior, Creature, Hero, Line, Play};
pub use game::{Action, Deckbound};
pub use read::{Clash, Read, clash};
pub use state::{Phase, State};
pub use stats::{Armor, Body, DamageType};
