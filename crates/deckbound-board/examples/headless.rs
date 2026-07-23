//! **Headless driver** - run the card-table game with NO window, jumping straight to the intention handlers
//! (the same `CardTableGame::apply` a click would produce) through the PUBLIC seam (the `BoardGame` trait +
//! the `Scene`), and dump the resulting SCENE as text after each step. This is the "what does the screen
//! show" loop without a GUI: it opens a fight, selects heroes, aims, and commits, printing `Scene::describe`
//! - the same serializer the running app writes to `ui-scene.txt`.
//!
//! It does NOT compute pixel geometry (that needs the Bevy layout pass and a display); it shows the scene's
//! CONTENT - tiles and their attention states, choices, effects, prompt, log - which is what the handlers
//! produce. For geometry/overlap use the running app's `screen.txt`.
//!
//! Run: `cargo run -p deckbound-board --example headless [encounter-label]`
//! (default: "Ashfen Crossing", the capstone). It self-plays the fight, printing the scene at each decision.

use cardtable_model::{Board, BoardGame, CardId, Highlight, PileId, Scene, SceneBody, Team, Tile};
use deckbound_board::{CardTableGame, Intention, sample_table};

fn main() {
    let place_label = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Ashfen Crossing".to_string());

    let game = CardTableGame::default();
    let mut board = sample_table();

    let place = march_party_to(&mut board, &place_label);
    game.apply(&mut board, &[Intention::Fight { place }]);
    println!("=== opened the fight at {place_label} ===\n");
    dump(&game, &board);

    // Self-play through the PUBLIC seam: select a ringed hero, take its first choice, or commit when no order
    // is owed - reading everything off the Scene, exactly what a player sees.
    let mut guard = 0;
    loop {
        guard += 1;
        if guard > 400 {
            println!("(guard hit - stopping)");
            break;
        }
        let Some(scene) = game.scene(&board, board.focus_id()) else {
            println!("=== fight over ===");
            break;
        };
        let tiles = all_tiles(&scene);

        // A hero being commanded (Active on our side)? Take its first live choice (verb -> target, or hold).
        let has_active = tiles
            .iter()
            .any(|t| t.team == Team::Left && t.highlight == Highlight::Active);
        if has_active {
            if let Some(intent) = game.choice_intention(&board, first_enabled_choice(&scene)) {
                game.apply(&mut board, &[intent]);
            }
            dump(&game, &board);
            continue;
        }

        // A hero owing an order (ringed / available on our side)? Select it (a tap).
        let owed = tiles.iter().find(|t| {
            t.team == Team::Left
                && matches!(t.highlight, Highlight::Targeted | Highlight::Available)
        });
        if let Some(t) = owed {
            if let Some(intent) = game.tap_intention(&board, t.card) {
                game.apply(&mut board, &[intent]);
            }
            dump(&game, &board);
            continue;
        }

        // Nobody owes an order - commit the wave.
        game.apply(&mut board, &[Intention::Commit]);
        println!("=== committed ===\n");
        dump(&game, &board);
    }
}

fn dump(game: &CardTableGame, board: &Board) {
    match game.scene(board, board.focus_id()) {
        Some(s) => println!("{}\n", s.describe()),
        None => println!("(felt - no fight up)\n"),
    }
}

/// Every tile in the scene body, left and right, in one flat list.
fn all_tiles(scene: &Scene) -> Vec<&Tile> {
    let mut out = Vec::new();
    match &scene.body {
        SceneBody::Lanes(lanes) => {
            for lane in lanes {
                out.extend(lane.left.iter());
                out.extend(lane.right.iter());
            }
        }
        SceneBody::Rows(rows) => {
            for row in rows {
                out.extend(row.tiles.iter());
            }
        }
    }
    out
}

/// The index of the first choice that can be taken (enabled), or 0 as a fallback.
fn first_enabled_choice(scene: &Scene) -> usize {
    scene
        .choices
        .iter()
        .position(|c| c.why_not.is_empty())
        .unwrap_or(0)
}

/// Move each kit's map position from the home cell to `place_label`, returning the place pile.
fn march_party_to(board: &mut Board, place_label: &str) -> PileId {
    let locations = top_deck(board, "Locations").expect("Locations deck");
    let home = board.pile(locations).unwrap().subpiles()[4]; // Ashfen, the home cell
    let place = board
        .pile(locations)
        .unwrap()
        .subpiles()
        .into_iter()
        .find(|&p| board.pile(p).map(|q| q.label.as_str()) == Some(place_label))
        .unwrap_or_else(|| panic!("no location {place_label}"));
    let progress = top_deck(board, "Progress").expect("Progress deck");
    let heroes: Vec<CardId> = board
        .content_cards(home)
        .into_iter()
        .filter(|&c| board.card(c).map(|k| k.card_type()) == Some("hero"))
        .collect();
    for h in heroes {
        let _ = board.move_character(h, place, progress);
    }
    place
}

fn top_deck(board: &Board, label: &str) -> Option<PileId> {
    board
        .pile(board.root_id())?
        .subpiles()
        .into_iter()
        .find(|&p| board.pile(p).map(|q| q.label.as_str()) == Some(label))
}
