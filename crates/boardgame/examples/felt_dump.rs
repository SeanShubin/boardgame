//! **Headless felt dump** - drive the real card-table renderer with no window, no winit, no GPU, settle a
//! chosen table state, and print `screen.txt` (the current-state log `mirror_screen` writes).
//!
//! The card-table UI is almost entirely math: a card's on-screen rect, which cards are movable, which cards
//! overlap - all of it is computed and logged. This harness lets that math be read back without a human at
//! the window. It builds a **mustering solo** position through the public `BoardGame` seam (march two heroes
//! onto a lone-fight cell, press Fight to open the muster, tap one hero out so only the Raider is chosen),
//! drills into that cell, then runs the actual `BoardGamePlugin` + `LoggingPlugin` for a few frames so the
//! felt settles and the state log is written. It prints the muster state (which heroes are selected/ringed,
//! and the Confirm control + whether it is disabled) and then `screen.txt` - the cell laid out as a row
//! (encounter, rumors, the heroes), each card's rect, and the `overlaps` line the running app shows.
//!
//! Run: `cargo run -p boardgame --example felt_dump`

use bevy::prelude::*;
use cardtable::{BoardGamePlugin, LoggingPlugin, Table};
use cardtable_model::BoardGame;
use deckbound_board::{CardTableGame, Intention};

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

    let name_in = |board: &cardtable_model::Board, pile, name: &str| {
        board
            .pile(pile)
            .unwrap()
            .cards()
            .into_iter()
            .find(|&c| board.card(c).is_some_and(|k| k.front_title() == name))
    };

    // Both heroes march onto the solo cell - locations are uncapped.
    for hero in ["Marksman", "Raider"] {
        let h = name_in(&board, home, hero).expect("hero stationed at Ashfen");
        game.apply(
            &mut board,
            &[Intention::March {
                position: h,
                to: solo,
            }],
        );
    }

    // Press Fight to open the muster: nobody is chosen yet, every hero a candidate. Tap the Raider to pick
    // it (selected -> ringed); the Marksman stays a candidate.
    game.apply(&mut board, &[Intention::Fight { place: solo }]);
    let raider = name_in(&board, solo, "Raider").expect("the Raider stands on the cell");
    if let Some(toggle) = game.tap_intention(&board, raider) {
        game.apply(&mut board, &[toggle]);
    }

    // Report the muster state that the rings and the Confirm control are drawn from (rings are transient
    // overlay dots, not cards, so they are not in screen.txt - but the selection that drives them is here).
    let names = |ids: &[cardtable_model::CardId]| -> Vec<String> {
        ids.iter()
            .filter_map(|&c| board.card(c).map(|k| k.front_title().to_string()))
            .collect()
    };
    let ringed = board.selection().to_vec();
    let affordances: Vec<String> = game
        .affordances(&board, solo)
        .into_iter()
        .map(|(label, _)| label)
        .collect();
    let disabled = game.disabled_affordances(&board, solo);
    println!("MUSTER: ringed (selected) heroes = {:?}", names(&ringed));
    println!("MUSTER: controls = {affordances:?}, disabled indices = {disabled:?}\n");

    // ---- 2. Drill into the solo cell so the felt shows that zone. ----
    board.focus(solo).expect("focus the solo cell");

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
        for _ in 0..80 {
            app.update();
        }
        std::fs::read_to_string("screen.txt")
            .unwrap_or_else(|e| format!("no screen.txt written ({e}) - the felt never settled"))
    };

    let _ = std::fs::remove_file("screen.txt");
    println!("========== SOLO CELL, mustering (Raider chosen) ==========");
    println!("{}", settle(&mut app));
}
