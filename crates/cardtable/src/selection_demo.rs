//! **Selection-states demo** - a standalone window showing the four attention states a card can be in during
//! a multi-part selection (a source -> action -> target gesture), rendered with the **real** combat tile
//! code (`draw_scene_tile` / `tile_look`) and the **real** rotating-ring animation, so what you see is
//! exactly what the game draws - not a re-implementation.
//!
//! The four states (mapped to the renderer's rules-blind [`Highlight`] vocabulary):
//!
//! - **Background** - not interactable right now -> [`Highlight::Dim`]: receded, no border.
//! - **In the selection** - a part already chosen (the source, the action) -> [`Highlight::Active`]: the
//!   bright, locked-in cue.
//! - **Completing the selection** - the next step (a drop target / the candidates that finish the gesture) ->
//!   [`Highlight::Targeted`]: carries the one animated cue, the rotating dotted ring.
//! - **Selectable** - a legal start for a DIFFERENT selection (abort this one, pick another source) ->
//!   [`Highlight::Available`]: the steady "you could pick this" cue.
//!
//! Answering the design question: this stays an EXAMPLE (a thin `main` calling this crate fn) - it does not
//! need to be its own module - because the real drawing lives here in the crate and the example just runs it,
//! the same pattern as `run_card_gallery`.
//!
//! Run: `cargo run -p cardtable --example selection_states`

use bevy::prelude::*;
use cardtable_model::{
    Badge, CardId, Highlight, Lane, Scene, SceneBody, Team, Tile, Tone, Track, TrackItem,
};

use crate::{
    CardScreenRects, FELT, INK, MUTED, SceneState, animate_target_rings, draw_scene_tile,
    install_ui_fonts, setup_camera, track_card_rects,
};

/// Launch the demo window.
pub fn run_selection_states() {
    let scene = demo_scene();
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Selection states".into(),
                resolution: (900u32, 420u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(FELT))
        .init_resource::<CardScreenRects>()
        // The real ring system reads the scene body for `Targeted` tiles; give it the same tiles we draw.
        .insert_resource(SceneState(Some(scene)))
        .add_systems(Startup, (setup_camera, install_ui_fonts, build).chain())
        // The REAL systems: track each tile's rect, then ring the `Targeted` ones - identical to combat.
        .add_systems(Update, (track_card_rects, animate_target_rings).chain())
        .run();
}

/// One demo entry: a caption for the row and the real [`Tile`] to draw.
struct Entry {
    caption: &'static str,
    tile: Tile,
}

/// Build one demo tile in a given state, with a couple of badge lines so it reads like a combat tile.
fn tile(id: u64, title: &str, team: Team, highlight: Highlight, badges: &[&str]) -> Tile {
    Tile {
        card: CardId(id),
        title: title.to_string(),
        team,
        highlight,
        badges: badges
            .iter()
            .map(|b| Badge {
                text: (*b).to_string(),
                tone: Tone::Muted,
            })
            .collect(),
        draggable: false,
        tappable: false,
    }
}

/// The demo scene: the four states, illustrated as a source -> action -> target gesture in progress, plus a
/// labeled one-per-state legend. Every tile carries the real [`Highlight`] so it renders exactly as combat.
fn demo_scene() -> Scene {
    // The gesture in progress + the alternatives + the background. Short caption per tile, so each reads as
    // a compact cell (caption above the tile) that can flow into a grid.
    let entries = vec![
        Entry {
            caption: "source (chosen)",
            tile: tile(1, "Raider", Team::Left, Highlight::Active, &["your source"]),
        },
        Entry {
            caption: "action (chosen)",
            tile: tile(2, "Strike", Team::Left, Highlight::Active, &["your action"]),
        },
        Entry {
            caption: "target (completing)",
            tile: tile(3, "The Wall", Team::Right, Highlight::Targeted, &["1 hp"]),
        },
        Entry {
            caption: "target (completing)",
            tile: tile(4, "The Sniper", Team::Right, Highlight::Targeted, &["1 hp"]),
        },
        Entry {
            caption: "selectable (switch)",
            tile: tile(
                5,
                "Bastion",
                Team::Left,
                Highlight::Available,
                &["could pick"],
            ),
        },
        Entry {
            caption: "selectable (switch)",
            tile: tile(
                6,
                "Marksman",
                Team::Left,
                Highlight::Available,
                &["could pick"],
            ),
        },
        Entry {
            caption: "background",
            tile: tile(7, "Bombardier", Team::Left, Highlight::Dim, &["reserve"]),
        },
        Entry {
            caption: "background",
            tile: tile(8, "Kestrel", Team::Left, Highlight::Dim, &["reserve"]),
        },
    ];
    // The tiles also live in the scene body so the real ring system finds the `Targeted` ones.
    let all: Vec<Tile> = entries.iter().map(|e| e.tile.clone()).collect();
    CAPTIONED.set(entries);
    Scene {
        tracks: vec![Track {
            title: "Gesture".to_string(),
            items: vec![
                TrackItem {
                    label: "source".into(),
                    current: false,
                },
                TrackItem {
                    label: "action".into(),
                    current: false,
                },
                TrackItem {
                    label: "target".into(),
                    current: true,
                },
            ],
        }],
        heading: "Selection states".to_string(),
        prompt: "source -> action -> target: each card shows the state it is in".to_string(),
        body: SceneBody::Lanes(vec![Lane {
            label: "cards".into(),
            left: all,
            right: vec![],
        }]),
        links: vec![],
        choices: vec![],
        log_title: String::new(),
        log: vec![],
        legend: vec![],
        reference: vec![],
        disabled_controls: vec![],
    }
}

/// A one-shot hand-off of the captioned tiles from `demo_scene` (built before the App) to `build` (a
/// startup system). The scene in `SceneState` carries only the tiles the ring system needs; the captions
/// ride here.
use std::sync::OnceLock;
static CAPTIONED: Captioned = Captioned(OnceLock::new());
struct Captioned(OnceLock<Vec<(String, Tile)>>);
impl Captioned {
    fn set(&self, entries: Vec<Entry>) {
        let _ = self.0.set(
            entries
                .into_iter()
                .map(|e| (e.caption.to_string(), e.tile))
                .collect(),
        );
    }
    fn get(&self) -> Vec<(String, Tile)> {
        self.0.get().cloned().unwrap_or_default()
    }
}

/// Lay the captioned tiles out as compact cells (caption ABOVE the tile) that WRAP into a grid, so the
/// demo is wide rather than tall. Each cell is drawn with the REAL [`draw_scene_tile`] (which resolves the
/// tile's look via the real `tile_look`).
fn build(mut commands: Commands) {
    let entries = CAPTIONED.get();
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart,
            row_gap: Val::Px(14.0),
            padding: UiRect::all(Val::Px(24.0)),
            ..default()
        })
        .with_children(|root| {
            text(root, "Selection states", 26.0, INK);
            text(
                root,
                "A source -> action -> target gesture in progress. Each card is drawn by the real combat tile code.",
                14.0,
                MUTED,
            );
            // The tiles as a WRAPPING GRID of cells: caption on top, tile below - so a caption never pushes
            // its tile sideways, and the cells flow across the width instead of stacking into one column.
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::FlexStart,
                column_gap: Val::Px(18.0),
                row_gap: Val::Px(16.0),
                ..default()
            })
            .with_children(|grid| {
                for (caption, t) in &entries {
                    grid.spawn(Node {
                        width: Val::Px(150.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(6.0),
                        ..default()
                    })
                    .with_children(|cell| {
                        cell.spawn((Text::new(caption.clone()), TextColor(MUTED)));
                        draw_scene_tile(cell, t);
                    });
                }
            });
        });
}

fn text(parent: &mut ChildSpawnerCommands, s: &str, size: f32, color: Color) {
    parent.spawn((
        Text::new(s.to_string()),
        TextFont {
            font_size: bevy::text::FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    ));
}
