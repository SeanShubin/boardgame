//! Deckbound — the cooperative card-combat game, as an [`engine::Game`].
//!
//! Combat is a sequence of **rounds** built on the documented model: two defense channels —
//! outer **Body** and inner **Fear/Spirit** — each resolved cut→bar→pool, only Body with a pool
//! (`stats`); data-driven action/effect cards (`cards`); the **charge-and-gauntlet** battle on a
//! single **Tempo** budget, with overflow free-hits, the gauntlet, and AoE (`combat`); and an
//! optional four-card **Clash** mix-up (Strike/Anticipate/Gather/Evade + Force) that replaces a
//! same-range trade (`duel`). Actors are Characters (human) or Creatures (scripted). No Bevy
//! dependency, so it's unit-testable and seed-reproducible; the `tabletop` plugin renders it. All
//! numbers live in `data/booklet.ron`.

pub mod actor;
pub mod balance;
pub mod campaign;
pub mod cards;
pub mod combat;
pub mod currency;
pub mod duel;
pub mod encounter;
pub mod form;
pub mod game;
pub mod reference;
pub mod ruleset;
pub mod scenarios;
pub mod solver;
pub mod state;
pub mod stats;
pub mod transcript;
pub mod world;
pub mod zones;

pub use actor::{Actor, Attack, Behavior, Driver, Instinct, Range, Script, TargetRule};
pub use campaign::{CampAction, Campaign, CampaignState, reference_campaign};
pub use cards::{Card, Effect, RoleKind};
pub use currency::{Currency, Track};
pub use duel::{Move, Side, resolve};
pub use encounter::{EncounterCard, RosterEntry};
pub use form::{Form, StatCard};
pub use reference::{ReferenceScenario, check_combat_bands, check_invariants, reference_scenario};
// `zones::Move` (Recover/Disrupt) stays module-qualified to avoid colliding with `duel::Move`.
pub use game::{Action, Deckbound};
pub use scenarios::{
    CatalogEntry, RewardId, Scenario, build_character, build_encounter_foes, campaign,
    card_catalog, god, rewards_for, tutorials, versus,
};
pub use solver::auto_resolve;
pub use state::{Clash, Menu, Phase, Round, State};
pub use stats::{DamageType, Defense, Health, Offense};
pub use transcript::{TranscriptScenario, transcribe, transcript_scenarios};
pub use world::{Coord, Layout, Location, Run};
pub use zones::{Zone, ZoneBehavior};
