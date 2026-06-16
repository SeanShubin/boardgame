//! A Bevy presentation layer that can draw and drive any [`engine::Game`].
//!
//! [`TabletopPlugin`] is generic over the game. It holds the game's rules, runs
//! a fresh match, renders the game's [`TableView`](engine::TableView), and
//! offers the current player's legal actions as buttons. Clicking a button
//! applies that action and the table redraws; Escape (or Backspace) applies the
//! game's [`cancel_action`](engine::Game::cancel_action) to rewind a multi-step
//! choice. Because the plugin only ever talks to the [`engine::Game`] trait, it
//! never needs to know which game it is showing.
//!
//! Cards are drawn collectible-card-game style — a title bar, a type line, a
//! body of stat / rules lines, and a corner badge — coloured by the card's
//! [`Accent`](engine::Accent). There is no art, so the space is information.

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::picking::hover::HoverMap;
use bevy::prelude::*;
use bevy::ui::{ComputedNode, OverflowAxis, ScrollPosition};
use engine::{Accent, CardFace, Game, TableView, ZoneView};

/// Drives a single match of `G` on a Bevy app.
pub struct TabletopPlugin<G: Game> {
    game: G,
    seed: u64,
    players: usize,
}

impl<G: Game> TabletopPlugin<G> {
    /// Sets up a match of `game` for `players` seats, seeded by `seed`.
    pub fn new(game: G, seed: u64, players: usize) -> Self {
        Self {
            game,
            seed,
            players,
        }
    }
}

impl<G: Game + Clone> Plugin for TabletopPlugin<G> {
    fn build(&self, app: &mut App) {
        let game = self.game.clone();
        let state = game.new_game(self.seed, self.players);
        app.insert_resource(GameRes(game))
            .insert_resource(StateRes::<G>(state))
            .insert_resource(NeedsRedraw(true))
            .insert_resource(Platform::detect())
            .add_systems(Startup, (setup_camera, install_ui_font))
            .add_observer(on_scroll_handler)
            .add_systems(Update, (adjust_zoom, send_scroll_events))
            .add_systems(
                Update,
                (
                    apply_clicked_action::<G>,
                    cancel_on_key::<G>,
                    quit_if_requested::<G>,
                    redraw::<G>,
                )
                    .chain(),
            );
    }
}

/// The immutable rules of the running game.
#[derive(Resource)]
struct GameRes<G: Game>(G);

/// The mutable state of the running game.
#[derive(Resource)]
struct StateRes<G: Game>(G::State);

/// Set whenever the table needs to be rebuilt.
#[derive(Resource)]
struct NeedsRedraw(bool);

/// What the host platform can do for the running app. This is the one place
/// that distinguishes a native window from a browser tab, so the rest of the
/// presentation asks in plain terms (e.g. "can we quit?") instead of testing
/// the target architecture inline.
#[derive(Resource, Clone, Copy)]
struct Platform {
    /// Whether the app can quit itself. A native window can; a browser tab
    /// cannot — calling `AppExit` there just freezes the canvas — so actions
    /// that would request a quit are hidden instead of offered.
    can_quit: bool,
}

impl Platform {
    fn detect() -> Self {
        Self {
            can_quit: !cfg!(target_arch = "wasm32"),
        }
    }
}

/// Marks the root entity of the current table so it can be torn down on redraw.
#[derive(Component)]
struct TableRoot;

/// An action button, carrying its index into the current legal-action list.
#[derive(Component)]
struct ActionButton(usize);

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// Inter (Regular), bundled so displayed text can render the typographic glyphs
/// games actually use — em dashes, curly quotes, arrows — that Bevy's built-in
/// `FiraMono-subset` font lacks (they'd otherwise show as tofu boxes).
/// SIL Open Font License; see `fonts/Inter-LICENSE.txt`.
const UI_FONT: &[u8] = include_bytes!("../fonts/Inter-Regular.ttf");

/// Replace Bevy's ASCII-only default font with the bundled Inter face. Bevy
/// registers its default font at `AssetId::default()`, and every
/// `TextFont { ..default() }` in this crate points there, so overwriting that
/// one asset reskins all UI text without threading a font handle through each
/// label. Runs in `Startup`, after `TextPlugin` has inserted the original.
fn install_ui_font(mut fonts: ResMut<Assets<Font>>) {
    let font = Font::try_from_bytes(UI_FONT.to_vec()).expect("bundled UI font is valid");
    fonts
        .insert(AssetId::default(), font)
        .expect("override the default font");
}

fn apply_clicked_action<G: Game + Clone>(
    buttons: Query<(&Interaction, &ActionButton), Changed<Interaction>>,
    game: Res<GameRes<G>>,
    mut state: ResMut<StateRes<G>>,
    mut redraw: ResMut<NeedsRedraw>,
) {
    for (interaction, button) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        // The action list is a pure function of the unchanged state, so the
        // index captured when the button was built is still valid here.
        let actions = game.0.legal_actions(&state.0);
        if let Some(action) = actions.get(button.0).cloned() {
            if game.0.apply(&mut state.0, &action).is_ok() {
                redraw.0 = true;
            }
        }
    }
}

/// Escape / Backspace rewind one step of a multi-step decision.
fn cancel_on_key<G: Game + Clone>(
    keys: Res<ButtonInput<KeyCode>>,
    game: Res<GameRes<G>>,
    mut state: ResMut<StateRes<G>>,
    mut redraw: ResMut<NeedsRedraw>,
) {
    if keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::Backspace) {
        if let Some(action) = game.0.cancel_action(&state.0) {
            if game.0.apply(&mut state.0, &action).is_ok() {
                redraw.0 = true;
            }
        }
    }
}

/// Close the app when the game requests it (e.g. the player chose "Exit") and
/// the platform can honor a quit. On the web the quit-requesting action is
/// never offered (see `redraw`), so this is a no-op there.
fn quit_if_requested<G: Game + Clone>(
    platform: Res<Platform>,
    game: Res<GameRes<G>>,
    state: Res<StateRes<G>>,
    mut exit: MessageWriter<AppExit>,
) {
    if platform.can_quit && game.0.exit_requested(&state.0) {
        exit.write(AppExit::Success);
    }
}

/// Keyboard zoom. Because the whole table is Bevy UI (not world-space sprites),
/// a camera zoom would do nothing — `UiScale` is the lever that scales it. `=`
/// / `+` zooms in, `-` zooms out, `0` resets to 1.0. Driving `UiScale` keeps
/// zoom fully programmatic, so a later system can snap to a region (e.g. an
/// active duel) and restore the player's zoom when the duel is done.
fn adjust_zoom(keys: Res<ButtonInput<KeyCode>>, mut ui_scale: ResMut<UiScale>) {
    const STEP: f32 = 0.1;
    const MIN: f32 = 0.3;
    const MAX: f32 = 2.5;

    let mut scale = ui_scale.0;
    if keys.just_pressed(KeyCode::Equal) || keys.just_pressed(KeyCode::NumpadAdd) {
        scale += STEP;
    }
    if keys.just_pressed(KeyCode::Minus) || keys.just_pressed(KeyCode::NumpadSubtract) {
        scale -= STEP;
    }
    if keys.just_pressed(KeyCode::Digit0) || keys.just_pressed(KeyCode::Numpad0) {
        scale = 1.0;
    }
    let scale = scale.clamp(MIN, MAX);
    if scale != ui_scale.0 {
        ui_scale.0 = scale;
    }
}

/// One mouse-wheel turn, aimed at the UI node under the cursor. It is an
/// `EntityEvent` so it bubbles up the hierarchy to the nearest scrollable
/// ancestor, which consumes it.
#[derive(EntityEvent, Debug)]
#[entity_event(propagate, auto_propagate)]
struct Scroll {
    entity: Entity,
    /// Scroll amount in logical pixels.
    delta: Vec2,
}

/// Logical pixels to scroll per wheel line (when the wheel reports lines).
const SCROLL_LINE_HEIGHT: f32 = 21.0;

/// Turn raw wheel input into a [`Scroll`] aimed at whatever node the pointer is
/// over; the event then bubbles to the scrollable container.
fn send_scroll_events(
    mut wheel: MessageReader<MouseWheel>,
    hover_map: Res<HoverMap>,
    mut commands: Commands,
) {
    for event in wheel.read() {
        let mut delta = -Vec2::new(event.x, event.y);
        if event.unit == MouseScrollUnit::Line {
            delta *= SCROLL_LINE_HEIGHT;
        }
        for pointer_map in hover_map.values() {
            for entity in pointer_map.keys().copied() {
                commands.trigger(Scroll { entity, delta });
            }
        }
    }
}

/// Apply a [`Scroll`] to a node if it scrolls on that axis and is not already
/// at the end; otherwise let it bubble to the parent. Mirrors Bevy's UI scroll
/// example.
fn on_scroll_handler(
    mut scroll: On<Scroll>,
    mut nodes: Query<(&mut ScrollPosition, &Node, &ComputedNode)>,
) {
    let Ok((mut position, node, computed)) = nodes.get_mut(scroll.entity) else {
        return;
    };

    let max_offset = (computed.content_size() - computed.size()) * computed.inverse_scale_factor();
    let delta = &mut scroll.delta;

    if node.overflow.x == OverflowAxis::Scroll && delta.x != 0.0 {
        let at_end = if delta.x > 0.0 {
            position.x >= max_offset.x
        } else {
            position.x <= 0.0
        };
        if !at_end {
            position.x += delta.x;
            delta.x = 0.0;
        }
    }

    if node.overflow.y == OverflowAxis::Scroll && delta.y != 0.0 {
        let at_end = if delta.y > 0.0 {
            position.y >= max_offset.y
        } else {
            position.y <= 0.0
        };
        if !at_end {
            position.y += delta.y;
            delta.y = 0.0;
        }
    }

    // Once fully applied, stop bubbling so an ancestor does not also scroll.
    if *delta == Vec2::ZERO {
        scroll.propagate(false);
    }
}

fn redraw<G: Game + Clone>(
    mut commands: Commands,
    platform: Res<Platform>,
    game: Res<GameRes<G>>,
    state: Res<StateRes<G>>,
    mut redraw: ResMut<NeedsRedraw>,
    roots: Query<Entity, With<TableRoot>>,
) {
    if !redraw.0 {
        return;
    }
    redraw.0 = false;

    for entity in &roots {
        commands.entity(entity).despawn();
    }

    let view = game.0.view(&state.0, None);
    // Each button carries its index into the full legal-action list, so hiding
    // some (e.g. Exit on the web) never misaligns clicks with actions.
    let buttons: Vec<(usize, String)> = game
        .0
        .legal_actions(&state.0)
        .iter()
        .enumerate()
        .filter(|(_, action)| platform.can_quit || !game.0.is_exit_action(&state.0, action))
        .map(|(index, action)| (index, game.0.action_label(&state.0, action)))
        .collect();

    build_table(&mut commands, &view, &buttons);
}

// ---- palette ------------------------------------------------------------

const FELT: Color = Color::srgb(0.06, 0.13, 0.10);
const INK: Color = Color::srgb(0.92, 0.95, 0.93);
const PANEL: Color = Color::srgb(0.10, 0.18, 0.15);
const CARD_FACE: Color = Color::srgb(0.94, 0.92, 0.84);
const CARD_INK: Color = Color::srgb(0.10, 0.10, 0.13);
const CARD_BACK: Color = Color::srgb(0.20, 0.24, 0.42);
const CARD_BACK_INNER: Color = Color::srgb(0.30, 0.35, 0.56);
/// A dark edge drawn around every card so overlapping cards stay distinct.
const CARD_EDGE: Color = Color::srgb(0.12, 0.11, 0.10);
const CARD_BORDER: f32 = 2.0;
const BADGE: Color = Color::srgb(0.14, 0.14, 0.18);
const TITLE_INK: Color = Color::srgb(0.97, 0.97, 0.98);
const BUTTON: Color = Color::srgb(0.18, 0.40, 0.60);

fn accent_color(accent: Accent) -> Color {
    match accent {
        Accent::Neutral => Color::srgb(0.34, 0.36, 0.40),
        Accent::Ally => Color::srgb(0.20, 0.42, 0.66),
        Accent::Foe => Color::srgb(0.62, 0.22, 0.24),
        Accent::Warn => Color::srgb(0.72, 0.48, 0.14),
        Accent::Good => Color::srgb(0.22, 0.52, 0.32),
        Accent::Selected => Color::srgb(0.66, 0.56, 0.16),
    }
}

fn build_table(commands: &mut Commands, view: &TableView, actions: &[(usize, String)]) {
    commands
        .spawn((
            TableRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                ..default()
            },
            BackgroundColor(FELT),
        ))
        .with_children(|root| {
            // LEFT: the choices to make, stacked top to bottom.
            root.spawn((
                Node {
                    width: Val::Px(300.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(12.0)),
                    row_gap: Val::Px(8.0),
                    // Scroll instead of clip, so a long action list stays
                    // reachable when it overflows the panel height.
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                BackgroundColor(PANEL),
            ))
            .with_children(|left| {
                left.spawn((
                    Text::new("Choose an action"),
                    TextFont {
                        font_size: 17.0,
                        ..default()
                    },
                    TextColor(INK),
                ));
                for (index, label) in actions {
                    spawn_action_button(left, *index, label);
                }
            });

            // CENTER: status on top, then the board filling the rest.
            root.spawn(Node {
                flex_grow: 1.0,
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(12.0),
                overflow: Overflow::clip(),
                ..default()
            })
            .with_children(|main| {
                // Status / log panel.
                main.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(PANEL),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new(view.status.clone()),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(INK),
                    ));
                });

                // The zones fill the remaining space, and scroll vertically when
                // the board is taller than the area (e.g. duels need more room).
                main.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.0),
                    flex_grow: 1.0,
                    overflow: Overflow::scroll_y(),
                    ..default()
                })
                .with_children(|zones| {
                    for zone in &view.zones {
                        spawn_zone(zones, zone);
                    }
                });
            });
        });
}

fn spawn_zone(parent: &mut ChildSpawnerCommands, zone: &ZoneView) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|col| {
            col.spawn((
                Text::new(zone.label.clone()),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                TextColor(INK),
            ));
            col.spawn(Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::FlexStart,
                column_gap: Val::Px(8.0),
                row_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|row| {
                // Collapse runs of identical cards into one overlapped stack, so
                // you read the top card and see how many there are.
                let cards = &zone.cards;
                let mut i = 0;
                while i < cards.len() {
                    let mut j = i + 1;
                    while j < cards.len() && cards[j] == cards[i] {
                        j += 1;
                    }
                    spawn_card_group(row, &cards[i].face, j - i);
                    i = j;
                }
            });
        });
}

const STACK_PEEK: f32 = 24.0;

/// Render `count` identical cards: a single card if one, else an overlapped
/// stack — the top card fully readable, the rest peeking — with an `xN` badge.
fn spawn_card_group(parent: &mut ChildSpawnerCommands, face: &CardFace, count: usize) {
    if count <= 1 {
        spawn_card(parent, face);
        return;
    }
    let width = CARD_W + (count as f32 - 1.0) * STACK_PEEK;
    parent
        .spawn(Node {
            width: Val::Px(width),
            height: Val::Px(CARD_H),
            ..default()
        })
        .with_children(|stack| {
            for k in 0..count {
                stack
                    .spawn(Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(k as f32 * STACK_PEEK),
                        top: Val::Px(0.0),
                        ..default()
                    })
                    .with_children(|slot| spawn_card(slot, face));
            }
            stack
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(4.0),
                        top: Val::Px(4.0),
                        padding: UiRect::axes(Val::Px(7.0), Val::Px(3.0)),
                        ..default()
                    },
                    BackgroundColor(BADGE),
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new(format!("x{count}")),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(TITLE_INK),
                    ));
                });
        });
}

const CARD_W: f32 = 156.0;
const CARD_H: f32 = 196.0;

fn spawn_card(parent: &mut ChildSpawnerCommands, face: &CardFace) {
    match face {
        CardFace::Down => spawn_card_back(parent),
        CardFace::Up {
            title,
            type_line,
            body,
            corner,
            accent,
        } => spawn_card_face(parent, title, type_line.as_deref(), body, corner.as_deref(), *accent),
    }
}

fn spawn_card_back(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                width: Val::Px(CARD_W),
                height: Val::Px(CARD_H),
                padding: UiRect::all(Val::Px(10.0)),
                border: UiRect::all(Val::Px(CARD_BORDER)),
                ..default()
            },
            BackgroundColor(CARD_BACK),
            BorderColor::all(CARD_EDGE),
        ))
        .with_children(|card| {
            card.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(CARD_BACK_INNER),
            ));
        });
}

fn spawn_card_face(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    type_line: Option<&str>,
    body: &[String],
    corner: Option<&str>,
    accent: Accent,
) {
    parent
        .spawn((
            Node {
                width: Val::Px(CARD_W),
                height: Val::Px(CARD_H),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip(),
                border: UiRect::all(Val::Px(CARD_BORDER)),
                ..default()
            },
            BackgroundColor(CARD_FACE),
            BorderColor::all(CARD_EDGE),
        ))
        .with_children(|card| {
            // Title bar (accent-coloured).
            card.spawn((
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
                    ..default()
                },
                BackgroundColor(accent_color(accent)),
            ))
            .with_children(|bar| {
                bar.spawn((
                    Text::new(title.to_string()),
                    TextFont {
                        font_size: 15.0,
                        ..default()
                    },
                    TextColor(TITLE_INK),
                ));
            });

            // Type line.
            if let Some(t) = type_line {
                card.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.86, 0.84, 0.74)),
                ))
                .with_children(|line| {
                    line.spawn((
                        Text::new(t.to_string()),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.28, 0.28, 0.30)),
                    ));
                });
            }

            // Body — stat / rules lines.
            card.spawn(Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(3.0),
                flex_grow: 1.0,
                ..default()
            })
            .with_children(|b| {
                for line in body {
                    b.spawn((
                        Text::new(line.clone()),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(CARD_INK),
                    ));
                }
            });

            // Corner badge (the power/toughness spot).
            if let Some(c) = corner {
                card.spawn(Node {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::FlexEnd,
                    padding: UiRect::all(Val::Px(6.0)),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        Node {
                            padding: UiRect::axes(Val::Px(8.0), Val::Px(3.0)),
                            ..default()
                        },
                        BackgroundColor(BADGE),
                    ))
                    .with_children(|badge| {
                        badge.spawn((
                            Text::new(c.to_string()),
                            TextFont {
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(TITLE_INK),
                        ));
                    });
                });
            }
        });
}

fn spawn_action_button(parent: &mut ChildSpawnerCommands, index: usize, label: &str) {
    parent
        .spawn((
            Button,
            ActionButton(index),
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(10.0)),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BUTTON),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label.to_string()),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(INK),
            ));
        });
}
