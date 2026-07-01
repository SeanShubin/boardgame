//! The **card-table application** — the first-class entry point and the deployed product.
//!
//! It drives the game-agnostic card-table renderer ([`cardtable::CardTablePlugin`]) with a starting
//! [`Tableau`]. No game is wired in yet: this is the small seed the UI grows from, one feature at a
//! time. The full Deckbound combat game now lives as a reference scenario in the `deckbound-sample`
//! crate.
//!
//! Runs natively and on the web — Trunk builds this bin to WebAssembly (see `index.html` and
//! `.github/workflows/deploy.yml`).

use bevy::prelude::*;
use cardtable::{ActionRequests, CardTablePlugin, CardTableSet, StatusLine, Table};
use cardtable_model::sample_table;

fn main() -> AppExit {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Boardgame".into(),
            resolution: (1320u32, 860u32).into(),
            // On the web, track the browser viewport so resizing the window
            // reflows the table. Ignored natively, where `resolution` sets the
            // initial size.
            fit_canvas_to_parent: true,
            ..default()
        }),
        ..default()
    }));

    app.add_plugins(CardTablePlugin)
        .insert_resource(Table(sample_table()))
        .insert_resource(StatusLine(
            "Click a pile to enter it · click a card to grow it · drag to arrange".into(),
        ))
        // No game yet: drain the core's click outbox each frame so requests don't accumulate. A
        // future feature (or a game adapter) will consume these instead of discarding them.
        .add_systems(Update, drain_requests.in_set(CardTableSet::Apply));

    app.run()
}

/// Placeholder consumer of the core's action outbox until a real feature handles clicks.
fn drain_requests(mut requests: ResMut<ActionRequests>) {
    requests.0.clear();
}
