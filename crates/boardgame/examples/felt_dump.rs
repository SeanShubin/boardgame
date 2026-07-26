//! **Headless felt dump** - drive the real card-table renderer with no window, no winit, no GPU, settle a
//! chosen table state, and print `screen.txt` (the current-state log `mirror_screen` writes).
//!
//! The card-table UI is almost entirely math: a card's on-screen rect, which cards are movable, which cards
//! overlap - all of it is computed and logged. This harness lets that math be read back without a human at
//! the window. It builds a **solo cell with heroes present** through the public `BoardGame` seam (march two
//! heroes onto a lone-fight cell with room for one - the first auto-fills the encounter, the second benches),
//! drills into that cell, then runs the actual `BoardGamePlugin` + `LoggingPlugin` for a few frames so the
//! felt settles and the state log is written. It prints the assignment (who is in the encounter area vs on
//! the bench, and the Fight control) and then `screen.txt` - each card's rect and the `overlaps` line.
//!
//! Run: `cargo run -p boardgame --example felt_dump`

use bevy::prelude::*;
use cardtable::{BoardGamePlugin, LoggingPlugin, Table};
use cardtable_model::BoardGame;
use deckbound_board::CardTableGame;

fn main() {
    // ---- 1. Build a seated-solo position through the public seam (mirrors the board_game unit test). ----
    let game = CardTableGame::default();
    let mut board = game.opening();
    let root = board.root_id();
    let locations = board
        .pile(root)
        .unwrap()
        .subpiles()
        .into_iter()
        .find(|&p| board.pile(p).is_some_and(|z| z.label == "Locations"))
        .expect("the map has a Locations deck");
    let cells = board.pile(locations).unwrap().subpiles();
    let home = cells[4]; // Ashfen (centre) - where the party starts stationed
    let solo = cells[1]; // Cinderwatch Keep - an orthogonal lone-fight cell

    // Reproduce the crowded case: the whole party (four heroes) stands at Ashfen (the capstone cell), so the
    // location screen must lay out an encounter + four bench heroes + the rumor without flinging cards off.
    let _ = solo;

    // Report the assignment the encounter area and Fight control are drawn from: the assigned heroes live in
    // the cell's encounter (its sub-pile), the rest stand on the bench (the cell's own content).
    let names = |ids: &[cardtable_model::CardId]| -> Vec<String> {
        ids.iter()
            .filter_map(|&c| board.card(c).map(|k| k.front_title().to_string()))
            .collect()
    };
    let area = board
        .pile(home)
        .unwrap()
        .subpiles()
        .into_iter()
        .next()
        .expect("the cell has an encounter area (empty drop-zone marker)");
    let _ = area;
    // Assignment is card SELECTION: the heroes all stand at the cell; the selected ones are assigned.
    let heroes: Vec<cardtable_model::CardId> = board
        .pile(home)
        .unwrap()
        .cards()
        .into_iter()
        .filter(|&c| board.card(c).is_some_and(|k| k.card_type() == "hero"))
        .collect();
    let assigned: Vec<cardtable_model::CardId> = heroes
        .iter()
        .copied()
        .filter(|&c| board.is_selected(c))
        .collect();
    let bench: Vec<cardtable_model::CardId> = heroes
        .iter()
        .copied()
        .filter(|&c| !board.is_selected(c))
        .collect();
    let affordances: Vec<String> = game
        .affordances(&board, home)
        .into_iter()
        .map(|(label, _)| label)
        .collect();
    println!("ENCOUNTER: assigned (in the area) = {:?}", names(&assigned));
    println!("ENCOUNTER: bench (standing) = {:?}", names(&bench));
    println!("ENCOUNTER: controls = {affordances:?}\n");

    // ---- 2. Drill into the cell so the felt shows that zone. ----
    board.focus(home).expect("focus the cell");

    // ---- 3. Run the real renderer headlessly; let mirror_screen write screen.txt. ----
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: None, // no window: setup_camera takes its headless RenderTarget::None branch
                exit_condition: bevy::window::ExitCondition::DontExit,
                ..default()
            })
            .set(bevy::render::RenderPlugin {
                render_creation: bevy::render::settings::RenderCreation::Automatic(Box::new(
                    bevy::render::settings::WgpuSettings {
                        backends: None, // no GPU adapter; UI still lays out, which is all we measure
                        ..default()
                    },
                )),
                ..default()
            })
            .build()
            .disable::<bevy::winit::WinitPlugin>(), // we drive the schedule by hand
    );
    app.add_plugins((BoardGamePlugin(CardTableGame::default()), LoggingPlugin));
    // Override the plugin's opening table with our seated-solo, drilled-in board.
    app.insert_resource(Table(board));

    // Settle the felt (fonts load, UI builds, a settled frame is written), then read screen.txt back.
    let settle = |app: &mut App| {
        for _ in 0..150 {
            app.update();
        }
        std::fs::read_to_string("screen.txt")
            .unwrap_or_else(|e| format!("no screen.txt written ({e}) - the felt never settled"))
    };

    let _ = std::fs::remove_file("screen.txt");
    println!("========== ASHFEN: encounter area (party assigned) + bench ==========");
    println!("{}", settle(&mut app));
}
