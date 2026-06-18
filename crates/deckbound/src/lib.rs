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
pub mod currency;
pub mod duel;
pub mod encounter;
pub mod form;
pub mod game;
pub mod reference;
pub mod scenarios;
pub mod solver;
pub mod state;
pub mod stats;
pub mod world;
pub mod zones;

pub use actor::{Actor, Attack, Behavior, Driver, Instinct, Range, Script, TargetRule};
pub use cards::{Card, Effect};
pub use currency::{Coins, Currency, balance};
pub use duel::{Move, Side, resolve};
pub use encounter::{EncounterCard, RosterEntry};
pub use form::{Form, StatCard};
pub use reference::{ReferenceScenario, check_invariants, reference_scenario};
// `zones::Move` (Recover/Disrupt) stays module-qualified to avoid colliding with `duel::Move`.
pub use game::{Action, Deckbound};
pub use scenarios::{Scenario, build_encounter_foes, campaign, god, tutorials, versus};
pub use solver::auto_resolve;
pub use state::{Clash, Lane, Menu, Phase, Round, State};
pub use stats::{Aspect, DamageType, Defense, Health, Offense};
pub use world::{Coord, Layout, Location, Run};
pub use zones::{Zone, ZoneBehavior};
