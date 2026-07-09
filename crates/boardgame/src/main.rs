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
    ActionRequests, ArenaCombat, ArenaState, BoardGamePlugin, BuildInfo, CardTableSet,
    CombatRequest, FactoryBase, ManualCombatRequest, NeedsRebuild, StatusLine, Table,
};
use cardtable_combat::{begin_manual_combat, resolve_encounter};
use cardtable_model::{Tableau, sample_table};
use deckbound_cardtable::CardTableGame;

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
    app.add_plugins(BoardGamePlugin(CardTableGame))
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
            "Click a pile to enter it · click a card to grow it · drag to arrange".into(),
        ))
        // No game yet: drain the core's click outbox each frame so requests don't accumulate. A
        // future feature (or a game adapter) will consume these instead of discarding them.
        .add_systems(Update, drain_requests.in_set(CardTableSet::Apply))
        // The first bit of game wired into the product: resolve a requested fight (auto or manual).
        .add_systems(Update, resolve_combat.in_set(CardTableSet::Apply))
        .add_systems(Update, resolve_manual_combat.in_set(CardTableSet::Apply))
        .add_systems(Update, autosave);

    app.run()
}

/// Placeholder consumer of the core's action outbox until a real feature handles clicks.
fn drain_requests(mut requests: ResMut<ActionRequests>) {
    requests.0.clear();
}

/// Resolve a fight the player asked for: the [`CombatCard`](cardtable) click records the place in
/// [`CombatRequest`]; here we resolve it against the game rules and fold the result onto the table, then
/// request a redraw. Deterministic — the seed is the current day, so a fight varies day to day but
/// replays identically. This is where the product reaches the combat rules.
fn resolve_combat(
    mut table: ResMut<Table>,
    mut request: ResMut<CombatRequest>,
    mut rebuild: ResMut<NeedsRebuild>,
) {
    let Some(place) = request.0.take() else {
        return;
    };
    let seed = day_seed(&table.0);
    let outcome = resolve_encounter(&mut table.0, place, seed);
    info!("combat resolved: {outcome:?}");
    rebuild.0 = true;
}

/// Open the interactive **arena** for a manual fight the player asked for: instantiate the encounter's foes
/// as real cards (into a scratch pile) and hand the [`cardtable_combat::ManualCombat`] to the renderer via
/// [`ArenaCombat`]. From there the renderer's `drive_arena` steps the fight (the player decides, the AI
/// answers the foes) and folds it back on the end. Deterministic — seeded by the current day.
fn resolve_manual_combat(
    mut table: ResMut<Table>,
    mut request: ResMut<ManualCombatRequest>,
    mut arena: ResMut<ArenaCombat>,
    mut rebuild: ResMut<NeedsRebuild>,
) {
    let Some(place) = request.0.take() else {
        return;
    };
    if arena.0.is_some() {
        return; // a fight is already up — don't stack another
    }
    let seed = day_seed(&table.0);
    let root = table.0.root_id();
    let Some(bestiary) = find_top(&table.0, "Bestiary") else {
        return;
    };
    let Ok(scratch) = table.0.add_pile(root, "Arena") else {
        return;
    };
    match begin_manual_combat(&mut table.0, place, scratch, bestiary, seed) {
        Some(combat) => {
            arena.0 = Some(ArenaState {
                combat,
                place,
                bestiary,
                scratch,
            });
            rebuild.0 = true; // switch the felt to the arena view
        }
        None => {
            let _ = table.0.remove_pile(scratch);
        }
    }
}

/// A top-level deck by label (the game-state helpers reach a few fixed zones by name).
fn find_top(table: &Tableau, label: &str) -> Option<cardtable_model::PileId> {
    let root = table.root_id();
    table
        .pile(root)
        .map(|r| r.subpiles())
        .unwrap_or_default()
        .into_iter()
        .find(|&p| table.pile(p).is_some_and(|pile| pile.label == label))
}

/// A deterministic combat seed derived from game state: the current day count (so a fight is reproducible
/// yet differs day to day). Falls back to `1` before the day clock exists.
fn day_seed(table: &Tableau) -> u64 {
    let root = table.root_id();
    let progress = table
        .pile(root)
        .map(|r| r.subpiles())
        .unwrap_or_default()
        .into_iter()
        .find(|&p| table.pile(p).is_some_and(|pile| pile.label == "Progress"));
    progress.map(|p| table.current_day(p) as u64).unwrap_or(1)
}

/// Periodically persist the table — at most every [`AUTOSAVE_SECS`], and only when the serialized RON
/// differs from what was last written. Dedup matters because the renderer touches `Table` every frame
/// (sizes, obstacles), so change-detection alone would rewrite constantly. Cheap: the table is small.
fn autosave(
    table: Res<Table>,
    arena: Res<ArenaCombat>,
    time: Res<Time>,
    mut cooldown: Local<f32>,
    mut last: Local<Option<String>>,
) {
    // Don't persist mid-fight: the table then holds the transient arena scratch pile + instantiated foes,
    // but the fight itself (the `ArenaCombat` resource) isn't saved — so a reload would strand an orphan
    // pile. The fight folds back cleanly on its end, and the next tick saves that.
    if arena.0.is_some() {
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
