//! A Bevy renderer that draws the **card-table metaphor** — every zone a deck, the unattended
//! collapsed into labelled, counted piles. Click a deck to open it, click the table to close them all,
//! and drag cards (and whole decks) between decks.
//!
//! # Two layers
//!
//! - **The core (this module) is game-agnostic.** It draws whatever is in the [`Table`] resource (a
//!   [`DeckTree`]) plus an [`ActionRail`] of loose actions and a [`StatusLine`], handles focus/zoom
//!   itself, and reports clicks on actionable controls by pushing their index into [`ActionRequests`].
//!   It never mentions `Game`. This is the shared code: `boardgame` and feature prototypes both drive
//!   it. Prototype a feature with [`CardTablePlugin`] + a hand-built `Table` (see
//!   [`cardtable_model::fixtures`]) and no game at all — `cargo run -p cardtable --example sandbox`.
//! - **The `game` feature adds the adapter** ([`GamePlugin`]): it binds a [`contract::Game`] to the
//!   core — building the `Table`/`ActionRail`/`StatusLine` from the game's view and draining
//!   `ActionRequests` into `Game::apply`. Only the launcher needs it.
//!
//! Rendering is `bevy_ui` (flexbox), matching `tabletop`; the deck model is renderer-agnostic, so a
//! future 3D table could be built against the same [`Table`] — see
//! `docs/games/deckbound/presentation/card-table-ui.md` §7.

use bevy::picking::events::{Click, Drag, DragDrop, DragEnd, DragStart, Pointer};
use bevy::prelude::*;
use bevy::ui::{BoxShadow, ComputedNode};

use cardtable_model::{Card, CardId, DeckId, DeckTree, Face};

#[cfg(feature = "game")]
pub use game::GamePlugin;

// ---- public presentation state (the shared inputs) ----------------------

/// The board: the deck tree the core draws. Mutated in place for focus/zoom; replaced wholesale when
/// the source (a game, or a prototype) rebuilds it.
#[derive(Resource, Default)]
pub struct Table(pub DeckTree);

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

/// The game-agnostic renderer. Add it, put a [`DeckTree`] in [`Table`], and you have a clickable card
/// table. Add [`GamePlugin`] (feature `game`) on top to drive it from a [`contract::Game`].
pub struct CardTablePlugin;

impl Plugin for CardTablePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Table>()
            .init_resource::<ActionRail>()
            .init_resource::<StatusLine>()
            .init_resource::<ActionRequests>()
            .init_resource::<DragGuard>()
            .insert_resource(NeedsRebuild(true))
            .configure_sets(
                Update,
                (CardTableSet::Input, CardTableSet::Apply, CardTableSet::Draw).chain(),
            )
            .add_systems(Startup, (setup_camera, install_ui_font))
            .add_systems(Update, (sync_deck_sizes, sync_surface_size, animate_decks))
            .add_systems(Update, redraw.in_set(CardTableSet::Draw))
            // Input is picking-driven, so it runs in observers rather than the Input system set:
            // clicks open/close decks and fire actions; a card drag drops into a deck; a deck drag
            // slides it freely across the table.
            .add_observer(on_drag_start)
            .add_observer(on_click)
            .add_observer(on_drop)
            .add_observer(on_deck_drag)
            .add_observer(on_deck_drag_end);
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

/// Marks a deck's node as a drop target: a card dropped here moves into this deck.
#[derive(Component, Clone, Copy)]
struct DeckDropZone(DeckId);

/// Marks a top-level deck's absolutely-positioned wrapper, carrying its [`DeckId`]. Dragging it slides
/// the deck freely across the table (live), committing the final position on release.
#[derive(Component, Clone, Copy)]
struct TableDeck(DeckId);

/// Marks the table surface — the positioning context for decks. Its size is fed to the model as the
/// wall bounds that keep decks inside.
#[derive(Component)]
struct TableSurface;

/// True while a pointer drag is in progress. Bevy fires a `Click` at the end of *every* drag (press
/// and release over the same entity, regardless of the drag), so this guards the click handler from
/// treating a drag's release as a real click. Set on [`DragStart`], cleared on [`DragEnd`].
#[derive(Resource, Default)]
struct DragGuard(bool);

/// Set when the UI must be torn down and rebuilt — *structural* changes only (open/close a deck, move
/// a card, a new game snapshot). Deck positions are not structural; they animate, so repositioning
/// never sets this. See [`redraw`] and [`animate_decks`].
#[derive(Resource)]
struct NeedsRebuild(bool);

// ---- systems ------------------------------------------------------------

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// A clean UI typeface bundled in the crate, for crisp small text on cards. Covers the punctuation and
/// arrows the renderer uses (em dashes, curly quotes, arrows) that Bevy's built-in `FiraMono-subset`
/// font lacks — otherwise they show as tofu boxes. SIL Open Font License; see `fonts/Inter-LICENSE.txt`.
const UI_FONT: &[u8] = include_bytes!("../fonts/Inter-Regular.ttf");

/// Replace Bevy's ASCII-only default font with the bundled Inter face. Bevy registers its default font
/// at `AssetId::default()`, and every `TextFont { ..default() }` here points there, so overwriting that
/// one asset reskins all UI text without threading a font handle through each label.
fn install_ui_font(mut fonts: ResMut<Assets<Font>>) {
    let font = Font::from_bytes(UI_FONT.to_vec());
    fonts
        .insert(AssetId::default(), font)
        .expect("override the default font");
}

/// A picking click, resolved against the most specific target: an actionable control records its
/// action; a (non-actionable) card consumes the click so it doesn't bubble; a deck opens (focus); the
/// table background closes all decks. Inner nodes (e.g. a card's text) match nothing and fall through
/// to their parent via propagation. Global observer, so it survives the per-change UI rebuild.
fn on_drag_start(_on: On<Pointer<DragStart>>, mut guard: ResMut<DragGuard>) {
    guard.0 = true;
}

#[allow(clippy::type_complexity)]
fn on_click(
    mut on: On<Pointer<Click>>,
    guard: Res<DragGuard>,
    targets: Query<(
        Option<&ActionControl>,
        Option<&CardRef>,
        Option<&DeckDropZone>,
        Has<CardTableRoot>,
    )>,
    mut table: ResMut<Table>,
    mut requests: ResMut<ActionRequests>,
    mut rebuild: ResMut<NeedsRebuild>,
) {
    if guard.0 {
        return; // the release that ends a drag also fires Click — that's not an intentional click
    }
    let Ok((action, card, deck, is_background)) = targets.get(on.event().entity) else {
        return;
    };
    if let Some(action) = action {
        requests.0.push(action.0);
    } else if card.is_some() {
        // A non-actionable card: consume the click (don't focus a deck or close the table).
    } else if let Some(deck) = deck {
        let _ = table.0.focus(deck.0);
        rebuild.0 = true;
    } else if is_background {
        let root = table.0.root_id();
        let _ = table.0.focus(root);
        rebuild.0 = true;
    } else {
        return; // not interactive — let it propagate to an ancestor that is
    }
    on.propagate(false);
}

/// A picking drop: move a dragged **card** into the target's deck (a deck, or a card's home deck).
/// Decks are not nested on drop — they are repositioned by [`on_deck_drag`] — so a dragged deck is
/// ignored here. Presentation-level; mapping drops to game actions is future work.
fn on_drop(
    mut on: On<Pointer<DragDrop>>,
    cards: Query<&CardRef>,
    decks: Query<&DeckDropZone>,
    mut table: ResMut<Table>,
    mut rebuild: ResMut<NeedsRebuild>,
) {
    let event = on.event();
    let Ok(dragged) = cards.get(event.event.dropped) else {
        return; // only cards drop *into* decks
    };
    let dest = if let Ok(zone) = decks.get(event.entity) {
        zone.0
    } else if let Ok(card) = cards.get(event.entity) {
        match table.0.card(card.0) {
            Some(c) => c.home(),
            None => return,
        }
    } else {
        return; // not a drop target — let it propagate
    };
    on.propagate(false);
    let at = table.0.deck(dest).map_or(0, |deck| deck.cards().len());
    let _ = table.0.move_card(dragged.0, dest, at);
    rebuild.0 = true;
}

/// Slide a top-level deck across the table while it is dragged — freely, even off the edge. Moves the
/// wrapper's `Node` and the model position together (a position change is not structural, so there is
/// no rebuild mid-drag); settling on release brings an off-edge deck back. A card drag is consumed
/// here so it doesn't also slide the deck under it.
fn on_deck_drag(
    mut on: On<Pointer<Drag>>,
    cards: Query<&CardRef>,
    mut decks: Query<(&TableDeck, &mut Node)>,
    mut table: ResMut<Table>,
) {
    let target = on.event().entity;
    if cards.get(target).is_ok() {
        on.propagate(false);
        return;
    }
    if let Ok((deck, mut node)) = decks.get_mut(target) {
        let delta = on.event().event.delta;
        let (x, y) = (px(node.left) + delta.x, px(node.top) + delta.y);
        // Follow the cursor anywhere — even past the table edge. The settling on release clamps it
        // back inside and the animation slides it into view. Keep the model in step with the live
        // node so the animation doesn't fight the drag.
        node.left = Val::Px(x);
        node.top = Val::Px(y);
        let _ = table.0.set_deck_pos(deck.0, x, y);
        on.propagate(false);
    }
}

/// Commit a dragged deck's final position to the model on release (one rebuild, at rest).
fn on_deck_drag_end(
    mut on: On<Pointer<DragEnd>>,
    cards: Query<&CardRef>,
    decks: Query<(&TableDeck, &Node)>,
    mut table: ResMut<Table>,
    mut guard: ResMut<DragGuard>,
) {
    guard.0 = false; // the drag is over; let real clicks through again
    let target = on.event().entity;
    if cards.get(target).is_ok() {
        on.propagate(false);
        return;
    }
    if let Ok((deck, node)) = decks.get(target) {
        let _ = table.0.set_deck_pos(deck.0, px(node.left), px(node.top));
        // Settle: clamp the (possibly off-edge) deck back inside and shove overlaps clear — the
        // anchor included, so a deck dropped past the border is pulled into view, then the animation
        // slides it the rest of the way.
        table.0.separate(deck.0);
        on.propagate(false);
    }
}

/// The pixel value of a `Val`, or `0.0` for the non-pixel variants (decks always use `Px`).
fn px(value: Val) -> f32 {
    match value {
        Val::Px(p) => p,
        _ => 0.0,
    }
}

/// Feed each top-level deck's laid-out size back into the model (logical px), so [`DeckTree::separate`]
/// works on real AABBs. Runs every frame; deck sizes are stable, so it's cheap.
fn sync_deck_sizes(decks: Query<(&TableDeck, &ComputedNode)>, mut table: ResMut<Table>) {
    for (deck, computed) in &decks {
        let size = computed.size * computed.inverse_scale_factor;
        let _ = table.0.set_deck_size(deck.0, size.x, size.y);
    }
}

/// Feed the table surface's laid-out size to the model as the wall bounds that contain the decks.
fn sync_surface_size(surfaces: Query<&ComputedNode, With<TableSurface>>, mut table: ResMut<Table>) {
    if let Ok(computed) = surfaces.single() {
        let size = computed.size * computed.inverse_scale_factor;
        table.0.set_surface(size.x, size.y);
    }
}

/// Ease each deck's wrapper toward its model position, so a separation (or any reposition) *slides*
/// into place instead of snapping. The dragged deck keeps target == position, so it doesn't ease;
/// decks already at rest are skipped so the node (and its layout) isn't touched every frame.
fn animate_decks(time: Res<Time>, table: Res<Table>, mut decks: Query<(&TableDeck, &mut Node)>) {
    let t = (SLIDE_SPEED * time.delta_secs()).min(1.0);
    for (deck, mut node) in &mut decks {
        let Some(d) = table.0.deck(deck.0) else {
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

/// Rebuild the whole UI only on a *structural* change (open/close a deck, move a card, a new game
/// snapshot). Deck positions are not structural — they animate (see [`animate_decks`]) — so
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
/// A second back shade so alternating layers in a deck's stack read as distinct cards.
const CARD_BACK_ALT: Color = Color::srgb(0.28, 0.32, 0.52);
/// Highlight edge for a card/deck that carries a legal move.
const ACTIONABLE: Color = Color::srgb(0.30, 0.70, 0.62);
/// A dark edge around every card so overlapping cards stay distinct.
const CARD_EDGE: Color = Color::srgb(0.12, 0.11, 0.10);
/// Soft drop shadow lifting cards and decks off the felt.
const SHADOW: Color = Color::srgba(0.0, 0.0, 0.0, 0.35);

/// A soft drop shadow used on cards and deck chips (offset down, blurred).
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

/// How fast a deck eases toward its target position, as a fraction closed per second (higher = snappier).
const SLIDE_SPEED: f32 = 12.0;

/// A collapsed deck's front-face footprint, the per-card stack step (offset along two edges), and the
/// visual depth cap so a deep deck doesn't grow without bound.
const CHIP_W: f32 = 120.0;
const CHIP_H: f32 = 64.0;
const STACK_OFFSET: f32 = 2.0;
const MAX_STACK: usize = 10;

fn build_ui(commands: &mut Commands, tree: &DeckTree, rail: &[RailAction], status: &str) {
    commands
        .spawn((
            CardTableRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                ..default()
            },
            BackgroundColor(FELT),
        ))
        .with_children(|root| {
            // LEFT: the action rail — choices not on a card. Hidden when empty (e.g. a prototype).
            if !rail.is_empty() {
                root.spawn((
                    Node {
                        width: Val::Px(280.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(12.0)),
                        row_gap: Val::Px(8.0),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    BackgroundColor(PANEL),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("Actions"),
                        TextFont {
                            font_size: FONT_HEAD,
                            ..default()
                        },
                        TextColor(INK),
                    ));
                    for action in rail {
                        spawn_rail_button(panel, action);
                    }
                });
            }

            // CENTER: status, then the decks. Clicking empty space here (the felt) closes all decks.
            root.spawn(Node {
                flex_grow: 1.0,
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(12.0),
                overflow: Overflow::scroll_y(),
                ..default()
            })
            .with_children(|main| {
                if !status.is_empty() {
                    main.spawn((
                        Text::new(status.to_string()),
                        TextFont {
                            font_size: FONT_HEAD,
                            ..default()
                        },
                        TextColor(INK),
                    ));
                }

                // The table surface: a fill area holding absolutely-placed decks the player drags
                // anywhere. Each top-level deck sits in a positioned wrapper at its model position.
                main.spawn((
                    TableSurface,
                    Node {
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        ..default()
                    },
                ))
                .with_children(|surface| {
                    let root_deck = tree.deck(tree.root_id()).expect("root exists");
                    for &id in root_deck.subdecks() {
                        let pos = tree.deck(id).expect("deck id from root").pos();
                        surface
                            .spawn((
                                TableDeck(id),
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(pos.x),
                                    top: Val::Px(pos.y),
                                    ..default()
                                },
                            ))
                            .with_children(|wrapper| spawn_deck(wrapper, tree, id));
                    }
                });
            });
        });
}

/// Draws a collapsed deck as a short stack of offset layers — two alternating colors, stepped along
/// the left and bottom edges, capped at [`MAX_STACK`] — hinting at how many cards are inside. The
/// front layer (top-right, on top) carries the label and count; the whole stack is one drop target.
fn spawn_deck_chip(parent: &mut ChildSpawnerCommands, id: DeckId, label: &str, count: usize) {
    let depth = count.clamp(1, MAX_STACK);
    let spread = (depth - 1) as f32 * STACK_OFFSET;
    parent
        .spawn((
            DeckDropZone(id),
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

/// Draws a deck: a compact, counted chip when collapsed, or a fanned panel of its cards when open.
fn spawn_deck(parent: &mut ChildSpawnerCommands, tree: &DeckTree, id: DeckId) {
    let deck = tree.deck(id).expect("deck id from tree");
    if deck.collapsed {
        let count = deck.cards().len() + deck.subdecks().len();
        spawn_deck_chip(parent, id, &deck.label, count);
    } else {
        parent
            .spawn((
                DeckDropZone(id),
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.0)),
                    row_gap: Val::Px(8.0),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(PANEL),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new(deck.label.clone()),
                    TextFont {
                        font_size: FONT_HEAD,
                        ..default()
                    },
                    TextColor(INK),
                ));
                panel
                    .spawn((
                        DeckDropZone(id),
                        Node {
                            flex_direction: FlexDirection::Row,
                            flex_wrap: FlexWrap::Wrap,
                            column_gap: Val::Px(8.0),
                            row_gap: Val::Px(8.0),
                            min_height: Val::Px(140.0),
                            ..default()
                        },
                    ))
                    .with_children(|cards| {
                        for &cid in deck.cards() {
                            spawn_card(cards, tree.card(cid).expect("card id from deck"));
                        }
                        for &sid in deck.subdecks() {
                            spawn_deck(cards, tree, sid);
                        }
                    });
            });
    }
}

/// Draws one card: a light face showing its title, or a dark back. Actionable cards get a highlight
/// edge and become clickable.
fn spawn_card(parent: &mut ChildSpawnerCommands, card: &Card) {
    // A face-down card shows only its back — no glyph, which also reads more like a real card.
    let (title, bg, ink) = match &card.face {
        Face::Up { title } => (Some(title.clone()), CARD_FACE, CARD_INK),
        Face::Down => (None, CARD_BACK, INK),
    };
    let mut entity = parent.spawn((
        CardRef(card.id),
        Node {
            width: Val::Px(96.0),
            height: Val::Px(132.0),
            padding: UiRect::all(Val::Px(8.0)),
            border: UiRect::all(Val::Px(2.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(Val::Px(12.0)),
            ..default()
        },
        BackgroundColor(bg),
        BorderColor::all(if card.is_actionable() {
            ACTIONABLE
        } else {
            CARD_EDGE
        }),
        card_shadow(),
    ));
    if let Some(index) = card.actionable {
        entity.insert(ActionControl(index));
    }
    entity.with_children(|c| {
        if let Some(title) = title {
            c.spawn((
                Text::new(title),
                TextFont {
                    font_size: FONT_TITLE,
                    ..default()
                },
                TextColor(ink),
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

    use cardtable_model::{DeckTree, from_table_view};
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

    /// Build the presentation state from a game state: the board (zones → decks), the loose-action
    /// rail (legal actions not bound to a card), and the status caption.
    fn snapshot<G: Game>(game: &G, state: &G::State) -> (DeckTree, Vec<RailAction>, String) {
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
