//! A feature-prototyping sandbox: the card-table renderer core driven by a hand-built table, with
//! **no game wired in**. This is the pattern for prototyping an individual UI feature in isolation —
//! drive the shared [`CardTablePlugin`] with a fixture [`Table`] and observe the core's outputs.
//!
//! Run with: `cargo run -p cardtable --example sandbox`
//!
//! Click a collapsed deck to focus (fan) it; use "Zoom out" to step back. Actionable cards (Knight,
//! Mage) report their index — here we just log it, where a game would apply it.

use bevy::prelude::*;
use cardtable::{ActionRequests, CardTablePlugin, CardTableSet, StatusLine, Table};
use cardtable_model::sample_table;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "cardtable - sandbox".into(),
                resolution: (1100u32, 760u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(CardTablePlugin)
        .insert_resource(Table(sample_table()))
        .insert_resource(StatusLine(
            "Click a pile to enter it | click a card to grow it | Back / Exit to navigate".into(),
        ))
        // Stand in for a game: drain the core's click outbox. Runs in `Apply`, after the core's input.
        .add_systems(Update, log_actions.in_set(CardTableSet::Apply))
        .run();
}

fn log_actions(mut requests: ResMut<ActionRequests>) {
    for index in requests.0.drain(..) {
        info!("actionable control clicked: index {index}");
    }
}
