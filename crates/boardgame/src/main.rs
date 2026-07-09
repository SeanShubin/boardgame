//! The **card-table application** — the deployed product. It drives the game-agnostic card-table
//! renderer ([`cardtable`]) from the **deckbound card-table `Game`** ([`deckbound_cardtable::CardTableWorld`])
//! via the renderer's game adapter ([`GamePlugin`]): the table is built from the game's view, and clicks
//! plus pairing drops (drag a hero onto a kit to equip, a character onto a location to march, onto an
//! encounter to fight) flow into `Game::apply`. Deckbound's whole card-table world — the banks, the map,
//! the interactive per-blow arena — now runs through the `contract::Game` seam; there is no hand-wired
//! bypass.
//!
//! Runs natively and on the web — Trunk builds this bin to WebAssembly (see `index.html` and
//! `.github/workflows/deploy.yml`).

use bevy::prelude::*;
use cardtable::{BuildInfo, GamePlugin};
use deckbound_cardtable::CardTableWorld;

/// A fixed fight seed for now — deterministic per launch. (A varying seed returns with the action-stream
/// save; see the reunification plan P2.4.)
const SEED: u64 = 1;

fn main() -> AppExit {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Boardgame".into(),
            resolution: (1320u32, 860u32).into(),
            // On the web, track the browser viewport so resizing the window reflows the table. Ignored
            // natively, where `resolution` sets the initial size.
            fit_canvas_to_parent: true,
            ..default()
        }),
        ..default()
    }));

    // Drive the renderer from the deckbound card-table `Game`. `GamePlugin` adds the renderer core itself,
    // builds the `Table` from the game's `view`, and turns the core's action outbox (clicks + pairing
    // drops) into `Game::apply`.
    app.add_plugins(GamePlugin::new(CardTableWorld, SEED, 1))
        // The git commit this binary was built from (see build.rs) — shown as the Version card in the
        // System deck: the hash, its date, and how long ago it was built.
        .insert_resource(BuildInfo {
            hash: option_env!("BUILD_GIT_HASH").unwrap_or("unknown").into(),
            date: option_env!("BUILD_GIT_DATE").unwrap_or("").into(),
            timestamp: option_env!("BUILD_GIT_TIMESTAMP").and_then(|s| s.parse::<i64>().ok()),
        });

    app.run()
}
