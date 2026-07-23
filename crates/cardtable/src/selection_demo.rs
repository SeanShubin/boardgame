//! **Selection-states demo** - a standalone window for designing the attention states a card takes during a
//! multi-part `source -> action -> target` gesture, rendered with the **real** combat tile code
//! (`draw_scene_tile` / `tile_look`) and the **real** overlay animations (the flowing link dots and the
//! rotating target ring), so what you see is exactly what the game draws - not a re-implementation.
//!
//! **The brainstorm this window is for.** Four states were being carried by four different tile borders, and
//! two of them (the *chosen* parts of the gesture and the *selectable* alternatives) read almost the same. So
//! this demo tries a different split: let an animated **connector** - the two-color dots flowing
//! `source -> action -> target` - carry "these cards are the gesture in progress", which frees *chosen* and
//! *selectable* to share ONE look. Then only three tile looks remain:
//!
//! - **Background** -> [`Highlight::Dim`]: receded, dim face. (Distinct already.)
//! - **In the gesture / selectable** -> [`Highlight::Available`]: the steady amber "live card" cue. The
//!   flowing connector, not the border, says which of these live cards are the ones you already chose.
//! - **Completing** -> [`Highlight::Targeted`]: green border + the rotating ring - the one card the gesture
//!   is reaching for next.
//!
//! The connector is two-color on purpose: the locked part of the chain (`source -> action`) flows green
//! (confirmed), the proposed reach (`action -> target`) flows amber (not yet committed).
//!
//! Run: `cargo run -p cardtable --example selection_states`

use bevy::prelude::*;
use cardtable_model::{
    Badge, CardId, Highlight, Lane, Link, Scene, SceneBody, Team, Tile, Tone, Track, TrackItem,
};

use crate::{
    CardScreenRects, FELT, INK, MUTED, SceneState, animate_target_arrows, animate_target_rings,
    draw_scene_tile, install_ui_fonts, setup_camera, track_card_rects,
};

/// Launch the demo window.
pub fn run_selection_states() {
    let scene = demo_scene();
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Selection states".into(),
                resolution: (940u32, 520u32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(FELT))
        .init_resource::<CardScreenRects>()
        // The real overlay systems read this scene: its `body` for the `Targeted` tiles (rings) and its
        // `links` for the flowing connector. Give them the same tiles/links we draw.
        .insert_resource(SceneState(Some(scene)))
        .add_systems(Startup, (setup_camera, install_ui_fonts, build).chain())
        // The REAL systems, in the real order: track each drawn tile's rect, then draw the connector dots and
        // the target rings from those rects - identical to a live fight.
        .add_systems(
            Update,
            (
                track_card_rects,
                animate_target_arrows,
                animate_target_rings,
            )
                .chain(),
        )
        .run();
}

/// One captioned tile to draw.
struct Cell {
    caption: &'static str,
    tile: Tile,
}

/// Build one demo tile in a given state, with a badge line so it reads like a combat tile.
fn tile(id: u64, title: &str, team: Team, highlight: Highlight, badge: &str) -> Tile {
    Tile {
        card: CardId(id),
        title: title.to_string(),
        team,
        highlight,
        badges: vec![Badge {
            text: badge.to_string(),
            tone: Tone::Muted,
        }],
        draggable: false,
        // A `Dim` tile is background (not interactable); everything else is a live card, so it is tappable -
        // which is also what earns it a `TileCard` in `draw_scene_tile`, and thus the rect-tracking the
        // flowing connector and the rotating ring both need to find their endpoints.
        tappable: highlight != Highlight::Dim,
    }
}

// The card ids. The trio the connector runs through; the legend samples one of each remaining look.
const SOURCE: u64 = 1;
const ACTION: u64 = 2;
const TARGET: u64 = 3;
const LEGEND_SELECTABLE: u64 = 4;
const LEGEND_TARGET: u64 = 5;
const LEGEND_BACKGROUND: u64 = 6;

/// The three cards of the gesture in progress, in order. Chosen (`source`, `action`) deliberately share the
/// **selectable** look ([`Highlight::Available`]); only the connector marks them as the chosen ones. The
/// `target` is the one card still carrying its own look (the ring).
fn trio() -> Vec<Cell> {
    vec![
        Cell {
            caption: "source (chosen)",
            tile: tile(SOURCE, "Raider", Team::Left, Highlight::Available, "you"),
        },
        Cell {
            caption: "action (chosen)",
            tile: tile(ACTION, "Strike", Team::Left, Highlight::Available, "verb"),
        },
        Cell {
            caption: "target (completing)",
            tile: tile(TARGET, "The Wall", Team::Right, Highlight::Targeted, "1 hp"),
        },
    ]
}

/// The remaining looks in isolation, so the three distinct treatments can be compared side by side.
fn legend() -> Vec<Cell> {
    vec![
        Cell {
            caption: "selectable / chosen (same look)",
            tile: tile(
                LEGEND_SELECTABLE,
                "Bastion",
                Team::Left,
                Highlight::Available,
                "could pick",
            ),
        },
        Cell {
            caption: "completing (the ring)",
            tile: tile(
                LEGEND_TARGET,
                "The Sniper",
                Team::Right,
                Highlight::Targeted,
                "1 hp",
            ),
        },
        Cell {
            caption: "background",
            tile: tile(
                LEGEND_BACKGROUND,
                "Bombardier",
                Team::Left,
                Highlight::Dim,
                "reserve",
            ),
        },
    ]
}

/// The demo scene the real overlay systems read. Every drawn tile is in `body` (so the ring system finds the
/// `Targeted` ones), and `links` chains the trio: `source -> action` confirmed (green), `action -> target`
/// proposed (amber) - the two-color flow.
fn demo_scene() -> Scene {
    let all: Vec<Tile> = trio().into_iter().chain(legend()).map(|c| c.tile).collect();
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
        prompt: "source -> action -> target: the connector marks the chosen chain".to_string(),
        body: SceneBody::Lanes(vec![Lane {
            label: "cards".into(),
            left: all,
            right: vec![],
        }]),
        links: vec![
            // The locked part of the chain flows green (confirmed).
            Link {
                from: CardId(SOURCE),
                to: CardId(ACTION),
                confirmed: true,
                broad: false,
            },
            // The proposed reach to the not-yet-chosen target flows amber (not confirmed).
            Link {
                from: CardId(ACTION),
                to: CardId(TARGET),
                confirmed: false,
                broad: false,
            },
        ],
        choices: vec![],
        log_title: String::new(),
        log: vec![],
        legend: vec![],
        reference: vec![],
        disabled_controls: vec![],
    }
}

/// Lay the demo out: a wide gesture row (the trio the connector runs through, spaced so the flowing dots have
/// room), then a compact legend row of the three distinct looks. Each cell is caption-above-tile so a caption
/// never pushes its tile sideways. Every tile is drawn with the REAL [`draw_scene_tile`].
fn build(mut commands: Commands) {
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart,
            row_gap: Val::Px(18.0),
            padding: UiRect::all(Val::Px(24.0)),
            ..default()
        })
        .with_children(|root| {
            text(root, "Selection states", 26.0, INK);
            text(
                root,
                "The gesture in progress. The two-color dots flowing source -> action -> target mark the chosen chain, so 'chosen' and 'selectable' can share one look.",
                14.0,
                MUTED,
            );

            // The gesture row: the trio, spaced wide so the connector dots have a clear run between centers.
            section_label(root, "the gesture");
            cell_row(root, &trio(), 150.0, 110.0);

            // The legend: the three distinct looks in isolation, packed tighter.
            section_label(root, "the looks, in isolation");
            cell_row(root, &legend(), 170.0, 28.0);
        });
}

/// A row of caption-above-tile cells with a fixed cell width and inter-cell gap.
fn cell_row(root: &mut ChildSpawnerCommands, cells: &[Cell], cell_w: f32, gap: f32) {
    root.spawn(Node {
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        align_items: AlignItems::FlexStart,
        column_gap: Val::Px(gap),
        row_gap: Val::Px(16.0),
        ..default()
    })
    .with_children(|row| {
        for cell in cells {
            row.spawn(Node {
                width: Val::Px(cell_w),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(6.0),
                ..default()
            })
            .with_children(|c| {
                c.spawn((Text::new(cell.caption), TextColor(MUTED)));
                draw_scene_tile(c, &cell.tile);
            });
        }
    });
}

fn section_label(parent: &mut ChildSpawnerCommands, s: &str) {
    text(parent, s, 13.0, MUTED);
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
