//! The **Deckbound sample scenario** — a full game wired into a renderer, kept as a reference and for
//! compatibility. This is no longer the main thrust of development (that is the card-table product in
//! the `boardgame` crate); it demonstrates a complete `contract::Game` driving a renderer end to end.
//!
//! It wires [`Deckbound`] into a generic renderer and runs it. The game opens on its menu, where every
//! mode — Duels / Cooperation / God-tier / Versus, the world-map **Campaign**, and the rules
//! encyclopedia — is one card. Any type implementing `contract::Game` could be swapped in here.
//!
//! The renderer is `tabletop` by default; build with `--features cardtable` to use the card-table
//! renderer instead. Both consume the same `contract::TableView`.

use bevy::prelude::*;
use deckbound::Deckbound;

/// The seed for this match. A fixed seed makes a session reproducible; vary it
/// to change the warband's bluffs.
const SEED: u64 = 1;

/// Deckbound's combat menu seats one player who commands the whole party.
const PLAYERS: usize = 1;

fn main() -> AppExit {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Deckbound (sample)".into(),
            resolution: (1320u32, 860u32).into(),
            // On the web, track the browser viewport so resizing the window
            // reflows the table — the parity the desktop window already has.
            // Ignored natively, where `resolution` sets the initial size.
            fit_canvas_to_parent: true,
            ..default()
        }),
        ..default()
    }));

    #[cfg(feature = "cardtable")]
    app.add_plugins(cardtable::GamePlugin::new(Deckbound, SEED, PLAYERS));
    #[cfg(not(feature = "cardtable"))]
    app.add_plugins(tabletop::TabletopPlugin::new(Deckbound, SEED, PLAYERS));

    app.run()
}
