//! A Bevy renderer that draws any [`contract::Game`] through the **card-table metaphor**: every zone
//! is a deck, decks collapse into labelled, counted piles when unattended, and clicking a deck fans
//! it out (focus) while clicking the table zooms back out. It is the second renderer in the
//! workspace, parallel to `tabletop`, built on the same [`contract::TableView`] seam — so the rules
//! and tests do not move.
//!
//! This crate is the thin Bevy shell; all the deck/card *behavior* lives in the pure
//! [`cardtable_model`] crate (selecting, reordering, moving between decks, focus/zoom), which is
//! game-free and unit-tested in isolation. Here we only: build the model from the game's view, draw
//! it as `bevy_ui`, and turn clicks into model focus changes or game actions.
//!
//! **Rendering approach:** `bevy_ui` (flexbox), matching `tabletop`. The deck/zoom model is renderer
//! agnostic, so a future 3D table could be built against the same model — see
//! `docs/games/deckbound/presentation/card-table-ui.md` §7 (the open flexbox-vs-3D question).
//!
//! Status: a first increment. It shows the deck/collapse/zoom interaction and keeps the game fully
//! playable (any action not bound to a card appears as a button). Card faces show titles only for now;
//! richer faces, selection, drag-to-move, and the damage deck are future work on top of the model.

use bevy::prelude::*;

use cardtable_model::{Card, DeckId, DeckTree, Face, from_table_view};
use contract::Game;

/// Drives a single match of `G`, drawn as a card table.
pub struct CardTablePlugin<G: Game> {
    game: G,
    seed: u64,
    players: usize,
}

impl<G: Game> CardTablePlugin<G> {
    /// Sets up a match of `game` for `players` seats, seeded by `seed`.
    pub fn new(game: G, seed: u64, players: usize) -> Self {
        Self {
            game,
            seed,
            players,
        }
    }
}

impl<G: Game + Clone> Plugin for CardTablePlugin<G> {
    fn build(&self, app: &mut App) {
        let game = self.game.clone();
        let state = game.new_game(self.seed, self.players);
        let tree = from_table_view(&game.view(&state, None));
        app.insert_resource(GameRes(game))
            .insert_resource(StateRes::<G>(state))
            .insert_resource(Tree(tree))
            .insert_resource(NeedsRedraw(true))
            .add_systems(Startup, setup_camera)
            .add_systems(
                Update,
                (
                    handle_focus,
                    handle_zoom_out,
                    handle_action::<G>,
                    redraw::<G>,
                )
                    .chain(),
            );
    }
}

// ---- resources ----------------------------------------------------------

/// The immutable rules of the running game.
#[derive(Resource)]
struct GameRes<G: Game>(G);

/// The mutable state of the running game.
#[derive(Resource)]
struct StateRes<G: Game>(G::State);

/// The presentation model — the deck tree with its focus/selection. Rebuilt from the game's view
/// whenever the game advances; mutated in place for focus-only (zoom) changes.
#[derive(Resource)]
struct Tree(DeckTree);

/// Set whenever the table needs to be rebuilt.
#[derive(Resource)]
struct NeedsRedraw(bool);

// ---- components ---------------------------------------------------------

/// The UI root, despawned and rebuilt each redraw.
#[derive(Component)]
struct CardTableRoot;

/// A clickable collapsed deck — clicking focuses (fans) it.
#[derive(Component)]
struct FocusButton(DeckId);

/// A clickable control that zooms out one level.
#[derive(Component)]
struct ZoomOutButton;

/// A clickable card or button bound to the legal action at this index.
#[derive(Component)]
struct ActionCard(usize);

// ---- systems ------------------------------------------------------------

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// Click a collapsed deck → focus it (fan it open, collapse the rest).
fn handle_focus(
    buttons: Query<(&Interaction, &FocusButton), Changed<Interaction>>,
    mut tree: ResMut<Tree>,
    mut redraw: ResMut<NeedsRedraw>,
) {
    for (interaction, button) in &buttons {
        if *interaction == Interaction::Pressed && tree.0.focus(button.0).is_ok() {
            redraw.0 = true;
        }
    }
}

/// Click the zoom-out control → focus moves to the current focus's parent.
fn handle_zoom_out(
    buttons: Query<&Interaction, (Changed<Interaction>, With<ZoomOutButton>)>,
    mut tree: ResMut<Tree>,
    mut redraw: ResMut<NeedsRedraw>,
) {
    if buttons.iter().any(|i| *i == Interaction::Pressed) {
        tree.0.zoom_out();
        redraw.0 = true;
    }
}

/// Click an actionable card or an action button → apply that legal action and rebuild the model from
/// the new view.
fn handle_action<G: Game + Clone>(
    buttons: Query<(&Interaction, &ActionCard), Changed<Interaction>>,
    game: Res<GameRes<G>>,
    mut state: ResMut<StateRes<G>>,
    mut tree: ResMut<Tree>,
    mut redraw: ResMut<NeedsRedraw>,
) {
    for (interaction, button) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        // The action list is a pure function of the unchanged state, so the index captured when the
        // control was built is still valid here.
        let actions = game.0.legal_actions(&state.0);
        if let Some(action) = actions.get(button.0).cloned()
            && game.0.apply(&mut state.0, &action).is_ok()
        {
            tree.0 = from_table_view(&game.0.view(&state.0, None));
            redraw.0 = true;
        }
    }
}

fn redraw<G: Game + Clone>(
    mut commands: Commands,
    game: Res<GameRes<G>>,
    state: Res<StateRes<G>>,
    tree: Res<Tree>,
    mut redraw: ResMut<NeedsRedraw>,
    roots: Query<Entity, With<CardTableRoot>>,
) {
    if !redraw.0 {
        return;
    }
    redraw.0 = false;
    for entity in &roots {
        commands.entity(entity).despawn();
    }

    let view = game.0.view(&state.0, None);
    // Actions already bound to an on-card control are not also shown as buttons (no choice appears
    // twice — the card *is* the control).
    let bound: std::collections::HashSet<usize> = view
        .zones
        .iter()
        .flat_map(|z| z.cards.iter().filter_map(|c| c.action))
        .collect();
    let buttons: Vec<(usize, String)> = game
        .0
        .legal_actions(&state.0)
        .iter()
        .enumerate()
        .filter(|(index, _)| !bound.contains(index))
        .map(|(index, action)| (index, game.0.action_label(&state.0, action)))
        .collect();

    build_ui(&mut commands, &tree.0, &view.status, &buttons);
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
/// Highlight edge for a card/deck that carries a legal move.
const ACTIONABLE: Color = Color::srgb(0.30, 0.70, 0.62);

const FONT_HEAD: f32 = 18.0;
const FONT_TITLE: f32 = 15.0;
const FONT_BODY: f32 = 13.0;

fn build_ui(commands: &mut Commands, tree: &DeckTree, status: &str, buttons: &[(usize, String)]) {
    let at_root = tree.focus_id() == tree.root_id();
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
            // LEFT: any actions not represented as a card on the table.
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
            .with_children(|left| {
                left.spawn((
                    Text::new("Actions"),
                    TextFont {
                        font_size: FONT_HEAD,
                        ..default()
                    },
                    TextColor(INK),
                ));
                for (index, label) in buttons {
                    spawn_button(left, *index, label);
                }
            });

            // CENTER: status, a zoom-out control, then the decks.
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
                main.spawn((
                    Text::new(status.to_string()),
                    TextFont {
                        font_size: FONT_HEAD,
                        ..default()
                    },
                    TextColor(INK),
                ));

                if !at_root {
                    main.spawn((
                        Button,
                        ZoomOutButton,
                        Node {
                            padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                            border_radius: BorderRadius::all(Val::Px(6.0)),
                            align_self: AlignSelf::FlexStart,
                            ..default()
                        },
                        BackgroundColor(BUTTON),
                    ))
                    .with_children(|b| {
                        b.spawn((
                            Text::new("\u{25B2} Zoom out"),
                            TextFont {
                                font_size: FONT_TITLE,
                                ..default()
                            },
                            TextColor(INK),
                        ));
                    });
                }

                // The decks under the root, each collapsed into a chip or fanned open.
                main.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(12.0),
                    row_gap: Val::Px(12.0),
                    align_items: AlignItems::FlexStart,
                    ..default()
                })
                .with_children(|decks| {
                    let root_deck = tree.deck(tree.root_id()).expect("root exists");
                    for &zone in root_deck.subdecks() {
                        spawn_deck(decks, tree, zone);
                    }
                });
            });
        });
}

/// Draws a deck: a compact, counted chip when collapsed, or a fanned panel of its cards when open.
fn spawn_deck(parent: &mut ChildSpawnerCommands, tree: &DeckTree, id: DeckId) {
    let deck = tree.deck(id).expect("deck id from tree");
    if deck.collapsed {
        let count = deck.cards().len() + deck.subdecks().len();
        parent
            .spawn((
                Button,
                FocusButton(id),
                Node {
                    width: Val::Px(120.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.0)),
                    row_gap: Val::Px(4.0),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(CARD_BACK),
            ))
            .with_children(|chip| {
                chip.spawn((
                    Text::new(deck.label.clone()),
                    TextFont {
                        font_size: FONT_TITLE,
                        ..default()
                    },
                    TextColor(INK),
                ));
                chip.spawn((
                    Text::new(format!("{count} cards")),
                    TextFont {
                        font_size: FONT_BODY,
                        ..default()
                    },
                    TextColor(MUTED),
                ));
            });
    } else {
        parent
            .spawn((
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
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(8.0),
                        row_gap: Val::Px(8.0),
                        ..default()
                    })
                    .with_children(|cards| {
                        for &cid in deck.cards() {
                            spawn_card(cards, tree.card(cid).expect("card id from deck"));
                        }
                        // Nested decks (none from the current binding, but keep the recursion honest).
                        for &sid in deck.subdecks() {
                            spawn_deck(cards, tree, sid);
                        }
                    });
            });
    }
}

/// Draws one card: a light face showing its title, or a dark back. Actionable cards get a highlight
/// edge and become clickable (applying their bound legal action).
fn spawn_card(parent: &mut ChildSpawnerCommands, card: &Card) {
    let (label, bg, ink) = match &card.face {
        Face::Up { title } => (title.clone(), CARD_FACE, CARD_INK),
        Face::Down => ("\u{25AF}".to_string(), CARD_BACK, INK),
    };
    let mut entity = parent.spawn((
        Node {
            width: Val::Px(96.0),
            height: Val::Px(132.0),
            padding: UiRect::all(Val::Px(8.0)),
            border: UiRect::all(Val::Px(2.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(bg),
        BorderColor::all(if card.is_actionable() { ACTIONABLE } else { bg }),
    ));
    if let Some(index) = card.actionable {
        entity.insert((Button, ActionCard(index)));
    }
    entity.with_children(|c| {
        c.spawn((
            Text::new(label),
            TextFont {
                font_size: FONT_TITLE,
                ..default()
            },
            TextColor(ink),
        ));
    });
}

/// A left-panel button for a legal action not bound to a card.
fn spawn_button(parent: &mut ChildSpawnerCommands, index: usize, label: &str) {
    parent
        .spawn((
            Button,
            ActionCard(index),
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
                Text::new(label.to_string()),
                TextFont {
                    font_size: FONT_TITLE,
                    ..default()
                },
                TextColor(INK),
            ));
        });
}
