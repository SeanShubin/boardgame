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

use bevy::picking::events::{Click, DragDrop, Pointer};
use bevy::prelude::*;
use bevy::ui::BoxShadow;

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
            .configure_sets(
                Update,
                (CardTableSet::Input, CardTableSet::Apply, CardTableSet::Draw).chain(),
            )
            .add_systems(Startup, (setup_camera, install_ui_font))
            .add_systems(Update, redraw.in_set(CardTableSet::Draw))
            // Input is picking-driven (clicks open/close decks and fire actions; drags move
            // cards/decks), so it runs in observers rather than the Input system set.
            .add_observer(on_click)
            .add_observer(on_drop);
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
    let font = Font::try_from_bytes(UI_FONT.to_vec()).expect("bundled UI font is valid");
    fonts
        .insert(AssetId::default(), font)
        .expect("override the default font");
}

/// A picking click, resolved against the most specific target: an actionable control records its
/// action; a (non-actionable) card consumes the click so it doesn't bubble; a deck opens (focus); the
/// table background closes all decks. Inner nodes (e.g. a card's text) match nothing and fall through
/// to their parent via propagation. Global observer, so it survives the per-change UI rebuild.
fn on_click(
    mut on: On<Pointer<Click>>,
    actions: Query<&ActionControl>,
    cards: Query<&CardRef>,
    decks: Query<&DeckDropZone>,
    background: Query<(), With<CardTableRoot>>,
    mut table: ResMut<Table>,
    mut requests: ResMut<ActionRequests>,
) {
    let target = on.event().entity;
    if let Ok(action) = actions.get(target) {
        requests.0.push(action.0);
    } else if cards.get(target).is_ok() {
        // A non-actionable card: consume the click (don't focus a deck or close the table).
    } else if let Ok(deck) = decks.get(target) {
        let _ = table.0.focus(deck.0);
    } else if background.get(target).is_ok() {
        let root = table.0.root_id();
        let _ = table.0.focus(root);
    } else {
        return; // not interactive — let it propagate to an ancestor that is
    }
    on.propagate(false);
}

/// A picking drop: move the dragged card or deck into the target's deck — a deck, a card's home deck,
/// or the table background (the root). Cards dropped on the background are ignored; invalid deck moves
/// (cycles) are no-ops via the model. Presentation-level; mapping drops to game actions is future work.
fn on_drop(
    mut on: On<Pointer<DragDrop>>,
    cards: Query<&CardRef>,
    decks: Query<&DeckDropZone>,
    background: Query<(), With<CardTableRoot>>,
    mut table: ResMut<Table>,
) {
    let event = on.event();
    let dropped = event.event.dropped;
    let target = event.entity;

    let dragged_card = cards.get(dropped).ok().map(|c| c.0);
    let dragged_deck = decks.get(dropped).ok().map(|d| d.0);
    if dragged_card.is_none() && dragged_deck.is_none() {
        return; // dragged something that is neither a card nor a deck
    }

    let dest = if let Ok(deck) = decks.get(target) {
        Some(deck.0)
    } else if let Ok(card) = cards.get(target) {
        table.0.card(card.0).map(|c| c.home())
    } else if background.get(target).is_ok() {
        Some(table.0.root_id())
    } else {
        return; // not a drop target — let it propagate
    };
    let Some(dest) = dest else {
        return;
    };

    on.propagate(false);
    if let Some(card) = dragged_card {
        // Cards live only inside decks, and the root's own cards aren't drawn — so ignore card-on-table.
        if dest != table.0.root_id() {
            let at = table.0.deck(dest).map_or(0, |deck| deck.cards().len());
            let _ = table.0.move_card(card, dest, at);
        }
    } else if let Some(deck) = dragged_deck {
        let at = table.0.deck(dest).map_or(0, |d| d.subdecks().len());
        let _ = table.0.move_deck(deck, dest, at);
    }
}

/// Rebuild the UI whenever the presentation state changes (focus/zoom mutate `Table`; a consumer may
/// replace `Table`/`ActionRail`/`StatusLine`). Change-detection drives this — no manual dirty flag.
fn redraw(
    mut commands: Commands,
    table: Res<Table>,
    rail: Res<ActionRail>,
    status: Res<StatusLine>,
    roots: Query<Entity, With<CardTableRoot>>,
) {
    if !(table.is_changed() || rail.is_changed() || status.is_changed()) {
        return;
    }
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

const FONT_HEAD: f32 = 18.0;
const FONT_TITLE: f32 = 15.0;
const FONT_BODY: f32 = 13.0;

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
                DeckDropZone(id),
                Node {
                    width: Val::Px(120.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.0)),
                    row_gap: Val::Px(4.0),
                    border_radius: BorderRadius::all(Val::Px(10.0)),
                    ..default()
                },
                BackgroundColor(CARD_BACK),
                card_shadow(),
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
        ActionRail, ActionRequests, CardTablePlugin, CardTableSet, RailAction, StatusLine, Table,
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
