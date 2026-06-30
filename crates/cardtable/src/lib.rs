//! A Bevy renderer that draws the **card-table metaphor** — everything is a card; a pile is a stack of
//! cards in one footprint. You navigate with **single-click and drag only**: click a pile to drill into
//! its zone, click a card to grow it through its sizes, click Back / Exit cards to move around, and drag
//! piles to arrange them on the table. The current zone's name sits centered at the top (default
//! "Table"). Meaning comes from *what* you click, never a second gesture.
//!
//! # Two layers
//!
//! - **The core (this module) is game-agnostic.** It draws whatever is in the [`Table`] resource (a
//!   [`Tableau`]) plus an [`ActionRail`] of loose actions and a [`StatusLine`], handles focus/zoom
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

use bevy::picking::events::{Click, Drag, DragDrop, DragEnd, DragStart, Pointer};
use bevy::prelude::*;
use bevy::ui::{BoxShadow, ComputedNode};

use cardtable_model::{Card, CardId, CardKind, Face, PileId, Size, Tableau};

#[cfg(feature = "game")]
pub use game::GamePlugin;

// ---- public presentation state (the shared inputs) ----------------------

/// The board: the pile tree the core draws. Mutated in place for focus/zoom; replaced wholesale when
/// the source (a game, or a prototype) rebuilds it.
#[derive(Resource, Default)]
pub struct Table(pub Tableau);

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

/// The game-agnostic renderer. Add it, put a [`Tableau`] in [`Table`], and you have a clickable card
/// table. Add [`GamePlugin`] (feature `game`) on top to drive it from a [`contract::Game`].
pub struct CardTablePlugin;

impl Plugin for CardTablePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Table>()
            .init_resource::<ActionRail>()
            .init_resource::<StatusLine>()
            .init_resource::<ActionRequests>()
            .init_resource::<DragGuard>()
            .init_resource::<DraggingCard>()
            .insert_resource(NeedsRebuild(true))
            .configure_sets(
                Update,
                (CardTableSet::Input, CardTableSet::Apply, CardTableSet::Draw).chain(),
            )
            .add_systems(Startup, (setup_camera, install_ui_font))
            .add_systems(
                Update,
                (
                    sync_pile_sizes,
                    sync_surface_size,
                    animate_piles,
                    animate_cards,
                ),
            )
            .add_systems(Update, redraw.in_set(CardTableSet::Draw))
            // Input is picking-driven, so it runs in observers rather than the Input system set:
            // clicks open/close piles and fire actions; a card drag drops into a pile; a pile drag
            // slides it freely across the table.
            .add_observer(on_drag_start)
            .add_observer(on_click)
            .add_observer(on_drop)
            .add_observer(on_pile_drag)
            .add_observer(on_pile_drag_end)
            .add_observer(on_card_drag)
            .add_observer(on_card_drag_end);
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

/// Marks a top-level pile's absolutely-positioned wrapper, carrying its [`PileId`]. Dragging it slides
/// the pile freely across the table (live), committing the final position on release.
#[derive(Component, Clone, Copy)]
struct TablePile(PileId);

/// Marks the table surface — the positioning context for piles. Its size is fed to the model as the
/// wall bounds that keep piles inside.
#[derive(Component)]
struct TableSurface;

/// A utility card that navigates up one zone level when clicked.
#[derive(Component)]
struct BackCard;

/// A utility card that quits the app when clicked (desktop only).
#[derive(Component)]
struct ExitCard;

/// Marks a card's grid tile inside a drilled zone, carrying its [`CardId`]. Dragging it slides the
/// card freely; on release it reorders into the nearest grid cell and the rest reflow.
#[derive(Component, Clone, Copy)]
struct TableCard(CardId);

/// True while a pointer drag is in progress. Bevy fires a `Click` at the end of *every* drag (press
/// and release over the same entity, regardless of the drag), so this guards the click handler from
/// treating a drag's release as a real click. Set on [`DragStart`], cleared on [`DragEnd`].
#[derive(Resource, Default)]
struct DragGuard(bool);

/// Set when the UI must be torn down and rebuilt — *structural* changes only (open/close a pile, move
/// a card, a new game snapshot). Pile positions are not structural; they animate, so repositioning
/// never sets this. See [`redraw`] and [`animate_piles`].
#[derive(Resource)]
struct NeedsRebuild(bool);

/// The card currently being dragged in a zone grid (if any), so its tile isn't snapped to the grid by
/// the animation while the pointer holds it.
#[derive(Resource, Default)]
struct DraggingCard(Option<CardId>);

// ---- systems ------------------------------------------------------------

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// The bundled UI typeface — **Nunito Sans** (a warm, friendly humanist sans that's still crisp for
/// small text on cards). Covers the punctuation the renderer uses (em dashes, curly quotes) that
/// Bevy's built-in `FiraMono-subset` lacks, which would otherwise show as tofu boxes. SIL Open Font
/// License; see `fonts/NunitoSans-OFL.txt`. A Latin static instance (~33 KB) keeps the wasm small.
const UI_FONT: &[u8] = include_bytes!("../fonts/NunitoSans-Regular.ttf");

/// Replace Bevy's ASCII-only default font with the bundled Nunito Sans face. Bevy registers its default
/// font at `AssetId::default()`, and every `TextFont { ..default() }` here points there, so overwriting
/// that one asset reskins all UI text without threading a font handle through each label.
fn install_ui_font(mut fonts: ResMut<Assets<Font>>) {
    let font = Font::from_bytes(UI_FONT.to_vec());
    fonts
        .insert(AssetId::default(), font)
        .expect("override the default font");
}

fn on_drag_start(_on: On<Pointer<DragStart>>, mut guard: ResMut<DragGuard>) {
    guard.0 = true;
}

/// A picking click, resolved by *what* the target is (the only meaning a click carries): a **Back**
/// card goes up a zone; an **Exit** card quits; an expandable **card** grows/shrinks; a loose action
/// fires; a **pile** is entered (its zone). Inner nodes (a card's text) match nothing and propagate to
/// their parent. Global observer, so it survives the per-change UI rebuild.
#[allow(clippy::type_complexity)]
fn on_click(
    mut on: On<Pointer<Click>>,
    guard: Res<DragGuard>,
    targets: Query<(
        Option<&ActionControl>,
        Option<&CardRef>,
        Option<&PileDropZone>,
        Has<BackCard>,
        Has<ExitCard>,
    )>,
    mut table: ResMut<Table>,
    mut requests: ResMut<ActionRequests>,
    mut rebuild: ResMut<NeedsRebuild>,
    mut exit: MessageWriter<AppExit>,
) {
    if guard.0 {
        return; // the release that ends a drag also fires Click — that's not an intentional click
    }
    let Ok((action, card, pile, is_back, is_exit)) = targets.get(on.event().entity) else {
        return;
    };
    if is_back {
        table.0.zoom_out(); // leave this zone for its parent
        rebuild.0 = true;
    } else if is_exit {
        exit.write(AppExit::Success);
    } else if let Some(card_ref) = card {
        // A card click first tries to grow/shrink it (cycle render size); an expandable card consumes
        // the click that way. Otherwise an actionable card fires its action; a name-only card absorbs it.
        let id = card_ref.0;
        if table.0.card(id).is_some_and(|c| c.is_expandable()) {
            let _ = table.0.cycle_card_size(id);
            rebuild.0 = true;
        } else if let Some(action) = action {
            requests.0.push(action.0);
        }
    } else if let Some(action) = action {
        requests.0.push(action.0); // a loose action (rail item)
    } else if let Some(pile) = pile {
        let _ = table.0.focus(pile.0); // drill in: this pile becomes the current zone
        rebuild.0 = true;
    } else {
        return; // background / inert — nothing to do (navigation is via cards, not the felt)
    }
    on.propagate(false);
}

/// A picking drop: move a dragged **card** into the pile it was dropped *onto*. Dropping a card onto
/// another card (or the felt) is not a move — that's an in-zone reorder, handled by [`on_card_drag_end`]
/// against the grid. Piles aren't nested on drop (they reposition via [`on_pile_drag`]), so a dragged
/// pile is ignored. Presentation-level; mapping drops to game actions is future work.
fn on_drop(
    mut on: On<Pointer<DragDrop>>,
    cards: Query<&CardRef>,
    piles: Query<&PileDropZone>,
    mut table: ResMut<Table>,
    mut rebuild: ResMut<NeedsRebuild>,
) {
    let event = on.event();
    let Ok(dragged) = cards.get(event.event.dropped) else {
        return; // only cards drop *into* piles
    };
    let dest = if let Ok(zone) = piles.get(event.entity) {
        zone.0
    } else {
        return; // dropped onto a card or the felt — in-zone reordering is handled by the grid
    };
    on.propagate(false);
    let at = table.0.pile(dest).map_or(0, |pile| pile.cards().len());
    let _ = table.0.move_card(dragged.0, dest, at);
    rebuild.0 = true;
}

/// Slide a top-level pile across the table while it is dragged — freely, even off the edge. Moves the
/// wrapper's `Node` and the model position together (a position change is not structural, so there is
/// no rebuild mid-drag); settling on release brings an off-edge pile back. A card drag is consumed
/// here so it doesn't also slide the pile under it.
fn on_pile_drag(
    mut on: On<Pointer<Drag>>,
    mut piles: Query<(&TablePile, &mut Node)>,
    mut table: ResMut<Table>,
) {
    if let Ok((pile, mut node)) = piles.get_mut(on.event().entity) {
        let delta = on.event().event.delta;
        let (x, y) = (px(node.left) + delta.x, px(node.top) + delta.y);
        // Follow the cursor anywhere — even past the table edge. The settling on release clamps it
        // back inside and the animation slides it into view. Keep the model in step with the live
        // node so the animation doesn't fight the drag.
        node.left = Val::Px(x);
        node.top = Val::Px(y);
        let _ = table.0.set_pile_pos(pile.0, x, y);
        on.propagate(false);
    }
}

/// Commit a dragged pile's final position to the model on release (one rebuild, at rest).
fn on_pile_drag_end(
    mut on: On<Pointer<DragEnd>>,
    piles: Query<(&TablePile, &Node)>,
    mut table: ResMut<Table>,
    mut guard: ResMut<DragGuard>,
) {
    guard.0 = false; // the drag is over; let real clicks through again
    if let Ok((pile, node)) = piles.get(on.event().entity) {
        let _ = table.0.set_pile_pos(pile.0, px(node.left), px(node.top));
        // Settle: clamp the (possibly off-edge) pile back inside and shove overlaps clear — the
        // anchor included, so a pile dropped past the border is pulled into view, then the animation
        // slides it the rest of the way.
        table.0.separate(pile.0);
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

/// Feed each top-level pile's laid-out size back into the model (logical px), so [`Tableau::separate`]
/// works on real AABBs. Runs every frame; pile sizes are stable, so it's cheap.
fn sync_pile_sizes(piles: Query<(&TablePile, &ComputedNode)>, mut table: ResMut<Table>) {
    for (pile, computed) in &piles {
        let size = computed.size * computed.inverse_scale_factor;
        let _ = table.0.set_pile_size(pile.0, size.x, size.y);
    }
}

/// Feed the table surface's laid-out size to the model as the wall bounds that contain the piles.
fn sync_surface_size(surfaces: Query<&ComputedNode, With<TableSurface>>, mut table: ResMut<Table>) {
    if let Ok(computed) = surfaces.single() {
        let size = computed.size * computed.inverse_scale_factor;
        table.0.set_surface(size.x, size.y);
    }
}

/// Ease each pile's wrapper toward its model position, so a separation (or any reposition) *slides*
/// into place instead of snapping. The dragged pile keeps target == position, so it doesn't ease;
/// piles already at rest are skipped so the node (and its layout) isn't touched every frame.
fn animate_piles(time: Res<Time>, table: Res<Table>, mut piles: Query<(&TablePile, &mut Node)>) {
    let t = (SLIDE_SPEED * time.delta_secs()).min(1.0);
    for (pile, mut node) in &mut piles {
        let Some(d) = table.0.pile(pile.0) else {
            continue;
        };
        let target = d.pos();
        let (cx, cy) = (px(node.left), px(node.top));
        if (target.x - cx).abs() < 0.5 && (target.y - cy).abs() < 0.5 {
            continue; // at rest
        }
        node.left = Val::Px(cx + (target.x - cx) * t);
        node.top = Val::Px(cy + (target.y - cy) * t);
    }
}

/// Slide a card freely while it is dragged — the tile follows the cursor anywhere, no rebuild. The
/// grab lands on the inner card visual; the event propagates up to the `TableCard` tile, which is the
/// node we actually move. Marking it the dragging card stops [`animate_cards`] from fighting the drag.
fn on_card_drag(
    mut on: On<Pointer<Drag>>,
    mut cards: Query<(&TableCard, &mut Node)>,
    mut dragging: ResMut<DraggingCard>,
) {
    if let Ok((card, mut node)) = cards.get_mut(on.event().entity) {
        let delta = on.event().event.delta;
        node.left = Val::Px(px(node.left) + delta.x);
        node.top = Val::Px(px(node.top) + delta.y);
        dragging.0 = Some(card.0);
        on.propagate(false);
    }
}

/// On release, snap a dragged card to the nearest grid cell by reordering it within its home pile; the
/// other cards then *slide* to their new cells ([`animate_cards`]). Reordering is not structural, so
/// there is no rebuild — that would kill the slide.
fn on_card_drag_end(
    mut on: On<Pointer<DragEnd>>,
    cards: Query<(&TableCard, &Node)>,
    mut table: ResMut<Table>,
    mut dragging: ResMut<DraggingCard>,
) {
    if let Ok((card, node)) = cards.get(on.event().entity) {
        on.propagate(false);
        dragging.0 = None;
        let cols = grid_cols(table.0.surface().x);
        // Nearest cell from the tile's dropped centre.
        let col = (((px(node.left) + CARD_W / 2.0) / (CARD_W + GRID_GAP))
            .floor()
            .max(0.0) as usize)
            .min(cols - 1);
        let row = ((px(node.top) + CARD_H / 2.0) / (CARD_H + GRID_GAP))
            .floor()
            .max(0.0) as usize;
        let (Some(home), Some(from)) = (
            table.0.card(card.0).map(|c| c.home()),
            table.0.card_index(card.0),
        ) else {
            return;
        };
        let len = table.0.pile(home).map_or(0, |pile| pile.cards().len());
        let to = (row * cols + col).min(len.saturating_sub(1));
        let _ = table.0.reorder(home, from, to);
    }
}

/// Ease each card tile toward the grid cell its index in the pile names — so a reorder *slides* the
/// other cards into their new cells instead of snapping. The card being dragged is left alone (it
/// follows the cursor); tiles already at rest are skipped so layout isn't touched every frame.
fn animate_cards(
    time: Res<Time>,
    table: Res<Table>,
    dragging: Res<DraggingCard>,
    mut cards: Query<(&TableCard, &mut Node)>,
) {
    let cols = grid_cols(table.0.surface().x);
    let t = (SLIDE_SPEED * time.delta_secs()).min(1.0);
    for (card, mut node) in &mut cards {
        if dragging.0 == Some(card.0) {
            continue; // free while held
        }
        let Some(index) = table.0.card_index(card.0) else {
            continue;
        };
        let (tx, ty) = grid_cell(index, cols);
        let (cx, cy) = (px(node.left), px(node.top));
        if (tx - cx).abs() < 0.5 && (ty - cy).abs() < 0.5 {
            continue; // at rest
        }
        node.left = Val::Px(cx + (tx - cx) * t);
        node.top = Val::Px(cy + (ty - cy) * t);
    }
}

/// Rebuild the whole UI only on a *structural* change (open/close a pile, move a card, a new game
/// snapshot). Pile positions are not structural — they animate (see [`animate_piles`]) — so
/// repositioning never triggers a rebuild.
fn redraw(
    mut commands: Commands,
    mut rebuild: ResMut<NeedsRebuild>,
    table: Res<Table>,
    rail: Res<ActionRail>,
    status: Res<StatusLine>,
    roots: Query<Entity, With<CardTableRoot>>,
) {
    if !rebuild.0 {
        return;
    }
    rebuild.0 = false;
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    build_ui(&mut commands, &table.0, &rail.0, &status.0);
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
/// Highlight edge for a card/pile that carries a legal move.
const ACTIONABLE: Color = Color::srgb(0.30, 0.70, 0.62);
/// A dark edge around every card so overlapping cards stay distinct.
const CARD_EDGE: Color = Color::srgb(0.12, 0.11, 0.10);
/// Soft drop shadow lifting cards and piles off the felt.
const SHADOW: Color = Color::srgba(0.0, 0.0, 0.0, 0.35);

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

const FONT_DISPLAY: FontSize = FontSize::Px(26.0);
const FONT_HEAD: FontSize = FontSize::Px(18.0);
const FONT_TITLE: FontSize = FontSize::Px(15.0);
const FONT_BODY: FontSize = FontSize::Px(13.0);

/// How fast a pile eases toward its target position, as a fraction closed per second (higher = snappier).
const SLIDE_SPEED: f32 = 12.0;

/// A collapsed pile's front-face footprint, the per-card stack step (offset along two edges), and the
/// visual depth cap so a deep pile doesn't grow without bound.
const CHIP_W: f32 = 120.0;
const CHIP_H: f32 = 64.0;
const STACK_OFFSET: f32 = 2.0;
const MAX_STACK: usize = 10;

/// A card's footprint and the gap between grid cells in a drilled zone. A grid cell is card+gap.
const CARD_W: f32 = 96.0;
const CARD_H: f32 = 132.0;
const GRID_GAP: f32 = 14.0;
/// Cap on grid columns, so the first frame (before the real surface size is known) doesn't lay every
/// card in one enormous row.
const MAX_COLS: usize = 16;

/// How many columns the card grid uses for a surface `width` (at least one, capped).
fn grid_cols(width: f32) -> usize {
    (((width / (CARD_W + GRID_GAP)).floor()) as usize).clamp(1, MAX_COLS)
}

/// The top-left position of grid cell `index` in a grid of `cols` columns (row-major).
fn grid_cell(index: usize, cols: usize) -> (f32, f32) {
    let col = index % cols;
    let row = index / cols;
    (
        col as f32 * (CARD_W + GRID_GAP),
        row as f32 * (CARD_H + GRID_GAP),
    )
}

fn build_ui(commands: &mut Commands, tree: &Tableau, rail: &[RailAction], status: &str) {
    let zone = tree.focus_id();
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
            // HEADER: the current zone's name, centered, with an optional caption beneath it.
            root.spawn(Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(0.0), Val::Px(10.0)),
                row_gap: Val::Px(2.0),
                ..default()
            })
            .with_children(|head| {
                head.spawn((
                    Text::new(pile_display_name(tree, zone)),
                    TextFont {
                        font_size: FONT_DISPLAY,
                        ..default()
                    },
                    TextColor(INK),
                ));
                if !status.is_empty() {
                    head.spawn((
                        Text::new(status.to_string()),
                        TextFont {
                            font_size: FONT_BODY,
                            ..default()
                        },
                        TextColor(MUTED),
                    ));
                }
            });

            // NAV: utility cards (Back when inside a zone, Exit on desktop) plus any loose actions.
            root.spawn(Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(10.0),
                padding: UiRect::all(Val::Px(10.0)),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|nav| {
                if !at_root {
                    spawn_nav_card(nav, BackCard, "Back");
                }
                if cfg!(not(target_arch = "wasm32")) {
                    spawn_nav_card(nav, ExitCard, "Exit");
                }
                for action in rail {
                    spawn_rail_button(nav, action);
                }
            });

            // SURFACE: the zone's contents. At the Table (root) zone, piles are freely positioned and
            // draggable; inside a pile's zone, its cards (and any sub-piles) flow in a wrapping grid.
            root.spawn((
                TableSurface,
                Node {
                    width: Val::Percent(100.0),
                    flex_grow: 1.0,
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
            ))
            .with_children(|surface| {
                let pile = tree.pile(zone).expect("zone pile exists");
                if at_root {
                    for &id in pile.subpiles() {
                        let pos = tree.pile(id).expect("pile id from zone").pos();
                        surface
                            .spawn((
                                TablePile(id),
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(pos.x),
                                    top: Val::Px(pos.y),
                                    ..default()
                                },
                            ))
                            .with_children(|wrapper| spawn_pile(wrapper, tree, id));
                    }
                } else {
                    // The zone's cards lay out in a row-major grid; each is its own draggable tile that
                    // reorders (others reflow) on drop. ×N grouping doesn't apply here — every card is a
                    // stable tile so the reflow can animate smoothly.
                    let cols = grid_cols(tree.surface().x);
                    for (index, &cid) in pile.cards().iter().enumerate() {
                        let (x, y) = grid_cell(index, cols);
                        surface
                            .spawn((
                                TableCard(cid),
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(x),
                                    top: Val::Px(y),
                                    ..default()
                                },
                            ))
                            .with_children(|tile| {
                                spawn_card(tile, tree.card(cid).expect("card id from zone"));
                            });
                    }
                    // Any sub-piles follow the cards in the grid as (clickable) chips.
                    let base = pile.cards().len();
                    for (k, &sid) in pile.subpiles().iter().enumerate() {
                        let (x, y) = grid_cell(base + k, cols);
                        surface
                            .spawn(Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(x),
                                top: Val::Px(y),
                                ..default()
                            })
                            .with_children(|wrapper| spawn_pile(wrapper, tree, sid));
                    }
                }
            });
        });
}

/// The display name of a pile/zone: "Table" for the root; otherwise the name of its top card when that
/// card's job is to name it (a [`CardKind::Zone`] card), else the pile's own label.
fn pile_display_name(tree: &Tableau, id: PileId) -> String {
    if id == tree.root_id() {
        return "Table".to_string();
    }
    let pile = tree.pile(id).expect("pile id");
    if let Some(&top) = pile.cards().last()
        && let Some(card) = tree.card(top)
        && matches!(card.kind(), CardKind::Zone)
    {
        return card.name().to_string();
    }
    pile.label.clone()
}

/// A utility card (Back / Exit) drawn in the nav row — a small card-styled, clickable control.
fn spawn_nav_card<M: Component>(parent: &mut ChildSpawnerCommands, marker: M, label: &str) {
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

/// Draws a collapsed pile as a short stack of offset layers — two alternating colors, stepped along
/// the left and bottom edges, capped at [`MAX_STACK`] — hinting at how many cards are inside. The
/// front layer (top-right, on top) carries the label and count; the whole stack is one drop target.
fn spawn_pile_chip(parent: &mut ChildSpawnerCommands, id: PileId, label: &str, count: usize) {
    let depth = count.clamp(1, MAX_STACK);
    let spread = (depth - 1) as f32 * STACK_OFFSET;
    parent
        .spawn((
            PileDropZone(id),
            Node {
                width: Val::Px(CHIP_W + spread),
                height: Val::Px(CHIP_H + spread),
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
                        width: Val::Px(CHIP_W),
                        height: Val::Px(CHIP_H),
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
                    stack
                        .spawn(bundle)
                        .insert(card_shadow())
                        .with_children(|face| {
                            face.spawn((
                                Text::new(label.to_string()),
                                TextFont {
                                    font_size: FONT_TITLE,
                                    ..default()
                                },
                                TextColor(INK),
                            ));
                            face.spawn((
                                Text::new(format!("{count} cards")),
                                TextFont {
                                    font_size: FONT_BODY,
                                    ..default()
                                },
                                TextColor(MUTED),
                            ));
                        });
                } else {
                    stack.spawn(bundle);
                }
            }
        });
}

/// Draws a pile as a compact, counted chip showing its display name. You see its *contents* by
/// clicking it to enter its zone — piles no longer fan open in place.
fn spawn_pile(parent: &mut ChildSpawnerCommands, tree: &Tableau, id: PileId) {
    let pile = tree.pile(id).expect("pile id from tree");
    let count = pile.cards().len() + pile.subpiles().len();
    spawn_pile_chip(parent, id, &pile_display_name(tree, id), count);
}

/// Draws one card at its current render [`Size`]: a small name chip, a detailed card face, or a full
/// utility panel. Every form carries `CardRef`, so a click can grow/shrink it.
fn spawn_card(parent: &mut ChildSpawnerCommands, card: &Card) {
    match card.size() {
        Size::Name => spawn_card_name(parent, card, 1),
        Size::Card => spawn_card_detail(parent, card),
        Size::Full => spawn_card_full(parent, card),
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

/// Smallest form — a 96×132 card showing just the name (or a blank back when face down), plus a `×N`
/// quantity beneath it when `quantity > 1` (several identical cards stacked into one chip).
fn spawn_card_name(parent: &mut ChildSpawnerCommands, card: &Card, quantity: usize) {
    let (label, bg, ink) = match &card.face {
        Face::Up { title } => (Some(title.clone()), CARD_FACE, CARD_INK),
        Face::Down => (None, CARD_BACK, INK),
    };
    let entity = parent.spawn((
        CardRef(card.id),
        Node {
            width: Val::Px(96.0),
            height: Val::Px(132.0),
            padding: UiRect::all(Val::Px(8.0)),
            border: UiRect::all(Val::Px(2.0)),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(2.0),
            border_radius: BorderRadius::all(Val::Px(12.0)),
            ..default()
        },
        BackgroundColor(bg),
        BorderColor::all(card_edge(card)),
        card_shadow(),
    ));
    finish_card(entity, card, |c| {
        if let Some(label) = label {
            c.spawn((
                Text::new(label),
                TextFont {
                    font_size: FONT_TITLE,
                    ..default()
                },
                TextColor(ink),
            ));
        }
        if quantity > 1 {
            c.spawn((
                Text::new(format!("×{quantity}")),
                TextFont {
                    font_size: FONT_TITLE,
                    ..default()
                },
                TextColor(ink),
            ));
        }
    });
}

/// Medium form — a card face: a name header above its detail (stat / rules) lines.
fn spawn_card_detail(parent: &mut ChildSpawnerCommands, card: &Card) {
    let entity = parent.spawn((
        CardRef(card.id),
        Node {
            width: Val::Px(200.0),
            min_height: Val::Px(132.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(10.0)),
            border: UiRect::all(Val::Px(2.0)),
            row_gap: Val::Px(4.0),
            border_radius: BorderRadius::all(Val::Px(12.0)),
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
                font_size: FONT_HEAD,
                ..default()
            },
            TextColor(CARD_INK),
        ));
        for line in card.detail() {
            c.spawn((
                Text::new(line.clone()),
                TextFont {
                    font_size: FONT_BODY,
                    ..default()
                },
                TextColor(CARD_INK),
            ));
        }
    });
}

/// Largest form — a utility panel (e.g. a combat log): a name header above its panel lines, scrollable.
fn spawn_card_full(parent: &mut ChildSpawnerCommands, card: &Card) {
    let entity = parent.spawn((
        CardRef(card.id),
        Node {
            width: Val::Px(320.0),
            max_height: Val::Px(360.0),
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
    finish_card(entity, card, |c| {
        c.spawn((
            Text::new(card.name().to_string()),
            TextFont {
                font_size: FONT_HEAD,
                ..default()
            },
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

    use cardtable_model::{Tableau, from_table_view};
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
    fn snapshot<G: Game>(game: &G, state: &G::State) -> (Tableau, Vec<RailAction>, String) {
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
