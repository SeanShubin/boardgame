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
//!
//! Presentation aims for *tactile* cards rather than flat sprites: rounded
//! corners, drop-shadows, a settle-in "deal" when the board redraws, and a
//! hover lift (the card scales up, rises, and its shadow deepens under the
//! pointer). All of it is pure presentation driven from `Interaction` into the
//! post-layout [`UiTransform`] / [`BoxShadow`], so it stays generic over the
//! game and never reflows neighbouring nodes. See [`animate_cards`].

use bevy::audio::{AddAudioSource, Decodable, Source};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::picking::hover::HoverMap;
use bevy::prelude::*;
use bevy::ui::{ComputedNode, GlobalZIndex, OverflowAxis, ScrollPosition};
use engine::{Accent, CardFace, CardView, Game, ProseLine, TableView, ZoneView};
use std::cell::Cell;
use std::time::Duration;

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
            .insert_resource(HelpVisible(false))
            .insert_resource(RulesVisible(false))
            .insert_resource(Muted(false))
            // Register the procedural sound-effect source (synthesised in code,
            // so there are no audio asset files to ship — see `Sfx`).
            .add_audio_source::<Sfx>()
            .add_systems(
                Startup,
                (
                    setup_camera,
                    install_ui_font,
                    setup_help,
                    setup_rules::<G>,
                    setup_sfx,
                ),
            )
            .add_observer(on_scroll_handler)
            .add_systems(Update, (adjust_zoom, send_scroll_events))
            // After `cancel_on_key` so that while the overlay is open it sees
            // `HelpVisible` still set, bows out of rewinding, and lets this
            // system consume Esc as "close help" instead.
            .add_systems(Update, toggle_help.after(cancel_on_key::<G>))
            .add_systems(Update, toggle_rules.after(cancel_on_key::<G>))
            // Pure-presentation juice: hover lift + settle-in. These read
            // `Interaction` and write `UiTransform`/shadow only, so they run
            // every frame independently of the redraw chain below.
            .add_systems(Update, (animate_cards, animate_buttons))
            // Sound: a click on action, a soft tick on card hover; `M` mutes.
            .add_systems(Update, (play_button_sfx, play_card_hover_sfx, toggle_mute))
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

/// Whether the help overlay is currently shown.
#[derive(Resource)]
struct HelpVisible(bool);

/// Whether the rules-reference (encyclopedia) overlay is currently shown.
#[derive(Resource)]
struct RulesVisible(bool);

/// Marks the rules-reference overlay so [`toggle_rules`] can show / hide it.
#[derive(Component)]
struct RulesOverlay;

/// Marks the root entity of the current table so it can be torn down on redraw.
#[derive(Component)]
struct TableRoot;

/// Marks the full-screen help overlay so [`toggle_help`] can show / hide it.
#[derive(Component)]
struct HelpOverlay;

/// Marks the always-visible "Press ? for help" hint, hidden while the overlay
/// itself is open so the two don't both show at once.
#[derive(Component)]
struct HelpHint;

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
    help: Res<HelpVisible>,
    rules: Res<RulesVisible>,
    game: Res<GameRes<G>>,
    mut state: ResMut<StateRes<G>>,
    mut redraw: ResMut<NeedsRedraw>,
) {
    // While an overlay is up, Esc closes it (handled by `toggle_help` / `toggle_rules`)
    // rather than rewinding a game step.
    if help.0 || rules.0 {
        return;
    }
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

/// Show / hide the help overlay. `?` (or `F1`) toggles it; `Esc` only ever
/// closes it (never opens it, so Esc stays free to rewind a move when no help
/// is showing). The hint is hidden while the overlay is up to avoid showing
/// both at once.
fn toggle_help(
    keys: Res<ButtonInput<KeyCode>>,
    mut visible: ResMut<HelpVisible>,
    mut overlay: Query<&mut Node, (With<HelpOverlay>, Without<HelpHint>)>,
    mut hint: Query<&mut Node, (With<HelpHint>, Without<HelpOverlay>)>,
) {
    let toggle = keys.just_pressed(KeyCode::Slash) || keys.just_pressed(KeyCode::F1);
    let close = visible.0 && keys.just_pressed(KeyCode::Escape);
    if !toggle && !close {
        return;
    }
    visible.0 = !close && !visible.0;

    let shown = |on: bool| if on { Display::Flex } else { Display::None };
    if let Ok(mut node) = overlay.single_mut() {
        node.display = shown(visible.0);
    }
    if let Ok(mut node) = hint.single_mut() {
        node.display = shown(!visible.0);
    }
}

/// `R` toggles the rules-reference overlay; Esc closes it. Mirrors [`toggle_help`].
fn toggle_rules(
    keys: Res<ButtonInput<KeyCode>>,
    mut visible: ResMut<RulesVisible>,
    mut overlay: Query<&mut Node, (With<RulesOverlay>, Without<HelpHint>)>,
    mut hint: Query<&mut Node, (With<HelpHint>, Without<RulesOverlay>)>,
) {
    let toggle = keys.just_pressed(KeyCode::KeyR);
    let close = visible.0 && keys.just_pressed(KeyCode::Escape);
    if !toggle && !close {
        return;
    }
    visible.0 = !close && !visible.0;

    let shown = |on: bool| if on { Display::Flex } else { Display::None };
    if let Ok(mut node) = overlay.single_mut() {
        node.display = shown(visible.0);
    }
    if let Ok(mut node) = hint.single_mut() {
        node.display = shown(!visible.0);
    }
}

/// Build the (initially hidden) rules-reference overlay from the game's [`reference`]: a
/// scrollable panel of entries grouped by category. Lives outside [`TableRoot`] so redraws
/// never tear it down. Generic over the game — the content is whatever it exposes.
///
/// [`reference`]: engine::Game::reference
fn setup_rules<G: Game + Clone>(mut commands: Commands, game: Res<GameRes<G>>) {
    let entries = game.0.reference();
    commands
        .spawn((
            RulesOverlay,
            GlobalZIndex(20),
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(SCRIM),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        width: Val::Px(680.0),
                        max_height: Val::Percent(84.0),
                        padding: UiRect::all(Val::Px(22.0)),
                        row_gap: Val::Px(8.0),
                        border_radius: BorderRadius::all(Val::Px(PANEL_RADIUS)),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    BackgroundColor(PANEL),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("Rules reference"),
                        TextFont {
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(TITLE_INK),
                    ));
                    let mut category = String::new();
                    for e in &entries {
                        if e.category != category {
                            category = e.category.clone();
                            panel.spawn((
                                Node {
                                    margin: UiRect::top(Val::Px(8.0)),
                                    ..default()
                                },
                                Text::new(e.category.clone()),
                                TextFont {
                                    font_size: 18.0,
                                    ..default()
                                },
                                TextColor(BUTTON),
                            ));
                        }
                        panel.spawn((
                            Text::new(e.term.clone()),
                            TextFont {
                                font_size: 15.0,
                                ..default()
                            },
                            TextColor(TITLE_INK),
                        ));
                        panel.spawn((
                            Text::new(e.text.clone()),
                            TextFont {
                                font_size: 13.0,
                                ..default()
                            },
                            TextColor(MUTED_INK),
                        ));
                    }
                    if entries.is_empty() {
                        panel.spawn((
                            Text::new("This game has no rules reference."),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(MUTED_INK),
                        ));
                    }
                    panel.spawn((
                        Node {
                            margin: UiRect::top(Val::Px(10.0)),
                            ..default()
                        },
                        Text::new("Press R or Esc to close"),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(MUTED_INK),
                    ));
                });
        });
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
    // Actions already bound to a clickable card are not also shown as buttons (no choice
    // appears twice — the card *is* the control).
    let bound: std::collections::HashSet<usize> = view
        .zones
        .iter()
        .flat_map(|z| z.cards.iter().filter_map(|c| c.action))
        .collect();
    // Each button carries its index into the full legal-action list, so hiding
    // some (e.g. Exit on the web, or actions bound to cards) never misaligns clicks.
    let buttons: Vec<(usize, String)> = game
        .0
        .legal_actions(&state.0)
        .iter()
        .enumerate()
        .filter(|(index, action)| {
            !bound.contains(index)
                && (platform.can_quit || !game.0.is_exit_action(&state.0, action))
        })
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
/// Dim wash drawn behind the help overlay so it reads as a modal layer.
const SCRIM: Color = Color::srgba(0.0, 0.0, 0.0, 0.6);
/// Backing for the always-visible help hint; the panel colour with some alpha
/// so it sits lightly over the table.
const HINT_BG: Color = Color::srgba(0.10, 0.18, 0.15, 0.85);
/// Muted ink for secondary lines (the overlay's "press … to close" footer).
const MUTED_INK: Color = Color::srgb(0.66, 0.72, 0.68);

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

// ---- motion & depth (the "juice") --------------------------------------
//
// All animation is post-layout: it drives each node's `UiTransform` (a render
// transform, like a CSS transform) and `BoxShadow`, so a card can swell over
// its neighbours on hover without pushing them around.

/// Corner rounding so cards read as physical objects, not rectangles.
const CARD_RADIUS: f32 = 10.0;
const BUTTON_RADIUS: f32 = 8.0;
const BADGE_RADIUS: f32 = 6.0;
const PANEL_RADIUS: f32 = 10.0;

/// How fast a hover blends in / out (higher = snappier), fed to [`approach`].
const HOVER_RATE: f32 = 16.0;

/// Settle-in ("deal") when a fresh board is drawn.
const CARD_INTRO: f32 = 0.16; // seconds for one card to land
const CARD_INTRO_SCALE: f32 = 0.94; // scale it grows from
const CARD_INTRO_DROP: f32 = 10.0; // px it rises from
const CARD_STAGGER: f32 = 0.012; // per-card deal delay
const CARD_STAGGER_MAX: f32 = 0.2; // cap so a big board still snaps in quickly

/// Hover lift for a card.
const CARD_HOVER_SCALE: f32 = 1.06;
const CARD_HOVER_LIFT: f32 = 10.0;
const CARD_SHADOW_ALPHA: f32 = 0.5;
const CARD_SHADOW_LIFT: f32 = 16.0; // shadow y-offset when fully hovered
const CARD_SHADOW_BLUR: f32 = 22.0; // shadow blur when fully hovered

/// Button feedback.
const BTN_HOVER_LIFT: f32 = 2.0;
const BTN_PRESS_SINK: f32 = 2.0;
const BUTTON_HOVER: Color = Color::srgb(0.24, 0.50, 0.72);
const BUTTON_PRESS: Color = Color::srgb(0.12, 0.30, 0.46);

/// Per-card animation state. `age` drives the settle-in and starts *negative*
/// to encode this card's stagger delay (it sits small and low until `age`
/// reaches 0, then pops into place). `hover` is the eased 0..1 hover blend.
#[derive(Component)]
struct CardAnim {
    age: f32,
    hover: f32,
}

/// Per-button animation state: eased hover and press blends.
#[derive(Component, Default)]
struct ButtonAnim {
    hover: f32,
    press: f32,
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Frame-rate-independent ease toward `target` (exponential smoothing).
fn approach(current: f32, target: f32, dt: f32, rate: f32) -> f32 {
    let k = 1.0 - (-rate * dt).exp();
    current + (target - current) * k
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Overshoot-and-settle, for a card that "pops" as it lands.
fn ease_out_back(t: f32) -> f32 {
    const C1: f32 = 1.70158;
    const C3: f32 = C1 + 1.0;
    let x = t - 1.0;
    1.0 + C3 * x * x * x + C1 * x * x
}

fn mix_color(a: Color, b: Color, t: f32) -> Color {
    let a = a.to_srgba();
    let b = b.to_srgba();
    Color::srgba(
        lerp(a.red, b.red, t),
        lerp(a.green, b.green, t),
        lerp(a.blue, b.blue, t),
        lerp(a.alpha, b.alpha, t),
    )
}

/// Give every card a tactile settle-in and a hover lift: it scales up, rises,
/// and its drop-shadow deepens while the pointer is over it. Reads only
/// `Interaction` and writes the post-layout `UiTransform` / `BoxShadow`, so
/// neighbouring cards never reflow.
fn animate_cards(
    time: Res<Time>,
    mut cards: Query<(
        &Interaction,
        &mut UiTransform,
        &mut BoxShadow,
        &mut CardAnim,
    )>,
) {
    // Clamp dt so a hitch (or a long first frame) doesn't snap everything.
    let dt = time.delta_secs().min(1.0 / 20.0);
    for (interaction, mut transform, mut shadow, mut anim) in &mut cards {
        anim.age += dt;
        let intro = (anim.age / CARD_INTRO).clamp(0.0, 1.0);
        let settle = ease_out_back(intro);

        let want = if *interaction == Interaction::None {
            0.0
        } else {
            1.0
        };
        anim.hover = approach(anim.hover, want, dt, HOVER_RATE);
        let h = smoothstep(anim.hover);

        transform.scale =
            Vec2::splat(lerp(CARD_INTRO_SCALE, 1.0, settle) * lerp(1.0, CARD_HOVER_SCALE, h));
        // UI y grows downward, so a rise is negative.
        let drop = (1.0 - intro) * CARD_INTRO_DROP;
        transform.translation = Val2::px(0.0, drop - h * CARD_HOVER_LIFT);

        if let Some(style) = shadow.0.first_mut() {
            let present = intro * (0.45 + 0.55 * h);
            style.color = Color::srgba(0.0, 0.0, 0.0, CARD_SHADOW_ALPHA * present);
            style.x_offset = Val::Px(0.0);
            style.y_offset = Val::Px(lerp(3.0, CARD_SHADOW_LIFT, h));
            style.spread_radius = Val::Px(0.0);
            style.blur_radius = Val::Px(lerp(8.0, CARD_SHADOW_BLUR, h));
        }
    }
}

/// Hover/press feedback for action buttons: a small lift on hover, a sink on
/// press, and a colour brighten/darken between the two.
fn animate_buttons(
    time: Res<Time>,
    mut buttons: Query<(
        &Interaction,
        &mut UiTransform,
        &mut BackgroundColor,
        &mut ButtonAnim,
    )>,
) {
    let dt = time.delta_secs().min(1.0 / 20.0);
    for (interaction, mut transform, mut bg, mut anim) in &mut buttons {
        let (want_hover, want_press) = match *interaction {
            Interaction::Pressed => (1.0, 1.0),
            Interaction::Hovered => (1.0, 0.0),
            Interaction::None => (0.0, 0.0),
        };
        anim.hover = approach(anim.hover, want_hover, dt, HOVER_RATE);
        anim.press = approach(anim.press, want_press, dt, HOVER_RATE * 1.5);
        let h = smoothstep(anim.hover);
        let p = smoothstep(anim.press);

        transform.translation = Val2::px(0.0, p * BTN_PRESS_SINK - h * BTN_HOVER_LIFT);
        transform.scale = Vec2::splat(1.0 + h * 0.02 - p * 0.03);
        bg.0 = mix_color(mix_color(BUTTON, BUTTON_HOVER, h), BUTTON_PRESS, p);
    }
}

// ---- sound (synthesised in code, no asset files) -----------------------
//
// Each effect is a short enveloped tone generated on the fly rather than a
// shipped audio file. This keeps the crate asset-free and makes sound behave
// the same natively and on the wasm/web build — the browser only needs a user
// gesture before any audio plays, and every effect here is triggered by a
// click or hover, so the first interaction unlocks the audio context.

/// Sample rate for synthesised effects.
const SFX_RATE: u32 = 44_100;

/// A short synthesised tone — fast attack, exponential decay — that plays as a
/// soft UI "blip". Implementing [`Source`] lets Bevy's audio backend stream it
/// straight to the device with no decoding step.
struct Blip {
    freq: f32,
    amp: f32,
    decay: f32,
    attack: u32,
    len: u32,
    pos: u32,
}

impl Iterator for Blip {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.pos >= self.len {
            return None;
        }
        let t = self.pos as f32 / SFX_RATE as f32;
        // Linear attack into an exponential decay — a click, not a pop.
        let env = if self.pos < self.attack {
            self.pos as f32 / self.attack as f32
        } else {
            let since = (self.pos - self.attack) as f32 / SFX_RATE as f32;
            (-self.decay * since).exp()
        };
        self.pos += 1;
        Some((std::f32::consts::TAU * self.freq * t).sin() * self.amp * env)
    }
}

impl Source for Blip {
    fn current_frame_len(&self) -> Option<usize> {
        Some((self.len - self.pos) as usize)
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        SFX_RATE
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f32(self.len as f32 / SFX_RATE as f32))
    }
}

/// A synthesised sound-effect asset: the parameters of one [`Blip`].
#[derive(Asset, TypePath, Clone, Copy)]
struct Sfx {
    freq: f32,
    amp: f32,
    decay: f32,
    ms: u32,
    attack_ms: u32,
}

impl Decodable for Sfx {
    type DecoderItem = f32;
    type Decoder = Blip;

    fn decoder(&self) -> Blip {
        Blip {
            freq: self.freq,
            amp: self.amp,
            decay: self.decay,
            attack: (SFX_RATE * self.attack_ms / 1000).max(1),
            len: SFX_RATE * self.ms / 1000,
            pos: 0,
        }
    }
}

/// Handles to the synthesised effects, built once at startup.
#[derive(Resource)]
struct SfxHandles {
    click: Handle<Sfx>,
    hover: Handle<Sfx>,
}

/// Whether sound is muted. Toggled with `M`; advertised in the help overlay.
#[derive(Resource)]
struct Muted(bool);

/// `M` mutes / unmutes all sound effects.
fn toggle_mute(keys: Res<ButtonInput<KeyCode>>, mut muted: ResMut<Muted>) {
    if keys.just_pressed(KeyCode::KeyM) {
        muted.0 = !muted.0;
    }
}

fn setup_sfx(mut commands: Commands, mut assets: ResMut<Assets<Sfx>>) {
    let click = assets.add(Sfx {
        freq: 523.25,
        amp: 0.16,
        decay: 38.0,
        ms: 90,
        attack_ms: 2,
    });
    let hover = assets.add(Sfx {
        freq: 880.0,
        amp: 0.05,
        decay: 70.0,
        ms: 45,
        attack_ms: 1,
    });
    commands.insert_resource(SfxHandles { click, hover });
}

/// Play a click when an action button is pressed.
fn play_button_sfx(
    mut commands: Commands,
    sfx: Option<Res<SfxHandles>>,
    muted: Res<Muted>,
    buttons: Query<&Interaction, (Changed<Interaction>, With<ActionButton>)>,
) {
    if muted.0 {
        return;
    }
    let Some(sfx) = sfx else { return };
    for interaction in &buttons {
        if *interaction == Interaction::Pressed {
            commands.spawn((AudioPlayer(sfx.click.clone()), PlaybackSettings::DESPAWN));
        }
    }
}

/// Play a soft tick the moment the pointer moves onto a card.
fn play_card_hover_sfx(
    mut commands: Commands,
    sfx: Option<Res<SfxHandles>>,
    muted: Res<Muted>,
    cards: Query<&Interaction, (Changed<Interaction>, With<CardAnim>)>,
) {
    if muted.0 {
        return;
    }
    let Some(sfx) = sfx else { return };
    for interaction in &cards {
        if *interaction == Interaction::Hovered {
            commands.spawn((AudioPlayer(sfx.hover.clone()), PlaybackSettings::DESPAWN));
        }
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
                        border_radius: BorderRadius::all(Val::Px(PANEL_RADIUS)),
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

                // Prose content (a rules page) takes over the play area as a reading pane;
                // otherwise the card zones fill the remaining space (and scroll when taller
                // than the area, e.g. duels).
                if !view.prose.is_empty() {
                    spawn_prose_pane(main, &view.prose);
                } else {
                    main.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        flex_grow: 1.0,
                        overflow: Overflow::scroll_y(),
                        ..default()
                    })
                    .with_children(|zones| {
                        // A shared counter so cards deal in left-to-right,
                        // top-to-bottom across the whole board, not per-zone.
                        let order = Cell::new(0u32);
                        for zone in &view.zones {
                            spawn_zone(zones, zone, &order);
                        }
                    });
                }
            });
        });
}

/// Render prose as a centred, scrollable reading pane: headings, bold terms, and wrapping
/// body paragraphs. This is the readable home for rules / briefings (not fixed-size cards).
fn spawn_prose_pane(parent: &mut ChildSpawnerCommands, prose: &[ProseLine]) {
    parent
        .spawn(Node {
            flex_grow: 1.0,
            width: Val::Percent(100.0),
            overflow: Overflow::scroll_y(),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|outer| {
            outer
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(100.0),
                    max_width: Val::Px(760.0),
                    row_gap: Val::Px(4.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                })
                .with_children(|col| {
                    for line in prose {
                        match line {
                            ProseLine::Heading(t) => {
                                col.spawn((
                                    Node {
                                        margin: UiRect::new(
                                            Val::ZERO,
                                            Val::ZERO,
                                            Val::Px(14.0),
                                            Val::Px(4.0),
                                        ),
                                        ..default()
                                    },
                                    Text::new(t.clone()),
                                    TextFont {
                                        font_size: 26.0,
                                        ..default()
                                    },
                                    TextColor(TITLE_INK),
                                ));
                            }
                            ProseLine::Term(t) => {
                                col.spawn((
                                    Node {
                                        margin: UiRect::top(Val::Px(10.0)),
                                        ..default()
                                    },
                                    Text::new(t.clone()),
                                    TextFont {
                                        font_size: 18.0,
                                        ..default()
                                    },
                                    TextColor(BUTTON),
                                ));
                            }
                            ProseLine::Body(t) => {
                                col.spawn((
                                    Text::new(t.clone()),
                                    TextFont {
                                        font_size: 15.0,
                                        ..default()
                                    },
                                    TextColor(INK),
                                ));
                            }
                            ProseLine::Gap => {
                                col.spawn(Node {
                                    height: Val::Px(6.0),
                                    ..default()
                                });
                            }
                        }
                    }
                });
        });
}

fn spawn_zone(parent: &mut ChildSpawnerCommands, zone: &ZoneView, order: &Cell<u32>) {
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
                    spawn_card_group(row, &cards[i], j - i, order);
                    i = j;
                }
            });
        });
}

const STACK_PEEK: f32 = 24.0;

/// Render `count` identical cards: a single card if one, else an overlapped
/// stack — the top card fully readable, the rest peeking — with an `xN` badge.
fn spawn_card_group(
    parent: &mut ChildSpawnerCommands,
    card: &CardView,
    count: usize,
    order: &Cell<u32>,
) {
    if count <= 1 {
        spawn_card(parent, card, order);
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
                    .with_children(|slot| spawn_card(slot, card, order));
            }
            stack
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(4.0),
                        top: Val::Px(4.0),
                        padding: UiRect::axes(Val::Px(7.0), Val::Px(3.0)),
                        border_radius: BorderRadius::all(Val::Px(BADGE_RADIUS)),
                        ..default()
                    },
                    BackgroundColor(BADGE),
                    // Float the count above the cards so the hover-lift of the
                    // top card doesn't cover it.
                    GlobalZIndex(1),
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

fn spawn_card(parent: &mut ChildSpawnerCommands, card: &CardView, order: &Cell<u32>) {
    match &card.face {
        CardFace::Down => spawn_card_back(parent, card.action, order),
        CardFace::Up {
            title,
            type_line,
            body,
            corner,
            accent,
        } => spawn_card_face(
            parent,
            title,
            type_line.as_deref(),
            body,
            corner.as_deref(),
            *accent,
            card.action,
            order,
        ),
    }
}

/// The animation bundle every card root carries: a (driven) drop-shadow, an
/// `Interaction` so the focus system reports hover, and the per-card
/// [`CardAnim`] seeded with this card's stagger delay. Rounded corners live on
/// the card's `Node` (`BorderRadius` is a `Node` field, not a component). Each
/// call advances the shared deal counter.
fn card_anim_bundle(order: &Cell<u32>) -> (BoxShadow, Interaction, CardAnim) {
    let index = order.get();
    order.set(index + 1);
    let delay = (index as f32 * CARD_STAGGER).min(CARD_STAGGER_MAX);
    (
        // Placeholder; `animate_cards` overwrites every field each frame.
        BoxShadow::new(
            Color::NONE,
            Val::Px(0.0),
            Val::Px(3.0),
            Val::Px(0.0),
            Val::Px(8.0),
        ),
        Interaction::None,
        CardAnim {
            age: -delay,
            hover: 0.0,
        },
    )
}

fn spawn_card_back(parent: &mut ChildSpawnerCommands, action: Option<usize>, order: &Cell<u32>) {
    let mut card_cmd = parent.spawn((
        Node {
            width: Val::Px(CARD_W),
            height: Val::Px(CARD_H),
            padding: UiRect::all(Val::Px(10.0)),
            border: UiRect::all(Val::Px(CARD_BORDER)),
            border_radius: BorderRadius::all(Val::Px(CARD_RADIUS)),
            ..default()
        },
        BackgroundColor(CARD_BACK),
        BorderColor::all(CARD_EDGE),
        card_anim_bundle(order),
    ));
    if let Some(idx) = action {
        card_cmd.insert((Button, ActionButton(idx)));
    }
    card_cmd.with_children(|card| {
        card.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                border_radius: BorderRadius::all(Val::Px(CARD_RADIUS - CARD_BORDER - 4.0)),
                ..default()
            },
            BackgroundColor(CARD_BACK_INNER),
        ));
    });
}

#[allow(clippy::too_many_arguments)]
fn spawn_card_face(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    type_line: Option<&str>,
    body: &[String],
    corner: Option<&str>,
    accent: Accent,
    action: Option<usize>,
    order: &Cell<u32>,
) {
    let mut card_cmd = parent.spawn((
        Node {
            width: Val::Px(CARD_W),
            height: Val::Px(CARD_H),
            flex_direction: FlexDirection::Column,
            // No `overflow: clip` here: the clip rect is built from the
            // node's *unscaled* layout size, so on the hover-scale it would
            // crop the children (most visibly the title bar at the top
            // edge) to the original rectangle while the card frame grows.
            // Corner rounding comes from `border_radius`, not the clip, so
            // dropping it costs nothing for these fixed-size cards.
            border: UiRect::all(Val::Px(CARD_BORDER)),
            border_radius: BorderRadius::all(Val::Px(CARD_RADIUS)),
            ..default()
        },
        BackgroundColor(CARD_FACE),
        BorderColor::all(CARD_EDGE),
        card_anim_bundle(order),
    ));
    if let Some(idx) = action {
        card_cmd.insert((Button, ActionButton(idx)));
    }
    card_cmd.with_children(|card| {
        // Title bar (accent-coloured). Its top corners are rounded to sit
        // inside the card's rounded border (UI clipping is rectangular, so
        // this opaque bar would otherwise square off the card's top).
        card.spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
                border_radius: BorderRadius::px(
                    CARD_RADIUS - CARD_BORDER,
                    CARD_RADIUS - CARD_BORDER,
                    0.0,
                    0.0,
                ),
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
                        border_radius: BorderRadius::all(Val::Px(BADGE_RADIUS)),
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

/// The controls advertised in the help overlay, as `(keys, description)`. Keep
/// in sync with the keys handled in `adjust_zoom`, `toggle_help`,
/// `cancel_on_key`, and `toggle_mute`.
const CONTROLS: &[(&str, &str)] = &[
    ("= / +", "Zoom in"),
    ("\u{2212}", "Zoom out"),
    ("0", "Reset zoom"),
    ("Mouse wheel", "Scroll the board / action list"),
    ("M", "Mute / unmute sound"),
    ("Esc / Backspace", "Cancel \u{2013} go back a step"),
    ("? / F1", "Toggle this help"),
    ("R", "Toggle the rules reference"),
];

/// Spawn the discoverability hint and the (initially hidden) help overlay. Both
/// live outside [`TableRoot`], so a redraw never tears them down, and both carry
/// a positive [`GlobalZIndex`] so they paint above the freshly-spawned table.
fn setup_help(mut commands: Commands) {
    // Always-visible hint, bottom-right — the single affordance that tells a
    // first-time player help exists at all.
    commands
        .spawn((
            HelpHint,
            GlobalZIndex(10),
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                bottom: Val::Px(12.0),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                border_radius: BorderRadius::all(Val::Px(BADGE_RADIUS)),
                ..default()
            },
            BackgroundColor(HINT_BG),
        ))
        .with_children(|hint| {
            hint.spawn((
                Text::new("? help · R rules"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(INK),
            ));
        });

    // The overlay: a full-screen scrim centring a controls card. Hidden until
    // toggled (see `toggle_help`).
    commands
        .spawn((
            HelpOverlay,
            GlobalZIndex(20),
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(SCRIM),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        min_width: Val::Px(380.0),
                        padding: UiRect::all(Val::Px(24.0)),
                        row_gap: Val::Px(14.0),
                        border_radius: BorderRadius::all(Val::Px(PANEL_RADIUS)),
                        ..default()
                    },
                    BackgroundColor(PANEL),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("Controls"),
                        TextFont {
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(TITLE_INK),
                    ));
                    for (keys, desc) in CONTROLS {
                        spawn_control_row(panel, keys, desc);
                    }
                    panel.spawn((
                        Text::new("Press ?, F1, or Esc to close"),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(MUTED_INK),
                    ));
                });
        });
}

/// One row of the help overlay: a key-cap badge beside its description.
fn spawn_control_row(parent: &mut ChildSpawnerCommands, keys: &str, desc: &str) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(16.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Node {
                    width: Val::Px(150.0),
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                    border_radius: BorderRadius::all(Val::Px(BADGE_RADIUS)),
                    ..default()
                },
                BackgroundColor(BADGE),
            ))
            .with_children(|cap| {
                cap.spawn((
                    Text::new(keys.to_string()),
                    TextFont {
                        font_size: 15.0,
                        ..default()
                    },
                    TextColor(TITLE_INK),
                ));
            });
            row.spawn((
                Text::new(desc.to_string()),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                TextColor(INK),
            ));
        });
}

fn spawn_action_button(parent: &mut ChildSpawnerCommands, index: usize, label: &str) {
    parent
        .spawn((
            Button,
            ActionButton(index),
            ButtonAnim::default(),
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(10.0)),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(Val::Px(BUTTON_RADIUS)),
                ..default()
            },
            BackgroundColor(BUTTON),
            BoxShadow::new(
                Color::srgba(0.0, 0.0, 0.0, 0.25),
                Val::Px(0.0),
                Val::Px(2.0),
                Val::Px(0.0),
                Val::Px(5.0),
            ),
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
