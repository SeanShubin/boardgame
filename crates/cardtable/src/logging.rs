//! **Debug logs** that mirror the three layers (plan §0), so a play session can be read back and
//! checked. Two files, each truncated on launch (native only — no filesystem on the web):
//!
//! - `physical-cards.log` — the **physical model**: the conserved card tree as an indented hierarchy
//!   (each card's face up/down), alternating with the **transitions** (what moved / flipped / appeared /
//!   vanished) between one state and the next. Lets a human confirm each state transition by hand.
//! - `ui-state.log` — the **UI model + IO**: which view (zone) is entered, the settled layout of each
//!   card on that view (position, size, zoom), and every pick-up / drop / click with its pointer
//!   position. Lets a reader reconstruct exactly how the table was interacted with.
//!
//! Added by the product via [`LoggingPlugin`]; a pure observer/system side-channel that never mutates
//! the board or the UI.

use bevy::ecs::system::SystemParam;
use bevy::picking::events::{Click, DragStart, Pointer};
use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform, UiStack};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use cardtable_model::{Board, CardId, Node as TableNode, PileId};

use crate::board_driver::{DropTrace, SceneState};
use crate::{CardRef, Dragging, Movable, PileDropZone, Table};

/// A truncate-on-launch text log (native only; a no-op sink on the web).
struct Log(Mutex<Option<std::fs::File>>);

impl Log {
    fn create(path: &str) -> Self {
        if cfg!(target_arch = "wasm32") {
            return Log(Mutex::new(None));
        }
        Log(Mutex::new(std::fs::File::create(path).ok()))
    }
    fn write(&self, text: &str) {
        if let Ok(mut guard) = self.0.lock()
            && let Some(file) = guard.as_mut()
        {
            use std::io::Write;
            let _ = write!(file, "{text}");
            let _ = file.flush();
        }
    }
}

#[derive(Resource)]
struct PhysicalLog(Log);
#[derive(Resource)]
struct UiLog(Log);

/// Records the two debug logs. Added by the product; native-only file output.
pub struct LoggingPlugin;

impl Plugin for LoggingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PhysicalLog(Log::create("physical-cards.log")))
            .insert_resource(UiLog(Log::create("ui-state.log")))
            .add_systems(
                Update,
                (
                    log_physical,
                    log_view,
                    log_layout,
                    log_scene,
                    drain_drop_trace,
                ),
            )
            .add_observer(log_pickup)
            .add_observer(log_click);
    }
}

// ---- physical-cards.log: the card tree + transitions ---------------------------------------------

/// One card in a physical snapshot: where it lives (a `/`-joined pile-label path), which way it faces, and
/// its detail lines (a combatant's rank/HP/tempo and staged plan ride here) — so a detail-only change (e.g.
/// staging a combat plan) still counts as a physical change and re-logs.
#[derive(Clone, PartialEq)]
struct CardState {
    path: String,
    name: String,
    face_up: bool,
    detail: Vec<String>,
}

/// A physical snapshot: the rendered hierarchy (for the state block) and a per-card map (for diffing).
struct Snapshot {
    tree: String,
    cards: HashMap<CardId, CardState>,
}

fn snapshot(table: &Board) -> Snapshot {
    let mut tree = String::new();
    let mut cards = HashMap::new();
    walk(table, table.root_id(), 0, "", &mut tree, &mut cards);
    Snapshot { tree, cards }
}

fn walk(
    table: &Board,
    pid: PileId,
    depth: usize,
    parent_path: &str,
    tree: &mut String,
    cards: &mut HashMap<CardId, CardState>,
) {
    let Some(pile) = table.pile(pid) else { return };
    let indent = "  ".repeat(depth);
    let path = if parent_path.is_empty() {
        pile.label.clone()
    } else {
        format!("{parent_path}/{}", pile.label)
    };
    tree.push_str(&format!("{indent}[{}]\n", pile.label));
    for node in pile.children() {
        match node {
            TableNode::Card(cid) => {
                let Some(card) = table.card(*cid) else {
                    continue;
                };
                let face = if card.is_face_down() { "down" } else { "up" };
                let qty = card.quantity();
                let qty = if qty > 1 {
                    format!(" x{qty}")
                } else {
                    String::new()
                };
                tree.push_str(&format!(
                    "{indent}  - {} ({face}){qty}\n",
                    card.front_title()
                ));
                let detail: Vec<String> = card
                    .detail()
                    .iter()
                    .filter(|l| !l.is_empty())
                    .cloned()
                    .collect();
                for line in &detail {
                    tree.push_str(&format!("{indent}      - {line}\n"));
                }
                cards.insert(
                    *cid,
                    CardState {
                        path: path.clone(),
                        name: card.front_title().to_string(),
                        face_up: !card.is_face_down(),
                        detail,
                    },
                );
            }
            TableNode::Pile(child) => walk(table, *child, depth + 1, &path, tree, cards),
        }
    }
}

/// The transitions from `old` to `new` — what moved between piles, flipped, appeared, or vanished. Sorted
/// so a run is stable and readable.
fn transitions(old: &HashMap<CardId, CardState>, new: &HashMap<CardId, CardState>) -> Vec<String> {
    let mut lines = Vec::new();
    for (id, ns) in new {
        match old.get(id) {
            None => lines.push(format!("+ appeared {} in {}", ns.name, ns.path)),
            Some(os) => {
                if os.path != ns.path {
                    lines.push(format!("~ moved {}: {} -> {}", ns.name, os.path, ns.path));
                }
                if os.face_up != ns.face_up {
                    let f = if ns.face_up { "up" } else { "down" };
                    lines.push(format!("~ flipped {} {f}", ns.name));
                }
            }
        }
    }
    for (id, os) in old {
        if !new.contains_key(id) {
            lines.push(format!("- vanished {} from {}", os.name, os.path));
        }
    }
    lines.sort();
    lines
}

/// Log the physical card tree whenever it changes: the transitions since the last state, then the new
/// state. The first entry is the opening state. Ignores geometry / focus (those are UI, logged elsewhere).
fn log_physical(table: Res<Table>, log: Res<PhysicalLog>, mut last: Local<Option<Snapshot>>) {
    let now = snapshot(&table.0);
    match last.as_ref() {
        None => {
            log.0
                .write(&format!("=== opening state ===\n{}\n", now.tree));
        }
        Some(prev) => {
            if prev.cards == now.cards && prev.tree == now.tree {
                return; // no physical change
            }
            let diff = transitions(&prev.cards, &now.cards);
            let mut out = String::from("--- transitions ---\n");
            if diff.is_empty() {
                out.push_str("(reordering - no card changed pile or face)\n");
            } else {
                for line in diff {
                    out.push_str(&line);
                    out.push('\n');
                }
            }
            out.push_str(&format!("--- state ---\n{}\n", now.tree));
            log.0.write(&out);
        }
    }
    *last = Some(now);
}

// ---- ui-state.log: views, layout, pointer events -------------------------------------------------

/// Log a view switch when the focused (drilled-into) zone changes.
fn log_view(table: Res<Table>, log: Res<UiLog>, mut last: Local<Option<PileId>>) {
    let focus = table.0.focus_id();
    if *last == Some(focus) {
        return;
    }
    *last = Some(focus);
    let label = table
        .0
        .pile(focus)
        .map(|p| p.label.clone())
        .unwrap_or_default();
    log.0.write(&format!("\n=== view: [{label}] ===\n"));
}

/// One rendered element's settled box: top-left (`x`,`y`) and size (`w`,`h`) in logical pixels, its zoom
/// label, its render order (`z`; higher = drawn in front), and the pile it belongs to. `pile` is `None` for a
/// deck or a drop-zone, which are their own unit rather than part of a card stack.
struct LayoutBox {
    name: String,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    zoom: String,
    z: usize,
    pile: Option<PileId>,
}

/// The rendered elements `log_layout` reads.
#[derive(SystemParam)]
struct LayoutQuery<'w, 's> {
    /// Every *rendered card* carries `CardRef` — movable or not (a `Virtual` readout like a Rumors card has no
    /// `Movable`). Logging from here captures **every** card's exact rect, so overlaps involving a non-movable
    /// card are detectable from the log alone.
    cards: Query<
        'w,
        's,
        (
            Entity,
            &'static CardRef,
            &'static ComputedNode,
            &'static UiGlobalTransform,
        ),
    >,
    /// The felt's piles (decks), which are not cards.
    movables: Query<
        'w,
        's,
        (
            Entity,
            &'static Movable,
            &'static ComputedNode,
            &'static UiGlobalTransform,
        ),
    >,
    zones: Query<
        'w,
        's,
        (
            Entity,
            &'static PileDropZone,
            &'static ComputedNode,
            &'static UiGlobalTransform,
        ),
    >,
}

/// Log the settled layout of the current view: each rendered card's name, position, size and zoom. Logged
/// once the geometry stops changing (so it reflects the settled arrangement, not mid-animation frames).
fn log_layout(
    q: LayoutQuery,
    table: Res<Table>,
    ui_stack: Res<UiStack>,
    dragging: Res<Dragging>,
    log: Res<UiLog>,
    mut last_frame: Local<String>,
    mut last_logged: Local<String>,
) {
    // The render order: an element's index in the UI stack. Higher = drawn on top (in front). Logging it
    // makes "card rendered behind a drop target" visible — the two boxes overlap and the one with the lower
    // z is the one hidden. (From the previous frame's stack, computed in PostUpdate; fine for a log.)
    let z_of: HashMap<Entity, usize> = ui_stack
        .uinodes
        .iter()
        .enumerate()
        .map(|(i, &e)| (e, i))
        .collect();

    // Every positioned felt element (every card *and* every pile) with its settled box (top-left + size) in
    // logical pixels and its z (render order). Position + size are exact, so any overlap or inter-element gap
    // is computable — the layout is fully reconstructable from the log without rendering.
    // Each box also carries the **pile it belongs to** (a card's stack; `None` for a deck), so the overlap
    // check can tell an intentional stack from a spill.
    let mut boxes: Vec<LayoutBox> = Vec::new();
    let mut movable_piles: HashSet<PileId> = HashSet::new();
    for (entity, cref, cn, gt) in q.cards.iter() {
        let Some(card) = table.0.card(cref.0) else {
            continue;
        };
        let (center, half) = crate::node_box(cn, gt);
        let (size, tl) = (half * 2.0, center - half);
        boxes.push(LayoutBox {
            name: card.front_title().to_string(),
            x: tl.x,
            y: tl.y,
            w: size.x,
            h: size.y,
            zoom: format!("{:?}", card.size()),
            z: z_of.get(&entity).copied().unwrap_or(0),
            pile: table.0.pile_of(cref.0), // the stack this card belongs to
        });
    }
    for (entity, movable, cn, gt) in q.movables.iter() {
        let TableNode::Pile(pid) = movable.0 else {
            continue;
        };
        movable_piles.insert(pid);
        let Some(pile) = table.0.pile(pid) else {
            continue;
        };
        let (center, half) = crate::node_box(cn, gt);
        let (size, tl) = (half * 2.0, center - half);
        boxes.push(LayoutBox {
            name: format!("[{}]", pile.label),
            x: tl.x,
            y: tl.y,
            w: size.x,
            h: size.y,
            zoom: "-".to_string(),
            z: z_of.get(&entity).copied().unwrap_or(0),
            pile: None, // a deck is its own unit, not part of a card stack
        });
    }
    boxes.sort_by_key(|b| (b.y as i32, b.x as i32));

    let cards_block: String = boxes
        .iter()
        .map(|b| {
            let (name, x, y, w, h, zoom, z) = (&b.name, b.x, b.y, b.w, b.h, &b.zoom, b.z);
            format!("  {name} @ ({x:.0},{y:.0}) size ({w:.0}x{h:.0}) zoom {zoom} z{z}")
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Overlaps between elements of **different** stacks - the real errors (a spill). Two cards in the *same*
    // pile are an intentional stack (a location's characters, a deck's cards): the drop target surrounds the
    // whole stack, so their overlap is expected and is NOT logged. Everything else that overlaps - a card
    // spilling onto another stack, or two decks colliding - is a genuine bug.
    let mut overlaps = Vec::new();
    for i in 0..boxes.len() {
        for j in (i + 1)..boxes.len() {
            let (a, b) = (&boxes[i], &boxes[j]);
            if a.pile.is_some() && a.pile == b.pile {
                continue; // same-pile stack: intentional overlap, not an error
            }
            let ox = (a.x + a.w).min(b.x + b.w) - a.x.max(b.x);
            let oy = (a.y + a.h).min(b.y + b.h) - a.y.max(b.y);
            if ox > 0.5 && oy > 0.5 {
                let (ni, nj) = (&a.name, &b.name);
                let (front, back) = if a.z >= b.z { (ni, nj) } else { (nj, ni) };
                overlaps.push(format!(
                    "    ERROR overlap: {ni} & {nj} by ({ox:.0}x{oy:.0}) - {front} over {back}"
                ));
            }
        }
    }
    // The never-overlap invariant: in a **settled** layout no two elements may overlap (if space is tight
    // they clip off the edge instead). `log_layout` only writes settled frames and skips while a drag is in
    // progress, so any overlap that reaches here is a genuine layout bug - logged as an ERROR so the log can
    // be audited with `grep ERROR` instead of a human spotting it. Transient overlap (mid-drag, or mid-push
    // before it settles) is valid and never logged.
    let overlap_block = if dragging.0.is_some() {
        "  overlaps: (drag in progress - transient overlap allowed)".to_string()
    } else if overlaps.is_empty() {
        "  overlaps: none".to_string()
    } else {
        format!(
            "  ERROR: {} settled overlap(s) - cards must never overlap:\n{}",
            overlaps.len(),
            overlaps.join("\n")
        )
    };

    // Structured drop-zones (e.g. the Locations map's place cells, the formation rows) — the targets a drop
    // can land on. Not Movable, so listed separately, with their z so a card-behind-zone is spottable.
    let mut zone_boxes: Vec<(String, f32, f32, f32, f32, usize)> = q
        .zones
        .iter()
        .filter_map(|(entity, zone, cn, gt)| {
            if movable_piles.contains(&zone.0) {
                return None; // a top-level deck is both movable and a drop-zone; listed once (above)
            }
            let pile = table.0.pile(zone.0)?;
            let (center, half) = crate::node_box(cn, gt);
            let (size, tl) = (half * 2.0, center - half);
            Some((
                pile.label.clone(),
                tl.x,
                tl.y,
                size.x,
                size.y,
                z_of.get(&entity).copied().unwrap_or(0),
            ))
        })
        .collect();
    zone_boxes.sort_by_key(|b| (b.2 as i32, b.1 as i32));
    let zones_block = if zone_boxes.is_empty() {
        String::new()
    } else {
        let lines: Vec<String> = zone_boxes
            .iter()
            .map(|(name, x, y, w, h, z)| {
                format!("  [{name}] (drop-zone) @ ({x:.0},{y:.0}) size ({w:.0}x{h:.0}) z{z}")
            })
            .collect();
        format!("\n  drop-zones:\n{}", lines.join("\n"))
    };

    let snapshot = format!("{cards_block}\n{overlap_block}{zones_block}");
    // Only log once the layout has settled (this frame equals the last) and differs from what was logged.
    if snapshot == *last_frame && snapshot != *last_logged && !snapshot.is_empty() {
        log.0.write(&format!("layout:\n{snapshot}\n"));
        *last_logged = snapshot.clone();
    }
    *last_frame = snapshot;
}

fn card_name(table: &Board, cref: Option<&CardRef>) -> String {
    cref.and_then(|c| table.card(c.0))
        .map(|c| c.front_title().to_string())
        .unwrap_or_else(|| "(control card)".into())
}

/// The dragged/clicked card's name, from either a `CardRef` (table cards) or a `Movable(Card)` (bespoke tiles
/// like the arena's formation tiles, which carry no `CardRef`). `None` if the entity is neither.
fn interacted_card(
    table: &Board,
    entity: Entity,
    cards: &Query<&CardRef>,
    movables: &Query<&Movable>,
) -> Option<String> {
    if let Ok(cref) = cards.get(entity) {
        return Some(card_name(table, Some(cref)));
    }
    if let Ok(Movable(TableNode::Card(cid))) = movables.get(entity) {
        return table.card(*cid).map(|c| c.front_title().to_string());
    }
    None
}

/// Log a card pick-up (drag start) with its pointer position.
fn log_pickup(
    on: On<Pointer<DragStart>>,
    cards: Query<&CardRef>,
    movables: Query<&Movable>,
    table: Res<Table>,
    log: Res<UiLog>,
) {
    if let Some(name) = interacted_card(&table.0, on.event().entity, &cards, &movables) {
        let p = on.event().pointer_location.position;
        log.0
            .write(&format!("pick up: {name} at ({:.0},{:.0})\n", p.x, p.y));
    }
}

/// Drain the driver's resolved-drop trace into the UI log — the authoritative record of what each drop did
/// (dragged card, the *resolved* target, outcome). So a march's real destination shows here without needing
/// the physical log to disambiguate the raw (occluded) pick-hit.
fn drain_drop_trace(mut trace: ResMut<DropTrace>, log: Res<UiLog>) {
    for line in trace.0.drain(..) {
        log.0.write(&format!("{line}\n"));
    }
}

/// Log **every** click with its pointer position, what it hit, and its outcome — including the clicks the
/// game *ignores*, so a "dropped" click is visible here instead of vanishing. Two ignore paths mirror
/// `on_click`: a click landing inside the **drag-guard** window (a press that moved far enough to start a
/// drag — the usual cause of a lost tap) is suppressed, and a click on an entity with **no interactive
/// target** does nothing. Combat tiles / controls / cards are named by kind so the arena taps show up (they
/// carry `TileCard` / `AffordanceControl`, not `CardRef`, so the old logger missed them).
// A Bevy system: every parameter is a scheduler-injected Query/Res, so the arg count is inherent, not a smell.
#[allow(clippy::too_many_arguments)]
fn log_click(
    on: On<Pointer<Click>>,
    guard: Res<crate::DragGuard>,
    cards: Query<&CardRef>,
    movables: Query<&Movable>,
    units: Query<&crate::TileCard>,
    affordances: Query<&crate::AffordanceControl>,
    backs: Query<(), With<crate::BackCard>>,
    zones: Query<&PileDropZone>,
    table: Res<Table>,
    log: Res<UiLog>,
) {
    let entity = on.event().entity;
    let p = on.event().pointer_location.position;
    // A click **bubbles** up the node hierarchy, firing this observer once per ancestor. Log only the entity
    // that actually carries an interactive role (combatant / affordance / back / card / drop-zone), so one
    // physical click leaves one line instead of one per bubbled node. Order matters: a formation tile carries
    // both `TileCard` and `Movable`, and a card sits inside a drop-zone.
    let what = if let Ok(unit) = units.get(entity) {
        let name = table
            .0
            .card(unit.0)
            .map(|c| c.front_title().to_string())
            .unwrap_or_else(|| "(combatant)".into());
        format!("{name} [combatant]")
    } else if let Ok(ctrl) = affordances.get(entity) {
        format!("affordance #{} [control]", ctrl.0)
    } else if backs.get(entity).is_ok() {
        "Back [control]".into()
    } else if let Some(name) = interacted_card(&table.0, entity, &cards, &movables) {
        format!("{name} [card]")
    } else if let Ok(zone) = zones.get(entity) {
        let label = table
            .0
            .pile(zone.0)
            .map(|pile| pile.label.clone())
            .unwrap_or_default();
        format!("{label} [zone]")
    } else {
        return; // an inert bubbled node (a container / the felt) - not the click's real target
    };
    // The drag-guard holds the drag's start position while a drag is live; `on_click` drops the ending click
    // only if the pointer travelled past the tolerance. Mirror that here so a suppressed click is marked,
    // rather than looking like a click that did nothing.
    let outcome = match guard.0 {
        Some(start) if p.distance(start) > crate::CLICK_DRAG_TOLERANCE => "  IGNORED (drag-guard)",
        _ => "",
    };
    log.0.write(&format!(
        "click: {what} at ({:.0},{:.0}){outcome}\n",
        p.x, p.y
    ));
}

/// Log the **modal scene** — the combat screen — whenever its text changes.
///
/// Without this the debug log records the *cards* but not the *screen*, so a complaint like "the log says the
/// phase was skipped, but the phase card still says Intercept" could not be checked against what the player
/// actually saw; it had to be inferred from the arena's cards. Everything the screen says is written here:
/// which phase and step each track is on, the prompt, the decision being asked for (with each option's
/// consequence, or the reason it is barred), and the combat log lines themselves.
fn log_scene(scene: Res<SceneState>, log: Res<UiLog>, mut last: Local<String>) {
    let Some(s) = &scene.0 else {
        if !last.is_empty() {
            log.0
                .write("scene: (none - the fight is over; back on the felt)\n");
            last.clear();
        }
        return;
    };
    let mut out = format!("\nscene: {}\n", s.heading);
    for track in &s.tracks {
        let current = track
            .items
            .iter()
            .find(|i| i.current)
            .map(|i| i.label.as_str())
            .unwrap_or("(none)");
        out.push_str(&format!("  {}: {current}\n", track.title));
    }
    if !s.prompt.is_empty() {
        out.push_str(&format!("  prompt: {}\n", s.prompt));
    }
    for c in &s.choices {
        // A barred choice records *why* - the same reason the player is shown, so the screen and the log
        // cannot disagree about what was on offer.
        let state = if !c.enabled() {
            format!("BARRED: {}", c.why_not)
        } else if c.chosen {
            format!("CHOSEN: {}", c.consequence)
        } else {
            c.consequence.clone()
        };
        out.push_str(&format!("  choice [{}] {state}\n", c.label));
    }
    for line in &s.log {
        out.push_str(&format!("  | {line}\n"));
    }
    if out == *last {
        return; // unchanged - log the screen once per distinct state, not once per frame
    }
    *last = out.clone();
    log.0.write(&out);
}
