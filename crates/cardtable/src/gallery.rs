//! A dev **card gallery / text audit** — a standalone Bevy app (never shipped) that renders *every* card
//! in [`sample_table`](cardtable_model::sample_table) at all three render sizes and reports any whose text
//! overflows its fixed footprint. It's the full-coverage answer the in-app view structurally can't give:
//! it drives off the model (all zones and subdecks, deduped), not what happens to be on screen.
//!
//! Run with `cargo run -p cardtable --example card_gallery`. A window opens showing a scrollable grid —
//! each row is one card at Small / Medium / Large — and the terminal prints an overflow report. Cards that
//! overflow are framed in red so you can eyeball them alongside the report; close the window when done.
//!
//! What counts as overflow depends on the size, matching how each card is allowed to grow:
//! - **Small** — fully fixed, so *either* axis overflowing is a fault.
//! - **Medium** — width *and* height fixed (the model computes the height from the line count; the renderer
//!   clips to it), so *either* axis overflowing is a fault. This is what catches a footprint that under-sizes
//!   the text - the height formula in `cardtable_model::layout` guessing a line too short.
//! - **Large** — a scrollable panel, so only *horizontal* overflow is a fault (the rest scrolls).

use crate::demo::demo_table;
use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::ui::ScrollPosition;
use cardtable_model::{Board, CardId};

use crate::{
    CardRef, FELT, install_ui_fonts, setup_camera, spawn_card_large, spawn_card_medium,
    spawn_card_small,
};

/// One rendered sample: which card, at which size (for the report). Sits on a wrapper whose single child
/// is the card's own fixed-size box; the wrapper's background is the red overflow frame.
#[derive(Component)]
struct Sample {
    card: CardId,
    size: &'static str,
}

/// The scrolling column, so the wheel handler can find it (and only it — not the Large cards' own
/// inner scroll).
#[derive(Component)]
struct GalleryScroll;

/// The scrollbar thumb — sized/positioned each frame to reflect the scroll, and draggable to drive it.
#[derive(Component)]
struct ScrollbarThumb;

/// Logical px scrolled per wheel line (when the OS reports scroll in lines rather than pixels).
const SCROLL_LINE_PX: f32 = 28.0;
/// Scrollbar track width and the thumb's minimum height (so it stays grabbable on very long content).
const SCROLLBAR_W: f32 = 12.0;
const THUMB_MIN: f32 = 32.0;

/// The cards being shown, kept so the audit can resolve names.
#[derive(Resource)]
struct GalleryCards(Board);

/// Whether the one-shot audit has already run.
#[derive(Resource, Default)]
struct Audited(bool);

/// Build and run the gallery app. Blocks until the window is closed.
pub fn run_card_gallery() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Card Gallery - text audit".into(),
                resolution: (1100u32, 900u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(FELT))
        .insert_resource(GalleryCards(demo_table()))
        .init_resource::<Audited>()
        .add_systems(
            Startup,
            (setup_camera, install_ui_fonts, build_gallery).chain(),
        )
        .add_systems(Update, (audit_gallery, scroll_gallery, update_scrollbar))
        .run();
}

/// Spawn a scrollable column with one row per card: its three render sizes side by side.
fn build_gallery(mut commands: Commands, cards: Res<GalleryCards>) {
    let tree = &cards.0;
    commands
        .spawn((
            GalleryScroll,
            ScrollPosition::DEFAULT, // driven by `scroll_gallery`; Bevy's scroll_y only clips, never scrolls
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(16.0),
                ..default()
            },
        ))
        .with_children(|col| {
            for id in all_cards(tree) {
                let Some(card) = tree.card(id) else {
                    continue;
                };
                col.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::FlexStart,
                    column_gap: Val::Px(16.0),
                    ..default()
                })
                .with_children(|row| {
                    sample(row, id, "Small", |w| spawn_card_small(w, card, 1));
                    sample(row, id, "Medium", |w| spawn_card_medium(w, card));
                    sample(row, id, "Large", |w| spawn_card_large(w, card));
                });
            }
        });

    // A scrollbar overlaid on the right edge: a faint full-height track with a thumb that reflects the
    // scroll position (see `update_scrollbar`) and can be dragged to scroll (see `on_thumb_drag`).
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Px(SCROLLBAR_W),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.06)),
            GlobalZIndex(10), // above the cards
        ))
        .with_children(|track| {
            track
                .spawn((
                    ScrollbarThumb,
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(0.0),
                        width: Val::Px(SCROLLBAR_W),
                        height: Val::Px(0.0), // set each frame by `update_scrollbar`
                        border_radius: BorderRadius::all(Val::Px(SCROLLBAR_W * 0.5)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.35)),
                ))
                .observe(on_thumb_drag);
        });
}

/// Scroll the gallery column with the mouse wheel. Bevy's `Overflow::scroll_y` only *clips* — it never
/// moves the content — so we drive the column's [`ScrollPosition`] ourselves. We clamp to the scrollable
/// range so the wheel can't build up an offset past either end (which would read as an unresponsive "dead
/// zone" before the content moves again).
fn scroll_gallery(
    wheel: Res<AccumulatedMouseScroll>,
    mut column: Query<(&mut ScrollPosition, &ComputedNode), With<GalleryScroll>>,
) {
    if wheel.delta.y == 0.0 {
        return;
    }
    let dy = match wheel.unit {
        MouseScrollUnit::Line => wheel.delta.y * SCROLL_LINE_PX,
        MouseScrollUnit::Pixel => wheel.delta.y,
    };
    for (mut scroll, node) in &mut column {
        // ComputedNode sizes are physical; ScrollPosition is logical — convert the max before clamping.
        let max = (node.content_size.y - node.size.y + node.scrollbar_size.y).max(0.0)
            * node.inverse_scale_factor;
        scroll.0.y = (scroll.0.y - dy).clamp(0.0, max);
    }
}

/// The thumb's height (∝ visible fraction) and travel (∝ scroll offset), for a track spanning the given
/// viewport, all in logical px. Returns `None` when the content fits — nothing to scroll.
fn thumb_metrics(node: &ComputedNode, offset_logical: f32) -> Option<(f32, f32)> {
    let (viewport, content) = (node.size.y, node.content_size.y); // physical
    if content <= viewport + 0.5 {
        return None;
    }
    let track_h = viewport * node.inverse_scale_factor; // logical (the visible height)
    let thumb_h = ((viewport / content) * track_h).max(THUMB_MIN);
    let max_off = (content - viewport + node.scrollbar_size.y).max(0.0) * node.inverse_scale_factor;
    let frac = if max_off > 0.0 {
        (offset_logical / max_off).clamp(0.0, 1.0)
    } else {
        0.0
    };
    Some((thumb_h, frac * (track_h - thumb_h)))
}

/// Size and place the thumb each frame to reflect the current scroll (hidden when nothing scrolls).
fn update_scrollbar(
    column: Query<(&ScrollPosition, &ComputedNode), With<GalleryScroll>>,
    mut thumb: Query<&mut Node, With<ScrollbarThumb>>,
) {
    let (Ok((scroll, node)), Ok(mut thumb)) = (column.single(), thumb.single_mut()) else {
        return;
    };
    match thumb_metrics(node, scroll.0.y) {
        Some((height, top)) => {
            thumb.height = Val::Px(height);
            thumb.top = Val::Px(top);
        }
        None => thumb.height = Val::Px(0.0),
    }
}

/// Drag the thumb to scroll: a thumb move of `d` px maps to `d · max_offset / travel` of scroll, so the
/// thumb tracks the cursor. Clamped to the scrollable range.
fn on_thumb_drag(
    drag: On<Pointer<Drag>>,
    mut column: Query<(&mut ScrollPosition, &ComputedNode), With<GalleryScroll>>,
) {
    let Ok((mut scroll, node)) = column.single_mut() else {
        return;
    };
    let Some((thumb_h, _)) = thumb_metrics(node, scroll.0.y) else {
        return;
    };
    let track_h = node.size.y * node.inverse_scale_factor;
    let travel = (track_h - thumb_h).max(1.0);
    let max_off = (node.content_size.y - node.size.y + node.scrollbar_size.y).max(0.0)
        * node.inverse_scale_factor;
    scroll.0.y = (scroll.0.y + drag.event().event.delta.y * max_off / travel).clamp(0.0, max_off);
}

/// Spawn one card sample: a [`Sample`] wrapper (the red overflow frame) around one card face.
fn sample(
    row: &mut ChildSpawnerCommands,
    card: CardId,
    size: &'static str,
    face: impl FnOnce(&mut ChildSpawnerCommands),
) {
    row.spawn((
        Sample { card, size },
        Node {
            padding: UiRect::all(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(Color::NONE), // turns red if this sample overflows
    ))
    .with_children(face);
}

/// After a few frames (so text has laid out), measure every sample once, print the overflow report, and
/// frame the offenders in red.
#[allow(clippy::too_many_arguments)]
fn audit_gallery(
    mut audited: ResMut<Audited>,
    mut frames: Local<u32>,
    cards: Res<GalleryCards>,
    samples: Query<(Entity, &Sample, &Children)>,
    boxes: Query<(&ComputedNode, &UiGlobalTransform), With<CardRef>>,
    children_q: Query<&Children>,
    rect_q: Query<(&ComputedNode, &UiGlobalTransform)>,
    mut sample_bg: Query<&mut BackgroundColor, With<Sample>>,
) {
    if audited.0 {
        return;
    }
    *frames += 1;
    if *frames < 3 {
        return; // let the font load and the layout settle first
    }
    audited.0 = true;

    let tree = &cards.0;
    let (mut checked, mut flagged) = (0usize, 0usize);
    let mut offenders: Vec<Entity> = Vec::new();
    println!("CARD GALLERY TEXT AUDIT ----------------------------------------");
    for (wrapper, s, children) in &samples {
        // The card's fixed-size box is the wrapper's single card child.
        let Some(card_e) = children.iter().find(|&e| boxes.contains(e)) else {
            continue;
        };
        let Ok((cn, gt)) = boxes.get(card_e) else {
            continue;
        };
        checked += 1;
        let over = descendant_overflow(card_e, gt.translation, cn.size * 0.5, &children_q, &rect_q);
        // Vertical overflow is a fault for the fixed-height Small AND Medium cards (the model sizes them and
        // the renderer clips); only Large scrolls, so its vertical is free.
        let tall = if s.size == "Large" { 0.0 } else { over.y };
        if over.x > 1.0 || tall > 1.0 {
            flagged += 1;
            offenders.push(wrapper);
            let scale = cn.inverse_scale_factor; // physical → logical px
            let name = tree
                .card(s.card)
                .map(|c| c.name().to_string())
                .unwrap_or_default();
            // The card box is sized to the model footprint; `content_size` is what the text actually needs.
            // Printing both makes the miss concrete: "model 133, content 151" = the height formula is 18 short.
            let (box_h, content_h) = (cn.size.y * scale, cn.content_size.y * scale);
            println!(
                "  OVERFLOW [{:<6}] {name:?} +{:.0}px wide, +{:.0}px tall  (model {box_h:.0}px, content {content_h:.0}px)",
                s.size,
                over.x * scale,
                tall * scale
            );
        }
    }
    println!("CARD GALLERY: {flagged} of {checked} (card x size) samples overflow their footprint");

    let red = Color::srgb(0.80, 0.20, 0.20);
    for wrapper in offenders {
        if let Ok(mut bg) = sample_bg.get_mut(wrapper) {
            bg.0 = red;
        }
    }
}

/// Every distinct card in the table, walking all piles and sub-piles (deduped by id, since projections
/// show the same cards in more than one place).
fn all_cards(tree: &Board) -> Vec<CardId> {
    let mut ids = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut stack = vec![tree.root_id()];
    while let Some(pid) = stack.pop() {
        if let Some(pile) = tree.pile(pid) {
            for c in pile.cards() {
                if seen.insert(c) {
                    ids.push(c);
                }
            }
            for s in pile.subpiles() {
                stack.push(s);
            }
        }
    }
    ids
}

/// The worst distance (physical px, per axis) any descendant of `card`'s box extends *beyond* that box —
/// 0 on an axis that fits. Walks the whole subtree so wrapped text, a badge, or any nested node counts.
/// Rects are centre + half-size in the shared UI space.
fn descendant_overflow(
    card: Entity,
    center: Vec2,
    half: Vec2,
    children_q: &Query<&Children>,
    rect_q: &Query<(&ComputedNode, &UiGlobalTransform)>,
) -> Vec2 {
    let mut worst = Vec2::ZERO;
    let mut stack: Vec<Entity> = children_q
        .get(card)
        .map(|c| c.iter().collect())
        .unwrap_or_default();
    while let Some(e) = stack.pop() {
        if let Ok((cn, gt)) = rect_q.get(e) {
            let h = cn.size * 0.5;
            let c = gt.translation;
            let right = (c.x + h.x - (center.x + half.x)).max(0.0);
            let left = ((center.x - half.x) - (c.x - h.x)).max(0.0);
            let bottom = (c.y + h.y - (center.y + half.y)).max(0.0);
            let top = ((center.y - half.y) - (c.y - h.y)).max(0.0);
            worst.x = worst.x.max(right.max(left));
            worst.y = worst.y.max(bottom.max(top));
        }
        if let Ok(ch) = children_q.get(e) {
            stack.extend(ch.iter());
        }
    }
    worst
}
