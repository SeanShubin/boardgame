//! The launcher binary.
//!
//! It wires a chosen game into the generic [`tabletop`] renderer and runs it.
//! To play a different game, swap the game passed to
//! [`TabletopPlugin::new`] — any type implementing `engine::Game` works
//! (e.g. `treasure_dive::TreasureDive`).

use bevy::prelude::*;
use deckbound::Deckbound;
use tabletop::TabletopPlugin;

/// The seed for this match. A fixed seed makes a session reproducible; vary it
/// to change the warband's bluffs.
const SEED: u64 = 1;

/// Deckbound's sample combat seats one player who commands the whole party.
const PLAYERS: usize = 1;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Boardgame - Deckbound".into(),
                resolution: (1320u32, 860u32).into(),
                // On the web, track the browser viewport so resizing the window
                // reflows the table — the parity the desktop window already has.
                // Ignored natively, where `resolution` sets the initial size.
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(TabletopPlugin::new(Deckbound, SEED, PLAYERS))
        .run()
}
