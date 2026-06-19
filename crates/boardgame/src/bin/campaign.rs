//! The reference-scenario **Campaign** launcher.
//!
//! Run with `cargo run -p boardgame --bin campaign`. This plays the world-map reference run: move
//! the party token across locations, enter one to fight the §4 battle, buy Upgrades, and clear the
//! objective. The guide's suggested move is highlighted (teal); deviate freely, then rewind —
//! `Z` undoes one step, `R` rewinds to just before the last deviation from the guide.
//!
//! The default `boardgame` binary launches the combat menu (Cooperation / God / Tutorials /
//! Versus + encyclopedia) instead; both render the same [`engine::Game`] through [`tabletop`].

use bevy::prelude::*;
use deckbound::Campaign;
use tabletop::TabletopPlugin;

/// A fixed seed makes the run reproducible.
const SEED: u64 = 1;

/// The campaign seats one player who commands the whole party.
const PLAYERS: usize = 1;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Boardgame - Deckbound Campaign".into(),
                resolution: (1320u32, 860u32).into(),
                // On the web, track the browser viewport so resizing reflows the table.
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(TabletopPlugin::new(Campaign, SEED, PLAYERS))
        .run()
}
