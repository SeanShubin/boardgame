//! The **card-table application** — the first-class entry point and the deployed product.
//!
//! It drives the game-agnostic card-table renderer ([`cardtable::CardTablePlugin`]) with a starting
//! [`Tableau`]. No game is wired in yet: this is the small seed the UI grows from, one feature at a
//! time. The full Deckbound combat game now lives as a reference scenario in the `deckbound-sample`
//! crate.
//!
//! Runs natively and on the web — Trunk builds this bin to WebAssembly (see `index.html` and
//! `.github/workflows/deploy.yml`).

mod persistence;

use bevy::prelude::*;
use cardtable::{
    ActionRequests, BuildInfo, CardTablePlugin, CardTableSet, FactoryBase, StatusLine, Table,
};
use cardtable_model::sample_table;

/// Seconds between autosave checks; a save only writes when the RON actually changed.
const AUTOSAVE_SECS: f32 = 2.0;

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

    // Resume the last session if there's a save (web: localStorage, native: OS data dir); else a fresh
    // sample table. The System deck is re-injected idempotently, so a resumed table isn't doubled up.
    app.add_plugins(CardTablePlugin)
        .insert_resource(Table(persistence::load().unwrap_or_else(sample_table)))
        // The pristine table "Start Over" resets to (a fresh sample, discarding save + session).
        .insert_resource(FactoryBase(sample_table()))
        // The git commit this binary was built from (see build.rs) — shown in the System deck.
        .insert_resource(BuildInfo(
            option_env!("BUILD_GIT_HASH").unwrap_or("unknown").into(),
        ))
        .insert_resource(StatusLine(
            "Click a pile to enter it · click a card to grow it · drag to arrange".into(),
        ))
        // No game yet: drain the core's click outbox each frame so requests don't accumulate. A
        // future feature (or a game adapter) will consume these instead of discarding them.
        .add_systems(Update, drain_requests.in_set(CardTableSet::Apply))
        .add_systems(Update, autosave);

    app.run()
}

/// Placeholder consumer of the core's action outbox until a real feature handles clicks.
fn drain_requests(mut requests: ResMut<ActionRequests>) {
    requests.0.clear();
}

/// Periodically persist the table — at most every [`AUTOSAVE_SECS`], and only when the serialized RON
/// differs from what was last written. Dedup matters because the renderer touches `Table` every frame
/// (sizes, obstacles), so change-detection alone would rewrite constantly. Cheap: the table is small.
fn autosave(
    table: Res<Table>,
    time: Res<Time>,
    mut cooldown: Local<f32>,
    mut last: Local<Option<String>>,
) {
    *cooldown += time.delta_secs();
    if *cooldown < AUTOSAVE_SECS {
        return;
    }
    *cooldown = 0.0;
    let Some(text) = persistence::encode(&table.0) else {
        return;
    };
    if last.as_deref() != Some(text.as_str()) {
        persistence::write(&text);
        *last = Some(text);
    }
}
