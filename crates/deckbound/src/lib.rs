//! Deckbound — the cooperative card-combat game, as an [`engine::Game`].
//!
//! Combat is a sequence of **rounds** on the §4.6 **six-phase** model: one damage channel —
//! untyped **Might** into the **health** pool, resolved pile→bar→pool (`stats`, §2.2); stats read off
//! the **Form** deck (`form`, stats-as-deck §2.3); data-driven action/effect cards (`cards`); the
//! six-phase battle (Standoff → Fray → Volley → Breach → Reckoning → Lull) on a single per-round
//! **Tempo** budget, with the one Tempo contest, charges/flanks, the breach pre-empt, and AoE
//! (`combat`); and an optional four-card **Clash** mix-up (Strike/Anticipate/Gather/Evade + Force)
//! that replaces a same-range trade (`duel`). Actors are Characters (human) or Creatures (scripted).
//! No Bevy dependency, so it's unit-testable and seed-reproducible; the `tabletop` plugin renders it.
//! All numbers live in `data/booklet.ron`.

pub mod actor;
pub mod balance;
pub mod campaign;
pub mod cards;
pub mod combat;
pub mod currency;
pub mod decktree;
pub mod duel;
pub mod encounter;
pub mod engagement;
pub mod form;
pub mod game;
pub mod groups;
pub mod handbook;
pub mod layout;
pub mod policy;
pub mod reference;
pub mod rules;
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
pub use decktree::{Deck, FormTree, Stat, Suit, SuitDeck};
pub use duel::{Move, Side, resolve};
pub use encounter::{EncounterCard, RosterEntry};
pub use form::{Form, StatCard};
pub use reference::{ReferenceScenario, check_combat_bands, check_invariants, reference_scenario};
// `zones::Move` (Recover/Disrupt) stays module-qualified to avoid colliding with `duel::Move`.
pub use game::{Action, Deckbound};
// `CombatLayout` (the derived 2D combat board) is named to avoid colliding with `world::Layout`
// (the world-map grid/hex layout), which is already re-exported as `Layout` below.
pub use layout::{CombatLayout, Rank, SideLayout, Slot};
pub use scenarios::{
    CatalogEntry, RewardId, Scenario, build_character, build_encounter_foes, campaign,
    card_catalog, god, rewards_for, tutorials, versus,
};
pub use solver::{Solution, auto_resolve, solve, winnable};
pub use state::{Clash, Menu, Phase, Round, State};
pub use stats::{Defense, Health, Offense, PendingDamage};
pub use transcript::{TranscriptScenario, transcribe, transcript_scenarios};
pub use world::{Coord, Layout, Location, Run};
pub use zones::{Zone, ZoneBehavior};
