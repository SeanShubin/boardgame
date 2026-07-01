//! A Bevy renderer that draws the **card-table metaphor** — everything is a card; a pile is a stack of
//! cards in one footprint. You navigate with **single-click and drag only**: click a pile to drill into
//! its zone, click a card to grow it through its sizes, click the Back card to move up, and drag piles
//! to arrange them on the table. **System** is itself a pile on the felt — drag it like any other; to
//! quit, press it so its "Exit" card pops out beside it, then drag the deck onto that card. A stray
//! click never quits. The current zone's name sits centered at the top (default "Table").
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

use bevy::picking::events::{Click, Drag, DragDrop, DragEnd, DragStart, Pointer, Press, Release};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::ui::{BoxShadow, ComputedNode};

use std::collections::HashMap;

use cardtable_model::{
    Arrangement, Card, CardId, CardKind, Face, Layout, PileId, Pos, Size, Tableau, Utility,
};

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
            .init_resource::<ActionsDeckState>()
            .init_resource::<InitialTable>()
            .insert_resource(NeedsRebuild(true))
            .configure_sets(
                Update,
                (CardTableSet::Input, CardTableSet::Apply, CardTableSet::Draw).chain(),
            )
            .add_systems(Startup, (setup_camera, install_ui_font))
            // Inject the System deck, then snapshot the initial table for Reset (order matters).
            .add_systems(Startup, (inject_system_deck, snapshot_initial).chain())
            .add_systems(
                Update,
                (
                    sync_pile_sizes,
                    sync_surface_size,
                    animate_piles,
                    animate_cards,
                    animate_popped,
                ),
            )
            // Free-deck shove: sync card footprints, then re-settle when one changes (lay-out / resize).
            .add_systems(Update, (sync_card_sizes, settle_free_cards).chain())
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
            .add_observer(on_card_drag_end)
            .add_observer(on_actions_press)
            .add_observer(on_actions_release)
            .add_observer(on_actions_drag_end);
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

/// A popped-out action card spawned beside a pressed [`Arrangement::Actions`] deck — a *free* surface
/// entity (not a model pile, so popping it never shoves the game piles), drawn above everything, that
/// [`animate_popped`] slides into place and the deck is dropped onto to fire. Carries the spot it eases toward.
#[derive(Component)]
struct PoppedTarget {
    target: Pos,
}

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

/// The initial table, snapshotted once at startup so a **Reset** action can restore it. Game-agnostic:
/// whatever was in [`Table`] after setup (fixture or game view, plus the injected System deck).
#[derive(Resource, Default)]
struct InitialTable(Tableau);

/// One card popped out from a pressed [`Arrangement::Actions`] deck: the [`Utility`] it fires, the
/// rectangle it occupies (for the drop hit-test), and its spawned surface entity.
struct PoppedAction {
    utility: Utility,
    pos: Pos,
    size: Pos,
    entity: Entity,
}

/// Live state of the pressed **Actions** deck (e.g. System). While pressed, each of its content cards is
/// popped out as a [`PoppedAction`]; on release the deck fires the action of whichever popped card it
/// overlaps. All of it clears when the gesture ends or the UI rebuilds.
#[derive(Resource, Default)]
struct ActionsDeckState {
    pressed_pile: Option<PileId>,
    popped: Vec<PoppedAction>,
}

// ---- systems ------------------------------------------------------------

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// Inject the **System deck** — an [`Arrangement::Actions`] pile on the surface: press it to slide out
/// its action cards, then drag the deck onto one to fire it (see [`on_actions_press`]). It holds
/// **Reset** everywhere and **Exit** on desktop only — a browser can't quit its own tab, so the Exit
/// card never appears there. Runs once at startup.
fn inject_system_deck(mut table: ResMut<Table>) {
    let root = table.0.root_id();
    let Ok(pile) = table.0.add_pile(root, "System") else {
        return;
    };
    // Action cards sit under the "System" label. Reset is available everywhere; Exit is desktop-only.
    if let Ok(reset) = table.0.add_card(
        pile,
        Face::Up {
            title: "Reset".into(),
        },
        None,
    ) {
        let _ = table
            .0
            .set_card_kind(reset, CardKind::Utility(Utility::Reset));
    }
    if !cfg!(target_arch = "wasm32")
        && let Ok(exit) = table.0.add_card(
            pile,
            Face::Up {
                title: "Exit".into(),
            },
            None,
        )
    {
        let _ = table
            .0
            .set_card_kind(exit, CardKind::Utility(Utility::Exit));
    }
    // "System" is a Zone (naming) card — the deck's label, not one of its actions.
    if let Ok(system) = table.0.add_card(
        pile,
        Face::Up {
            title: "System".into(),
        },
        None,
    ) {
        let _ = table.0.set_card_kind(system, CardKind::Zone);
    }
    let _ = table.0.set_layout(
        pile,
        Layout {
            arrangement: Arrangement::Actions,
            editable: false,
        },
    );
    let _ = table.0.set_pile_pos(pile, 40.0, 470.0);
}

/// Snapshot the fully-initialised table (after [`inject_system_deck`]) so a **Reset** can restore it.
fn snapshot_initial(table: Res<Table>, mut initial: ResMut<InitialTable>) {
    initial.0 = table.0.clone();
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
/// card goes up a zone; an expandable **card** grows/shrinks; a loose action fires; a **pile** is entered
/// (its zone) — unless it is an [`Arrangement::Actions`] deck (press-driven, see [`on_actions_press`]) or
/// has nothing under its label to show. Inner nodes (a card's text) match nothing and propagate to their
/// parent. Global observer, so it survives the per-change UI rebuild.
#[allow(clippy::type_complexity)]
fn on_click(
    mut on: On<Pointer<Click>>,
    guard: Res<DragGuard>,
    targets: Query<(
        Option<&ActionControl>,
        Option<&CardRef>,
        Option<&PileDropZone>,
        Has<BackCard>,
    )>,
    mut table: ResMut<Table>,
    mut requests: ResMut<ActionRequests>,
    mut rebuild: ResMut<NeedsRebuild>,
) {
    if guard.0 {
        return; // the release that ends a drag also fires Click — that's not an intentional click
    }
    let Ok((action, card, pile, is_back)) = targets.get(on.event().entity) else {
        return;
    };
    if is_back {
        table.0.zoom_out(); // leave this zone for its parent
        rebuild.0 = true;
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
        let id = pile.0;
        // An Actions deck is press-driven (its slide-out menu), not click-to-drill; and a deck with
        // nothing under its label has nothing to show. Either way, a click does not drill in.
        let arrangement = table.0.pile(id).map(|p| p.layout().arrangement);
        let nothing_under = table.0.content_cards(id).is_empty()
            && table.0.pile(id).is_some_and(|p| p.subpiles().is_empty());
        if !matches!(arrangement, Some(Arrangement::Actions)) && !nothing_under {
            let _ = table.0.focus(id); // drill in: this pile becomes the current zone
            rebuild.0 = true;
        }
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
    mut table: ResMut<Table>,
) {
    if let Ok((card, mut node)) = cards.get_mut(on.event().entity) {
        let delta = on.event().event.delta;
        let (x, y) = (px(node.left) + delta.x, px(node.top) + delta.y);
        node.left = Val::Px(x);
        node.top = Val::Px(y);
        // Keep the model position in step — a Free deck reads it to shove and to animate at rest.
        let _ = table.0.set_card_pos(card.0, x, y);
        dragging.0 = Some(card.0);
        on.propagate(false);
    }
}

/// On release: a **Free** deck commits the dropped position and shoves overlapping cards clear
/// ([`separate_cards`]); any other layout snaps the card into the nearest grid cell by reordering. In
/// both cases the others then *slide* into place ([`animate_cards`]) — no rebuild, which would kill the slide.
fn on_card_drag_end(
    mut on: On<Pointer<DragEnd>>,
    cards: Query<(&TableCard, &Node)>,
    mut table: ResMut<Table>,
    mut dragging: ResMut<DraggingCard>,
) {
    if let Ok((card, node)) = cards.get(on.event().entity) {
        on.propagate(false);
        dragging.0 = None;
        let Some(home) = table.0.card(card.0).map(|c| c.home()) else {
            return;
        };
        if matches!(
            table.0.pile(home).map(|p| p.layout().arrangement),
            Some(Arrangement::Free)
        ) {
            // Unordered: keep it where dropped, then shove the rest out of its way.
            let _ = table.0.set_card_pos(card.0, px(node.left), px(node.top));
            table.0.separate_cards(home, card.0);
            return;
        }
        // Ordered grid: snap into the nearest cell by reordering among the *contents* only, so a drag
        // can never push a card above a zone card and steal its place as the pile's label.
        let cols = zone_cols(&table.0);
        let col = (((px(node.left) + SMALL_W / 2.0) / (SMALL_W + GRID_GAP))
            .floor()
            .max(0.0) as usize)
            .min(cols - 1);
        let row = ((px(node.top) + SMALL_H / 2.0) / (SMALL_H + GRID_GAP))
            .floor()
            .max(0.0) as usize;
        let Some(from) = table.0.card_index(card.0) else {
            return;
        };
        let len = table.0.content_cards(home).len();
        let to = (row * cols + col).min(len.saturating_sub(1));
        let _ = table.0.reorder(home, from, to);
    }
}

/// Press an [`Arrangement::Actions`] deck (e.g. System) to slide its action cards out beside it, arming
/// them. While held, drag the deck onto one to fire it; letting go without reaching one just tucks them
/// away (see [`settle_actions_deck`]), so a click never fires an action. The popped cards are free
/// surface entities drawn above the piles, since popping them doesn't shove the game piles aside.
fn on_actions_press(
    on: On<Pointer<Press>>,
    piles: Query<&TablePile>,
    surfaces: Query<Entity, With<TableSurface>>,
    table: Res<Table>,
    mut state: ResMut<ActionsDeckState>,
    mut commands: Commands,
) {
    if on.event().event.button != PointerButton::Primary {
        return;
    }
    let Ok(pile) = piles.get(on.event().entity) else {
        return; // press wasn't on a top-level pile
    };
    let Some(deck) = table.0.pile(pile.0) else {
        return;
    };
    if state.pressed_pile.is_some() || deck.layout().arrangement != Arrangement::Actions {
        return; // already popped, or not an Actions deck
    }
    // The cards to pop: each content card that carries a Utility action.
    let actions: Vec<(Utility, String)> = table
        .0
        .content_cards(pile.0)
        .iter()
        .filter_map(|&cid| match table.0.card(cid)?.kind() {
            CardKind::Utility(utility) => Some((utility, table.0.card(cid)?.name().to_string())),
            _ => None,
        })
        .collect();
    let Ok(surface_e) = surfaces.single() else {
        return;
    };
    if actions.is_empty() {
        return;
    }
    let (pos, size) = (deck.pos(), deck.size());
    let surface = table.0.surface();
    let card_size = Pos {
        x: LEAVE_W,
        y: LEAVE_H,
    };
    // Stack the menu below the deck — or above it if there is no room below — clamped to the surface.
    let menu_h = actions.len() as f32 * (card_size.y + LEAVE_GAP);
    let below = pos.y + size.y + LEAVE_GAP;
    let start_y = if below + menu_h <= surface.y {
        below
    } else {
        (pos.y - LEAVE_GAP - menu_h).max(0.0)
    };
    state.pressed_pile = Some(pile.0);
    for (i, (utility, label)) in actions.into_iter().enumerate() {
        let target = Pos {
            x: pos.x,
            y: start_y + i as f32 * (card_size.y + LEAVE_GAP),
        };
        let entity = spawn_popped_card(
            &mut commands,
            pos,
            target,
            card_size,
            &label,
            action_color(utility),
        );
        commands.entity(surface_e).add_child(entity);
        state.popped.push(PoppedAction {
            utility,
            pos: target,
            size: card_size,
            entity,
        });
    }
}

/// The fill colour for a popped action card, by what it does.
fn action_color(utility: Utility) -> Color {
    match utility {
        Utility::Exit => EXIT_CONFIRM_BG, // warm red — "this is the way out"
        Utility::Reset => Color::srgb(0.28, 0.42, 0.60), // blue
        Utility::Back => Color::srgb(0.30, 0.40, 0.45),
    }
}

/// On a primary release, settle the Actions deck (handles a press let go without reaching a card).
#[allow(clippy::too_many_arguments)]
fn on_actions_release(
    on: On<Pointer<Release>>,
    mut state: ResMut<ActionsDeckState>,
    mut table: ResMut<Table>,
    initial: Res<InitialTable>,
    mut rebuild: ResMut<NeedsRebuild>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    if on.event().event.button == PointerButton::Primary {
        settle_actions_deck(
            &mut state,
            &mut table,
            &initial.0,
            &mut rebuild,
            &mut commands,
            &mut exit,
        );
    }
}

/// The drag counterpart of [`on_actions_release`]: when any drag ends (including off-window, where
/// `Release` may not fire), settle the Actions deck.
fn on_actions_drag_end(
    _on: On<Pointer<DragEnd>>,
    mut state: ResMut<ActionsDeckState>,
    mut table: ResMut<Table>,
    initial: Res<InitialTable>,
    mut rebuild: ResMut<NeedsRebuild>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    settle_actions_deck(
        &mut state,
        &mut table,
        &initial.0,
        &mut rebuild,
        &mut commands,
        &mut exit,
    );
}

/// Settle a pressed Actions deck once the press/drag ends: fire the action of whichever popped card the
/// deck overlaps (Exit quits; Reset restores the initial table), then despawn the popped cards and
/// disarm. Called from both the release and drag-end paths — whichever fires first does the work, the
/// other finds `pressed_pile == None` and no-ops — so the outcome doesn't depend on their ordering.
fn settle_actions_deck(
    state: &mut ActionsDeckState,
    table: &mut Table,
    initial: &Tableau,
    rebuild: &mut NeedsRebuild,
    commands: &mut Commands,
    exit: &mut MessageWriter<AppExit>,
) {
    let Some(pile) = state.pressed_pile.take() else {
        return;
    };
    let fired = table.0.pile(pile).and_then(|deck| {
        let (dp, dsz) = (deck.pos(), deck.size());
        state
            .popped
            .iter()
            .find(|p| rects_overlap(dp, dsz, p.pos, p.size))
            .map(|p| p.utility)
    });
    for popped in state.popped.drain(..) {
        commands.entity(popped.entity).despawn();
    }
    match fired {
        Some(Utility::Exit) => {
            exit.write(AppExit::Success);
        }
        Some(Utility::Reset) => {
            table.0 = initial.clone();
            rebuild.0 = true;
        }
        Some(Utility::Back) => {
            table.0.zoom_out();
            rebuild.0 = true;
        }
        None => {}
    }
}

/// Whether two AABBs (top-left `pos`, `size`) overlap by more than a hair.
fn rects_overlap(ap: Pos, asz: Pos, bp: Pos, bsz: Pos) -> bool {
    let ox = (ap.x + asz.x).min(bp.x + bsz.x) - ap.x.max(bp.x);
    let oy = (ap.y + asz.y).min(bp.y + bsz.y) - ap.y.max(bp.y);
    ox > 0.01 && oy > 0.01
}

/// Ease each popped-out action card from the deck toward its target spot — the same eased settle the
/// table piles use. It only eases outward; on release it's despawned outright (see [`settle_actions_deck`]).
fn animate_popped(time: Res<Time>, mut popped: Query<(&PoppedTarget, &mut Node)>) {
    let t = (SLIDE_SPEED * time.delta_secs()).min(1.0);
    for (card, mut node) in &mut popped {
        let (cx, cy) = (px(node.left), px(node.top));
        let (tx, ty) = (card.target.x, card.target.y);
        if (tx - cx).abs() < 0.5 && (ty - cy).abs() < 0.5 {
            continue;
        }
        node.left = Val::Px(cx + (tx - cx) * t);
        node.top = Val::Px(cy + (ty - cy) * t);
    }
}

/// Ease each drilled-in card tile toward its target — its **grid cell** (ordered layouts) or its free
/// **model position** (a [`Arrangement::Free`] deck) — so a reorder or a shove *slides* the cards into
/// place instead of snapping. The dragged card is left alone (it follows the cursor); tiles already at
/// rest are skipped so layout isn't touched every frame.
fn animate_cards(
    time: Res<Time>,
    table: Res<Table>,
    dragging: Res<DraggingCard>,
    mut cards: Query<(&TableCard, &mut Node)>,
) {
    let free = matches!(
        table
            .0
            .pile(table.0.focus_id())
            .map(|p| p.layout().arrangement),
        Some(Arrangement::Free)
    );
    let cols = zone_cols(&table.0);
    let t = (SLIDE_SPEED * time.delta_secs()).min(1.0);
    for (card, mut node) in &mut cards {
        if dragging.0 == Some(card.0) {
            continue; // free while held
        }
        let (tx, ty) = if free {
            match table.0.card(card.0) {
                Some(c) => (c.pos().x, c.pos().y),
                None => continue,
            }
        } else {
            match table.0.card_index(card.0) {
                Some(index) => grid_cell(index, cols),
                None => continue,
            }
        };
        let (cx, cy) = (px(node.left), px(node.top));
        if (tx - cx).abs() < 0.5 && (ty - cy).abs() < 0.5 {
            continue; // at rest
        }
        node.left = Val::Px(cx + (tx - cx) * t);
        node.top = Val::Px(cy + (ty - cy) * t);
    }
}

/// Feed each drilled-in card tile's laid-out footprint back to the model (logical px), so a Free deck's
/// [`separate_cards`](Tableau::separate_cards) shoves on real AABBs. Runs every frame; cheap.
fn sync_card_sizes(cards: Query<(&TableCard, &ComputedNode)>, mut table: ResMut<Table>) {
    for (card, computed) in &cards {
        let size = computed.size * computed.inverse_scale_factor;
        let _ = table.0.set_card_footprint(card.0, size.x, size.y);
    }
}

/// Keep a **Free** deck's cards shoved apart when they first lay out or change size (a card expands or
/// collapses): when a card's footprint changes and nothing is being dragged, re-run [`separate_cards`]
/// anchored on the changed card — so a grown card holds its place and pushes its neighbours out. This
/// is what makes the shove trigger "whether expanded or not". `prev` remembers last-seen footprints.
fn settle_free_cards(
    mut table: ResMut<Table>,
    dragging: Res<DraggingCard>,
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
    let mut changed: Option<CardId> = None;
    for &c in &cards {
        let Some(footprint) = table.0.card(c).map(|k| k.footprint()) else {
            continue;
        };
        if footprint.x < 1.0 {
            continue; // not laid out yet
        }
        let was = prev.insert(c, footprint).unwrap_or_default();
        if (was.x - footprint.x).abs() > 0.5 || (was.y - footprint.y).abs() > 0.5 {
            changed = Some(c);
        }
    }
    if let Some(anchor) = changed {
        table.0.separate_cards(focus, anchor);
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
    mut actions_deck: ResMut<ActionsDeckState>,
    roots: Query<Entity, With<CardTableRoot>>,
) {
    if !rebuild.0 {
        return;
    }
    rebuild.0 = false;
    // The popped action cards are children of the surface we're about to despawn; forget them (and
    // cancel any in-flight gesture) so we never try to despawn a now-dead entity.
    actions_deck.popped.clear();
    actions_deck.pressed_pile = None;
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
/// The exit deck's popped-out "Leave" card — a warm red so the drop target reads as "this is the way out".
const EXIT_CONFIRM_BG: Color = Color::srgb(0.55, 0.22, 0.20);
/// Highlight edge for a card/pile that carries a legal move.
const ACTIONABLE: Color = Color::srgb(0.30, 0.70, 0.62);
/// A dark edge around every card so overlapping cards stay distinct.
const CARD_EDGE: Color = Color::srgb(0.12, 0.11, 0.10);
/// Soft drop shadow lifting cards and piles off the felt.
const SHADOW: Color = Color::srgba(0.0, 0.0, 0.0, 0.35);

/// The accent colour for a card **type** — a small designed palette for the common types, with a
/// stable hashed hue for any other type so a new type still reads as its own colour.
fn type_accent(card_type: &str) -> Color {
    match card_type.to_ascii_lowercase().as_str() {
        "location" => Color::srgb(0.36, 0.52, 0.34), // mossy green
        "adventurer" => Color::srgb(0.28, 0.46, 0.68), // heroic blue
        "hero" => Color::srgb(0.70, 0.32, 0.32),     // crimson
        "starter kit" => Color::srgb(0.28, 0.52, 0.52), // teal
        "ability" => Color::srgb(0.68, 0.36, 0.52),  // magenta
        "item" => Color::srgb(0.74, 0.58, 0.26),     // gold
        "log" => Color::srgb(0.44, 0.44, 0.52),      // slate
        "zone" => Color::srgb(0.50, 0.40, 0.62),     // violet — a structural / naming card
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

const FONT_DISPLAY: FontSize = FontSize::Px(26.0);
const FONT_HEAD: FontSize = FontSize::Px(18.0);
const FONT_TITLE: FontSize = FontSize::Px(15.0);
const FONT_BODY: FontSize = FontSize::Px(13.0);
/// The small type-badge caption.
const FONT_BADGE: FontSize = FontSize::Px(10.0);

/// How fast a pile eases toward its target position, as a fraction closed per second (higher = snappier).
const SLIDE_SPEED: f32 = 12.0;

/// The three planned **card footprints** (logical px). Every card, pile, and deck draws at one of
/// these — see [`Size`]. **Small** is the compact name+type form a deck and its contents share;
/// **Medium** is a full individual card face (adds detail lines); **Large** is a document / log panel.
const SMALL_W: f32 = 120.0;
const SMALL_H: f32 = 96.0;
const MEDIUM_W: f32 = 200.0;
const MEDIUM_MIN_H: f32 = 132.0;
const LARGE_W: f32 = 320.0;
const LARGE_MAX_H: f32 = 360.0;

/// The per-card stack step (offset along two edges) and the visual depth cap, so a deck reads as a
/// stack of Small cards without growing without bound.
const STACK_OFFSET: f32 = 2.0;
const MAX_STACK: usize = 10;

/// The popped-out "Leave" card's footprint and how far it sits from the Exit deck when popped.
const LEAVE_W: f32 = 120.0;
const LEAVE_H: f32 = 56.0;
const LEAVE_GAP: f32 = 14.0;

/// The gap between grid cells in a drilled zone. A grid cell is a Small card plus this gap.
const GRID_GAP: f32 = 14.0;
/// Cap on grid columns, so the first frame (before the real surface size is known) doesn't lay every
/// card in one enormous row.
const MAX_COLS: usize = 16;

/// How many columns the card grid uses for a surface `width` (at least one, capped).
fn grid_cols(width: f32) -> usize {
    (((width / (SMALL_W + GRID_GAP)).floor()) as usize).clamp(1, MAX_COLS)
}

/// Columns the **focused zone** lays its cards out in — the single source every layout path (draw,
/// drag-drop, animate) reads, so they always agree: a fixed count for a 2-D [`Arrangement::Grid`], or
/// a width-responsive count for a 1-D [`Arrangement::List`].
fn zone_cols(tree: &Tableau) -> usize {
    match tree.pile(tree.focus_id()).map(|p| p.layout().arrangement) {
        Some(Arrangement::Grid { columns }) => columns.max(1),
        _ => grid_cols(tree.surface().x),
    }
}

/// The top-left position of grid cell `index` in a grid of `cols` columns (row-major).
fn grid_cell(index: usize, cols: usize) -> (f32, f32) {
    let col = index % cols;
    let row = index / cols;
    (
        col as f32 * (SMALL_W + GRID_GAP),
        row as f32 * (SMALL_H + GRID_GAP),
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

            // NAV: the Back card (only inside a zone) plus any loose actions. Exit is no longer here —
            // it's a pile on the surface below (see `inject_exit_deck`).
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
                    // The zone lays its contents out — one shared path for every layout. An ordered
                    // layout (List / Grid) places cards on a row-major grid via `zone_cols`; a Free
                    // (unordered) deck places each card at its own model position and shoves overlaps.
                    // A zone card on top is the pile's label, not a content card (see `content_cards`).
                    let free = matches!(pile.layout().arrangement, Arrangement::Free);
                    // Free decks are drag-at-will; an ordered layout is draggable only when editable.
                    let draggable = free || pile.layout().editable;
                    let cols = zone_cols(tree);
                    let content = tree.content_cards(zone);
                    for (index, &cid) in content.iter().enumerate() {
                        let (x, y) = if free {
                            let p = tree.card(cid).map(|c| c.pos()).unwrap_or_default();
                            (p.x, p.y)
                        } else {
                            grid_cell(index, cols)
                        };
                        let mut tile = surface.spawn(Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(x),
                            top: Val::Px(y),
                            ..default()
                        });
                        if draggable {
                            tile.insert(TableCard(cid));
                        }
                        tile.with_children(|tile| {
                            spawn_card(tile, tree.card(cid).expect("card id from zone"));
                        });
                    }
                    // Any sub-piles follow the cards in the grid as (clickable) chips.
                    let base = content.len();
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

/// Spawn a popped-out action card (`label`, `bg`) as a free entity on the surface, starting at the deck
/// (`from`) so [`animate_popped`] can slide it out to `target`. A high [`GlobalZIndex`] keeps it above
/// every pile — since the pop-out doesn't shove the game piles aside, it must instead be drawn on top of
/// them. It's transparent to picking (the drop is detected by overlap geometry, not a hit-test).
fn spawn_popped_card(
    commands: &mut Commands,
    from: Pos,
    target: Pos,
    size: Pos,
    label: &str,
    bg: Color,
) -> Entity {
    commands
        .spawn((
            PoppedTarget { target },
            GlobalZIndex(100),
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(from.x),
                top: Val::Px(from.y),
                width: Val::Px(size.x),
                height: Val::Px(size.y),
                border: UiRect::all(Val::Px(2.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(bg),
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
                Pickable::IGNORE,
            ));
        })
        .id()
}

/// Draws a deck as a stack of **Small cards**: offset layers (two alternating colors, stepped along
/// the left and bottom edges, capped at [`MAX_STACK`]) hint at the depth, and the front layer is a
/// Small-card face ([`small_face`]) showing the top card's name, type, and count. The whole stack is
/// one drop target — a deck is a Small card wearing a stack.
fn spawn_pile_chip(
    parent: &mut ChildSpawnerCommands,
    id: PileId,
    label: &str,
    card_type: &str,
    count: usize,
) {
    let depth = count.clamp(1, MAX_STACK);
    let spread = (depth - 1) as f32 * STACK_OFFSET;
    parent
        .spawn((
            PileDropZone(id),
            Node {
                width: Val::Px(SMALL_W + spread),
                height: Val::Px(SMALL_H + spread),
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
                    // The front layer is a Small card face — the same [`small_face`] a lone card draws,
                    // with the pile's card count as its sub-line (omitted when empty, so a place with
                    // nothing under it reads as a plain named card).
                    let sub = (count > 0).then(|| format!("{count} cards"));
                    stack
                        .spawn(bundle)
                        .insert(card_shadow())
                        .with_children(|face| {
                            small_face(face, label, card_type, INK, sub);
                        });
                } else {
                    stack.spawn(bundle);
                }
            }
        });
}

/// Draws a pile as a compact, counted chip: the **name and type of its top card** over the card count.
/// You see its *contents* by clicking it to enter its zone — piles no longer fan open in place. A pile
/// whose top card is face-down (or that is empty) falls back to the pile's own display name, no type,
/// so a face-down deck reveals nothing.
fn spawn_pile(parent: &mut ChildSpawnerCommands, tree: &Tableau, id: PileId) {
    let pile = tree.pile(id).expect("pile id from tree");
    // Count the *contents*: a zone card on top is the label, not one of the cards it fronts.
    let count = tree.content_cards(id).len() + pile.subpiles().len();
    let top = pile.cards().last().and_then(|&cid| tree.card(cid));
    let (name, card_type) = match top {
        Some(card) if matches!(card.face, Face::Up { .. }) => {
            (card.name().to_string(), card.card_type().to_string())
        }
        _ => (pile_display_name(tree, id), String::new()),
    };
    spawn_pile_chip(parent, id, &name, &card_type, count);
}

/// Draws one card at its current render [`Size`]: **Small** (name + type), **Medium** (a full card
/// face with detail), or **Large** (a document / log panel). Every form carries `CardRef`, so a click
/// can grow/shrink it.
fn spawn_card(parent: &mut ChildSpawnerCommands, card: &Card) {
    match card.size() {
        Size::Small => spawn_card_small(parent, card, 1),
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
            font_size: FONT_TITLE,
            ..default()
        },
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
    let (label, bg, ink) = match &card.face {
        Face::Up { title } => (Some(title.clone()), CARD_FACE, CARD_INK),
        Face::Down => (None, CARD_BACK, INK),
    };
    let entity = parent.spawn((
        CardRef(card.id),
        Node {
            width: Val::Px(SMALL_W),
            height: Val::Px(SMALL_H),
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
        // Face down shows only the blank back; face up delegates to the shared Small face.
        if let Some(label) = label {
            let sub = (quantity > 1).then(|| format!("×{quantity}"));
            small_face(c, &label, card.card_type(), ink, sub);
        }
    });
}

/// Medium form — a card face: a name header above its detail (stat / rules) lines.
fn spawn_card_medium(parent: &mut ChildSpawnerCommands, card: &Card) {
    let entity = parent.spawn((
        CardRef(card.id),
        Node {
            width: Val::Px(MEDIUM_W),
            min_height: Val::Px(MEDIUM_MIN_H),
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
        spawn_type_badge(c, card.card_type());
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
fn spawn_card_large(parent: &mut ChildSpawnerCommands, card: &Card) {
    let entity = parent.spawn((
        CardRef(card.id),
        Node {
            width: Val::Px(LARGE_W),
            max_height: Val::Px(LARGE_MAX_H),
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
