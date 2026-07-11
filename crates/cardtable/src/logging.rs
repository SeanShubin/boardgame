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

use bevy::picking::events::{Click, DragStart, Pointer};
use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform, UiStack};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use cardtable_model::{Board, CardId, Node as TableNode, PileId};

use crate::board_driver::DropTrace;
use crate::{CardRef, Movable, PileDropZone, Table};

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
                (log_physical, log_view, log_layout, drain_drop_trace),
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

/// Log the settled layout of the current view: each rendered card's name, position, size and zoom. Logged
/// once the geometry stops changing (so it reflects the settled arrangement, not mid-animation frames).
fn log_layout(
    // Every *rendered card* carries `CardRef` — movable or not (a `Virtual` readout like a Rumors card has no
    // `Movable`). Logging from here captures **every** card's exact rect, so overlaps involving a non-movable
    // card are detectable from the log alone. The `Movable` query is only for the felt's piles (decks), which
    // are not cards.
    cards: Query<(Entity, &CardRef, &ComputedNode, &UiGlobalTransform)>,
    movables: Query<(Entity, &Movable, &ComputedNode, &UiGlobalTransform)>,
    zones: Query<(Entity, &PileDropZone, &ComputedNode, &UiGlobalTransform)>,
    table: Res<Table>,
    ui_stack: Res<UiStack>,
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
    let mut boxes: Vec<(String, f32, f32, f32, f32, String, usize)> = Vec::new();
    let mut movable_piles: HashSet<PileId> = HashSet::new();
    // All cards (movable or not — includes Virtual readouts, which do not participate in the shove logic and
    // so are exactly the cards that can silently overlap).
    for (entity, cref, cn, gt) in cards.iter() {
        let Some(card) = table.0.card(cref.0) else {
            continue;
        };
        let (center, half) = crate::node_box(cn, gt);
        let (size, tl) = (half * 2.0, center - half);
        boxes.push((
            card.front_title().to_string(),
            tl.x,
            tl.y,
            size.x,
            size.y,
            format!("{:?}", card.size()),
            z_of.get(&entity).copied().unwrap_or(0),
        ));
    }
    // The felt's piles (decks): Movable, but not cards, so listed too. Also note which piles are movable so
    // the drop-zone pass below doesn't double-list a deck that is both movable and a drop target.
    for (entity, movable, cn, gt) in movables.iter() {
        let TableNode::Pile(pid) = movable.0 else {
            continue;
        };
        movable_piles.insert(pid);
        let Some(pile) = table.0.pile(pid) else {
            continue;
        };
        let (center, half) = crate::node_box(cn, gt);
        let (size, tl) = (half * 2.0, center - half);
        boxes.push((
            format!("[{}]", pile.label),
            tl.x,
            tl.y,
            size.x,
            size.y,
            "-".to_string(),
            z_of.get(&entity).copied().unwrap_or(0),
        ));
    }
    boxes.sort_by_key(|b| (b.2 as i32, b.1 as i32));

    let cards_block: String = boxes
        .iter()
        .map(|(name, x, y, w, h, zoom, z)| {
            format!("  {name} @ ({x:.0},{y:.0}) size ({w:.0}x{h:.0}) zoom {zoom} z{z}")
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Overlaps among **all** rendered elements (every card + pile) — the visible result of the push/shove
    // (`separate`) logic. With it working nothing should overlap; a non-movable card (e.g. a Virtual readout)
    // that the shove logic ignores is exactly what shows up here. When two overlap, the lower z is hidden.
    let mut overlaps = Vec::new();
    for i in 0..boxes.len() {
        for j in (i + 1)..boxes.len() {
            let (ni, ax, ay, aw, ah, _, zi) = &boxes[i];
            let (nj, bx, by, bw, bh, _, zj) = &boxes[j];
            let ox = (ax + aw).min(bx + bw) - ax.max(*bx);
            let oy = (ay + ah).min(by + bh) - ay.max(*by);
            if ox > 0.5 && oy > 0.5 {
                let (front, back) = if zi >= zj { (ni, nj) } else { (nj, ni) };
                overlaps.push(format!(
                    "  OVERLAP: {ni} & {nj} by ({ox:.0}x{oy:.0}) - {front} over {back}"
                ));
            }
        }
    }
    let overlap_block = if overlaps.is_empty() {
        "  overlaps: none (push settled all clear)".to_string()
    } else {
        format!("  overlaps: {}\n{}", overlaps.len(), overlaps.join("\n"))
    };

    // Structured drop-zones (e.g. the Locations map's place cells, the formation rows) — the targets a drop
    // can land on. Not Movable, so listed separately, with their z so a card-behind-zone is spottable.
    let mut zone_boxes: Vec<(String, f32, f32, f32, f32, usize)> = zones
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
