//! The **card-table application** — the first-class entry point and the deployed product.
//!
//! It drives the game-agnostic card-table renderer ([`cardtable::CardTablePlugin`]) with the deckbound
//! card-table game wired in behind the [`BoardGame`](cardtable_model::BoardGame) seam
//! ([`deckbound_cardtable::CardTableGame`]): recruit / march / advance-day and the interactive combat arena
//! all run as intentions over the persistent board.
//!
//! Runs natively and on the web — Trunk builds this bin to WebAssembly (see `index.html` and
//! `.github/workflows/deploy.yml`).

mod persistence;

use bevy::prelude::*;
use cardtable::{
    ActionRequests, BoardGamePlugin, BuildInfo, CardTableSet, FactoryBase, LoggingPlugin,
    StatusLine, Table,
};
use deckbound_cardtable::CardTableGame;
use deckbound_cardtable::sample_table;

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

    // Drive the game-agnostic renderer from the deckbound `BoardGame` over the persistent board. The
    // plugin seeds `Table` from the game's opening position; we then override it with the saved session if
    // there is one (web: localStorage, native: OS data dir). The System deck is re-injected idempotently.
    app.add_plugins((BoardGamePlugin(CardTableGame), LoggingPlugin))
        .insert_resource(Table(persistence::load().unwrap_or_else(sample_table)))
        // The pristine table "Start Over" resets to (a fresh sample, discarding save + session).
        .insert_resource(FactoryBase(sample_table()))
        // The git commit this binary was built from (see build.rs) — shown as the Version card in the
        // System deck: the hash, its date, and how long ago it was built.
        .insert_resource(BuildInfo {
            hash: option_env!("BUILD_GIT_HASH").unwrap_or("unknown").into(),
            date: option_env!("BUILD_GIT_DATE").unwrap_or("").into(),
            timestamp: option_env!("BUILD_GIT_TIMESTAMP").and_then(|s| s.parse::<i64>().ok()),
        })
        .insert_resource(StatusLine(
            "Click a pile to enter it | click a card to grow it | drag to arrange".into(),
        ))
        // Loose rail-action clicks aren't consumed by the board game, so drain that outbox each frame.
        .add_systems(Update, drain_requests.in_set(CardTableSet::Apply))
        .add_systems(Update, autosave);

    app.run()
}

/// Drain the core's loose-action outbox (rail-item clicks the board game doesn't handle) each frame.
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
    // Don't persist mid-fight: the board then holds the transient `[Arena]` scratch pile + instantiated
    // foes, and the fight folds back cleanly on its end (the next tick saves that). A reload mid-fight
    // would strand an orphan pile / load stale per-combat detail.
    if deckbound_cardtable::arena::find_arena(&table.0).is_some() {
        return;
    }
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
