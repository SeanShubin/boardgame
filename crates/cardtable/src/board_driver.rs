//! The **Board-game driver** — drives the game-agnostic renderer from a [`BoardGame`] over the
//! *persistent* board (plan §17/§18), the cards-as-truth successor to the `contract::Game` [`GamePlugin`]
//! (which rebuilt a `Tableau` from a snapshot each frame). Here the board is the single source of truth:
//! the observers **record** a player's gesture into a request resource, and this driver interprets it
//! through the game and mutates the board in place. The renderer never mentions the game type.
//!
//! Record-in-renderer / apply-in-driver (the same pattern as `CombatRequest`): the core observers write
//! [`DropRequest`] / [`AffordanceClick`]; [`apply_drop`] / [`apply_affordance`] drain them; and
//! [`sync_affordances`] fills [`AffordanceLabels`] so the non-generic `redraw` can draw the game's
//! contextual actions as control cards without knowing the game.

use bevy::prelude::*;
use cardtable_model::{BoardGame, CardId, DropTarget};

use crate::{CardTablePlugin, CardTableSet, NeedsRebuild, Table};

// ---- core request/affordance state (non-generic — the observers record into these) ---------------

/// A drop the renderer recorded for the driver to interpret: the dragged card and what it landed on.
/// Drained by [`apply_drop`]; `None` when idle. Core-owned so the observers record without a game type.
#[derive(Resource, Default)]
pub struct DropRequest(pub Option<(CardId, DropTarget)>);

/// A recorded click on the affordance control card at this index (into [`AffordanceLabels`]).
#[derive(Resource, Default)]
pub struct AffordanceClick(pub Option<usize>);

/// The labels of the game actions offered in the current zone — `redraw` draws one control card each,
/// tagged [`AffordanceControl`] with its index. Filled by [`sync_affordances`]; empty with no game/actions.
#[derive(Resource, Default)]
pub struct AffordanceLabels(pub Vec<String>);

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
) where
    G: BoardGame + Send + Sync + 'static,
{
    let Some((dragged, onto)) = request.0.take() else {
        return;
    };
    if let Some(intention) = game.0.drop_intention(&table.0, dragged, onto) {
        game.0.apply(&mut table.0, &[intention]);
    } else if let DropTarget::Pile(dest) = onto {
        let at = table.0.pile(dest).map_or(0, |p| p.cards().len());
        let _ = table.0.move_card(dragged, dest, at);
    }
    rebuild.0 = true;
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

/// Recompute the current zone's affordances from the game each frame: labels for `redraw`, intentions for
/// [`apply_affordance`]. Cheap (a small lookup); runs before the Draw set so `redraw` sees fresh labels.
fn sync_affordances<G>(
    table: Res<Table>,
    game: Res<GameRes<G>>,
    mut labels: ResMut<AffordanceLabels>,
    mut affordances: ResMut<Affordances<G>>,
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
                    apply_affordance::<G>,
                    sync_affordances::<G>,
                )
                    .chain()
                    .in_set(CardTableSet::Apply),
            );
    }
}
