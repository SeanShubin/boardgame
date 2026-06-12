//! The launcher binary.
//!
//! It wires a chosen game into the generic [`tabletop`] renderer and runs it.
//! To play a different game, swap the game passed to
//! [`TabletopPlugin::new`] — any type implementing `engine::Game` works.

use bevy::prelude::*;
use tabletop::TabletopPlugin;
use treasure_dive::TreasureDive;

/// The seed for this match. A fixed seed makes a session reproducible; vary it
/// to deal a different deck.
const SEED: u64 = 1;

/// How many seats to deal in.
const PLAYERS: usize = 2;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Boardgame — Treasure Dive".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(TabletopPlugin::new(TreasureDive, SEED, PLAYERS))
        .run()
}
