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
//! - **Medium** — width fixed but height grows with line count, so only *horizontal* overflow is a fault.
//! - **Large** — a scrollable panel, so again only *horizontal* overflow is a fault.

use bevy::prelude::*;
use cardtable_model::{CardId, Tableau, sample_table};

use crate::{
    CardRef, FELT, install_ui_font, setup_camera, spawn_card_large, spawn_card_medium,
    spawn_card_small,
};

/// One rendered sample: which card, at which size (for the report). Sits on a wrapper whose single child
/// is the card's own fixed-size box; the wrapper's background is the red overflow frame.
#[derive(Component)]
struct Sample {
    card: CardId,
    size: &'static str,
}

/// The cards being shown, kept so the audit can resolve names.
#[derive(Resource)]
struct GalleryCards(Tableau);

/// Whether the one-shot audit has already run.
#[derive(Resource, Default)]
struct Audited(bool);

/// Build and run the gallery app. Blocks until the window is closed.
pub fn run_card_gallery() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Card Gallery — text audit".into(),
                resolution: (1100u32, 900u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(FELT))
        .insert_resource(GalleryCards(sample_table()))
        .init_resource::<Audited>()
        .add_systems(
            Startup,
            (setup_camera, install_ui_font, build_gallery).chain(),
        )
        .add_systems(Update, audit_gallery)
        .run();
}

/// Spawn a scrollable column with one row per card: its three render sizes side by side.
fn build_gallery(mut commands: Commands, cards: Res<GalleryCards>) {
    let tree = &cards.0;
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            overflow: Overflow::scroll_y(),
            padding: UiRect::all(Val::Px(16.0)),
            row_gap: Val::Px(16.0),
            ..default()
        })
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
        // Vertical overflow is only a fault for the fully-fixed Small card; Medium grows and Large scrolls.
        let tall = if s.size == "Small" { over.y } else { 0.0 };
        if over.x > 1.0 || tall > 1.0 {
            flagged += 1;
            offenders.push(wrapper);
            let scale = cn.inverse_scale_factor; // physical → logical px
            let name = tree
                .card(s.card)
                .map(|c| c.name().to_string())
                .unwrap_or_default();
            println!(
                "  OVERFLOW [{:<6}] {name:?} +{:.0}px wide, +{:.0}px tall",
                s.size,
                over.x * scale,
                tall * scale
            );
        }
    }
    println!("CARD GALLERY: {flagged} of {checked} (card × size) samples overflow their footprint");

    let red = Color::srgb(0.80, 0.20, 0.20);
    for wrapper in offenders {
        if let Ok(mut bg) = sample_bg.get_mut(wrapper) {
            bg.0 = red;
        }
    }
}

/// Every distinct card in the table, walking all piles and sub-piles (deduped by id, since projections
/// show the same cards in more than one place).
fn all_cards(tree: &Tableau) -> Vec<CardId> {
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
