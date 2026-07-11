//! The **Board-game driver** — drives the game-agnostic renderer from a [`BoardGame`] over the
//! *persistent* board (plan §17/§18), the cards-as-truth successor to the `contract::Game` [`GamePlugin`]
//! (which rebuilt a `Board` from a snapshot each frame). Here the board is the single source of truth:
//! the observers **record** a player's gesture into a request resource, and this driver interprets it
//! through the game and mutates the board in place. The renderer never mentions the game type.
//!
//! Record-in-renderer / apply-in-driver (the same pattern as `CombatRequest`): the core observers write
//! [`DropRequest`] / [`AffordanceClick`]; [`apply_drop`] / [`apply_affordance`] drain them; and
//! [`sync_affordances`] fills [`AffordanceLabels`] so the non-generic `redraw` can draw the game's
//! contextual actions as control cards without knowing the game.

use bevy::prelude::*;
use cardtable_model::{BoardGame, CardId, DropTarget, Scene};

use crate::{CardTablePlugin, CardTableSet, NeedsRebuild, Table};

// ---- core request/affordance state (non-generic — the observers record into these) ---------------

/// A drop the renderer recorded for the driver to interpret: the dragged card and what it landed on.
/// Drained by [`apply_drop`]; `None` when idle. Core-owned so the observers record without a game type.
#[derive(Resource, Default)]
pub struct DropRequest(pub Option<(CardId, DropTarget)>);

/// A recorded click on the affordance control card at this index (into [`AffordanceLabels`]).
#[derive(Resource, Default)]
pub struct AffordanceClick(pub Option<usize>);

/// A recorded **tap** on a board card the driver should interpret through the game's
/// [`tap_intention`](cardtable_model::BoardGame::tap_intention) — the third input verb (beside drop and
/// affordance) for in-place per-card actions (cycling a combatant's bid, picking a reaction). Drained by
/// [`apply_tap`]; `None` when idle. Core-owned so the observers record without a game type.
#[derive(Resource, Default)]
pub struct TapRequest(pub Option<CardId>);

/// The labels of the game actions offered in the current zone — `redraw` draws one control card each,
/// tagged [`AffordanceControl`] with its index. Filled by [`sync_affordances`]; empty with no game/actions.
#[derive(Resource, Default)]
pub struct AffordanceLabels(pub Vec<String>);

/// The **modal scene** the game wants drawn in place of the felt (a combat arena), or `None` for the ordinary
/// table. Filled by [`sync_affordances`] from [`BoardGame::scene`](cardtable_model::BoardGame::scene); the
/// renderer draws it without knowing what it means. Core-owned so `redraw` / the arrow overlay read it
/// without a game type.
#[derive(Resource, Default)]
pub struct SceneState(pub Option<Scene>);

/// A human-readable trace of each resolved drop — the dragged card, what it landed on (the *resolved*
/// [`DropTarget`], not the raw pick-hit), and the outcome. Pushed by [`apply_drop`] (the one place that sees
/// every drop's resolution), drained by the UI debug log so a session is reconstructable from that log alone.
#[derive(Resource, Default)]
pub struct DropTrace(pub Vec<String>);

/// Marks a control card as the affordance at this index (into [`AffordanceLabels`]); clicking it records
/// [`AffordanceClick`].
#[derive(Component, Clone, Copy)]
pub struct AffordanceControl(pub usize);

// ---- the game, and the per-game affordance intentions (generic) ----------------------------------

/// Wraps the game (a bevy-free [`BoardGame`]) so the driver systems can reach it as a resource.
#[derive(Resource)]
pub struct GameRes<G>(pub G);

/// The intentions behind the current zone's affordances, index-aligned with [`AffordanceLabels`].
#[derive(Resource)]
struct Affordances<G: BoardGame>(Vec<G::Intention>);

impl<G: BoardGame> Default for Affordances<G> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

// ---- systems -------------------------------------------------------------------------------------

/// Interpret a recorded drop through the game: a legal move is applied; otherwise, a drop into a pile is
/// the default card move (a drop onto a card that isn't a move does nothing — it settles back).
fn apply_drop<G>(
    mut request: ResMut<DropRequest>,
    mut table: ResMut<Table>,
    game: Res<GameRes<G>>,
    mut rebuild: ResMut<NeedsRebuild>,
    mut trace: ResMut<DropTrace>,
) where
    G: BoardGame + Send + Sync + 'static,
{
    let Some((dragged, onto)) = request.0.take() else {
        return;
    };
    // Describe the drop before applying it (the cards move on apply). The target is the *resolved*
    // DropTarget (e.g. the map place a march landed on), which the raw pointer event can't report.
    let dragged_name = table
        .0
        .card(dragged)
        .map(|c| c.front_title().to_string())
        .unwrap_or_default();
    let onto_desc = match onto {
        DropTarget::Card(t) => format!(
            "card {}",
            table.0.card(t).map(|c| c.front_title()).unwrap_or_default()
        ),
        DropTarget::Pile(p) => format!(
            "pile [{}]",
            table
                .0
                .pile(p)
                .map(|p| p.label.as_str())
                .unwrap_or_default()
        ),
    };
    let outcome = if let Some(intention) = game.0.drop_intention(&table.0, dragged, onto) {
        game.0.apply(&mut table.0, &[intention]);
        "applied a game move"
    } else if let DropTarget::Pile(dest) = onto {
        let at = table.0.pile(dest).map_or(0, |p| p.cards().len());
        let _ = table.0.move_card(dragged, dest, at);
        "default move into pile"
    } else {
        "no move (settled back)"
    };
    trace.0.push(format!(
        "drop: {dragged_name} onto {onto_desc} -> {outcome}"
    ));
    rebuild.0 = true;
}

/// Interpret a recorded tap through the game: if [`tap_intention`](cardtable_model::BoardGame::tap_intention)
/// recognizes it as a move, apply it. A tap the game ignores is a no-op here (the renderer already handled
/// the click as focus/zoom).
fn apply_tap<G>(
    mut request: ResMut<TapRequest>,
    mut table: ResMut<Table>,
    game: Res<GameRes<G>>,
    mut rebuild: ResMut<NeedsRebuild>,
) where
    G: BoardGame + Send + Sync + 'static,
{
    let Some(card) = request.0.take() else {
        return;
    };
    if let Some(intention) = game.0.tap_intention(&table.0, card) {
        game.0.apply(&mut table.0, &[intention]);
        rebuild.0 = true;
    }
}

/// Apply the game action behind a clicked affordance control card.
fn apply_affordance<G>(
    mut click: ResMut<AffordanceClick>,
    affordances: Res<Affordances<G>>,
    mut table: ResMut<Table>,
    game: Res<GameRes<G>>,
    mut rebuild: ResMut<NeedsRebuild>,
) where
    G: BoardGame + Send + Sync + 'static,
    G::Intention: Clone + Send + Sync + 'static,
{
    let Some(index) = click.0.take() else {
        return;
    };
    if let Some(intention) = affordances.0.get(index).cloned() {
        game.0.apply(&mut table.0, &[intention]);
        rebuild.0 = true;
    }
}

/// Recompute the current zone's affordances **and** modal scene from the game each frame: labels + intentions
/// for the controls, and the [`SceneState`] the renderer draws. Cheap (small lookups); runs before the Draw
/// set so `redraw` sees fresh state.
fn sync_affordances<G>(
    table: Res<Table>,
    game: Res<GameRes<G>>,
    mut labels: ResMut<AffordanceLabels>,
    mut affordances: ResMut<Affordances<G>>,
    mut scene: ResMut<SceneState>,
) where
    G: BoardGame + Send + Sync + 'static,
    G::Intention: Send + Sync + 'static,
{
    let focus = table.0.focus_id();
    let offered = game.0.affordances(&table.0, focus);
    labels.0 = offered.iter().map(|(label, _)| label.clone()).collect();
    affordances.0 = offered
        .into_iter()
        .map(|(_, intention)| intention)
        .collect();
    scene.0 = game.0.scene(&table.0, focus);
}

// ---- the plugin ----------------------------------------------------------------------------------

/// Drives the [`CardTablePlugin`] core from a [`BoardGame`] `G`: seeds the board from the game's opening
/// position and routes recorded gestures through the game onto the persistent board. Add this instead of a
/// hand-built `Table` to run a game.
pub struct BoardGamePlugin<G>(pub G);

impl<G> Plugin for BoardGamePlugin<G>
where
    G: BoardGame + Clone + Send + Sync + 'static,
    G::Intention: Clone + Send + Sync + 'static,
{
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<CardTablePlugin>() {
            app.add_plugins(CardTablePlugin);
        }
        app.insert_resource(GameRes(self.0.clone()))
            .insert_resource(Table(self.0.opening()))
            .init_resource::<Affordances<G>>()
            .add_systems(
                Update,
                (
                    apply_drop::<G>,
                    apply_tap::<G>,
                    apply_affordance::<G>,
                    sync_affordances::<G>,
                )
                    .chain()
                    .in_set(CardTableSet::Apply),
            );
    }
}
