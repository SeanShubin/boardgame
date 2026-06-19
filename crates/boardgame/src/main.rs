//! The launcher binary.
//!
//! It wires a chosen game into the generic [`tabletop`] renderer and runs it.
//! To play a different game, swap the game passed to
//! [`TabletopPlugin::new`] — any type implementing `engine::Game` works
//! (e.g. `treasure_dive::TreasureDive`).

use bevy::prelude::*;
use deckbound::Campaign;
use tabletop::TabletopPlugin;

/// The seed for this match. A fixed seed makes a session reproducible; vary it
/// to change the warband's bluffs.
const SEED: u64 = 1;

/// The campaign seats one player who commands the whole party.
const PLAYERS: usize = 1;

fn main() -> AppExit {
    // Play the **reference-scenario Campaign**: a world map you move on, entering locations to fight
    // the §4 battle, with a guided path you can deviate from (and rewind: Z = undo a step, R = back
    // to before the last deviation). To play combat-only instead, swap `Campaign` → `Deckbound`.
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Boardgame - Deckbound Campaign".into(),
                resolution: (1320u32, 860u32).into(),
                // On the web, track the browser viewport so resizing the window
                // reflows the table — the parity the desktop window already has.
                // Ignored natively, where `resolution` sets the initial size.
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(TabletopPlugin::new(Campaign, SEED, PLAYERS))
        .run()
}
