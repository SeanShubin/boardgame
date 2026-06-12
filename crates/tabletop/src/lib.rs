//! A Bevy presentation layer that can draw and drive any [`engine::Game`].
//!
//! [`TabletopPlugin`] is generic over the game. It holds the game's rules, runs
//! a fresh match, renders the game's [`TableView`](engine::TableView) as a
//! simple UI, and offers the current player's legal actions as buttons.
//! Clicking a button applies that action and the table redraws. Because the
//! plugin only ever talks to the [`engine::Game`] trait, it never needs to know
//! which game it is showing.
//!
//! This is a deliberately plain skeleton: cards are coloured boxes with a
//! label, zones are rows, and actions are buttons. It is enough to see a game
//! play through; richer table rendering can grow on top of the same seam.

use bevy::prelude::*;
use engine::{CardFace, Game, TableView, ZoneView};

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
            .add_systems(Update, (apply_clicked_action::<G>, redraw::<G>).chain());
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

/// Some shared colours for the table.
const FELT: Color = Color::srgb(0.05, 0.16, 0.11);
const INK: Color = Color::srgb(0.92, 0.95, 0.93);
const CARD_FACE: Color = Color::srgb(0.93, 0.91, 0.80);
const CARD_BACK: Color = Color::srgb(0.28, 0.30, 0.46);
const CARD_INK: Color = Color::srgb(0.08, 0.08, 0.12);
const BUTTON: Color = Color::srgb(0.20, 0.42, 0.62);

fn build_table(commands: &mut Commands, view: &TableView, action_labels: &[String]) {
    commands
        .spawn((
            TableRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::all(Val::Px(20.0)),
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(FELT),
        ))
        .with_children(|root| {
            // Status line.
            root.spawn((
                Text::new(view.status.clone()),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                TextColor(INK),
            ));

            // The zones, stacked top to bottom.
            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|zones| {
                for zone in &view.zones {
                    spawn_zone(zones, zone);
                }
            });

            // The action buttons for the current player.
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(12.0),
                ..default()
            })
            .with_children(|row| {
                for (index, label) in action_labels.iter().enumerate() {
                    spawn_action_button(row, index, label);
                }
            });
        });
}

fn spawn_zone(parent: &mut ChildSpawnerCommands, zone: &ZoneView) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Node {
                    width: Val::Px(230.0),
                    ..default()
                },
                Text::new(zone.label.clone()),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(INK),
            ));
            for card in &zone.cards {
                spawn_card(row, &card.face);
            }
        });
}

fn spawn_card(parent: &mut ChildSpawnerCommands, face: &CardFace) {
    let (text, background) = match face {
        CardFace::Down => ("\u{2592}".to_string(), CARD_BACK),
        CardFace::Up { title, value } => {
            let text = match value {
                Some(value) => format!("{title}\n{value}"),
                None => title.clone(),
            };
            (text, CARD_FACE)
        }
    };
    parent
        .spawn((
            Node {
                width: Val::Px(74.0),
                height: Val::Px(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(background),
        ))
        .with_children(|card| {
            card.spawn((
                Text::new(text),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(CARD_INK),
            ));
        });
}

fn spawn_action_button(parent: &mut ChildSpawnerCommands, index: usize, label: &str) {
    parent
        .spawn((
            Button,
            ActionButton(index),
            Node {
                padding: UiRect::axes(Val::Px(18.0), Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(BUTTON),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label.to_string()),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(INK),
            ));
        });
}
