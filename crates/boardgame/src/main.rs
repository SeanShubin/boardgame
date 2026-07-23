//! The **card-table application** — the first-class entry point and the deployed product.
//!
//! It drives the game-agnostic card-table renderer ([`cardtable::CardTablePlugin`]) with the deckbound
//! card-table game wired in behind the [`BoardGame`](cardtable_model::BoardGame) seam
//! ([`deckbound_board::CardTableGame`]): recruit / march / advance-day and the interactive combat arena
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
use deckbound_board::CardTableGame;
use deckbound_board::sample_table;

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
    app.add_plugins((BoardGamePlugin(CardTableGame::default()), LoggingPlugin))
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
        .add_systems(Update, autosave)
        // Debug: `BOARDGAME_AUTOFIGHT=1` jumps straight into the Ashfen fight on launch (no clicking), so the
        // combat screen can be captured (screen.txt) without a human driving the GUI.
        .add_systems(Update, autofight);

    app.run()
}

/// When `BOARDGAME_AUTOFIGHT` is set, open the Ashfen capstone fight once on startup, on a fresh table -
/// the same `open_fight` handler a click would reach. A headless-ish way to land on the combat screen for a
/// layout capture; a no-op otherwise.
fn autofight(
    mut table: ResMut<Table>,
    mut rebuild: ResMut<cardtable::NeedsRebuild>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    *done = true;
    if std::env::var("BOARDGAME_AUTOFIGHT").is_err() {
        return;
    }
    let mut board = sample_table();
    let Some(locations) = board.pile(board.root_id()).and_then(|r| {
        r.subpiles()
            .into_iter()
            .find(|&p| board.pile(p).map(|q| q.label.as_str()) == Some("Locations"))
    }) else {
        return;
    };
    let Some(place) = board.pile(locations).and_then(|p| {
        p.subpiles()
            .into_iter()
            .find(|&sp| board.pile(sp).map(|q| q.label.as_str()) == Some("Ashfen Crossing"))
    }) else {
        return;
    };
    deckbound_board::arena::open_fight(&mut board, place);
    // `BOARDGAME_AUTOFIGHT=play` also self-plays a few waves through the PUBLIC seam so the log grows long -
    // the state that used to clip the board. Reuses the same handlers a click reaches.
    if std::env::var("BOARDGAME_AUTOFIGHT").as_deref() == Ok("play") {
        self_play_a_while(&mut board);
    }
    table.0 = board;
    rebuild.0 = true;
}

/// Drive the open fight a bounded number of waves via the `BoardGame` seam (select a ringed hero, take its
/// first choice, else commit), stopping once a round's log is long - so the capture stresses the layout.
fn self_play_a_while(board: &mut cardtable_model::Board) {
    use cardtable_model::{BoardGame, Highlight, SceneBody, Team};
    let game = CardTableGame::default();
    for _ in 0..80 {
        let focus = board.focus_id();
        let Some(scene) = game.scene(board, focus) else {
            break;
        };
        // Stop once the log has a good number of lines (a full round's worth of narration is the stress).
        if scene.log.len() >= 12 {
            break;
        }
        let tiles: Vec<_> = match &scene.body {
            SceneBody::Lanes(lanes) => lanes
                .iter()
                .flat_map(|l| l.left.iter().chain(&l.right))
                .collect(),
            SceneBody::Rows(rows) => rows.iter().flat_map(|r| r.tiles.iter()).collect(),
        };
        let active = tiles
            .iter()
            .any(|t| t.team == Team::Left && t.highlight == Highlight::Active);
        if active {
            let idx = scene
                .choices
                .iter()
                .position(|c| c.why_not.is_empty())
                .unwrap_or(0);
            if let Some(i) = game.choice_intention(board, idx) {
                game.apply(board, &[i]);
            }
        } else if let Some(t) = tiles.iter().find(|t| {
            t.team == Team::Left
                && matches!(t.highlight, Highlight::Targeted | Highlight::Available)
        }) {
            if let Some(i) = game.tap_intention(board, t.card) {
                game.apply(board, &[i]);
            }
        } else {
            game.apply(board, &[deckbound_board::Intention::Commit]);
        }
    }
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
    if deckbound_board::arena::find_arena(&table.0).is_some() {
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
