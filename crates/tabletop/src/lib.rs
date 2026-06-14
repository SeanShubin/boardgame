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

use bevy::prelude::*;
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
            .add_systems(Startup, setup_camera)
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

/// Marks the root entity of the current table so it can be torn down on redraw.
#[derive(Component)]
struct TableRoot;

/// An action button, carrying its index into the current legal-action list.
#[derive(Component)]
struct ActionButton(usize);

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
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

/// Close the app when the game requests it (e.g. the player chose "Exit").
fn quit_if_requested<G: Game + Clone>(
    game: Res<GameRes<G>>,
    state: Res<StateRes<G>>,
    mut exit: MessageWriter<AppExit>,
) {
    if game.0.exit_requested(&state.0) {
        exit.write(AppExit::Success);
    }
}

fn redraw<G: Game + Clone>(
    mut commands: Commands,
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
    let actions = game.0.legal_actions(&state.0);
    let labels: Vec<String> = actions
        .iter()
        .map(|action| game.0.action_label(&state.0, action))
        .collect();

    build_table(&mut commands, &view, &labels);
}

// ---- palette ------------------------------------------------------------

const FELT: Color = Color::srgb(0.06, 0.13, 0.10);
const INK: Color = Color::srgb(0.92, 0.95, 0.93);
const PANEL: Color = Color::srgb(0.10, 0.18, 0.15);
const CARD_FACE: Color = Color::srgb(0.94, 0.92, 0.84);
const CARD_INK: Color = Color::srgb(0.10, 0.10, 0.13);
const CARD_BACK: Color = Color::srgb(0.20, 0.24, 0.42);
const CARD_BACK_INNER: Color = Color::srgb(0.30, 0.35, 0.56);
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

fn build_table(commands: &mut Commands, view: &TableView, action_labels: &[String]) {
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
                    overflow: Overflow::clip(),
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
                for (index, label) in action_labels.iter().enumerate() {
                    spawn_action_button(left, index, label);
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

                // The zones fill the remaining space.
                main.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.0),
                    flex_grow: 1.0,
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
                ..default()
            },
            BackgroundColor(CARD_BACK),
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
                ..default()
            },
            BackgroundColor(CARD_FACE),
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
