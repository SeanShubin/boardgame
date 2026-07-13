//! A Bevy renderer that draws the **card-table metaphor** — everything is a card; a pile is a stack of
//! cards in one footprint. You navigate with **single-click and drag only**: click a pile to drill into
//! its zone, click a card to grow it through its sizes, click the Back card to move up, and drag piles
//! to arrange them on the table. **System** is itself a pile on the felt — drag it like any other, or
//! click it to drill into its zone, where clicking the "Exit" card quits and "Start Over" resets. A
//! stray click never quits. The current zone's name sits centered at the top (default "Table").
//!
//! # Two layers
//!
//! - **The core (this module) is game-agnostic.** It draws whatever is in the [`Table`] resource (a
//!   [`Board`]) plus an [`ActionRail`] of loose actions and a [`StatusLine`], handles focus/zoom
//!   itself, and reports clicks on actionable controls by pushing their index into [`ActionRequests`].
//!   It never mentions `Game`. This is the shared code: `boardgame` and feature prototypes both drive
//!   it. Prototype a feature with [`CardTablePlugin`] + a hand-built `Table` (see
//!   [`cardtable_model::fixtures`]) and no game at all — `cargo run -p cardtable --example sandbox`.
//! - **The `game` feature adds the adapter** ([`GamePlugin`]): it binds a [`contract::Game`] to the
//!   core — building the `Table`/`ActionRail`/`StatusLine` from the game's view and draining
//!   `ActionRequests` into `Game::apply`. Only the launcher needs it.
//!
//! Rendering is `bevy_ui` (flexbox), matching `tabletop`; the pile model is renderer-agnostic, so a
//! future 3D table could be built against the same [`Table`] — see
//! `docs/games/deckbound/presentation/card-table-ui.md` §7.

use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::picking::events::{Click, Drag, DragDrop, DragEnd, DragStart, Pointer};
use bevy::prelude::*;
use bevy::ui::{BoxShadow, ComputedNode, Outline, ScrollPosition, UiGlobalTransform};

use std::collections::HashMap;

use cardtable_model::{
    Arrangement, Board, Card, CardId, CardKind, Choice, DropTarget, Face, Highlight, Lane, Layout,
    Node as TableNode, PileId, Pos, Row, Scene, SceneBody, Size, Team, Tile, Tone, Utility,
};

#[cfg(feature = "game")]
pub use game::GamePlugin;

mod board_driver;
pub mod palette;
use board_driver::{AffordanceClick, DropTrace, TapRequest};
pub use board_driver::{
    AffordanceControl, AffordanceLabels, BoardGamePlugin, ChoiceClick, ChoiceControl, DropRequest,
    SceneState, UndoClick, UndoControl,
};

mod logging;
pub use logging::LoggingPlugin;

mod demo;
mod gallery;
pub use demo::demo_table;
pub use gallery::{TextOverflow, audit_card_text, run_card_gallery};

// ---- public presentation state (the shared inputs) ----------------------

/// The board: the pile tree the core draws. Mutated in place for focus/zoom; replaced wholesale when
/// the source (a game, or a prototype) rebuilds it.
#[derive(Resource, Default)]
pub struct Table(pub Board);

/// Loose actions shown as an always-visible rail (choices not represented by a card on the table).
/// Each carries an opaque `index` the core echoes back in [`ActionRequests`] when clicked.
#[derive(Resource, Default)]
pub struct ActionRail(pub Vec<RailAction>);

/// One rail entry: a `label` to show and an opaque `index` to report on click.
#[derive(Clone, Debug)]
pub struct RailAction {
    pub index: usize,
    pub label: String,
}

/// A short caption shown above the board (e.g. whose turn it is). Empty = nothing shown.
#[derive(Resource, Default)]
pub struct StatusLine(pub String);

/// The core's outbox: indices of actionable controls clicked this frame, in click order. A consumer
/// (the `game` adapter, or a prototype) drains it. The core only appends.
#[derive(Resource, Default)]
pub struct ActionRequests(pub Vec<usize>);

/// Ordering for the per-frame pipeline so a consumer can slot work between input and draw:
/// [`Input`](CardTableSet::Input) (focus/zoom/collect clicks) → [`Apply`](CardTableSet::Apply)
/// (drain [`ActionRequests`], mutate [`Table`]) → [`Draw`](CardTableSet::Draw) (rebuild the UI).
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CardTableSet {
    Input,
    Apply,
    Draw,
}

/// The game-agnostic renderer. Add it, put a [`Board`] in [`Table`], and you have a clickable card
/// table. Add [`GamePlugin`] (feature `game`) on top to drive it from a [`contract::Game`].
pub struct CardTablePlugin;

impl Plugin for CardTablePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Table>()
            .init_resource::<ActionRail>()
            .init_resource::<StatusLine>()
            .init_resource::<ActionRequests>()
            .init_resource::<DragGuard>()
            .init_resource::<Dragging>()
            .init_resource::<FannedFront>()
            .init_resource::<FactoryBase>()
            .init_resource::<BuildInfo>()
            // Board-game driver request state (always present so the observers can record even with no
            // game added; drained by `BoardGamePlugin` when a game is present).
            .init_resource::<DropRequest>()
            .init_resource::<TapRequest>()
            .init_resource::<AffordanceClick>()
            .init_resource::<AffordanceLabels>()
            .init_resource::<SceneState>()
            .init_resource::<DropTrace>()
            .init_resource::<FontSample>()
            .init_resource::<CardScreenRects>()
            .insert_resource(NeedsRebuild(true))
            // The opening board is a wholesale board too - tidy its decks the same way Start Over does.
            .insert_resource(DecksNeedTidy(true))
            .insert_resource(make_debug_log())
            .configure_sets(
                Update,
                (CardTableSet::Input, CardTableSet::Apply, CardTableSet::Draw).chain(),
            )
            // Fonts first (they build `UiFonts`), then the System deck (its Fonts sub-deck reads `UiFonts`).
            .add_systems(
                Startup,
                (setup_camera, install_ui_fonts, inject_system_deck).chain(),
            )
            .add_systems(
                Update,
                (
                    animate_nodes,
                    fan_layout,
                    update_card_cues,
                    scroll_hovered_panel,
                    // Position authority first, then the feature that reads it.
                    (track_card_rects, animate_target_arrows).chain(),
                ),
            )
            // Shove: feed surface + overlay obstacles, then re-settle the Table's piles (new/resized deck,
            // window resize, moved title) and, in a Free zone, its cards. Element sizes are no longer fed
            // back from the render - the model computes every footprint (card and deck chip) itself.
            .add_systems(
                Update,
                (sync_surface_size, sync_pinned, settle_table_piles).chain(),
            )
            .add_systems(Update, settle_free_cards.after(sync_pinned))
            .add_systems(Update, redraw.in_set(CardTableSet::Draw))
            // Input is picking-driven, so it runs in observers rather than the Input system set:
            // clicks open/close piles and fire actions; a card drag drops into a pile; a pile drag
            // slides it freely across the table.
            .add_observer(on_drag_start)
            .add_observer(on_drag_end_clear_guard)
            .add_observer(on_click)
            .add_observer(on_drop)
            .add_observer(on_node_drag)
            .add_observer(on_node_drag_end)
            .add_observer(on_panel_drag);
    }
}

// ---- components ---------------------------------------------------------

/// The UI root, despawned and rebuilt each redraw.
#[derive(Component)]
struct CardTableRoot;

/// A card or rail button bound to the action at this opaque index.
#[derive(Component)]
struct ActionControl(usize);

/// Links a card's node back to its model [`CardId`] — the handle drag/drop moves.
#[derive(Component, Clone, Copy)]
struct CardRef(CardId);

/// Marks a pile's node as a drop target: a card dropped here moves into this pile.
#[derive(Component, Clone, Copy)]
struct PileDropZone(PileId);

/// Marks a **movable felt element** — the absolutely-positioned wrapper of a card *or* a nested pile,
/// carrying which [`Node`](TableNode) it is. Dragging it slides that element freely (live) and commits
/// on release; a card and a pile are dragged, sized, and animated by the *same* handlers, differing only
/// in their leaf behaviour on drop (a pile just repositions; a card may reorder or move between zones).
#[derive(Component, Clone, Copy)]
struct Movable(TableNode);

/// Marks the table surface — the scroll viewport for a zone.
#[derive(Component)]
struct TableSurface;

/// Marks the **content region** inside the surface where a zone's content lives. Its size is fed to the
/// model as the usable bounds. Structured layouts (grid / list / rows) inset it below the overlay band;
/// a freely-placed zone (Free / the root) does not — there, content shares the felt with the overlays and
/// the [`Pinned`] fixtures shove it clear instead.
#[derive(Component)]
struct TableContent;

/// A **pinned felt fixture** — the centered zone title, the Back card — whose rectangle is fed to the
/// model so freely-placed content settles clear of it (top priority; see [`sync_pinned`],
/// [`Board::set_pinned`]). Fixed in place: it pushes cards but never moves for one.
#[derive(Component)]
struct Pinned;

/// A utility card that navigates up one zone level when clicked.
#[derive(Component)]
struct BackCard;

/// A control card that **advances the day** — shown in the Progress zone's overlay. Clicking it draws a
/// new `Day Passed` card onto Progress (the day count ticks up) and stands every move marker back up.
#[derive(Component)]
struct AdvanceDayCard;

/// A **scene tile** the player can tap for an in-place game action. Carries the tile's board [`CardId`]; the
/// tap is recorded into [`TapRequest`] and interpreted by the game's `tap_intention` (the game decides what
/// tapping it does). Also drives the tile's screen-rect tracking for arrow endpoints.
#[derive(Component, Clone, Copy)]
struct TileCard(CardId);

/// Marks a draggable tile that belongs to a **modal scene** (a formation tile). It carries [`Movable`] so the
/// drag pipeline picks it up, but its position is owned by flex layout, not the table's model positions — so
/// [`animate_nodes`] excludes `ModalTile` by construction (`Without<ModalTile>`) rather than special-casing
/// the arena. Resolves the old dual-ownership: one marker for "the table animates me", another for "I drag
/// but flex places me".
#[derive(Component)]
struct ModalTile;

/// One transient dot of an attention arrow (a top-level overlay node). Re-spawned each frame by
/// [`animate_target_arrows`] so the dots flow toward the target. The *which* cards link is now decided by the
/// game (a [`Scene`]'s links); this marker just tags a drawn dot for cleanup.
#[derive(Component)]
struct ArrowDot;

/// **The single authority for where a card is on screen** — each on-screen card's rect in *logical* pixels
/// (viewport origin, top-left), rebuilt every frame by [`track_card_rects`]. Any feature that needs a card's
/// position (the targeting arrows, and future point-at-a-card work) reads this instead of re-deriving the
/// physical→logical / center conversion from `UiGlobalTransform` + `ComputedNode` — the source of a whole
/// class of coordinate bugs (a node's transform is in *physical* px; `Val::Px` is *logical*). The physical
/// metaphor's core promise — *every card has a place* — should be a one-line query: `rects.center(card)`.
#[derive(Resource, Default)]
struct CardScreenRects(HashMap<CardId, Rect>);

impl CardScreenRects {
    /// The card's on-screen centre in logical pixels, if it is currently rendered as a card.
    fn center(&self, card: CardId) -> Option<Vec2> {
        self.0.get(&card).map(Rect::center)
    }
}

/// A card face whose panel **scrolls** — its content can exceed the card, so the wheel
/// ([`scroll_hovered_panel`]) and a drag ([`on_panel_drag`]) move it. Worn only by expanded
/// [`CardKind::Virtual`] readouts (a combat log), which can run long; ordinary panel cards clip.
#[derive(Component)]
struct ScrollPanel;

/// Logical px scrolled per wheel line (when the OS reports scroll in lines, not pixels).
const SCROLL_LINE_PX: f32 = 28.0;

/// A **debug event log** written to `cardtable-debug.log` next to the launch dir (truncated each launch,
/// with a launch stamp), recording drags, drops (cursor position + each row's hover state) and the
/// resulting Active-row state — so drop behaviour can be traced exactly. No file on the web.
#[derive(Resource)]
struct DebugLog(std::sync::Mutex<Option<std::fs::File>>);

impl DebugLog {
    fn line(&self, msg: impl AsRef<str>) {
        if let Ok(mut guard) = self.0.lock()
            && let Some(file) = guard.as_mut()
        {
            use std::io::Write;
            let _ = writeln!(file, "{}", msg.as_ref());
            let _ = file.flush();
        }
    }
}

/// Create the debug log, truncating `cardtable-debug.log` and stamping the launch so the file always
/// reflects the current run.
fn make_debug_log() -> DebugLog {
    // Native-only convenience: the web build has no filesystem, and
    // `SystemTime::now()` panics on wasm32-unknown-unknown (it aborts app
    // startup, which then cascades into a bogus "Unable to find a GPU!"). So on
    // wasm the log stays empty — no file, no wall-clock stamp.
    if cfg!(target_arch = "wasm32") {
        return DebugLog(std::sync::Mutex::new(None));
    }
    let file = std::fs::File::create("cardtable-debug.log").ok();
    let log = DebugLog(std::sync::Mutex::new(file));
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    log.line(format!("=== cardtable debug log - launch {stamp} ==="));
    log
}

/// True while a pointer drag is in progress. Bevy fires a `Click` at the end of *every* drag (press
/// and release over the same entity, regardless of the drag), so this guards the click handler from
/// treating a drag's release as a real click. Holds the pointer position where the current drag **began**
/// (`Some` while a drag is live, cleared to `None` on [`DragEnd`]); the click that ends the drag is only
/// suppressed if the pointer actually travelled past [`CLICK_DRAG_TOLERANCE`] — a jittery tap (a press that
/// wobbled a few pixels and back) still counts as a click. Set on [`DragStart`], cleared on [`DragEnd`].
#[derive(Resource, Default)]
struct DragGuard(Option<Vec2>);

/// How far (screen px) the pointer must travel between press and release before the release counts as a
/// **drag** rather than a **click**. Below this, a wobble-during-press is treated as a tap, so a slightly
/// shaky click (common on trackpads / touch) is no longer swallowed by the drag guard.
const CLICK_DRAG_TOLERANCE: f32 = 8.0;

/// Set when the UI must be torn down and rebuilt — *structural* changes only (open/close a pile, move
/// a card, a new game snapshot). Pile positions are not structural; they animate, so repositioning
/// never sets this. See [`redraw`] and [`animate_nodes`]. Public so an outer layer (e.g. the binary's
/// combat system) can request a redraw after mutating the [`Table`].
#[derive(Resource)]
pub struct NeedsRebuild(pub bool);

/// Set when the [`Table`]'s board has been **replaced wholesale** (startup, Start Over, a loaded save) —
/// the decks must be laid out from scratch rather than left wherever the new board's seed positions put
/// them. This can't be *inferred*: a fresh board reuses the same [`PileId`]s and, since the model computes
/// footprints up front, the same sizes, so [`settle_table_piles`]'s size-diff sees "no change" and would
/// never tidy (Start Over left every deck at its raw fixture seed). The rebuild itself is the trigger.
#[derive(Resource, Default)]
struct DecksNeedTidy(bool);

/// The felt element ([`Movable`]) currently being dragged (if any), so its tile isn't snapped back by the
/// animation while the pointer holds it. Either a card or a pile — the drag path is shared.
#[derive(Resource, Default)]
struct Dragging(Option<TableNode>);

/// The card currently pulled to the **front of a fan** (a [`Fan`](Arrangement::Fan) row's tapped card),
/// if any. A fanned row overlaps its cards so only each left edge shows; the front card is drawn fully, on
/// top of its neighbours — the "examine one at a time" reveal. `None` = the natural fan (the last card
/// shows). Set on tap (see [`on_click`]); a stale id simply matches nothing and the natural fan shows.
#[derive(Resource, Default)]
struct FannedFront(Option<CardId>);

/// A **fan row's container** — the relative box a [`Fan`](Arrangement::Fan)/`Rows` row's cards are placed
/// in. It flex-grows to fill the room left after the header, so its laid-out width is the space the fan
/// has to work with; [`fan_layout`] reads that width each frame and spaces the cards to match.
#[derive(Component)]
struct FanContainer;

/// One card in a fan, tagging its `index` along the row and its `card` id, so [`fan_layout`] can place it
/// (and know which one is the tapped [`FannedFront`], to open the fan around it).
#[derive(Component)]
struct FanCard {
    index: usize,
    card: CardId,
}

/// A **pristine "factory" table** the embedder supplies (e.g. `boardgame` inserts a fresh `sample_table`)
/// — the target of **Start Over**, which discards this session *and* the loaded save. The System deck is
/// (re)installed onto it when Start Over fires, so it need not carry one.
#[derive(Resource, Default)]
pub struct FactoryBase(pub Board);

/// The **build stamp** the embedder supplies (e.g. `boardgame` inserts its git commit) — shown as the
/// expandable **Version** card in the System deck so you can tell which commit is deployed and how long
/// ago it was built. Defaults to empty / unset (no stamp).
#[derive(Resource, Default)]
pub struct BuildInfo {
    /// The commit hash (e.g. `git describe` output). Empty = unknown.
    pub hash: String,
    /// The commit date, `YYYY-MM-DD`. Empty = unknown.
    pub date: String,
    /// The commit's unix timestamp (seconds), for the relative "n ago" line. `None` = unknown.
    pub timestamp: Option<i64>,
}

// ---- systems ------------------------------------------------------------

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// Inject the **System deck** — a regular drill-in pile on the table: click it to enter its zone, then
/// click a card inside to act (Exit quits, Start Over resets). It holds **Start Over** everywhere and
/// **Exit** on desktop only — a browser can't quit its own tab, so the Exit card never appears there —
/// plus a **Version** card. Runs once at startup.
fn inject_system_deck(mut table: ResMut<Table>, build: Res<BuildInfo>) {
    install_system_deck(&mut table.0, &build);
}

/// Add one [`Utility`] action card (face-up `title`) to `pile`.
fn add_util(table: &mut Board, pile: PileId, title: &str, utility: Utility) {
    if let Ok(id) = table.add_card(
        pile,
        Face::Up {
            title: title.into(),
        },
        None,
    ) {
        let _ = table.set_card_kind(id, CardKind::Utility(utility));
    }
}

/// Install the **System deck** into `table` — a regular [`Free`](Arrangement::Free) deck you drill into.
/// Holds **Start Over** (pristine table) everywhere, **Exit** on desktop (a browser can't quit its own
/// tab), and an expandable **Version** card (`build`, if a hash is known) so you can tell what's deployed.
/// Any existing System deck (e.g. from a resumed save) is **removed and rebuilt**, so the deck is never
/// doubled up *and* its version/actions always match the running build. Called at startup and by Start Over.
fn install_system_deck(table: &mut Board, build: &BuildInfo) {
    let root = table.root_id();
    let stale: Vec<PileId> = table.pile(root).map_or(Vec::new(), |p| {
        p.subpiles()
            .into_iter()
            .filter(|&s| table.pile(s).is_some_and(|d| d.label == "System"))
            .collect()
    });
    for s in stale {
        let _ = table.remove_pile(s);
    }
    let Ok(pile) = table.add_pile(root, "System") else {
        return;
    };
    add_util(table, pile, "Start Over", Utility::StartOver);
    if !cfg!(target_arch = "wasm32") {
        add_util(table, pile, "Exit", Utility::Exit);
    }
    // An expandable Version card: Small shows just "Version"; grown to Medium it shows the full hash, the
    // build date, and how long ago it was built (computed here so it's fixed to this launch).
    if !build.hash.is_empty()
        && let Ok(id) = table.add_card(
            pile,
            Face::Up {
                title: "Version".into(),
            },
            None,
        )
    {
        let _ = table.set_card_type(id, "version");
        let _ = table.set_card_detail(id, version_detail(build));
    }
    // The **Fonts** deck: a drill-in sub-deck with one card per bundled face. Tapping a font card opens the
    // sample grid (the whole palette rendered in that font). Names come from `FONT_NAMES`, the same source
    // `UiFonts` is built from, so the deck can only offer fonts that are actually loaded.
    if let Ok(fonts_pile) = table.add_pile(pile, "Fonts") {
        for name in FONT_NAMES {
            if let Ok(c) = table.add_card(
                fonts_pile,
                Face::Up {
                    title: name.to_string(),
                },
                None,
            ) {
                let _ = table.set_card_type(c, "font");
            }
        }
    }
    // "System" is a Zone (naming) card — the deck's label, not one of its actions.
    if let Ok(system) = table.add_card(
        pile,
        Face::Up {
            title: "System".into(),
        },
        None,
    ) {
        let _ = table.set_card_kind(system, CardKind::Zone);
    }
    let _ = table.set_layout(
        pile,
        Layout {
            arrangement: Arrangement::Free,
            editable: true,
        },
    );
    // Give the freshly built deck a non-overlapping home *now*, in the model, before the first frame is
    // drawn - otherwise it sits at (0,0) on top of an existing deck until `settle_table_piles` shoves it
    // clear. The model computes the deck's chip footprint itself (from its card count), so placement needs
    // no render. Placing it clear at creation is prevention (the deck never overlaps); the later settle only
    // tidies it into the row.
    let _ = table.place_clear(pile);
    // Seed a tidy grid below the overlay band: a Free deck reads each child's own position, and freshly
    // added children are all at (0,0) — i.e. stacked in the top-left, behind the Back button. Lay them out
    // in rows (three across) starting one gap under the band, matching the fixtures' `grid_layout` spacing.
    // **Both** cards and the Fonts sub-deck (a pile) are seeded: skipping the pile left it at (0,0) until the
    // first drill-in shoved it into place, so the deck visibly jumped the first time you opened System.
    for (i, node) in table.movable_children(pile).into_iter().enumerate() {
        let (col, row) = (i % 3, i / 3);
        let x = GAP + col as i32 * (CARD_W + GAP);
        let y = OVERLAY_BAND + GAP + row as i32 * (CARD_H + GAP);
        match node {
            TableNode::Card(c) => {
                let _ = table.set_card_pos(c, x, y);
            }
            TableNode::Pile(p) => {
                let _ = table.set_pile_pos(p, x, y);
            }
        }
    }
}

/// The **Version** card's detail lines (shown when it's grown to Medium): the full commit hash, the build
/// date (when known), and a relative "{n} {unit} ago" (when the timestamp is known). The relative line is
/// computed against the *current* wall-clock time via [`web_time`] (so it works on wasm, where
/// `std::time::SystemTime::now()` panics), and omitted when the build timestamp is unknown.
fn version_detail(build: &BuildInfo) -> Vec<String> {
    let mut lines = vec![build.hash.clone()];
    if !build.date.is_empty() {
        lines.push(format!("Updated {}", build.date));
    }
    if let (Some(built), Some(now)) = (build.timestamp, now_unix()) {
        lines.push(relative_time(now - built));
    }
    lines
}

/// The current wall-clock time as unix seconds, via [`web_time`] so it's safe on wasm (where
/// `std::time::SystemTime::now()` panics). `None` if the clock is before the epoch.
fn now_unix() -> Option<i64> {
    web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64)
}

/// `"{quantity} {unit}"` with the unit pluralized to match — `"1 hour"`, `"2 hours"` — so a count never
/// reads as "1 hours" or "2 hour". Picks `singular` when `quantity == 1`, else `plural`.
fn pluralize(quantity: i64, singular: &str, plural: &str) -> String {
    let unit = if quantity == 1 { singular } else { plural };
    format!("{quantity} {unit}")
}

/// A coarse human "how long ago" for `seconds_ago` (now − then): `"just now"` under a minute, else the
/// largest whole unit that fits — minutes, hours, or days — as `"N minutes ago"` (with [`pluralize`] so
/// the 1-unit cases read `"1 hour ago"`, never "1 hours ago"). A zero or negative age (a future or
/// just-now stamp) is `"just now"`.
fn relative_time(seconds_ago: i64) -> String {
    if seconds_ago < 60 {
        return "just now".to_string();
    }
    let (quantity, unit) = if seconds_ago < 3600 {
        (seconds_ago / 60, "minute")
    } else if seconds_ago < 86_400 {
        (seconds_ago / 3600, "hour")
    } else {
        (seconds_ago / 86_400, "day")
    };
    format!("{} ago", pluralize(quantity, unit, &format!("{unit}s")))
}

/// The bundled UI typefaces: **DejaVu Sans** (proportional, for cards and body) and **DejaVu Sans Mono** (for
/// the combat log's aligned columns). DejaVu covers the whole [`palette`] in one font - arrows, card suits,
/// geometric shapes, dingbats, math, dashes - which base Noto Sans did not (that gap was caught by the
/// `fonts_cover_palette` test). Bitstream Vera / public-domain license; see `fonts/DejaVu-LICENSE.txt`.
/// Static faces, ~1.1 MB total; subset to the palette (~30 KB) before the wasm ships.
const DEJAVU_SANS: &[u8] = include_bytes!("../fonts/DejaVuSans.ttf");
const DEJAVU_SANS_MONO: &[u8] = include_bytes!("../fonts/DejaVuSansMono.ttf");

/// The bundled UI font faces, by display name - the single source of truth for what the **font deck** offers.
/// Each is `(name, handle)`; the deck builds one card per entry and the sample view renders the palette in the
/// matching handle, so the deck can never list a font that isn't loaded.
const FONT_NAMES: [&str; 2] = ["DejaVu Sans", "DejaVu Sans Mono"];

/// The monospace font handle (DejaVu Sans Mono), for UI that wants aligned columns - the combat log.
#[derive(Resource, Clone)]
pub struct MonoFont(pub Handle<Font>);

/// The loaded UI fonts as `(name, handle)`, index-aligned with [`FONT_NAMES`]. Drives the font deck's cards
/// and the sample grid.
#[derive(Resource, Clone)]
pub struct UiFonts(pub Vec<(String, Handle<Font>)>);

/// The font currently shown large in the sample grid (its name), or `None`. Set by tapping a font card,
/// cleared by the sample view's Back card.
#[derive(Resource, Default)]
pub struct FontSample(pub Option<String>);

/// Install the UI fonts: override Bevy's default with DejaVu Sans (so every `TextFont { ..default() }` picks it
/// up without threading a handle through each label), register DejaVu Sans Mono in [`MonoFont`] for monospace
/// labels, and record both in [`UiFonts`] for the font deck.
fn install_ui_fonts(mut commands: Commands, mut fonts: ResMut<Assets<Font>>) {
    fonts
        .insert(AssetId::default(), Font::from_bytes(DEJAVU_SANS.to_vec()))
        .expect("override the default font");
    let sans: Handle<Font> = Handle::default(); // the overridden default asset
    let mono = fonts.add(Font::from_bytes(DEJAVU_SANS_MONO.to_vec()));
    commands.insert_resource(MonoFont(mono.clone()));
    commands.insert_resource(UiFonts(vec![
        (FONT_NAMES[0].to_string(), sans),
        (FONT_NAMES[1].to_string(), mono),
    ]));
}

fn on_drag_start(on: On<Pointer<DragStart>>, mut guard: ResMut<DragGuard>) {
    // Remember where the drag began; the click that ends it is only suppressed if the pointer actually
    // travelled from here (see `on_click`).
    guard.0 = Some(on.event().pointer_location.position);
}

/// Clear the drag guard whenever *any* drag ends, so only the click that ends a drag is suppressed and
/// real clicks work again afterward. Covers every draggable — piles, grid cards, and projection cards
/// (which carry no `Movable`, so the specific card-drag handler never runs for them).
fn on_drag_end_clear_guard(_on: On<Pointer<DragEnd>>, mut guard: ResMut<DragGuard>) {
    guard.0 = None;
}

/// A picking click, resolved by *what* the target is (the only meaning a click carries): a **Back**
/// card goes up a zone; a **utility** card fires its action (Exit quits, Start Over resets); an expandable
/// **card** grows/shrinks; a loose action fires; a **pile** is entered (its zone) — unless it has nothing
/// under its label to show. Inner nodes (a card's text) match nothing and propagate to their parent.
/// Global observer, so it survives the per-change UI rebuild.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn on_click(
    mut on: On<Pointer<Click>>,
    guard: Res<DragGuard>,
    targets: Query<(
        Option<&ActionControl>,
        Option<&CardRef>,
        Option<&PileDropZone>,
        Has<BackCard>,
        Has<AdvanceDayCard>,
        Option<&AffordanceControl>,
        Option<&TileCard>,
        Has<UndoControl>,
        Option<&ChoiceControl>,
    )>,
    mut table: ResMut<Table>,
    mut requests: ResMut<ActionRequests>,
    mut rebuild: ResMut<NeedsRebuild>,
    mut affordance_click: ResMut<AffordanceClick>,
    mut undo_click: ResMut<UndoClick>,
    mut choice_click: ResMut<ChoiceClick>,
    mut history: ResMut<crate::board_driver::BoardHistory>,
    mut tap_request: ResMut<TapRequest>,
    mut font_sample: ResMut<FontSample>,
    mut front: ResMut<FannedFront>,
    mut tidy: ResMut<DecksNeedTidy>,
    factory: Res<FactoryBase>,
    build: Res<BuildInfo>,
    mut exit: MessageWriter<AppExit>,
) {
    if let Some(start) = guard.0
        && on.event().pointer_location.position.distance(start) > CLICK_DRAG_TOLERANCE
    {
        return; // the release that ends a *real* drag also fires Click — not an intentional click. A tap
        // that barely moved (within the tolerance) falls through and is treated as the click it is.
    }
    let Ok((action, card, pile, is_back, is_advance_day, affordance, arena_unit, is_undo, choice)) =
        targets.get(on.event().entity)
    else {
        return;
    };
    // A scene **choice** card: the decision the game is asking for. Recorded for the driver, which asks the
    // game what it means (`choice_intention`) - the renderer never learns what any option does.
    if let Some(c) = choice {
        choice_click.0 = Some(c.0);
        rebuild.0 = true;
        on.propagate(false);
        return;
    }
    // The scene's **Back** card: rewind one move. Recorded for the driver's `apply_undo`, which just puts the
    // previous board back — the renderer never learns what it undid.
    if is_undo {
        undo_click.0 = true;
        rebuild.0 = true;
        on.propagate(false);
        return;
    }
    // A **scene tile** tap: record the board card for the driver's `apply_tap`, which asks the game's
    // `tap_intention` to interpret it (the renderer never knows what the tap means).
    if let Some(unit) = arena_unit {
        tap_request.0 = Some(unit.0);
        rebuild.0 = true;
        on.propagate(false);
        return;
    }
    // A game **affordance** control (Fight / Commit / Advance Day, …): record the click index for the board
    // driver's `apply_affordance`, which turns it into a `Game::apply`.
    if let Some(ctrl) = affordance {
        affordance_click.0 = Some(ctrl.0);
        rebuild.0 = true;
        on.propagate(false);
        return;
    }
    // The font **sample** grid is modal: its Back card closes the sample rather than leaving a zone.
    if is_back && font_sample.0.is_some() {
        font_sample.0 = None;
        rebuild.0 = true;
        on.propagate(false);
        return;
    }
    if is_back {
        table.0.zoom_out(); // leave this zone for its parent
        rebuild.0 = true;
    } else if is_advance_day {
        // Advance the day: lay a new `Day Passed` card on Progress and stand every move marker back up.
        if let (Some(progress), Some(events)) =
            (top_deck(&table.0, "Progress"), top_deck(&table.0, "Events"))
        {
            let _ = table.0.advance_day(progress, events);
            rebuild.0 = true;
        }
    } else if let Some(card_ref) = card {
        let id = card_ref.0;
        // A **font card** (in the System deck's Fonts sub-deck): tapping it opens the sample grid - the whole
        // palette rendered large in that font.
        if table.0.card(id).map(|c| c.card_type()) == Some("font") {
            font_sample.0 = table.0.card(id).map(|c| c.front_title().to_string());
            rebuild.0 = true;
            on.propagate(false);
            return;
        }
        // In a **fan** (a card in a `Rows` zone, the header aside), a tap pulls that card to the front so
        // you can examine it — its full face rises above its overlapping neighbours. Everywhere else a tap
        // fires the card's utility action, grows/shrinks the card (cycle render size), fires a loose action,
        // or is absorbed by a name-only card.
        let kind = table.0.card(id).map(|c| c.kind());
        let in_fan = matches!(
            table
                .0
                .pile(table.0.focus_id())
                .map(|p| p.layout().arrangement),
            Some(Arrangement::Rows)
        ) && kind != Some(CardKind::Header);
        if in_fan {
            // Just record the new front card — no rebuild. `fan_layout` reads this every frame and slides
            // the cards / lifts the front one in place; despawning the whole UI would only cause a flicker.
            front.0 = Some(id);
        } else if let Some(CardKind::Utility(utility)) = kind {
            // A utility card fires on click: Exit quits; Start Over discards this session for a pristine
            // table (then reinstalls the System deck so its version/actions match the running build).
            match utility {
                Utility::Exit => {
                    exit.write(AppExit::Success);
                }
                Utility::StartOver => {
                    table.0 = factory.0.clone();
                    install_system_deck(&mut table.0, &build);
                    rebuild.0 = true;
                    // The board was replaced wholesale, so every remembered step leads to a board that no
                    // longer exists. There is nothing to go Back to.
                    history.clear();
                    // A wholesale new board: its decks sit at the fixtures' raw seed positions, so they must
                    // be re-tidied into the row. Say so explicitly - the size-diff can't notice (same ids,
                    // same footprints).
                    tidy.0 = true;
                }
            }
        } else if table.0.card(id).is_some_and(|c| c.is_expandable()) {
            let _ = table.0.cycle_card_size(id);
            rebuild.0 = true;
        } else if let Some(action) = action {
            requests.0.push(action.0);
        }
    } else if let Some(action) = action {
        requests.0.push(action.0); // a loose action (rail item)
    } else if let Some(pile) = pile {
        let id = pile.0;
        // A deck with nothing under its label has nothing to show, so a click does not drill in; any other
        // deck (including System) is entered.
        let nothing_under = table.0.content_cards(id).is_empty()
            && table.0.pile(id).is_some_and(|p| p.subpiles().is_empty())
            && table.0.pile(id).is_some_and(|p| p.projection().is_empty());
        if !nothing_under {
            let _ = table.0.focus(id); // drill in: this pile becomes the current zone
            rebuild.0 = true;
        }
    } else {
        return; // background / inert — nothing to do (navigation is via cards, not the felt)
    }
    on.propagate(false);
}

/// A picking drop: move a dragged **card** into the pile it was dropped *onto*. Dropping a card onto
/// another card (or the felt) is not a move — that's an in-zone reorder, handled by [`on_node_drag_end`]
/// against the grid. Piles aren't nested on drop (they reposition via [`on_node_drag`]), so a dragged
/// pile is ignored. Presentation-level; mapping drops to game actions is future work.
fn on_drop(
    mut on: On<Pointer<DragDrop>>,
    cards: Query<&CardRef>,
    piles: Query<&PileDropZone>,
    mut drop_request: ResMut<DropRequest>,
) {
    let event = on.event();
    let Ok(dragged) = cards.get(event.event.dropped) else {
        return; // only cards drop *into* piles
    };
    // Record what the card landed on — another card, or a pile — for the board-game driver to interpret
    // (equip / un-equip / march) or, failing that, perform the default move into the pile. The renderer
    // stays game-agnostic. A drop onto the bare felt is an in-zone reorder, handled by `on_node_drag_end`.
    let onto = if let Ok(target) = cards.get(event.entity) {
        DropTarget::Card(target.0)
    } else if let Ok(zone) = piles.get(event.entity) {
        DropTarget::Pile(zone.0)
    } else {
        return;
    };
    on.propagate(false);
    drop_request.0 = Some((dragged.0, onto));
}

/// A short label for a node, for the debug log.
fn node_label(table: &Board, node: TableNode) -> String {
    match node {
        TableNode::Card(cid) => {
            format!("card={:?}", table.card(cid).map(|c| c.name()).unwrap_or(""))
        }
        TableNode::Pile(pid) => format!(
            "pile={:?}",
            table.pile(pid).map(|p| p.label.as_str()).unwrap_or("")
        ),
    }
}

/// The top-level deck with `label`, if present (a lookup by name for the fixed system zones).
fn top_deck(table: &Board, label: &str) -> Option<PileId> {
    table
        .pile(table.root_id())?
        .subpiles()
        .into_iter()
        .find(|&s| table.pile(s).map(|p| p.label.as_str()) == Some(label))
}

/// Slide a dragged **felt element** — a card or a nested pile — freely under the cursor, even off the
/// edge, live (no rebuild mid-drag; a position change isn't structural). The grab lands on the inner
/// visual; the event propagates up to the [`Movable`] wrapper, the node we move. The model position is
/// kept in step so a Free deck can shove and animate at rest; marking it the dragging node stops
/// [`animate_nodes`] from fighting the drag. Settling on release brings an off-edge element back.
fn on_node_drag(
    mut on: On<Pointer<Drag>>,
    mut movables: Query<(&Movable, &mut Node)>,
    mut dragging: ResMut<Dragging>,
    mut table: ResMut<Table>,
    mut commands: Commands,
    log: Res<DebugLog>,
) {
    if let Ok((movable, mut node)) = movables.get_mut(on.event().entity) {
        if dragging.0 != Some(movable.0) {
            // First frame of this drag: lift the tile onto the held layer so it floats above everything
            // it slides over — the "pick it up off the table" gesture. Released back down in `on_node_drag_end`.
            commands
                .entity(on.event().entity)
                .insert(GlobalZIndex(HELD_Z));
            log.line(format!(
                "DRAG_START {} at cursor={:?}",
                node_label(&table.0, movable.0),
                on.event().pointer_location.position
            ));
        }
        let delta = on.event().event.delta;
        let (x, y) = (px(node.left) + delta.x, px(node.top) + delta.y);
        node.left = Val::Px(x);
        node.top = Val::Px(y);
        match movable.0 {
            TableNode::Card(cid) => {
                let _ = table.0.set_card_pos(cid, x as i32, y as i32);
            }
            TableNode::Pile(pid) => {
                let _ = table.0.set_pile_pos(pid, x as i32, y as i32);
            }
        }
        dragging.0 = Some(movable.0);
        on.propagate(false);
    }
}

/// The pixel value of a `Val`, or `0.0` for the non-pixel variants (piles always use `Px`).
fn px(value: Val) -> f32 {
    match value {
        Val::Px(p) => p,
        _ => 0.0,
    }
}

/// Feed the **content region's** laid-out size (the surface below the overlay band) to the model as the
/// wall bounds that contain the movable elements — so content stays inside the usable felt.
fn sync_surface_size(content: Query<&ComputedNode, With<TableContent>>, mut table: ResMut<Table>) {
    if let Ok(computed) = content.single() {
        let size = computed.size * computed.inverse_scale_factor;
        table.0.set_bounds(size.x as i32, size.y as i32);
    }
}

/// Feed the **pinned fixtures'** rectangles (the centered title, the Back card) to the model, in the
/// content region's logical coordinate space — so [`Board::separate`] shoves freely-placed content clear
/// of them. In a structured (inset) zone the fixtures land above the content region and simply don't
/// bite; in a Free / root zone they sit on the felt and push the cards. Runs each frame; there are few.
fn sync_pinned(
    pinned: Query<(&ComputedNode, &UiGlobalTransform), With<Pinned>>,
    content: Query<(&ComputedNode, &UiGlobalTransform), With<TableContent>>,
    mut table: ResMut<Table>,
) {
    // The content region's top-left (physical), so a fixture rect converts into the content coordinate
    // space that model positions live in.
    let Some(origin) = content
        .single()
        .ok()
        .map(|(cn, gt)| gt.translation - cn.size() * 0.5)
    else {
        return;
    };
    let rects: Vec<(Pos, Pos)> = pinned
        .iter()
        .map(|(cn, gt)| {
            let sf = cn.inverse_scale_factor; // physical → logical, matching model positions
            let size = cn.size();
            let top_left = (gt.translation - size * 0.5 - origin) * sf;
            (
                Pos {
                    x: top_left.x as i32,
                    y: top_left.y as i32,
                },
                Pos {
                    x: (size.x * sf) as i32,
                    y: (size.y * sf) as i32,
                },
            )
        })
        .collect();
    table.0.set_pinned(rects);
}

/// Ease each **movable element's** wrapper toward its model position, so a separation (or any reposition)
/// *slides* into place instead of snapping — a card and a pile alike. The dragged element is left free;
/// those at rest are skipped so the node (and its layout) isn't touched every frame. A card in an ordered
/// zone targets its grid cell; everything else targets its own model position. A projection view lays its
/// cards out with flexbox (not by model position), so it is left alone.
fn animate_nodes(
    time: Res<Time>,
    table: Res<Table>,
    dragging: Res<Dragging>,
    // `ModalTile`s (formation tiles) are flex-positioned; the table never owns their `left/top`, so they are
    // excluded here by construction rather than special-cased.
    mut movables: Query<(&Movable, &mut Node), Without<ModalTile>>,
) {
    if table
        .0
        .pile(table.0.focus_id())
        .is_some_and(|p| !p.projection().is_empty())
    {
        return;
    }
    let focus = table.0.focus_id();
    // The table (root) is never a structured zone — it's laid out by `settle_table_piles` (an exact
    // constant-gap row), so its piles keep their model position. Only a *drilled-in* List/Grid reflows
    // here, mirroring how `build_ui` special-cases `at_root`.
    let structured = focus != table.0.root_id()
        && matches!(
            table.0.pile(focus).map(|p| p.layout().arrangement),
            Some(Arrangement::List | Arrangement::Grid { .. })
        );
    // A structured zone (List/Grid) reflows footprint-aware; everything else (Free, the root) reads each
    // node's own model position. Compute the structured layout once, then look each node up.
    let layout: HashMap<TableNode, Pos> = if structured {
        table
            .0
            .structured_positions(
                focus,
                GAP,
                GAP,
                Pos {
                    x: CARD_W,
                    y: CARD_H,
                },
            )
            .into_iter()
            .collect()
    } else {
        HashMap::new()
    };
    let t = (SLIDE_SPEED * time.delta_secs()).min(1.0);
    for (movable, mut node) in &mut movables {
        if dragging.0 == Some(movable.0) {
            continue; // free while held
        }
        let target = if structured {
            match layout.get(&movable.0) {
                Some(&p) => p,
                None => continue,
            }
        } else {
            match movable.0 {
                TableNode::Pile(pid) => match table.0.pile(pid) {
                    Some(d) => d.pos(),
                    None => continue,
                },
                TableNode::Card(cid) => match table.0.card(cid) {
                    Some(c) => c.pos(),
                    None => continue,
                },
            }
        };
        let (cx, cy) = (px(node.left), px(node.top));
        let (tx, ty) = (target.x as f32, target.y as f32);
        if (tx - cx).abs() < 0.5 && (ty - cy).abs() < 0.5 {
            continue; // at rest
        }
        node.left = Val::Px(cx + (tx - cx) * t);
        node.top = Val::Px(cy + (ty - cy) * t);
    }
}

/// The game action a **pairing** drop performs: dragging `dragged` onto `target` when `dragged` carries a
/// pairing onto `target`'s [`pair_key`](cardtable_model::Card::pair_key). Game-agnostic — the game declares
/// the pairings (in the view); the renderer just performs the gesture and reports the action index. This is
/// how a game-meaningful drag (drag a hero onto a kit to equip, a character onto a location to march) flows
/// through the seam, replacing the renderer's hardcoded equip/march rules.
fn pairing_action(table: &Board, dragged: CardId, target: CardId) -> Option<usize> {
    let key = table.card(target)?.pair_key()?;
    table
        .card(dragged)?
        .pairings()
        .iter()
        .find(|(onto, _)| *onto == key)
        .map(|&(_, action)| action)
}

/// Whether the held card `dragged` may legally be dropped on the card `target` — a **pairing** (the game
/// declared one), or the legacy **inn equip** rule: inside a projection (the inn) a kit and a hero pair,
/// i.e. exactly one of the two carries a recipe.
fn can_drop_on_card(table: &Board, dragged: CardId, target: CardId) -> bool {
    if pairing_action(table, dragged, target).is_some() {
        return true;
    }
    if dragged == target
        || table
            .pile(table.focus_id())
            .is_none_or(|p| p.projection().is_empty())
    {
        return false;
    }
    let d_kit = table.card(dragged).is_some_and(|c| c.recipe().is_some());
    let t_kit = table.card(target).is_some_and(|c| c.recipe().is_some());
    d_kit != t_kit
}

/// Whether `id` is a hero's **map position** copy — a `hero` card whose home is one of the Locations
/// grid's place piles (as opposed to a hero copy in the Heroes deck, a character deck, or Progress).
fn is_map_position(table: &Board, id: CardId) -> bool {
    let Some(home) = table
        .card(id)
        .filter(|c| c.card_type() == "hero")
        .map(|c| c.home())
    else {
        return false;
    };
    top_deck(table, "Locations")
        .and_then(|loc| table.pile(loc))
        .is_some_and(|loc| loc.subpiles().contains(&home))
}

/// Whether two place piles are **orthogonally adjacent** on the Locations grid — one step up, down, left,
/// or right (Manhattan distance 1) by their row/column, read from the grid's `columns`. `false` if either
/// isn't a place or the Locations deck isn't a grid.
fn places_orthogonally_adjacent(table: &Board, a: PileId, b: PileId) -> bool {
    let Some(locations) = top_deck(table, "Locations") else {
        return false;
    };
    let Some(Arrangement::Grid { columns }) = table.pile(locations).map(|p| p.layout().arrangement)
    else {
        return false;
    };
    let places = table
        .pile(locations)
        .map(|p| p.subpiles())
        .unwrap_or_default();
    let (Some(ia), Some(ib)) = (
        places.iter().position(|&p| p == a),
        places.iter().position(|&p| p == b),
    ) else {
        return false;
    };
    let (ra, ca) = (ia / columns, ia % columns);
    let (rb, cb) = (ib / columns, ib % columns);
    ra.abs_diff(rb) + ca.abs_diff(cb) == 1
}

/// Whether the held card `dragged` may legally be dropped on the pile `target` — on the location **map**, a
/// character's position copy moves to an **orthogonally adjacent** place (one step up/down/left/right).
fn can_drop_on_pile(table: &Board, dragged: CardId, target: PileId) -> bool {
    if top_deck(table, "Locations") != Some(table.focus_id()) {
        return false;
    }
    let Some(card) = table.card(dragged).filter(|c| c.card_type() == "hero") else {
        return false;
    };
    places_orthogonally_adjacent(table, card.home(), target)
}

/// Whether dragging this card would trigger a **game action** (not just a visual re-arrange) — so it earns
/// the movable cue. A hero's map position copy moves places. Everything else (repositioning a Free card,
/// reordering a fan, dragging a deck) is presentation only — no cue.
///
/// This used to also cue the Inn's pairing (a hero card was worth picking up only while a kit was on show to
/// pair with, and vice versa) — which meant this game-agnostic renderer knew what a "kit" was. The Inn is
/// gone (see its removal commit) and so is that seam violation.
fn is_game_movable(table: &Board, id: CardId) -> bool {
    is_map_position(table, id)
}

/// Ensure entity `e` wears a cue [`Outline`] of `color`, toggling in place if it already has one (per
/// Bevy's guidance — cheaper than inserting/removing, no layout churn). The **target** glow is a touch
/// thicker; the **movable** ring is deliberately thin. On first insert it also gets a matching
/// [`BorderRadius`] so the ring rounds the card rather than boxing it (a bare `Movable` wrapper has none).
fn set_outline(
    commands: &mut Commands,
    e: Entity,
    outline: Option<Mut<Outline>>,
    node: &mut Node,
    color: Color,
) {
    let width = if color == TARGET_CUE {
        Val::Px(2.0)
    } else {
        Val::Px(1.0)
    };
    // Round the ring: a Bevy outline follows its node's border radius, and a bare `Movable` wrapper has
    // none. (Guarded so this is a one-time write, not a per-frame layout touch.)
    let radius = BorderRadius::all(CUE_RADIUS);
    if node.border_radius != radius {
        node.border_radius = radius;
    }
    match outline {
        Some(mut o) => {
            if o.color != color {
                o.color = color;
            }
            if o.width != width {
                o.width = width;
            }
        }
        None => {
            commands
                .entity(e)
                .insert(Outline::new(width, Val::Px(1.0), color));
        }
    }
}

/// Paint the card cues each frame. An amber [`MOVABLE_CUE`] ring marks cards whose drag would trigger a
/// **game action** (marching a character — not a visual re-arrange), so you can scan for what's worth
/// picking up; and while a drag is held, a green [`TARGET_CUE`] glow marks every place the held card can
/// legally land ([`can_drop_on_card`] / [`can_drop_on_pile`]) — so what glows is exactly what will accept
/// the drop. The held card itself drops its ring; both cues share one toggled [`Outline`].
fn update_card_cues(
    mut commands: Commands,
    table: Res<Table>,
    dragging: Res<Dragging>,
    mut movable: Query<(Entity, &Movable, Option<&mut Outline>, &mut Node)>,
    mut zones: Query<(Entity, &PileDropZone, Option<&mut Outline>, &mut Node), Without<Movable>>,
) {
    let dragged = dragging.0.and_then(|n| n.card());
    for (e, m, outline, mut node) in &mut movable {
        let color = if dragging.0 == Some(m.0) {
            Color::NONE // the held card floats; its ring would just clutter the drag
        } else {
            match m.0 {
                TableNode::Card(id)
                    if dragged.is_some_and(|d| can_drop_on_card(&table.0, d, id)) =>
                {
                    TARGET_CUE
                }
                TableNode::Card(id) if is_game_movable(&table.0, id) => MOVABLE_CUE,
                TableNode::Pile(pid)
                    if dragged.is_some_and(|d| can_drop_on_pile(&table.0, d, pid)) =>
                {
                    TARGET_CUE
                }
                _ => Color::NONE, // presentation-only drags (Free cards, deck chips) get no cue
            }
        };
        set_outline(&mut commands, e, outline, &mut node, color);
    }
    // Non-movable drop targets (the map's place cards) glow when the held card can land on them.
    for (e, z, outline, mut node) in &mut zones {
        let color = if dragged.is_some_and(|d| can_drop_on_pile(&table.0, d, z.0)) {
            TARGET_CUE
        } else {
            Color::NONE
        };
        set_outline(&mut commands, e, outline, &mut node, color);
    }
}

/// The scrollable range of a panel node in **logical** px: how far its content exceeds the viewport.
/// `ComputedNode` sizes are physical, so scale to logical before clamping a [`ScrollPosition`].
fn scroll_max(node: &ComputedNode) -> f32 {
    (node.content_size.y - node.size.y + node.scrollbar_size.y).max(0.0) * node.inverse_scale_factor
}

/// Scroll the [`ScrollPanel`] (an expanded combat log) under the cursor with the mouse wheel. Bevy's
/// `Overflow::scroll_y` only *clips*, so we drive the panel's [`ScrollPosition`] ourselves, clamped to the
/// content — the PC half of the parity (a drag scrolls it on touch, see [`on_panel_drag`]).
fn scroll_hovered_panel(
    wheel: Res<AccumulatedMouseScroll>,
    windows: Query<&Window>,
    mut panels: Query<(&mut ScrollPosition, &ComputedNode, &UiGlobalTransform), With<ScrollPanel>>,
) {
    if wheel.delta.y == 0.0 {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let dy = match wheel.unit {
        MouseScrollUnit::Line => wheel.delta.y * SCROLL_LINE_PX,
        MouseScrollUnit::Pixel => wheel.delta.y,
    };
    for (mut scroll, node, gt) in &mut panels {
        let sf = node.inverse_scale_factor;
        let center = gt.translation * sf;
        let half = node.size() * sf * 0.5;
        if (cursor.x - center.x).abs() <= half.x && (cursor.y - center.y).abs() <= half.y {
            scroll.0.y = (scroll.0.y - dy).clamp(0.0, scroll_max(node));
        }
    }
}

/// Drag a [`ScrollPanel`] to scroll it — the touch/iPad half of the parity (the log isn't Movable, so a
/// drag reaches here instead of moving it). Pulling up reveals lower lines. Clamped to the content.
fn on_panel_drag(
    mut on: On<Pointer<Drag>>,
    mut panels: Query<(&mut ScrollPosition, &ComputedNode), With<ScrollPanel>>,
) {
    if let Ok((mut scroll, node)) = panels.get_mut(on.event().entity) {
        scroll.0.y = (scroll.0.y - on.event().event.delta.y).clamp(0.0, scroll_max(node));
        on.propagate(false);
    }
}

/// Space each **fan row's** cards across its container, recomputed every frame so it tracks the real
/// available width — a window resize reflows it, matching how the grids reflow via [`animate_nodes`]. The
/// cards **spread as far as fits** (up to a full card + [`GAP`] step, no overlap) and pack tighter as the
/// room runs out, down to a [`FAN_SLIVER`] floor (past which the row simply overflows). The tapped
/// [`FannedFront`] card is drawn full on top (see [`build_ui`]); the fan **opens around it** by pushing the
/// cards to its right clear of its body — but only by however much they actually overlap, so a
/// fully-spread fan doesn't move. The dragged card is left alone so it follows the cursor.
fn fan_layout(
    containers: Query<(&ComputedNode, &Children), With<FanContainer>>,
    front: Res<FannedFront>,
    dragging: Res<Dragging>,
    mut cards: Query<(&FanCard, &mut Node, &mut ZIndex)>,
) {
    for (computed, children) in &containers {
        let width = computed.size.x * computed.inverse_scale_factor;
        let count = children.len();
        if count == 0 {
            continue;
        }
        // Which card (if any) is fronted — found by id among this container's cards.
        let front_idx = front.0.and_then(|f| {
            children
                .iter()
                .filter_map(|c| cards.get(c).ok())
                .find(|(fc, ..)| fc.card == f)
                .map(|(fc, ..)| fc.index)
        });
        for &child in children {
            let Ok((fc, mut node, mut z)) = cards.get_mut(child) else {
                continue;
            };
            // Lift the front card above all the slivers; otherwise keep index order (later cards on top).
            let want_z = if front.0 == Some(fc.card) {
                FAN_FRONT_Z
            } else {
                fc.index as i32
            };
            if z.0 != want_z {
                z.0 = want_z; // guarded so we don't churn change-detection when unchanged
            }
            if dragging.0 == Some(TableNode::Card(fc.card)) {
                continue; // position free while held
            }
            let left = fan_left(width, count, front_idx, fc.index);
            if (px(node.left) - left).abs() > 0.5 {
                node.left = Val::Px(left); // guarded so we don't thrash layout when unchanged
            }
        }
    }
}

// (Retired with the Active-row recruit flow: `drop_card_into_active` / `put_pair_back` copied a kit and
// discarded it — a mint + destroy. Recruiting is now the conservation-clean `try_equip` / `try_unequip`.)

/// A UI node's on-screen bounding box in **logical** px as `(centre, half-extents)`: its
/// [`UiGlobalTransform`] translation and half its [`ComputedNode`] size, both scaled from physical. This is
/// **the one place** the physical->logical box conversion lives; every consumer that needs a node's on-screen
/// box (the position authority, the drop hit-tests, the layout log) routes through it or [`node_rect`], so
/// the `inverse_scale_factor` math is written once. (The pin / size syncs deliberately use their own
/// conversions: `sync_pinned` maps into the content coordinate space, and the size syncs need only the size.)
fn node_box(cn: &ComputedNode, gt: &UiGlobalTransform) -> (Vec2, Vec2) {
    let sf = cn.inverse_scale_factor;
    (gt.translation * sf, cn.size() * sf * 0.5)
}

/// A UI node's on-screen [`Rect`] in **logical** px (centre + full size) — the [`Rect`] form of [`node_box`],
/// for the position authority and other rect consumers.
fn node_rect(cn: &ComputedNode, gt: &UiGlobalTransform) -> Rect {
    let (center, half) = node_box(cn, gt);
    Rect::from_center_size(center, half * 2.0)
}

/// Whether two `(centre, half)` boxes overlap (axis-aligned).
fn boxes_overlap(a: (Vec2, Vec2), b: (Vec2, Vec2)) -> bool {
    (a.0.x - b.0.x).abs() <= a.1.x + b.1.x && (a.0.y - b.0.y).abs() <= a.1.y + b.1.y
}

/// The single item of an iterator, or `None` if it yields zero or more than one — the "exactly one" rule
/// for a snappy drop: any overlap with exactly one target lands there; none, or an ambiguous two-plus,
/// snaps back.
fn exactly_one<T>(mut it: impl Iterator<Item = T>) -> Option<T> {
    match (it.next(), it.next()) {
        (Some(x), None) => Some(x),
        _ => None,
    }
}

// ---- per-context drag-drop resolvers (the named branches of `on_node_drag_end`) ----------------------

/// The one **valid** place a dragged hero token's box overlaps on the location map (the source place and any
/// illegal place are excluded by [`can_drop_on_pile`]); `None` if zero or ambiguously many overlap.
fn dropped_map_place(
    table: &Board,
    card: CardId,
    drag_box: (Vec2, Vec2),
    zones: &Query<(&PileDropZone, &ComputedNode, &UiGlobalTransform)>,
) -> Option<PileId> {
    exactly_one(zones.iter().filter(|&(z, cn, gt)| {
        can_drop_on_pile(table, card, z.0) && boxes_overlap(drag_box, node_box(cn, gt))
    }))
    .map(|(z, _, _)| z.0)
}

/// The scene **row** drop-pile whose centre is nearest the dragged tile's centre (source row included, so a
/// small nudge stays put and a straddling tile lands where it is more over).
fn nearest_row_pile(
    rows: &[Row],
    tile_center: Vec2,
    zones: &Query<(&PileDropZone, &ComputedNode, &UiGlobalTransform)>,
) -> Option<PileId> {
    zones
        .iter()
        .filter(|(z, _, _)| rows.iter().any(|r| r.drop_pile == z.0))
        .map(|(z, cn, gt)| (z.0, (node_box(cn, gt).0 - tile_center).length_squared()))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id)
}

/// The other projected card the release cursor landed on (the equip target), by logical-px hit-test. Needed
/// because the dragged fan tile follows the cursor and occludes the picking hit-test.
fn projected_card_under_cursor(
    cursor: Vec2,
    dragged: CardId,
    geom: &Query<(&Movable, &ComputedNode, &UiGlobalTransform)>,
) -> Option<CardId> {
    geom.iter()
        .filter_map(|(m, cn, gt)| m.0.card().map(|c| (c, cn, gt)))
        .filter(|&(c, _, _)| c != dragged)
        .find(|&(_, cn, gt)| {
            let (center, half) = node_box(cn, gt); // logical px, matching the cursor
            (cursor.x - center.x).abs() <= half.x && (cursor.y - center.y).abs() <= half.y
        })
        .map(|(c, _, _)| c)
}

/// A drag that resolved to no game move: the card stays in its **home** pile — repositioned (Free) or
/// reordered into the nearest structured slot — and the settle is traced so a silent snap-back is visible.
fn settle_card_home(table: &mut Board, card: CardId, node: &Node, trace: &mut Vec<String>) {
    let Some(home) = table.card(card).map(|c| c.home()) else {
        return;
    };
    let card_name = table
        .card(card)
        .map(|c| c.front_title().to_string())
        .unwrap_or_default();
    let home_label = table
        .pile(home)
        .map(|p| p.label.clone())
        .unwrap_or_default();
    if matches!(
        table.pile(home).map(|p| p.layout().arrangement),
        Some(Arrangement::Free)
    ) {
        // Unordered: keep it where dropped, then shove the rest out of its way.
        let _ = table.set_card_pos(card, px(node.left) as i32, px(node.top) as i32);
        table.separate(home, TableNode::Card(card));
        trace.push(format!(
            "drag-end: {card_name} repositioned within [{home_label}] (no pile change)"
        ));
        return;
    }
    // Structured (List/Grid): reorder among the *contents* only (never above a zone card) into the nearest
    // footprint-aware slot.
    let drop = Pos {
        x: px(node.left) as i32,
        y: px(node.top) as i32,
    };
    let nearest = table
        .structured_positions(
            home,
            GAP,
            GAP,
            Pos {
                x: CARD_W,
                y: CARD_H,
            },
        )
        .into_iter()
        .filter_map(|(n, p)| n.card().map(|c| (c, p)))
        .min_by_key(|(_, p)| {
            let (dx, dy) = ((p.x - drop.x) as i64, (p.y - drop.y) as i64);
            dx * dx + dy * dy
        })
        .map(|(c, _)| c);
    if let (Some(from), Some(to)) = (
        table.card_index(card),
        nearest.and_then(|c| table.card_index(c)),
    ) {
        let _ = table.reorder(home, from, to);
    }
    trace.push(format!(
        "drag-end: {card_name} reordered within [{home_label}] (no pile change - snapped back)"
    ));
}

/// On release, settle a dragged felt element by dispatching to the applicable context resolver: a **pile**
/// repositions and shoves among its siblings; a **card** is routed by context — the location map (march), a
/// scene's assignment rows (formation), a projection (the inn equip), else it settles in its home pile. Each
/// game-move context records a [`DropRequest`] for the driver to interpret and rebuilds; the home-settle
/// path mutates the board directly and lets the others *slide* into place ([`animate_nodes`]).
#[allow(clippy::too_many_arguments)]
fn on_node_drag_end(
    mut on: On<Pointer<DragEnd>>,
    movables: Query<(&Movable, &Node)>,
    geom: Query<(&Movable, &ComputedNode, &UiGlobalTransform)>,
    drop_zones: Query<(&PileDropZone, &ComputedNode, &UiGlobalTransform)>,
    scene: Res<SceneState>,
    mut table: ResMut<Table>,
    mut dragging: ResMut<Dragging>,
    mut rebuild: ResMut<NeedsRebuild>,
    mut guard: ResMut<DragGuard>,
    mut drop_request: ResMut<DropRequest>,
    mut trace: ResMut<DropTrace>,
    mut commands: Commands,
) {
    guard.0 = None; // the drag is over; let real clicks through again
    if let Ok((movable, node)) = movables.get(on.event().entity) {
        on.propagate(false);
        dragging.0 = None;
        // Set the tile back down onto the felt: drop the held-layer lift so it stacks normally again. (A
        // card-path drop rebuilds and respawns this tile anyway; the pile path doesn't, so remove it here.)
        commands.entity(on.event().entity).remove::<GlobalZIndex>();
        // A pile just repositions and shoves among its siblings; the rest is card-only leaf behaviour.
        let card = match movable.0 {
            TableNode::Pile(pid) => {
                let _ = table
                    .0
                    .set_pile_pos(pid, px(node.left) as i32, px(node.top) as i32);
                let parent = table
                    .0
                    .pile(pid)
                    .and_then(|p| p.parent())
                    .unwrap_or(table.0.root_id());
                table.0.separate(parent, TableNode::Pile(pid));
                return;
            }
            TableNode::Card(cid) => cid,
        };
        let entity = on.event().entity;
        // Map (Locations drilled into): a hero token dropped onto a place marches there. The token
        // cursor-follows and occludes picking, so the destination is the one valid place its **box** overlaps.
        if top_deck(&table.0, "Locations") == Some(table.0.focus_id())
            && table.0.card(card).is_some_and(|c| c.card_type() == "hero")
        {
            let drag_box = geom.get(entity).ok().map(|(_, cn, gt)| node_box(cn, gt));
            if let Some(dest) =
                drag_box.and_then(|db| dropped_map_place(&table.0, card, db, &drop_zones))
            {
                drop_request.0 = Some((card, DropTarget::Pile(dest)));
            }
            rebuild.0 = true;
            return;
        }
        // Scene assignment rows (a formation): drop the tile into the nearest row's pile.
        if let Some(scene) = &scene.0
            && let SceneBody::Rows(rows) = &scene.body
        {
            let center = geom.get(entity).ok().map(|(_, cn, gt)| node_box(cn, gt).0);
            if let Some(dest) = center.and_then(|c| nearest_row_pile(rows, c, &drop_zones)) {
                drop_request.0 = Some((card, DropTarget::Pile(dest)));
            }
            rebuild.0 = true;
            return;
        }
        // Projection (the inn): a hero/kit dropped onto its counterpart equips. The target is the projected
        // card the release cursor landed on (the dragged tile occludes the pick, so hit-test by geometry).
        if table
            .0
            .pile(table.0.focus_id())
            .is_some_and(|p| !p.projection().is_empty())
        {
            let cursor = on.event().pointer_location.position;
            if let Some(target) = projected_card_under_cursor(cursor, card, &geom) {
                drop_request.0 = Some((card, DropTarget::Card(target)));
            }
            rebuild.0 = true; // snap the dragged card back to its projected slot (or show the new deck)
            return;
        }
        // No game path applied: the card stays in its home pile (reposition / reorder), traced.
        settle_card_home(&mut table.0, card, node, &mut trace.0);
    }
}

/// The fill colour a [`Utility`] card wears (its card background), by what it does — so it reads as a
/// coloured button even as an ordinary card in the System deck.
fn action_color(utility: Utility) -> Color {
    match utility {
        Utility::Exit => EXIT_CONFIRM_BG, // warm red — "this is the way out"
        Utility::StartOver => Color::srgb(0.62, 0.44, 0.24), // amber — a bigger, permanent wipe
    }
}

/// Keep a **Free** deck's cards shoved apart when they first lay out or change size (a card expands or
/// collapses): when a card's footprint changes and nothing is being dragged, re-run [`separate`] anchored
/// on the changed card, so a grown card holds its place and pushes its neighbours out. `prev` remembers
/// the last-seen footprints.
fn settle_free_cards(
    mut table: ResMut<Table>,
    dragging: Res<Dragging>,
    mut prev: Local<HashMap<CardId, Pos>>,
) {
    if dragging.0.is_some() {
        return;
    }
    let focus = table.0.focus_id();
    if !matches!(
        table.0.pile(focus).map(|p| p.layout().arrangement),
        Some(Arrangement::Free)
    ) {
        return;
    }
    let cards: Vec<CardId> = table.0.content_cards(focus).to_vec();
    let mut anchor: Option<CardId> = None;
    for &c in &cards {
        let Some(footprint) = table.0.card(c).map(|k| k.footprint()) else {
            continue;
        };
        if footprint.x < 1 {
            continue; // not laid out yet
        }
        let was = prev.insert(c, footprint).unwrap_or_default();
        if was.x != footprint.x || was.y != footprint.y {
            anchor = Some(c);
        }
    }
    if let Some(anchor) = anchor {
        table.0.separate(focus, TableNode::Card(anchor));
    }
}

/// Keep the **Table's top-level piles** shoved apart when one first lays out or changes size, or when the
/// window (surface) resizes — the pile counterpart of [`settle_free_cards`]. When a pile's size changes (a
/// brand-new character-reflection deck appearing, or a deck growing), or the surface bounds move, and
/// nothing is being dragged, re-run [`Board::separate`] so every pile is re-clamped inside the surface
/// and pushed clear of its neighbours. A size-changed pile anchors the shove (the newcomer holds its
/// spot); a bare resize anchors the first pile. This is what makes both a freshly-rendered deck and a
/// window resize trigger the shove without hooking each site. `prev`/`prev_bounds` remember last-seen
/// sizes; only runs at the Table (root), where these piles are shown and sized.
fn settle_table_piles(
    mut table: ResMut<Table>,
    guard: Res<DragGuard>,
    mut tidy: ResMut<DecksNeedTidy>,
    mut prev: Local<HashMap<PileId, Pos>>,
    mut prev_bounds: Local<Pos>,
) {
    if guard.0.is_some() {
        return; // a drag is in progress — don't fight it
    }
    let root = table.0.root_id();
    if table.0.focus_id() != root {
        return; // top-level piles are only shown (and sized) at the Table
    }
    // Wait for a real, finite bounds before laying anything out. Until the renderer reports the felt size the
    // model holds an effectively-unbounded default (a huge sentinel width), and laying out against it puts
    // every deck in one un-wrappable row - which the first real bounds then has to shove off its edges into a
    // pile. Skipping until the width is a plausible screen size means the *first* tidy happens at the true
    // width, so `arrange_row` wraps the decks to fit from the start instead of correcting after the fact.
    let bounds = table.0.bounds();
    if !(1..=100_000).contains(&bounds.x) {
        return;
    }
    // The board was replaced wholesale (startup / Start Over / a load): its decks sit at the fixtures' raw
    // seed positions and must be laid out from scratch. Forget the old board's sizes so the diff below can't
    // mistake the fresh decks for unchanged ones - a new board reuses the same ids *and*, now that footprints
    // are computed rather than measured, the same sizes, which is exactly how Start Over used to leave every
    // deck at its seed. Consumed here, once the bounds are real, so the tidy happens at the true width.
    let rebuilt = std::mem::take(&mut tidy.0);
    if rebuilt {
        prev.clear();
    }
    let piles: Vec<PileId> = table
        .0
        .pile(root)
        .map(|p| p.subpiles().to_vec())
        .unwrap_or_default();
    let mut sized = false;
    for &p in &piles {
        // The model computes each deck-chip footprint (from its card count), so a pile's size is known
        // without a render - a new/grown deck changes this immediately, which is what triggers the re-tidy.
        let size = table.0.pile_footprint(p);
        let was = prev.insert(p, size).unwrap_or_default();
        if was.x != size.x || was.y != size.y {
            sized = true;
        }
    }
    // Track a **width** change only — the bounds *height* also flips as you enter/leave a zone's
    // overlay-band inset (the root has none; a structured zone insets by `OVERLAY_BAND`), so keying on
    // height would mistake every navigation back to the Table for a resize.
    let resized = bounds.x != prev_bounds.x;
    if resized {
        *prev_bounds = bounds;
    }
    // A fresh board, or a deck first sizing (or its chip changing size), lays the decks out as one clean
    // constant-gap row. A window *resize*, by contrast, does NOT re-tidy — it just **bumps decks off the new
    // edges**: `separate` clamps any that now fall outside back inside and de-overlaps, preserving the manual
    // arrangement (decks that still fit don't move). Between these events a manual drag sticks.
    if rebuilt || sized {
        table.0.arrange_row(root, GAP, OVERLAY_BAND);
    } else if resized && let Some(anchor) = piles.first().copied() {
        table.0.separate(root, TableNode::Pile(anchor));
    }
}

/// Rebuild the whole UI only on a *structural* change (open/close a pile, move a card, a new game
/// snapshot). Pile positions are not structural — they animate (see [`animate_nodes`]) — so
/// repositioning never triggers a rebuild.
#[allow(clippy::too_many_arguments)] // a Bevy draw system — its inputs are resources, not a god-param
fn redraw(
    mut commands: Commands,
    mut rebuild: ResMut<NeedsRebuild>,
    table: Res<Table>,
    rail: Res<ActionRail>,
    front: Res<FannedFront>,
    affordances: Res<AffordanceLabels>,
    scene: Res<SceneState>,
    history: Res<crate::board_driver::BoardHistory>,
    font_sample: Res<FontSample>,
    ui_fonts: Option<Res<UiFonts>>,
    roots: Query<Entity, With<CardTableRoot>>,
) {
    if !rebuild.0 {
        return;
    }
    rebuild.0 = false;
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    // The font sample grid is modal: while a font is selected, it is the whole felt.
    if let (Some(name), Some(fonts)) = (font_sample.0.as_deref(), ui_fonts.as_deref()) {
        build_font_sample(&mut commands, name, fonts);
        return;
    }
    // A game **scene** (a combat arena, etc.) is modal: the game declares it, the renderer draws it blind.
    if let Some(scene) = &scene.0 {
        draw_scene(&mut commands, scene, &affordances.0, history.can_undo());
        return;
    }
    build_ui(&mut commands, &table.0, &rail.0, front.0, &affordances.0);
}

/// Build the modal **font sample**: the whole [`palette`] rendered large in the chosen font, in a neat grid,
/// with a Back card to close. The glyphs are guaranteed drawable by the `fonts_cover_palette` test, so no cell
/// is ever tofu.
fn build_font_sample(commands: &mut Commands, name: &str, fonts: &UiFonts) {
    let handle = fonts
        .0
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, h)| h.clone())
        .unwrap_or_default();
    let chars = palette::available();
    commands
        .spawn((
            CardTableRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(FELT),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new(name.to_string()),
                TextFont {
                    font: handle.clone().into(),
                    font_size: FONT_HEAD,
                    ..default()
                },
                TextColor(INK),
            ));
            spawn_prompt_line(
                root,
                &format!("{} characters available in this font", chars.len()),
            );
            // A scrollable grid of the palette, one cell per glyph, drawn in the selected font.
            root.spawn(Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                align_content: AlignContent::FlexStart,
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(6.0),
                row_gap: Val::Px(6.0),
                padding: UiRect::bottom(Val::Px(56.0)),
                overflow: Overflow::scroll_y(),
                ..default()
            })
            .with_children(|grid| {
                for c in chars {
                    spawn_glyph_cell(grid, c, handle.clone());
                }
            });
            // Footer: a Back card pinned to the bottom, closing the sample.
            root.spawn(Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(8.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                padding: UiRect::vertical(Val::Px(6.0)),
                ..default()
            })
            .insert(BackgroundColor(FELT))
            .with_children(|row| spawn_nav_card(row, (BackCard, Pinned), "Back"));
        });
}

/// One glyph cell in the font sample: the character drawn large (in `handle`) over its `U+XXXX` codepoint.
fn spawn_glyph_cell(parent: &mut ChildSpawnerCommands, c: char, handle: Handle<Font>) {
    parent
        .spawn((
            Node {
                width: Val::Px(56.0),
                height: Val::Px(56.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(PANEL),
            BorderColor::all(MUTED),
        ))
        .with_children(|cell| {
            cell.spawn((
                Text::new(if c == ' ' {
                    "space".to_string()
                } else {
                    c.to_string()
                }),
                TextFont {
                    font: handle.into(),
                    font_size: if c == ' ' {
                        FONT_BADGE
                    } else {
                        FontSize::Px(26.0)
                    },
                    ..default()
                },
                TextColor(INK),
            ));
            cell.spawn((
                Text::new(format!("{:04X}", c as u32)),
                TextFont {
                    font_size: FONT_BADGE,
                    ..default()
                },
                TextColor(MUTED),
            ));
        });
}

// ---- generic scene drawers (labels, tracks, dividers, panels) -----------

/// A small muted text label at the left of a row, over its hint (what the row is *for*) if it carries one.
/// The hint gets the width — a row you drag things into has to say what dragging them there is *good for*, and
/// a name alone ("Outrider") cannot.
fn spawn_row_label(parent: &mut ChildSpawnerCommands, name: &str, hint: &str) {
    let width = if hint.is_empty() { 84.0 } else { 240.0 };
    parent
        .spawn(Node {
            width: Val::Px(width),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            ..default()
        })
        .with_children(|c| {
            c.spawn((
                Text::new(name.to_string()),
                TextFont {
                    font_size: FONT_BODY,
                    ..default()
                },
                TextColor(INK),
            ));
            if !hint.is_empty() {
                c.spawn((
                    Text::new(hint.to_string()),
                    TextFont {
                        font_size: FONT_BADGE,
                        ..default()
                    },
                    TextColor(MUTED),
                ));
            }
        });
}

/// A **row-header card** — a fixed-width labeled card anchoring the left of a lane, so lanes read as neat
/// left-aligned cards with their tiles laid out to the right (mirroring a labeled pile on the table).
fn spawn_lane_label(parent: &mut ChildSpawnerCommands, name: &str) {
    parent
        .spawn((
            Node {
                width: Val::Px(96.0),
                min_height: Val::Px(SMALL_H),
                padding: UiRect::all(Val::Px(6.0)),
                border: UiRect::all(Val::Px(2.0)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(PANEL),
            BorderColor::all(FACE_DOWN_EDGE),
            card_shadow(),
        ))
        .with_children(|c| {
            c.spawn((
                Text::new(name.to_string()),
                TextFont {
                    font_size: FONT_BODY,
                    ..default()
                },
                TextColor(INK),
            ));
        });
}

/// The centre divider between the party (left) and the foes (right) in a lane.
fn spawn_divider(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        Node {
            width: Val::Px(2.0),
            height: Val::Px(SMALL_H),
            margin: UiRect::horizontal(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(MUTED),
    ));
}

/// A **progress-track card**: a titled vertical list of step labels (`order`) with the `current` one
/// highlighted, so how far along a sequence is reads at a glance. The game supplies the labels and which is
/// current; the renderer just draws the list.
fn spawn_track(parent: &mut ChildSpawnerCommands, title: &str, order: &[&str], current: &str) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                min_width: Val::Px(128.0),
                padding: UiRect::all(Val::Px(6.0)),
                row_gap: Val::Px(2.0),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(PANEL),
            BorderColor::all(FACE_DOWN_EDGE),
            card_shadow(),
        ))
        .with_children(|deck| {
            deck.spawn((
                Text::new(title.to_string()),
                TextFont {
                    font_size: FONT_BADGE,
                    ..default()
                },
                TextColor(MUTED),
            ));
            for name in order {
                let is_current = *name == current;
                deck.spawn((
                    Node {
                        padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(if is_current { TARGET_CUE } else { Color::NONE }),
                ))
                .with_children(|row| {
                    row.spawn((
                        Text::new(name.to_string()),
                        TextFont {
                            font_size: FONT_BODY,
                            ..default()
                        },
                        TextLayout::no_wrap(),
                        TextColor(if is_current { CARD_INK } else { MUTED }),
                    ));
                });
            }
        });
}

/// A **text log panel**: a large card under the body listing the scene's log lines. Un-indented lines are
/// section headers (bright); leading-space lines are entries (muted). The game supplies every line.
/// The **decision row**: one small card per [`Choice`], sitting just above the log that explains it.
///
/// Each card carries its own consequence, because a bare label ("Strike Back") names an action while what the
/// player needs is what it will *do to them* ("spend 1 tempo, deal 7 back"). A **barred** option is still
/// drawn — inert, and showing *why* it cannot be taken — so "why can I not do that?" is answerable by looking.
/// Silently omitting it teaches nothing and reads as a bug. The staged option wears a bright ring.
fn spawn_choice_row(parent: &mut ChildSpawnerCommands, choices: &[Choice]) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(10.0),
            margin: UiRect::top(Val::Px(8.0)),
            ..default()
        })
        .with_children(|row| {
            for (i, choice) in choices.iter().enumerate() {
                let enabled = choice.enabled();
                let (bg, ink) = if enabled {
                    (BUTTON, INK)
                } else {
                    (PANEL, MUTED) // barred: present, inert, and explaining itself
                };
                let mut card = row.spawn((
                    Node {
                        width: Val::Px(190.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(10.0)),
                        row_gap: Val::Px(4.0),
                        border: UiRect::all(Val::Px(2.0)),
                        border_radius: BorderRadius::all(Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(bg),
                    // The staged option is ringed, so what will happen on Commit is visible at a glance.
                    BorderColor::all(if choice.chosen {
                        TARGET_CUE
                    } else {
                        Color::NONE
                    }),
                    card_shadow(),
                ));
                if enabled {
                    card.insert(ChoiceControl(i));
                }
                card.with_children(|c| {
                    c.spawn((
                        Text::new(choice.label.clone()),
                        TextFont {
                            font_size: FONT_TITLE,
                            ..default()
                        },
                        TextColor(ink),
                    ));
                    // The consequence when it can be taken; the reason when it cannot.
                    let line = if enabled {
                        choice.consequence.clone()
                    } else {
                        choice.why_not.clone()
                    };
                    if !line.is_empty() {
                        c.spawn((
                            Text::new(line),
                            TextFont {
                                font_size: FONT_BODY,
                                ..default()
                            },
                            TextColor(if enabled { INK } else { WARN_CUE }),
                        ));
                    }
                });
            }
        });
}

fn spawn_log_panel(parent: &mut ChildSpawnerCommands, title: &str, lines: &[String]) {
    parent
        .spawn((
            Node {
                width: Val::Px(720.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                margin: UiRect::top(Val::Px(6.0)),
                row_gap: Val::Px(2.0),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(PANEL),
            BorderColor::all(MUTED),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new(if title.is_empty() { "Log" } else { title }.to_string()),
                TextFont {
                    font_size: FONT_TITLE,
                    ..default()
                },
                TextColor(INK),
            ));
            for line in lines {
                let header = !line.starts_with(' ');
                panel.spawn((
                    Text::new(line.clone()),
                    TextFont {
                        font_size: FONT_BODY,
                        ..default()
                    },
                    TextColor(if header { INK } else { MUTED }),
                ));
            }
        });
}

/// The **legend card** — a standing reference in the sidebar for the abbreviations the tiles are forced to
/// use. Same text convention as the log: un-indented lines are headers, leading-space lines are entries.
fn spawn_legend_panel(parent: &mut ChildSpawnerCommands, lines: &[String]) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(10.0)),
                row_gap: Val::Px(2.0),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(PANEL),
            BorderColor::all(MUTED),
        ))
        .with_children(|panel| {
            for line in lines {
                let header = !line.starts_with(' ');
                panel.spawn((
                    Text::new(line.clone()),
                    TextFont {
                        font_size: if header { FONT_BODY } else { FONT_BADGE },
                        ..default()
                    },
                    TextColor(if header { INK } else { MUTED }),
                ));
            }
        });
}

/// A muted instruction line on the felt (per-step prompt / contacts summary).
fn spawn_prompt_line(parent: &mut ChildSpawnerCommands, text: &str) {
    parent.spawn((
        Text::new(text.to_string()),
        TextFont {
            font_size: FONT_BODY,
            ..default()
        },
        TextColor(MUTED),
    ));
}

/// A greyed, non-interactive nav card — a control that is present but not yet live (the disabled Start while
/// heroes remain unranked). Carries no marker, so a click does nothing.
fn spawn_disabled_nav(parent: &mut ChildSpawnerCommands, label: &str) {
    parent
        .spawn((
            Node {
                padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(PANEL),
            BorderColor::all(MUTED),
        ))
        .with_children(|c| {
            c.spawn((
                Text::new(label.to_string()),
                TextFont {
                    font_size: FONT_TITLE,
                    ..default()
                },
                TextColor(MUTED),
            ));
        });
}

// ---- drawing a game Scene (a rules-blind modal: tracks, tiles, rows/lanes, links, log) ----------------

/// Draw a game-declared [`Scene`] in place of the felt: progress tracks down the left, a heading + prompt,
/// the body (assignment rows or two-sided lanes) in a scroll region, an optional log, and the zone's
/// affordance controls pinned to the footer. The renderer draws this **without knowing what any of it means**
/// — the game decided every tile, badge, highlight and link. (Links are drawn separately by
/// [`animate_target_arrows`], which reads the same [`SceneState`].)
fn draw_scene(commands: &mut Commands, scene: &Scene, affordances: &[String], can_undo: bool) {
    commands
        .spawn((
            CardTableRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Stretch,
                ..default()
            },
            BackgroundColor(FELT),
        ))
        .with_children(|root| {
            // Left sidebar: the progress tracks, stacked (fixed order, current highlighted).
            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|side| {
                for track in &scene.tracks {
                    let labels: Vec<&str> = track.items.iter().map(|i| i.label.as_str()).collect();
                    let current = track
                        .items
                        .iter()
                        .find(|i| i.current)
                        .map(|i| i.label.as_str())
                        .unwrap_or("");
                    spawn_track(side, &track.title, &labels, current);
                }
                // The legend sits under the tracks: always on screen, and never competing for room with the
                // body, the decision or the log.
                if !scene.legend.is_empty() {
                    spawn_legend_panel(side, &scene.legend);
                }
            });

            // Main column: heading, prompt, then the body + log (fill + scroll). Bottom padding keeps the
            // last row clear of the pinned footer bar.
            root.spawn(Node {
                flex_grow: 1.0,
                min_width: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|main| {
                if !scene.heading.is_empty() {
                    main.spawn((
                        Text::new(scene.heading.clone()),
                        TextFont {
                            font_size: FONT_HEAD,
                            ..default()
                        },
                        TextColor(INK),
                    ));
                }
                if !scene.prompt.is_empty() {
                    spawn_prompt_line(main, &scene.prompt);
                }
                // The body **takes the leftover space and clips**. Only the body: the decision and the log
                // that explains it must never be pushed out of sight by a tall formation, which is exactly
                // what happened when they lived in here — Bevy's `Overflow::scroll_y` only *clips*, it does
                // not scroll (nothing drives a `ScrollPosition` on this node), so anything below the fold was
                // not merely out of view, it was unreachable. A player could be asked a question they could
                // not see.
                // The body takes only the room it needs, and clips if it needs more. It used to *grow* to fill
                // the column, which shoved everything after it to the bottom of the screen - so the reading
                // order ran top, then a gulf, then bottom, and the log and the decision were as far from the
                // board as the layout could put them.
                main.spawn(Node {
                    width: Val::Percent(100.0),
                    flex_grow: 0.0,
                    flex_shrink: 1.0,
                    min_height: Val::Px(0.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::FlexStart,
                    row_gap: Val::Px(8.0),
                    overflow: Overflow::clip(),
                    ..default()
                })
                .with_children(|mid| match &scene.body {
                    SceneBody::Rows(rows) => draw_scene_rows(mid, rows),
                    SceneBody::Lanes(lanes) => draw_scene_lanes(mid, lanes),
                });

                // **The log, then the decision, then the controls** - the order you read them in. The log is
                // what the decision is *about* ("The Wall reaches you, slipping costs 2"), so it cannot come
                // second: you would be choosing before reading the thing that tells you how. And the controls
                // sit with them rather than pinned to the far bottom of the viewport, because Commit is the
                // last step of that same thought, not a separate piece of furniture.
                main.spawn(Node {
                    width: Val::Percent(100.0),
                    flex_shrink: 0.0,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|panel| {
                    if !scene.log.is_empty() {
                        spawn_log_panel(panel, &scene.log_title, &scene.log);
                    }
                    if !scene.choices.is_empty() {
                        spawn_choice_row(panel, &scene.choices);
                    }
                    // The zone's affordances. A control the scene marks disabled is drawn inert.
                    panel
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::Center,
                            column_gap: Val::Px(10.0),
                            padding: UiRect::vertical(Val::Px(6.0)),
                            ..default()
                        })
                        .with_children(|row| {
                            // **Back** - rewind one move. It sits with the scene's own controls but is the
                            // renderer's, not the game's: the board is the whole state, so stepping back is
                            // just restoring the previous board, and nothing here needs to know what the move
                            // meant. Keep pressing and you walk back out of the fight entirely, onto the
                            // location you opened it from.
                            if can_undo {
                                spawn_nav_card(row, (UndoControl, Pinned), "Back");
                            }
                            for (i, label) in affordances.iter().enumerate() {
                                if scene.disabled_controls.contains(&i) {
                                    spawn_disabled_nav(row, label);
                                } else {
                                    spawn_nav_card(row, (AffordanceControl(i), Pinned), label);
                                }
                            }
                        });
                });
            });
        });
}

/// **Assignment rows** (a formation being arranged): each row is a full-width `PileDropZone` over its pile, a
/// label, then its tiles. Dragging a `draggable` tile into any row's drop zone moves the card there.
fn draw_scene_rows(root: &mut ChildSpawnerCommands, rows: &[Row]) {
    for row in rows {
        root.spawn((
            PileDropZone(row.drop_pile),
            Node {
                position_type: PositionType::Relative,
                width: Val::Px(720.0),
                min_height: Val::Px(SMALL_H + 12.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                padding: UiRect::all(Val::Px(6.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(PANEL),
            BorderColor::all(MUTED),
        ))
        .with_children(|r| {
            spawn_row_label(r, &row.label, &row.hint);
            for tile in &row.tiles {
                draw_scene_tile(r, tile);
            }
        });
    }
}

/// **Two-sided lanes** (a face-off): one row per lane — a label, the left group in a fixed-width cell (so the
/// divider lines up across lanes), the divider, then the right group.
fn draw_scene_lanes(root: &mut ChildSpawnerCommands, lanes: &[Lane]) {
    // Fix the left cell to the widest lane's left count so the divider aligns vertically across lanes.
    let max_left = lanes.iter().map(|l| l.left.len()).max().unwrap_or(0);
    let left_w = if max_left == 0 {
        0.0
    } else {
        max_left as f32 * SMALL_W + (max_left - 1) as f32 * 8.0
    };
    root.spawn(Node {
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::FlexStart,
        row_gap: Val::Px(6.0),
        ..default()
    })
    .with_children(|col| {
        for lane in lanes {
            col.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                min_height: Val::Px(SMALL_H + 8.0),
                ..default()
            })
            .with_children(|ln| {
                spawn_lane_label(ln, &lane.label);
                ln.spawn(Node {
                    width: Val::Px(left_w),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|left| {
                    for tile in &lane.left {
                        draw_scene_tile(left, tile);
                    }
                });
                spawn_divider(ln);
                for tile in &lane.right {
                    draw_scene_tile(ln, tile);
                }
            });
        }
    });
}

/// The (background, border, ink, border-width) look for a tile's [`Highlight`] and [`Team`]. This is where a
/// neutral emphasis level becomes a concrete ring/dim — the renderer's presentation choice, not the game's.
fn tile_look(highlight: Highlight, team: Team) -> (Color, Color, Color, f32) {
    match (highlight, team) {
        (Highlight::Spent, _) => (CARD_BACK, MUTED, MUTED, 2.0),
        (Highlight::Active, Team::Left) => (CARD_FACE, ARMED_CUE, CARD_INK, 3.0),
        (Highlight::Active, Team::Right) => (CARD_FACE, TARGET_CUE, CARD_INK, 3.0),
        (Highlight::Available, Team::Left) => (CARD_FACE, SELECTABLE_CUE, CARD_INK, 2.0),
        (Highlight::Available, Team::Right) => (CARD_FACE, TARGET_CUE, CARD_INK, 2.0),
        (Highlight::Dim, _) => (DIM_FACE, MUTED, MUTED, 2.0),
        (Highlight::Idle, Team::Left) => (CARD_FACE, type_accent("hero"), CARD_INK, 2.0),
        (Highlight::Idle, Team::Right) => (CARD_BACK, type_accent("foe"), INK, 2.0),
    }
}

/// A [`Tone`] resolved to a palette color (the game's neutral emphasis/hue -> a concrete color).
fn tone_color(tone: Tone) -> Color {
    match tone {
        Tone::Muted => MUTED,
        Tone::Warn => WARN_CUE,
        Tone::Good => TARGET_CUE,
        Tone::Cool => COOL_ACCENT,
        Tone::Warm => WARM_ACCENT,
        Tone::Faded => FACE_DOWN_EDGE,
    }
}

/// One [`Scene`] tile: a card with a title, badge lines, a highlight ring, and its input verbs. Tagged
/// [`TileCard`] when tappable (a tap routes to the game's `tap_intention`) and [`Movable`] when
/// draggable (a drag drops into a row's pile). The renderer draws it; the game gave it all its meaning.
fn draw_scene_tile(parent: &mut ChildSpawnerCommands, tile: &Tile) {
    let (bg, border, ink, border_w) = tile_look(tile.highlight, tile.team);
    let mut node = parent.spawn((
        Node {
            width: Val::Px(SMALL_W),
            min_height: Val::Px(SMALL_H),
            padding: UiRect::all(Val::Px(8.0)),
            border: UiRect::all(Val::Px(border_w)),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(3.0),
            border_radius: BorderRadius::all(Val::Px(10.0)),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(bg),
        BorderColor::all(border),
        card_shadow(),
    ));
    if tile.tappable {
        node.insert(TileCard(tile.card));
    }
    if tile.draggable {
        // Movable so the drag pipeline picks it up; ModalTile so animate_nodes leaves its (flex-owned)
        // position alone.
        node.insert((Movable(TableNode::Card(tile.card)), ModalTile));
    }
    node.with_children(|c| {
        c.spawn((
            Text::new(tile.title.clone()),
            TextFont {
                font_size: title_font(&tile.title, FONT_TITLE, SMALL_INNER),
                ..default()
            },
            TextLayout::no_wrap(),
            TextColor(ink),
        ));
        // The first badge is the stat bar (body font); the rest are compact cues (badge font).
        for (i, badge) in tile.badges.iter().enumerate() {
            c.spawn((
                Text::new(badge.text.clone()),
                TextFont {
                    font_size: if i == 0 { FONT_BODY } else { FONT_BADGE },
                    ..default()
                },
                TextColor(tone_color(badge.tone)),
            ));
        }
    });
}

/// Rebuild [`CardScreenRects`] from the current UI layout — **the one place** the physical→logical
/// conversion lives. `UiGlobalTransform.translation` is a node's *physical*-pixel centre; `ComputedNode`
/// carries its size and the `inverse_scale_factor` that maps both to logical pixels (what `Val::Px` uses).
/// Covers table cards (`CardRef`) and scene tiles (`TileCard`); rebuilt fresh each frame so despawned
/// cards drop out.
fn track_card_rects(
    mut rects: ResMut<CardScreenRects>,
    cards: Query<(&CardRef, &ComputedNode, &UiGlobalTransform)>,
    units: Query<(&TileCard, &ComputedNode, &UiGlobalTransform)>,
) {
    rects.0.clear();
    let mut put = |id: CardId, cn: &ComputedNode, gt: &UiGlobalTransform| {
        rects.0.insert(id, node_rect(cn, gt));
    };
    for (c, cn, gt) in &cards {
        put(c.0, cn, gt);
    }
    for (u, cn, gt) in &units {
        put(u.0, cn, gt);
    }
}

/// Draw the **attention arrows** — the game-declared card-to-card links ([`Scene`] links) made visible as a
/// flowing line of dots from each `from` card to its `to` card. Endpoints come from [`CardScreenRects`] (the
/// single position authority) — never re-derived here. The renderer knows only "these two cards are linked
/// right now" (the game reads the link as combat targeting; the renderer does not). Dots are transient
/// top-level overlay nodes re-spawned each frame so they animate toward the target; they sit above the felt
/// (`GlobalZIndex`) and never eat clicks. Presentation-only — wall-clock drives the flow, never the rules.
fn animate_target_arrows(
    mut commands: Commands,
    time: Res<Time>,
    rects: Res<CardScreenRects>,
    scene: Res<SceneState>,
    dots: Query<Entity, With<ArrowDot>>,
) {
    for e in &dots {
        commands.entity(e).despawn(); // clear last frame's flow
    }
    let Some(scene) = &scene.0 else {
        return; // no modal scene -> no links to draw
    };
    let phase = time.elapsed_secs();
    for link in &scene.links {
        if let (Some(a), Some(b)) = (rects.center(link.from), rects.center(link.to)) {
            spawn_arrow_dots(&mut commands, a, b, link.confirmed, link.broad, phase);
        }
    }
}

/// Spawn one arrow's worth of dots from `a` to `b` (tile centers, logical px), flowing toward `b`. Confirmed
/// arrows are denser and green; possible ones sparser and amber. Dots grow toward the target as a soft head.
/// An `area` source fans three parallel threads into a broad sweep; every dot carries a dark rim so it reads
/// even when its fill matches the card it flows over.
fn spawn_arrow_dots(
    commands: &mut Commands,
    a: Vec2,
    b: Vec2,
    confirmed: bool,
    area: bool,
    phase: f32,
) {
    let dir = b - a;
    let len = dir.length();
    if len < 72.0 {
        return; // tiles adjacent/overlapping - nothing useful to draw
    }
    let unit = dir / len;
    let perp = Vec2::new(-unit.y, unit.x); // sideways - the offset for an area strike's fanned threads
    // Start clear of the source tile and stop short of the target so the dots read as a gap-spanning arrow.
    let start = a + unit * 36.0;
    let end = b - unit * 36.0;
    let span = (end - start).length();
    let spacing = if confirmed { 11.0 } else { 17.0 };
    let flow = (phase * 42.0).rem_euclid(spacing); // px/sec toward the target
    let color = if confirmed {
        TARGET_CUE
    } else {
        SELECTABLE_CUE
    };
    // A dark rim keeps each dot legible on any background: where its fill matches the card the rim shows,
    // where the card is dark the bright fill shows.
    let rim = Color::srgba(0.0, 0.0, 0.0, 0.8);
    // A single strike is one thread; an area strike fans three parallel threads into a broad band.
    let lanes: &[f32] = if area { &[-7.0, 0.0, 7.0] } else { &[0.0] };
    for &off in lanes {
        let mut d = flow;
        while d <= span {
            let t = d / span;
            let size = (if confirmed { 5.5 } else { 4.5 }) * (0.6 + 0.9 * t); // grow into a head near the target
            let p = start + unit * d + perp * off;
            commands.spawn((
                ArrowDot,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(p.x - size * 0.5),
                    top: Val::Px(p.y - size * 0.5),
                    width: Val::Px(size),
                    height: Val::Px(size),
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Percent(50.0)),
                    ..default()
                },
                BackgroundColor(color),
                BorderColor::all(rim),
                GlobalZIndex(1000),
                Pickable {
                    should_block_lower: false,
                    is_hoverable: false,
                },
            ));
            d += spacing;
        }
    }
}

// ---- drawing ------------------------------------------------------------

const FELT: Color = Color::srgb(0.06, 0.13, 0.10);
const PANEL: Color = Color::srgb(0.10, 0.18, 0.15);
const INK: Color = Color::srgb(0.92, 0.95, 0.93);
const MUTED: Color = Color::srgb(0.66, 0.72, 0.68);
const BUTTON: Color = Color::srgb(0.18, 0.40, 0.60);
const CARD_FACE: Color = Color::srgb(0.94, 0.92, 0.84);
const CARD_INK: Color = Color::srgb(0.10, 0.10, 0.13);
const CARD_BACK: Color = Color::srgb(0.20, 0.24, 0.42);
/// A second back shade so alternating layers in a pile's stack read as distinct cards.
const CARD_BACK_ALT: Color = Color::srgb(0.28, 0.32, 0.52);
/// The accent (border + foot bar) marking a **face-down** card — a warm amber against the dark back, the
/// unmistakable "this is face down" cue that still leaves the name readable.
const FACE_DOWN_EDGE: Color = Color::srgb(0.72, 0.56, 0.30);
/// The Exit card's fill — a warm red so it reads as "this is the way out".
const EXIT_CONFIRM_BG: Color = Color::srgb(0.55, 0.22, 0.20);
/// Highlight edge for a card/pile that carries a legal move.
const ACTIONABLE: Color = Color::srgb(0.30, 0.70, 0.62);
/// A dark edge around every card so overlapping cards stay distinct.
const CARD_EDGE: Color = Color::srgb(0.12, 0.11, 0.10);
/// Soft drop shadow lifting cards and piles off the felt.
const SHADOW: Color = Color::srgba(0.0, 0.0, 0.0, 0.35);
/// The **movable cue** — a *thin, pale* ring worn by every card whose drag triggers a game action, so
/// they're scannable without shouting. Deliberately subtle (cool off-white, low alpha, 1px — see
/// [`set_outline`]); toggled to [`Color::NONE`] on the card currently held.
const MOVABLE_CUE: Color = Color::srgba(0.86, 0.90, 0.97, 0.50);
/// The **valid-drop-target glow** — a *thicker* ring worn, while a drag is held, by every place the held
/// card can legally land ([`can_drop_on_card`] / [`can_drop_on_pile`]). Bright green so "drop here" reads.
const TARGET_CUE: Color = Color::srgba(0.36, 0.86, 0.42, 0.95);
/// The **`Available`** tile cue (left team) — a tile you can act on this step. Warm amber "available".
const SELECTABLE_CUE: Color = Color::srgb(0.92, 0.74, 0.34);
/// The **`Active`** tile cue (left team) — the current in-progress choice. A bright silver-white ring,
/// distinct from the green (right-team) target cue and the amber switch cue.
const ARMED_CUE: Color = Color::srgb(0.95, 0.96, 1.0);
/// The **`Dim`** tile face — nothing to act on this step; a greyed face that recedes so live tiles stand out.
const DIM_FACE: Color = Color::srgb(0.44, 0.46, 0.44);
/// A **cool-hued** badge accent ([`Tone::Cool`]) — a cool blue.
const COOL_ACCENT: Color = Color::srgb(0.45, 0.72, 0.95);
/// A **warm-hued** badge accent ([`Tone::Warm`]) — a warm steel-orange.
const WARM_ACCENT: Color = Color::srgb(0.95, 0.58, 0.38);
/// A **warning** badge cue ([`Tone::Warn`]) — a soft red.
const WARN_CUE: Color = Color::srgb(0.92, 0.40, 0.40);
/// Corner radius for a cue ring, matching a card's own [`BorderRadius`] so the outline rounds instead of
/// boxing the card — a Bevy outline follows its node's radius, and a bare `Movable` wrapper has none.
const CUE_RADIUS: Val = Val::Px(12.0);

/// The accent colour for a card **type** — a small designed palette for the common types, with a
/// stable hashed hue for any other type so a new type still reads as its own colour.
fn type_accent(card_type: &str) -> Color {
    match card_type.to_ascii_lowercase().as_str() {
        "location" => Color::srgb(0.36, 0.52, 0.34), // mossy green
        "adventurer" => Color::srgb(0.28, 0.46, 0.68), // heroic blue
        "hero" => Color::srgb(0.70, 0.32, 0.32),     // crimson
        "kit" => Color::srgb(0.28, 0.52, 0.52),      // teal
        "ability" => Color::srgb(0.68, 0.36, 0.52),  // magenta
        "item" => Color::srgb(0.74, 0.58, 0.26),     // gold
        "log" => Color::srgb(0.44, 0.44, 0.52),      // slate
        "zone" => Color::srgb(0.50, 0.40, 0.62),     // violet — a structural / naming card
        "encounter" => Color::srgb(0.74, 0.44, 0.22), // burnt orange — a fight to be had
        "foe" => Color::srgb(0.54, 0.24, 0.28), // oxblood — a creature to fight (darker than hero crimson)
        other => hashed_accent(other),
    }
}

/// A stable, pleasant accent colour derived from a type name (FNV-1a hue at fixed saturation/value),
/// so any unlisted type still gets its own consistent colour instead of a shared default.
fn hashed_accent(s: &str) -> Color {
    let mut h: u32 = 0x811c_9dc5;
    for b in s.bytes() {
        h = (h ^ b as u32).wrapping_mul(0x0100_0193);
    }
    hsv_to_rgb((h % 360) as f32, 0.45, 0.62)
}

/// Ink colour that reads on a given badge fill — dark on light fills, light on dark ones.
fn badge_ink(bg: Color) -> Color {
    let c = bg.to_srgba();
    let luminance = 0.299 * c.red + 0.587 * c.green + 0.114 * c.blue;
    if luminance > 0.6 { CARD_INK } else { INK }
}

/// HSV (hue in degrees, saturation and value in `0..=1`) to an sRGB [`Color`].
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Color {
    let c = v * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h as u32 / 60) % 6 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Color::srgb(r + m, g + m, b + m)
}

/// A small colour-filled pill showing a card's **type** — the visual type indicator (colour + label).
/// A no-op for an untyped card (empty type draws no badge).
fn spawn_type_badge(parent: &mut ChildSpawnerCommands, card_type: &str) {
    if card_type.is_empty() {
        return;
    }
    let bg = type_accent(card_type);
    parent
        .spawn((
            Node {
                padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(bg),
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(card_type.to_string()),
                TextFont {
                    font_size: FONT_BADGE,
                    ..default()
                },
                TextColor(badge_ink(bg)),
            ));
        });
}

/// A soft drop shadow used on cards and pile chips (offset down, blurred).
fn card_shadow() -> BoxShadow {
    BoxShadow::new(
        SHADOW,
        Val::Px(0.0),
        Val::Px(3.0),
        Val::Px(0.0),
        Val::Px(6.0),
    )
}

const FONT_HEAD: FontSize = FontSize::Px(18.0);
const FONT_TITLE: FontSize = FontSize::Px(15.0);
const FONT_BODY: FontSize = FontSize::Px(13.0);
/// The small type-badge caption.
const FONT_BADGE: FontSize = FontSize::Px(10.0);

/// A title never wraps: it **shrinks to fit its card on one line** — the grand-archive title-bar look,
/// where even a long name sits in its bar. [`title_font`] picks the size; below [`TITLE_MIN`] a rare
/// over-long name clips (paired with `LineBreak::NoWrap`) rather than dropping to a second line.
const TITLE_MIN: f32 = 8.0;
/// Rough average glyph advance as a fraction of the font size for the default proportional font — used to
/// estimate the size whose line just fills the available width. A touch generous (erring wide), so the
/// fit leans toward *not* wrapping.
const GLYPH_ADVANCE: f32 = 0.58;

/// How fast a pile eases toward its target position, as a fraction closed per second (higher = snappier).
const SLIDE_SPEED: f32 = 12.0;

/// The three planned **card footprints** (logical px). Every card, pile, and deck draws at one of
/// these — see [`Size`]. **Small** is the compact name+type form a deck and its contents share;
/// **Medium** is a full individual card face (adds detail lines); **Large** is a document / log panel.
// Card sizes come from the model's layout module - the single source of truth - so a card's node is drawn at
// exactly the footprint the layout math uses. The renderer never defines its own card dimensions.
// The model carries integer coordinates; the renderer converts to f32 here, at the graphics boundary, since
// Bevy's `Val::Px` wants floats. These pixel-drawing aliases are f32; the layout constants that *feed* the
// model (GAP, CARD_W, ...) stay i32.
use cardtable_model::layout as card_layout;
const SMALL_W: f32 = card_layout::SMALL_W as f32;
const SMALL_H: f32 = card_layout::SMALL_H as f32;
const MEDIUM_W: f32 = card_layout::MEDIUM_W as f32;
const LARGE_W: f32 = card_layout::LARGE_W as f32;

/// The inner text width of a Small / Medium card — its width less the padding + border on both sides.
/// This is the room a title has to fit on one line (see [`title_font`]): Small has 8px padding + 2px
/// border a side; Medium has 10px + 2px.
const SMALL_INNER: f32 = SMALL_W - 2.0 * (8.0 + 2.0);
const MEDIUM_INNER: f32 = MEDIUM_W - 2.0 * (10.0 + 2.0);
const LARGE_INNER: f32 = LARGE_W - 2.0 * 12.0;

/// The per-card stack step (offset along two edges) and the visual depth cap, so a deck reads as a
/// stack of Small cards without growing without bound. Aliased from the model, which owns the chip geometry.
const STACK_OFFSET: f32 = card_layout::STACK_OFFSET as f32;
const MAX_STACK: usize = card_layout::MAX_STACK;

/// The one constant **gap** between anything on the felt — adjacent cards, piles, and the surface edges —
/// so spacing is uniform everywhere it's computed (see [`Board::structured_positions`],
/// [`Board::arrange_row`]). Integer, because it feeds the model's (integer) layout math.
const GAP: i32 = 12;
/// A rendered Small card's outer size: its footprint plus the 2px border on each side. The stand-in box a
/// not-yet-measured card gets, so the first frame of a structured layout is sane (see [`build_ui`]). Integer,
/// feeding the model's layout math.
const CARD_W: i32 = card_layout::SMALL_W + 4;
const CARD_H: i32 = card_layout::SMALL_H + 4;
/// Height of the **overlay band** at the top of a zone — the strip the floating title / Back / rail
/// occupy. A **structured** zone (grid / list / rows), whose cards can't be shoved, insets its content
/// region by this so nothing lands under an overlay. A **freely-placed** zone (Free / root) uses no
/// inset — its cards share the felt and the [`Pinned`] fixtures shove them clear instead. See [`build_ui`].
const OVERLAY_BAND: i32 = 52;

fn build_ui(
    commands: &mut Commands,
    tree: &Board,
    rail: &[RailAction],
    front: Option<CardId>,
    affordances: &[String],
) {
    // Defensive: a stale / incompatible save could focus a pile that no longer exists — fall back to the
    // root rather than panic the draw.
    let zone = if tree.pile(tree.focus_id()).is_some() {
        tree.focus_id()
    } else {
        tree.root_id()
    };
    let at_root = zone == tree.root_id();

    commands
        .spawn((
            CardTableRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(FELT),
        ))
        .with_children(|root| {
            // SURFACE: the whole window is felt — no title bar. The zone title and Back are floating
            // overlays on top (see below). A **freely-placed** zone (the Table root, or a Free deck)
            // shares the felt with them: its content sits on the whole surface and the Pinned fixtures
            // shove it clear. A **structured** zone (grid / list / rows) can't be shoved, so its content
            // region is inset below the overlay band instead — one inset, applied here, not per layout.
            let freely_placed = at_root
                || matches!(
                    tree.pile(zone).map(|p| p.layout().arrangement),
                    Some(Arrangement::Free)
                );
            let content_inset = if freely_placed {
                0.0
            } else {
                OVERLAY_BAND as f32
            };
            root.spawn((
                TableSurface,
                Node {
                    width: Val::Percent(100.0),
                    flex_grow: 1.0,
                    overflow: Overflow::scroll_y(),
                    padding: UiRect::top(Val::Px(content_inset)),
                    ..default()
                },
            ))
            .with_children(|surf| {
                surf.spawn((
                    TableContent,
                    Node {
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        ..default()
                    },
                ))
                .with_children(|surface| {
                    let pile = tree.pile(zone).expect("zone pile exists");
                    if at_root {
                        for id in pile.subpiles() {
                            let pos = tree.pile(id).expect("pile id from zone").pos();
                            surface
                                .spawn((
                                    Movable(TableNode::Pile(id)),
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(pos.x as f32),
                                        top: Val::Px(pos.y as f32),
                                        ..default()
                                    },
                                ))
                                .with_children(|wrapper| spawn_pile(wrapper, tree, id));
                        }
                    } else if matches!(pile.layout().arrangement, Arrangement::Rows) {
                        // A Rows view (the inn's assignment view): a column of horizontal rows, each led by
                        // its Header card, then its cards. The Hero and Kit rows come from the projection,
                        // the Active row from the pile's own cards (see `Board::row_groups`).
                        // The fan's available width is exactly what the flexbox will give each row's
                        // container: the felt width less the column's padding on both sides, the header
                        // card, and the header→fan gap. Computing it here (not after layout) lets us seed
                        // each card's spread position so the very first frame is already right.
                        let fan_width = (tree.bounds().x as f32
                            - 2.0 * INN_PAD
                            - CARD_W as f32
                            - INN_HEADER_GAP)
                            .max(1.0);
                        surface
                            .spawn(Node {
                                flex_direction: FlexDirection::Column,
                                width: Val::Percent(100.0),
                                padding: UiRect::all(Val::Px(INN_PAD)),
                                row_gap: Val::Px(14.0),
                                ..default()
                            })
                            .with_children(|col| {
                                for (header, cards) in tree.row_groups(zone) {
                                    // Rows span the full width; each is a header card leading a fan.
                                    let mut row = col.spawn(Node {
                                        width: Val::Percent(100.0),
                                        flex_direction: FlexDirection::Row,
                                        align_items: AlignItems::Center,
                                        column_gap: Val::Px(INN_HEADER_GAP),
                                        ..default()
                                    });
                                    row.with_children(|row| {
                                        // The header names the row and isn't part of the fan — it leads it.
                                        spawn_card(row, tree.card(header).expect("row header"));
                                        // The row's cards are a horizontal **fan**. The container flex-grows
                                        // to fill the room left after the header; [`fan_layout`] reads that
                                        // width each frame and **spreads the cards as far as it fits** (up to a
                                        // full card + gap, no overlap), overlapping only when the room runs out
                                        // — down to a left-edge sliver. A tapped card is pulled to `front`
                                        // (drawn fully, above its siblings) and the fan opens around it. Here we
                                        // only tag the pieces; positions are computed dynamically in that system.
                                        row.spawn((
                                            FanContainer,
                                            Node {
                                                position_type: PositionType::Relative,
                                                flex_grow: 1.0,
                                                min_width: Val::Px(0.0),
                                                height: Val::Px(CARD_H as f32),
                                                ..default()
                                            },
                                        ))
                                        .with_children(
                                            |fan| {
                                                let count = cards.len();
                                                let front_idx = front.and_then(|f| {
                                                    cards.iter().position(|&c| c == f)
                                                });
                                                for (j, cid) in cards.into_iter().enumerate() {
                                                    let card = tree.card(cid).expect("row card");
                                                    // Content cards are draggable — drop one on the Active row
                                                    // to move it in. Seeded at its computed spread position (so
                                                    // frame one is correct); `fan_layout` then owns `left` and
                                                    // the z-order each frame — baseline `ZIndex(index)` so later
                                                    // cards sit on top and the left slivers show, lifting the
                                                    // front card above the rest.
                                                    let front_z = front == Some(cid);
                                                    fan.spawn((
                                                        Movable(TableNode::Card(cid)),
                                                        FanCard {
                                                            index: j,
                                                            card: cid,
                                                        },
                                                        ZIndex(if front_z {
                                                            FAN_FRONT_Z
                                                        } else {
                                                            j as i32
                                                        }),
                                                        Node {
                                                            position_type: PositionType::Absolute,
                                                            left: Val::Px(fan_left(
                                                                fan_width, count, front_idx, j,
                                                            )),
                                                            top: Val::Px(0.0),
                                                            ..default()
                                                        },
                                                    ))
                                                    // Always Small in the fan: the inn is a *projection*
                                                    // for identifying/selecting a card (a card's `size` is
                                                    // shared state — growing it here would grow it
                                                    // everywhere), and the fan's spacing assumes uniform
                                                    // widths. Full detail lives in the card's home deck.
                                                    .with_children(|tile| {
                                                        spawn_card_small(
                                                            tile,
                                                            card,
                                                            card.quantity() as usize,
                                                        )
                                                    });
                                                }
                                            },
                                        );
                                    });
                                }
                            });
                    } else if !pile.projection().is_empty() {
                        // A projection view (the inn): each source deck's cards under a header, plus this
                        // pile's own cards ("Here" — characters standing at the location). Every card is
                        // draggable so you can drop a hero onto a kit (or a kit onto a hero) to equip — see
                        // `on_drop`. The cards keep their real home; the projection only shows them.
                        surface
                            .spawn(Node {
                                flex_direction: FlexDirection::Column,
                                width: Val::Percent(100.0),
                                padding: UiRect::all(Val::Px(12.0)),
                                row_gap: Val::Px(14.0),
                                ..default()
                            })
                            .with_children(|col| {
                                let mut sections: Vec<(String, Vec<CardId>)> = tree
                                    .projection_groups(zone)
                                    .into_iter()
                                    .map(|(src, cards)| (pile_display_name(tree, src), cards))
                                    .collect();
                                let own = tree.content_cards(zone).to_vec();
                                if !own.is_empty() {
                                    sections.push(("Here".to_string(), own));
                                }
                                for (header, group) in sections {
                                    col.spawn(Node {
                                        flex_direction: FlexDirection::Column,
                                        row_gap: Val::Px(6.0),
                                        ..default()
                                    })
                                    .with_children(
                                        |section| {
                                            section.spawn((
                                                Text::new(header),
                                                TextFont {
                                                    font_size: FONT_HEAD,
                                                    ..default()
                                                },
                                                TextColor(INK),
                                            ));
                                            section
                                                .spawn(Node {
                                                    flex_direction: FlexDirection::Row,
                                                    flex_wrap: FlexWrap::Wrap,
                                                    column_gap: Val::Px(10.0),
                                                    row_gap: Val::Px(10.0),
                                                    ..default()
                                                })
                                                .with_children(|row| {
                                                    // Projection cards get no `Movable`: they stay put while
                                                    // dragged (no cursor-follow), so the card you release *onto*
                                                    // is reliably the drop target — the equip in `on_drop`. The
                                                    // drag itself is picking-level, so it still fires.
                                                    for cid in group {
                                                        spawn_card(
                                                            row,
                                                            tree.card(cid)
                                                                .expect("card id from projection"),
                                                        );
                                                    }
                                                });
                                        },
                                    );
                                }
                            });
                    } else if Some(zone) == top_deck(tree, "Locations") {
                        // The location **map**: a fixed-column grid of place cells. Each cell is a place card
                        // with the character tokens standing there **cascaded below it** — every token slid one
                        // title strip down so the card above still shows its title (title-at-top), later tokens
                        // on top. The cell carries an explicit height, so a place with more tokens is a taller
                        // cell and the wrap-grid pushes the rows below it down — the map stays aligned on both
                        // axes as characters gather. Drag a token onto another place card to move that character
                        // (`on_node_drag_end` -> `Board::move_character`); its home stays the place pile, so
                        // relocating it *is* the move. A place card's exposed title strip drills in (it carries
                        // `PileDropZone`). Columns come from the Locations `Grid` arrangement (a real map, not a
                        // width-responsive reflow), so the grid is sized to fit exactly that many.
                        let cols = match tree.pile(zone).map(|p| p.layout().arrangement) {
                            Some(Arrangement::Grid { columns }) => columns.max(1),
                            _ => 3,
                        };
                        let grid_w =
                            cols as f32 * SMALL_W + (cols.saturating_sub(1)) as f32 * MAP_CELL_GAP;
                        surface
                            .spawn(Node {
                                flex_direction: FlexDirection::Row,
                                flex_wrap: FlexWrap::Wrap,
                                align_items: AlignItems::FlexStart, // top-align cells so rows read as rows
                                width: Val::Px(grid_w),
                                margin: UiRect {
                                    top: Val::Px(MAP_PAD),
                                    left: Val::Auto,
                                    right: Val::Auto,
                                    bottom: Val::Px(MAP_PAD),
                                },
                                column_gap: Val::Px(MAP_CELL_GAP),
                                row_gap: Val::Px(MAP_CELL_GAP),
                                ..default()
                            })
                            .with_children(|grid| {
                                for place in tree.pile(zone).expect("map zone").subpiles() {
                                    // The heroes stationed at this place (their `hero` position copies)
                                    // cascade below its place card.
                                    let tokens: Vec<CardId> = tree
                                        .content_cards(place)
                                        .into_iter()
                                        .filter(|&c| {
                                            tree.card(c).is_some_and(|k| k.card_type() == "hero")
                                        })
                                        .collect();
                                    // The cell is the cascade's **union box** (the model's cascade_footprint):
                                    // the place card plus one title strip per stationed token. Because this
                                    // whole box is the PileDropZone, its green drop-target glow wraps the full
                                    // stack, not just the top card - dropping a token anywhere over the place
                                    // *or its stacked tokens* moves the character here.
                                    let cell_h =
                                        card_layout::cascade_footprint(tokens.len()).y as f32;
                                    grid.spawn((
                                        PileDropZone(place),
                                        Node {
                                            position_type: PositionType::Relative,
                                            width: Val::Px(SMALL_W),
                                            height: Val::Px(cell_h),
                                            ..default()
                                        },
                                    ))
                                    .with_children(|cell| {
                                        // The place card is the base of the cascade, at the cell's top.
                                        cell.spawn(Node {
                                            position_type: PositionType::Absolute,
                                            left: Val::Px(0.0),
                                            top: Val::Px(0.0),
                                            ..default()
                                        })
                                        .with_children(|slot| spawn_place_card(slot, tree, place));
                                        // Each token cascades one strip lower and sits above the last, so the
                                        // card above shows only its title. Movable so the drag observers fire.
                                        for (i, tok) in tokens.into_iter().enumerate() {
                                            let card = tree.card(tok).expect("token card");
                                            cell.spawn((
                                                Movable(TableNode::Card(tok)),
                                                ZIndex(i as i32 + 1),
                                                Node {
                                                    position_type: PositionType::Absolute,
                                                    left: Val::Px(0.0),
                                                    top: Val::Px((i as f32 + 1.0) * TITLE_OFFSET),
                                                    ..default()
                                                },
                                            ))
                                            .with_children(|t| spawn_card_small(t, card, 1));
                                        }
                                    });
                                }
                            });
                    } else {
                        // The zone lays its contents out — one shared path for every layout. A **structured**
                        // layout (List / Grid) gets footprint-aware positions (`structured_positions`), so a
                        // grown card reflows its neighbours instead of overlapping; a Free (unordered) deck
                        // reads each node's own model position and shoves overlaps. The zone card on top is
                        // the pile's label, not content (see `content_cards`).
                        let free = matches!(pile.layout().arrangement, Arrangement::Free);
                        // Free decks are drag-at-will; a structured layout is draggable only when editable.
                        let draggable = free || pile.layout().editable;
                        // Same order as `movable_children`, so we zip by index below.
                        let placed: Vec<(TableNode, Pos)> = if free {
                            Vec::new()
                        } else {
                            tree.structured_positions(
                                zone,
                                GAP,
                                GAP,
                                Pos {
                                    x: CARD_W,
                                    y: CARD_H,
                                },
                            )
                        };
                        // One uniform pass over the movable children: a card and a nested pile alike get a
                        // position, a drag marker, and (Free) shove — they differ only in their leaf face (a
                        // card grows; a pile is a drillable chip).
                        for (index, node) in tree.movable_children(zone).into_iter().enumerate() {
                            let (x, y) = if free {
                                let p = match node {
                                    TableNode::Card(cid) => tree.card(cid).map(|c| c.pos()),
                                    TableNode::Pile(pid) => tree.pile(pid).map(|d| d.pos()),
                                }
                                .unwrap_or_default();
                                (p.x as f32, p.y as f32)
                            } else {
                                let p = placed.get(index).map(|&(_, p)| p).unwrap_or_default();
                                (p.x as f32, p.y as f32)
                            };
                            let mut tile = surface.spawn(Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(x),
                                top: Val::Px(y),
                                ..default()
                            });
                            match node {
                                TableNode::Card(cid) => {
                                    let card = tree.card(cid).expect("card id from zone");
                                    // An expanded card lifts above its neighbours so it stays readable.
                                    tile.insert(card_elevation(card));
                                    // A virtual readout (a combat log) is not rearranged — a drag on it
                                    // scrolls its panel instead of moving it, so it isn't Movable.
                                    if draggable && card.kind() != CardKind::Virtual {
                                        tile.insert(Movable(node));
                                    }
                                    tile.with_children(|tile| spawn_card(tile, card));
                                }
                                TableNode::Pile(pid) => {
                                    if draggable {
                                        tile.insert(Movable(node));
                                    }
                                    tile.with_children(|tile| spawn_pile(tile, tree, pid));
                                }
                            }
                        }
                    }
                });
            });

            // FLOATING OVERLAYS, drawn above the felt and out of flow: the zone title centered at the top
            // (plain text, no bar), Back at the top-left inside a zone, and any loose actions at the
            // top-right. The title and Back carry `Pinned`, so on a freely-placed felt the cards settle
            // clear of them; a structured zone insets its content region instead.
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(6.0),
                    left: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                GlobalZIndex(10),
                Pickable::IGNORE,
            ))
            .with_children(|title| {
                title.spawn((
                    Pinned,
                    Text::new(zone_title_with_count(tree, zone)),
                    TextFont {
                        font_size: FONT_HEAD,
                        ..default()
                    },
                    TextColor(INK),
                    Pickable::IGNORE,
                ));
            });
            if !at_root {
                root.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(6.0),
                        left: Val::Px(8.0),
                        ..default()
                    },
                    GlobalZIndex(10),
                ))
                .with_children(|slot| spawn_nav_card(slot, (BackCard, Pinned), "Back"));
            }
            // The **combat** controls, mirroring Back but on the right, on a location that holds both a hero
            // and an encounter: the player picks **Auto** (play it out) or **Manual** (decide every step).
            // The game's **contextual affordances** (Fight, Commit sub-phase, Advance Day, …) — one control
            // card each, declared by the game per focused zone (`BoardGame::affordances`) and drained back
            // into `Game::apply` by the board driver. Supersedes the old hardcoded combat / advance-day cards.
            if !affordances.is_empty() {
                root.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(6.0),
                        right: Val::Px(8.0),
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(8.0),
                        ..default()
                    },
                    GlobalZIndex(10),
                ))
                .with_children(|slot| {
                    for (i, label) in affordances.iter().enumerate() {
                        spawn_nav_card(slot, (AffordanceControl(i), Pinned), label);
                    }
                });
            }
            if !rail.is_empty() {
                root.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(6.0),
                        right: Val::Px(8.0),
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(8.0),
                        ..default()
                    },
                    GlobalZIndex(10),
                ))
                .with_children(|slot| {
                    for action in rail {
                        spawn_rail_button(slot, action);
                    }
                });
            }
        });
}

/// The display name of a pile/zone: "Table" for the root; otherwise its [zone card](Board::zone_card)'s
/// name (the card whose job is to name it), else the pile's own label.
fn pile_display_name(tree: &Board, id: PileId) -> String {
    if id == tree.root_id() {
        return "Table".to_string();
    }
    tree.zone_card(id)
        .and_then(|c| tree.card(c))
        .map(|c| c.name().to_string())
        .unwrap_or_else(|| tree.pile(id).expect("pile id").label.clone())
}

/// The floating zone title with a space-efficient physical-card tally as a `(N)` **prefix**, e.g.
/// `"(10) Location"` — the same recursive [`Board::physical_card_count`] the deck chips show (every
/// physical card counted once, its own title card included), so the chip and the drilled-in title
/// agree. The root ("Table") and a software-only deck (count 0, e.g. System) show a bare name — no
/// tally, matching the chip.
fn zone_title_with_count(tree: &Board, zone: PileId) -> String {
    let name = pile_display_name(tree, zone);
    let count = tree.physical_card_count(zone);
    if zone == tree.root_id() || count == 0 {
        return name;
    }
    format!("({count}) {name}")
}

/// A utility card (e.g. Back) drawn in the nav row — a small card-styled, clickable control. `marker` is
/// any bundle, so a card can carry more than one tag.
fn spawn_nav_card<B: Bundle>(parent: &mut ChildSpawnerCommands, marker: B, label: &str) {
    parent
        .spawn((
            marker,
            Node {
                padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(CARD_BACK),
            BorderColor::all(CARD_EDGE),
            card_shadow(),
        ))
        .with_children(|c| {
            c.spawn((
                Text::new(label.to_string()),
                TextFont {
                    font_size: FONT_TITLE,
                    ..default()
                },
                TextColor(INK),
            ));
        });
}

/// Draws a deck as a stack of **Small cards**: offset layers (two alternating colors, stepped along
/// the left and bottom edges, capped at [`MAX_STACK`]) hint at the depth, and the front layer is a
/// Small-card face ([`small_face`]) showing the top card's `(N)`-prefixed name and its type. The whole
/// stack is one drop target — a deck is a Small card wearing a stack.
fn spawn_pile_chip(
    parent: &mut ChildSpawnerCommands,
    id: PileId,
    label: &str,
    card_type: &str,
    count: usize,
) {
    let depth = count.clamp(1, MAX_STACK);
    let spread = (depth - 1) as f32 * STACK_OFFSET;
    // The chip's overall box is the model's `chip_footprint` - one source of truth, so the drawn chip and the
    // model's drop-target/layout box are identical by construction.
    let chip = card_layout::chip_footprint(count);
    parent
        .spawn((
            PileDropZone(id),
            Node {
                width: Val::Px(chip.x as f32),
                height: Val::Px(chip.y as f32),
                ..default()
            },
        ))
        .with_children(|stack| {
            // Deepest layer first so it renders behind; the front layer (offset 0) is spawned last.
            for layer in (0..depth).rev() {
                let offset = layer as f32 * STACK_OFFSET;
                let color = if layer % 2 == 0 {
                    CARD_BACK
                } else {
                    CARD_BACK_ALT
                };
                let bundle = (
                    Node {
                        position_type: PositionType::Absolute,
                        // Front layer sits at top-right; deeper layers step down-left, so the stack
                        // peeks out along the left and bottom edges.
                        left: Val::Px(spread - offset),
                        top: Val::Px(offset),
                        width: Val::Px(SMALL_W),
                        height: Val::Px(SMALL_H),
                        border: UiRect::all(Val::Px(1.0)),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        padding: UiRect::all(Val::Px(10.0)),
                        row_gap: Val::Px(4.0),
                        border_radius: BorderRadius::all(Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(color),
                    BorderColor::all(CARD_EDGE),
                );
                if layer == 0 {
                    // The front layer is a Small card face — the same [`small_face`] a lone card draws.
                    // The card count rides as a compact `(N)` **prefix** on the name ("(9) Location")
                    // rather than its own sub-line, so the face needs one fewer line and can stay small.
                    // Empty piles carry no prefix, so a place with nothing under it reads as a plain name.
                    let name = if count > 0 {
                        format!("({count}) {label}")
                    } else {
                        label.to_string()
                    };
                    stack
                        .spawn(bundle)
                        .insert(card_shadow())
                        .with_children(|face| {
                            small_face(face, &name, card_type, INK, None);
                        });
                } else {
                    stack.spawn(bundle);
                }
            }
        });
}

/// Draws a pile as a compact, counted chip: its top card's **name prefixed with the physical count**
/// (`(9) Location`) over that card's type. You see its *contents* by clicking it to enter its zone —
/// piles no longer fan open in place. A pile
/// whose top card is face-down (or that is empty) falls back to the pile's own display name, no type,
/// so a face-down deck reveals nothing.
fn spawn_pile(parent: &mut ChildSpawnerCommands, tree: &Board, id: PileId) {
    let pile = tree.pile(id).expect("pile id from tree");
    // The recursive **physical** count (quantities counted, chrome and projections excluded) — the same
    // tally the drilled-in zone title shows, so the chip and the zone agree on "how many are in here".
    let count = tree.physical_card_count(id);
    let (name, card_type) = if matches!(pile.layout().arrangement, Arrangement::Rows)
        || !pile.projection().is_empty()
    {
        // An organizational view (the inn): named by its own label and typed as a "Label" — content
        // dropped into it (a recruited hero landing on top) must never hijack the chip's name.
        (pile_display_name(tree, id), "Label".to_string())
    } else if let Some(zc) = tree.zone_card(id).and_then(|c| tree.card(c)) {
        // The pile's label is its zone card, identified by kind wherever it sits — a sub-pile added after
        // it can never demote it to content or steal the chip's name.
        (zc.name().to_string(), zc.card_type().to_string())
    } else {
        (pile_display_name(tree, id), String::new())
    };
    spawn_pile_chip(parent, id, &name, &card_type, count);
}

/// A **place card** on the location map: a Small, named drop target for one location. Dropping a
/// character's token here moves them to this place (resolved by [`on_node_drag_end`] against its
/// [`PileDropZone`]); clicking it drills into the place (the Inn lives inside Ashfen). It wears the card
/// back so it reads as a fixed board square, distinct from the light-faced character tokens on it.
fn spawn_place_card(parent: &mut ChildSpawnerCommands, tree: &Board, place: PileId) {
    // Carry the same `(N)` physical tally the deck chips show — here it counts the place's own location
    // card plus whatever is stacked under it (encounters, character tokens, or the inn), and updates as
    // characters move in and out. It rides in the top strip, which stays exposed above cascaded tokens.
    let name = zone_title_with_count(tree, place);
    parent
        .spawn((
            // Pure visual — the drop target + click-to-drill live on the enclosing cell (which spans the
            // whole cascade), so a click here bubbles up to it and a token dropped anywhere on the stack
            // still lands on this place.
            Node {
                width: Val::Px(SMALL_W),
                height: Val::Px(SMALL_H),
                padding: UiRect::all(Val::Px(8.0)),
                border: UiRect::all(Val::Px(2.0)),
                flex_direction: FlexDirection::Column,
                // Title at the top: on the map a token cascades over the place card's body, so its name
                // must sit in the top strip that stays exposed above the tokens.
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                row_gap: Val::Px(2.0),
                border_radius: BorderRadius::all(Val::Px(12.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(CARD_BACK),
            BorderColor::all(CARD_EDGE),
            card_shadow(),
        ))
        .with_children(|face| {
            small_face(face, &name, "Location", INK, None);
        });
}

/// The **held** layer: an element being dragged floats here — above the felt tiles and the floating
/// overlays (title / Back at [`GlobalZIndex(10)`]) — so "picking a card up off the table" reads literally:
/// it stays on top of everything it slides over until you set it down. Applied on drag-start, removed on
/// release (see [`on_node_drag`] / [`on_node_drag_end`]).
const HELD_Z: i32 = 50;

/// How far each card in a **fan** is offset from the previous one — the width of the uncovered left-edge
/// sliver. Small enough to overlap heavily (examine one at a time), wide enough that the sliver shows the
/// start of the name to tell cards apart.
const FAN_SLIVER: f32 = 34.0;
/// Local draw order for the card pulled to the front of a fan — above every sliver in its row. Local (not
/// global), so a dragged card (on the global held layer) still floats above it.
const FAN_FRONT_Z: i32 = 1000;
/// Inn **Rows** layout metrics, named so the flexbox that lays a row out and the fan's build-time width
/// estimate stay in lockstep (see [`fan_left`], [`build_ui`]): the padding around the rows column, and the
/// gap between a row's header and its fan. Getting these exactly right is what makes a freshly-built fan
/// land on the correct spread on its *first* frame, with no measure-and-correct hop.
const INN_PAD: f32 = 12.0;
const INN_HEADER_GAP: f32 = 8.0;

/// Location **map** metrics: the padding around the cell grid, and the gap between cells (a cell is a
/// place card over its character tokens). One cell is [`SMALL_W`] wide; the gap gives the tokens room to
/// read as *stationed here*, not crowding the next place.
const MAP_PAD: f32 = 16.0;
const MAP_CELL_GAP: f32 = 24.0;
/// The cascade step for a map cell: each character token stationed at a place is slid this far below the
/// card above it, so that card's top **title strip** stays visible (title-at-top). Aliased from the model,
/// which owns the cascade geometry (see [`cardtable_model::layout::cascade_footprint`]).
const TITLE_OFFSET: f32 = card_layout::CASCADE_OFFSET as f32;

/// The x offset of fan card `index` (of `count`) within a fan `width` px wide, when `front_idx` — if any —
/// is the card pulled to the front. The single source of truth for fan geometry: [`build_ui`] seeds each
/// card with it from the *known* surface width (so a fresh fan is right on frame one), and [`fan_layout`]
/// re-applies it every frame from the *measured* width (so it tracks resizes and the live front card).
///
/// The cards **spread to fit** — a full card + [`GAP`] step at most (no overlap), down to a [`FAN_SLIVER`]
/// floor — with the last card right-edged at `width`. To show the front card fully, it is pulled left and
/// the slivers to its left compress to yield the room, so the right side never shoves off screen; the last
/// card needs no adjustment. See the call sites for the fuller rationale.
fn fan_left(width: f32, count: usize, front_idx: Option<usize>, index: usize) -> f32 {
    let pitch = if count > 1 {
        ((width - CARD_W as f32) / (count - 1) as f32).clamp(FAN_SLIVER, (CARD_W + GAP) as f32)
    } else {
        0.0
    };
    match front_idx {
        // Only a card that isn't the last one opens the fan (the last shows fully at baseline).
        Some(fi) if fi + 1 < count => {
            let front_left =
                ((fi + 1) as f32 * pitch - CARD_W as f32).clamp(0.0, fi as f32 * pitch);
            if index < fi {
                let pitch_left = if fi > 0 { front_left / fi as f32 } else { 0.0 };
                index as f32 * pitch_left
            } else if index == fi {
                front_left
            } else {
                index as f32 * pitch
            }
        }
        _ => index as f32 * pitch,
    }
}

/// Draw order for a card tile: an **expanded** (non-Small) card lifts above its siblings, so the card
/// you just grew to read is never buried under a neighbour it now overlaps. Small cards stay at the base
/// layer, preserving spawn order among themselves.
fn card_elevation(card: &Card) -> ZIndex {
    ZIndex(if matches!(card.size(), Size::Small) {
        0
    } else {
        1
    })
}

/// Draws one card at its current render [`Size`]: **Small** (name + type), **Medium** (a full card
/// face with detail), or **Large** (a document / log panel). Every form carries `CardRef`, so a click
/// can grow/shrink it.
fn spawn_card(parent: &mut ChildSpawnerCommands, card: &Card) {
    match card.size() {
        // Show the stack's `×N` (PC.2) when it stands for several identical physical cards.
        Size::Small => spawn_card_small(parent, card, card.quantity() as usize),
        Size::Medium => spawn_card_medium(parent, card),
        Size::Large => spawn_card_large(parent, card),
    }
}

/// Edge colour for a card: highlighted when it carries a legal move, else the dark card edge.
fn card_edge(card: &Card) -> Color {
    if card.is_actionable() {
        ACTIONABLE
    } else {
        CARD_EDGE
    }
}

/// Tag a freshly-spawned card entity as actionable (so a loose action still fires), then run `build`
/// to fill its children.
fn finish_card(
    mut entity: EntityCommands,
    card: &Card,
    build: impl FnOnce(&mut ChildSpawnerCommands),
) {
    if let Some(index) = card.actionable {
        entity.insert(ActionControl(index));
    }
    entity.with_children(build);
}

/// The font size for a card `title` so it fills at most `inner` px on **one line**: the `base` size for a
/// short name, shrinking for a long one so it never wraps (floored at [`TITLE_MIN`] for legibility). Pair
/// the returned size with `TextLayout::no_wrap()` so a title past the floor clips rather than
/// wraps. See the grand-archive reference: a long title sits in its bar at a reduced size.
fn title_font(title: &str, base: FontSize, inner: f32) -> FontSize {
    let base_px = match base {
        FontSize::Px(p) => p,
        _ => 15.0,
    };
    let chars = title.chars().count().max(1) as f32;
    FontSize::Px((inner / (chars * GLYPH_ADVANCE)).clamp(TITLE_MIN, base_px))
}

/// The **shared Small-card face** — the one content-rendering logic that lone cards *and* deck/pile
/// fronts delegate to: the name on top, the type badge beneath, and an optional sub-line (a deck's
/// card count, or a card's `×N` quantity). `ink` colours the name to suit the fill it sits on.
fn small_face(
    c: &mut ChildSpawnerCommands,
    name: &str,
    card_type: &str,
    ink: Color,
    sub: Option<String>,
) {
    c.spawn((
        Text::new(name.to_string()),
        TextFont {
            font_size: title_font(name, FONT_TITLE, SMALL_INNER),
            ..default()
        },
        TextLayout::no_wrap(),
        TextColor(ink),
    ));
    spawn_type_badge(c, card_type);
    if let Some(sub) = sub {
        c.spawn((
            Text::new(sub),
            TextFont {
                font_size: FONT_BODY,
                ..default()
            },
            TextColor(MUTED),
        ));
    }
}

/// Small form — a [`SMALL_W`]×[`SMALL_H`] card showing name over type (or a blank back when face
/// down), plus a `×N` line when `quantity > 1`. Its face is drawn by [`small_face`], the same content
/// a deck's front layer uses — a lone card and a deck render the same way.
fn spawn_card_small(parent: &mut ChildSpawnerCommands, card: &Card, quantity: usize) {
    let face_down = card.is_face_down();
    let (label, bg, ink) = match &card.face {
        // A **utility** card wears its action colour as the card background, so it reads as a coloured
        // button (Exit red, Start Over amber) even as an ordinary card; its ink adapts to stay legible.
        Face::Up { title } => match card.kind() {
            CardKind::Utility(u) => {
                let bg = action_color(u);
                (title.clone(), bg, badge_ink(bg))
            }
            _ => (title.clone(), CARD_FACE, CARD_INK),
        },
        // A **face-down** card still shows its name (so a spent marker is identifiable), but on the dark
        // card **back** in muted ink — the light-face / dark-back contrast is what says "face down".
        Face::Down { title } => (title.clone(), CARD_BACK, MUTED),
    };
    let entity = parent.spawn((
        CardRef(card.id),
        Node {
            width: Val::Px(SMALL_W),
            height: Val::Px(SMALL_H),
            padding: UiRect::all(Val::Px(8.0)),
            border: UiRect::all(Val::Px(2.0)),
            flex_direction: FlexDirection::Column,
            // Title-at-top (was centred): the name sits in a strip at the top edge so cards overlapped
            // vertically still show their names (the Grand Archive title-bar look) — the survey-all cascade.
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Center,
            row_gap: Val::Px(2.0),
            border_radius: BorderRadius::all(Val::Px(12.0)),
            // Fully fixed footprint: contain overflowing text to the box so it can never spill onto a
            // neighbour. Overflow is a *paint* clip — layout is unaffected, so the text audit still sees it.
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(bg),
        // A face-down card wears a distinct dashed-look accent border (a warm slate) so, beyond the dark
        // fill, there's an unmistakable "this is face down" cue that doesn't hide the name.
        BorderColor::all(if face_down {
            FACE_DOWN_EDGE
        } else {
            card_edge(card)
        }),
        card_shadow(),
    ));
    finish_card(entity, card, |c| {
        let sub = if face_down {
            // A face-down `hero` card is a spent move-marker on Progress: it means the hero has *moved*
            // this day. Spell that out so face-down reads as a state, not just a blank back.
            (card.card_type() == "hero").then(|| "moved".to_string())
        } else {
            // Face-up: a stack of N identical physical cards shows its `×N` (PC.2).
            (quantity > 1).then(|| format!("x{quantity}"))
        };
        small_face(c, &label, card.card_type(), ink, sub);
        // A clear, font-safe face-down stamp: a slim accent bar pinned across the card's foot. It reads as
        // a "flipped" marker at a glance without obscuring the name above it.
        if face_down {
            c.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(6.0),
                    right: Val::Px(6.0),
                    bottom: Val::Px(6.0),
                    height: Val::Px(4.0),
                    border_radius: BorderRadius::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(FACE_DOWN_EDGE),
                Pickable::IGNORE,
            ));
        }
    });
}

/// Medium form — a card face: a name header above its detail (stat / rules) lines.
fn spawn_card_medium(parent: &mut ChildSpawnerCommands, card: &Card) {
    let entity = parent.spawn((
        CardRef(card.id),
        Node {
            width: Val::Px(MEDIUM_W),
            // Sized to the **Medium** footprint (width + a height per detail line) - this function draws at
            // Medium, so it boxes at Medium regardless of the card's own size field (they match in the app;
            // the card gallery draws every size, so the box must follow the drawn size, not the stored one).
            // The renderer is a pass-through, not the authority. Content clips to fit (never spills).
            height: Val::Px(
                card_layout::footprint(Size::Medium, card.detail().len(), card.panel().len()).y
                    as f32,
            ),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(10.0)),
            border: UiRect::all(Val::Px(2.0)),
            row_gap: Val::Px(4.0),
            border_radius: BorderRadius::all(Val::Px(12.0)),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(CARD_FACE),
        BorderColor::all(card_edge(card)),
        card_shadow(),
    ));
    finish_card(entity, card, |c| {
        c.spawn((
            Text::new(card.name().to_string()),
            TextFont {
                font_size: title_font(card.name(), FONT_HEAD, MEDIUM_INNER),
                ..default()
            },
            TextLayout::no_wrap(),
            TextColor(CARD_INK),
        ));
        spawn_type_badge(c, card.card_type());
        for line in card.detail() {
            c.spawn((
                Text::new(line.clone()),
                TextFont {
                    font_size: FONT_BODY,
                    ..default()
                },
                // One line per detail line (no wrap) so the rendered height matches the model's line-count
                // footprint exactly; an over-long line clips horizontally.
                TextLayout::no_wrap(),
                TextColor(CARD_INK),
            ));
        }
    });
}

/// Largest form — a utility panel (e.g. a combat log): a name header above its panel lines, scrollable.
fn spawn_card_large(parent: &mut ChildSpawnerCommands, card: &Card) {
    let mut entity = parent.spawn((
        CardRef(card.id),
        Node {
            width: Val::Px(LARGE_W),
            // Sized to the **Large** footprint (capped at LARGE_MAX_H); content beyond it scrolls. Boxes at
            // Large regardless of the card's stored size, so the gallery (which draws every size) is accurate.
            height: Val::Px(
                card_layout::footprint(Size::Large, card.detail().len(), card.panel().len()).y
                    as f32,
            ),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(12.0)),
            row_gap: Val::Px(4.0),
            overflow: Overflow::scroll_y(),
            border_radius: BorderRadius::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(PANEL),
        card_shadow(),
    ));
    // A virtual readout (a combat log) can outrun the card, so its panel scrolls — the wheel and a drag
    // drive its `ScrollPosition` (Bevy's `scroll_y` only clips). Ordinary panel cards stay draggable and
    // simply clip, so only virtual cards opt in.
    if card.kind() == CardKind::Virtual {
        entity.insert((ScrollPanel, ScrollPosition::default()));
    }
    finish_card(entity, card, |c| {
        c.spawn((
            Text::new(card.name().to_string()),
            TextFont {
                font_size: title_font(card.name(), FONT_HEAD, LARGE_INNER),
                ..default()
            },
            TextLayout::no_wrap(),
            TextColor(INK),
        ));
        for line in card.panel() {
            c.spawn((
                Text::new(line.clone()),
                TextFont {
                    font_size: FONT_BODY,
                    ..default()
                },
                TextColor(MUTED),
            ));
        }
    });
}

/// A left-rail button for a loose action.
fn spawn_rail_button(parent: &mut ChildSpawnerCommands, action: &RailAction) {
    parent
        .spawn((
            ActionControl(action.index),
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(10.0)),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(BUTTON),
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(action.label.clone()),
                TextFont {
                    font_size: FONT_TITLE,
                    ..default()
                },
                TextColor(INK),
            ));
        });
}

// ---- the game adapter (feature `game`) ----------------------------------

#[cfg(feature = "game")]
mod game {
    use bevy::prelude::*;
    use std::collections::HashSet;

    use cardtable_model::{Board, from_table_view};
    use contract::Game;

    use crate::{
        ActionRail, ActionRequests, CardTablePlugin, CardTableSet, NeedsRebuild, RailAction,
        StatusLine, Table,
    };

    /// The immutable rules of the running game.
    #[derive(Resource)]
    struct GameRes<G: Game>(G);

    /// The mutable state of the running game.
    #[derive(Resource)]
    struct StateRes<G: Game>(G::State);

    /// Drives the [`CardTablePlugin`] core from a [`contract::Game`]: builds the table from the game's
    /// view and turns clicks (the core's [`ActionRequests`]) into `Game::apply`. Adds the core plugin
    /// itself, so the launcher only adds this.
    pub struct GamePlugin<G: Game> {
        game: G,
        seed: u64,
        players: usize,
    }

    impl<G: Game> GamePlugin<G> {
        /// Sets up a match of `game` for `players` seats, seeded by `seed`.
        pub fn new(game: G, seed: u64, players: usize) -> Self {
            Self {
                game,
                seed,
                players,
            }
        }
    }

    impl<G: Game + Clone> Plugin for GamePlugin<G> {
        fn build(&self, app: &mut App) {
            if !app.is_plugin_added::<CardTablePlugin>() {
                app.add_plugins(CardTablePlugin);
            }
            let game = self.game.clone();
            let state = game.new_game(self.seed, self.players);
            let (table, rail, status) = snapshot(&game, &state);
            app.insert_resource(GameRes(game))
                .insert_resource(StateRes::<G>(state))
                .insert_resource(Table(table))
                .insert_resource(ActionRail(rail))
                .insert_resource(StatusLine(status))
                .add_systems(Update, apply_requests::<G>.in_set(CardTableSet::Apply));
        }
    }

    /// Drain the core's click outbox into the game, rebuilding the presentation when state advances.
    fn apply_requests<G: Game + Clone>(
        mut requests: ResMut<ActionRequests>,
        game: Res<GameRes<G>>,
        mut state: ResMut<StateRes<G>>,
        mut table: ResMut<Table>,
        mut rail: ResMut<ActionRail>,
        mut status: ResMut<StatusLine>,
        mut rebuild: ResMut<NeedsRebuild>,
    ) {
        if requests.0.is_empty() {
            return;
        }
        let mut advanced = false;
        for index in requests.0.drain(..) {
            // The action list is a pure function of the current state, so the index captured when the
            // control was drawn is valid against the state as of this drain step.
            let actions = game.0.legal_actions(&state.0);
            if let Some(action) = actions.get(index).cloned()
                && game.0.apply(&mut state.0, &action).is_ok()
            {
                advanced = true;
            }
        }
        if advanced {
            let (t, r, s) = snapshot(&game.0, &state.0);
            table.0 = t;
            rail.0 = r;
            status.0 = s;
            rebuild.0 = true;
        }
    }

    /// Build the presentation state from a game state: the board (zones → piles), the loose-action
    /// rail (legal actions not bound to a card), and the status caption.
    fn snapshot<G: Game>(game: &G, state: &G::State) -> (Board, Vec<RailAction>, String) {
        let view = game.view(state, None);
        let table = from_table_view(&view);
        let bound: HashSet<usize> = view
            .zones
            .iter()
            .flat_map(|z| z.cards.iter().filter_map(|c| c.action))
            .collect();
        let rail = game
            .legal_actions(state)
            .iter()
            .enumerate()
            .filter(|(index, _)| !bound.contains(index))
            .map(|(index, action)| RailAction {
                index,
                label: game.action_label(state, action),
            })
            .collect();
        (table, rail, view.status)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DecksNeedTidy, DragGuard, GAP, OVERLAY_BAND, TITLE_MIN, Table, pluralize, relative_time,
        settle_table_piles, title_font,
    };
    use bevy::prelude::*;
    use bevy::text::FontSize;
    use cardtable_model::Board;

    /// A board of three decks parked at scattered seed positions (as the fixtures leave them), on a felt
    /// wide enough for one row.
    fn seeded_board() -> Board {
        let mut t = Board::new();
        let root = t.root_id();
        for (i, label) in ["A", "B", "C"].iter().enumerate() {
            let p = t.add_pile(root, *label).expect("root exists");
            // Scattered, uneven seeds - exactly what an untidied board looks like.
            t.set_pile_pos(p, 40 + i as i32 * 300, 40 + i as i32 * 160)
                .expect("pile exists");
        }
        t.set_bounds(1600, 900);
        t
    }

    /// The decks' x positions in child order, and their y positions.
    fn deck_positions(table: &Board) -> Vec<(i32, i32)> {
        let root = table.root_id();
        table
            .pile(root)
            .expect("root")
            .subpiles()
            .into_iter()
            .map(|p| {
                let d = table.pile(p).expect("pile");
                (d.pos().x, d.pos().y)
            })
            .collect()
    }

    /// Whether the decks sit in one tidy left-to-right row starting at the standard gap.
    fn is_tidy_row(table: &Board) -> bool {
        let pos = deck_positions(table);
        pos.first() == Some(&(GAP, OVERLAY_BAND)) && pos.iter().all(|&(_, y)| y == OVERLAY_BAND)
    }

    /// **Start Over must re-tidy the decks.** It replaces the board wholesale with a fresh one whose decks
    /// sit at their raw seed positions - but that board reuses the same `PileId`s and (since footprints are
    /// computed, not measured) the same sizes, so a size-diff sees "no change". Without an explicit rebuild
    /// signal the tidy never fires and Start Over leaves every deck at its seed: Heroes off to the right, a
    /// gap between decks, Numbers stranded low. The `DecksNeedTidy` flag is what makes it fire.
    #[test]
    fn start_over_relays_the_decks_into_a_row() {
        let mut app = App::new();
        app.insert_resource(Table(seeded_board()))
            .init_resource::<DragGuard>()
            .insert_resource(DecksNeedTidy(true))
            .add_systems(Update, settle_table_piles);

        // The opening board tidies into a row.
        app.update();
        assert!(
            is_tidy_row(&app.world().resource::<Table>().0),
            "opening board should tidy, got {:?}",
            deck_positions(&app.world().resource::<Table>().0)
        );

        // Start Over: the board is replaced by an identical fresh one, back at the scattered seeds.
        app.world_mut().resource_mut::<Table>().0 = seeded_board();
        app.world_mut().resource_mut::<DecksNeedTidy>().0 = true;
        assert!(!is_tidy_row(&app.world().resource::<Table>().0), "seeds");

        app.update();

        assert!(
            is_tidy_row(&app.world().resource::<Table>().0),
            "Start Over must re-tidy the decks, but they were left at their seeds: {:?}",
            deck_positions(&app.world().resource::<Table>().0)
        );
    }

    /// A title keeps the base size until it would overrun its one line, then shrinks to fit — bottoming
    /// out at the floor (past which `no_wrap` clips rather than wrapping).
    #[test]
    fn title_font_shrinks_a_long_name_to_fit_one_line() {
        let px = |title: &str| match title_font(title, FontSize::Px(15.0), 100.0) {
            FontSize::Px(p) => p,
            other => panic!("expected Px, got {other:?}"),
        };
        // A short name that fits keeps the base size.
        assert_eq!(px("Ok"), 15.0);
        // A long name shrinks below the base so it stays on one line.
        let long = px(&"x".repeat(20));
        assert!(long < 15.0 && long > TITLE_MIN, "got {long}");
        // A very long name bottoms out at the floor.
        assert_eq!(px(&"x".repeat(40)), TITLE_MIN);
    }

    #[test]
    fn pluralize_uses_the_singular_only_for_one() {
        assert_eq!(pluralize(1, "hour", "hours"), "1 hour");
        assert_eq!(pluralize(2, "hour", "hours"), "2 hours");
        assert_eq!(pluralize(0, "hour", "hours"), "0 hours");
        assert_eq!(pluralize(1, "day", "days"), "1 day");
    }

    #[test]
    fn relative_time_reports_the_largest_whole_unit() {
        // Under a minute — including a future/just-built stamp — reads "just now".
        assert_eq!(relative_time(0), "just now");
        assert_eq!(relative_time(-100), "just now");
        assert_eq!(relative_time(59), "just now");
        // Minutes.
        assert_eq!(relative_time(60), "1 minute ago");
        assert_eq!(relative_time(120), "2 minutes ago");
        assert_eq!(relative_time(3599), "59 minutes ago");
        // Hours (note the 1-unit boundary reads "1 hour", not "1 hours").
        assert_eq!(relative_time(3600), "1 hour ago");
        assert_eq!(relative_time(7200), "2 hours ago");
        assert_eq!(relative_time(86_399), "23 hours ago");
        // Days.
        assert_eq!(relative_time(86_400), "1 day ago");
        assert_eq!(relative_time(172_800), "2 days ago");
    }
}
