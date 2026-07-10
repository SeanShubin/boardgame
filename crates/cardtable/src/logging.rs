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

use cardtable_model::{CardId, Node as TableNode, PileId, Tableau};

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

fn snapshot(table: &Tableau) -> Snapshot {
    let mut tree = String::new();
    let mut cards = HashMap::new();
    walk(table, table.root_id(), 0, "", &mut tree, &mut cards);
    Snapshot { tree, cards }
}

fn walk(
    table: &Tableau,
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
    nodes: Query<(Entity, &Movable, &ComputedNode, &UiGlobalTransform)>,
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

    // Every positioned felt element (card *and* pile) with its settled box (top-left + size) in logical
    // pixels and its z (render order). Position + size are exact, so any overlap or inter-element gap is
    // computable — the layout is fully reconstructable without rendering.
    let mut boxes: Vec<(String, f32, f32, f32, f32, String, usize)> = Vec::new();
    let mut movable_piles: HashSet<PileId> = HashSet::new();
    for (entity, movable, cn, gt) in nodes.iter() {
        let (name, zoom) = match movable.0 {
            TableNode::Card(cid) => {
                let Some(card) = table.0.card(cid) else {
                    continue;
                };
                (card.front_title().to_string(), format!("{:?}", card.size()))
            }
            TableNode::Pile(pid) => {
                movable_piles.insert(pid);
                let Some(pile) = table.0.pile(pid) else {
                    continue;
                };
                (format!("[{}]", pile.label), "-".into())
            }
        };
        let sf = cn.inverse_scale_factor;
        let size = cn.size() * sf;
        let tl = gt.translation * sf - size * 0.5;
        boxes.push((
            name,
            tl.x,
            tl.y,
            size.x,
            size.y,
            zoom,
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

    // Overlaps among the movable elements — the visible result of the push/shove (`separate`) logic: with
    // it working nothing should overlap. When two do overlap, the lower z is the one hidden behind.
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
            let sf = cn.inverse_scale_factor;
            let size = cn.size() * sf;
            let tl = gt.translation * sf - size * 0.5;
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

fn card_name(table: &Tableau, cref: Option<&CardRef>) -> String {
    cref.and_then(|c| table.card(c.0))
        .map(|c| c.front_title().to_string())
        .unwrap_or_else(|| "(control card)".into())
}

/// The dragged/clicked card's name, from either a `CardRef` (table cards) or a `Movable(Card)` (bespoke tiles
/// like the arena's formation tiles, which carry no `CardRef`). `None` if the entity is neither.
fn interacted_card(
    table: &Tableau,
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
/// carry `ArenaUnitCard` / `AffordanceControl`, not `CardRef`, so the old logger missed them).
fn log_click(
    on: On<Pointer<Click>>,
    guard: Res<crate::DragGuard>,
    cards: Query<&CardRef>,
    movables: Query<&Movable>,
    units: Query<&crate::ArenaUnitCard>,
    affordances: Query<&crate::AffordanceControl>,
    backs: Query<(), With<crate::BackCard>>,
    table: Res<Table>,
    log: Res<UiLog>,
) {
    let entity = on.event().entity;
    let p = on.event().pointer_location.position;
    // Best-effort name + kind of what was under the pointer (combatant / affordance / back before the generic
    // card, since a formation tile carries both `ArenaUnitCard` and `Movable`).
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
    } else {
        "(no interactive target)".into()
    };
    // The drag-guard is still set when the click that ends a drag fires; `on_click` drops that click, so note
    // it here rather than let it look like a click that did nothing.
    let outcome = if guard.0 {
        "  IGNORED (drag-guard)"
    } else {
        ""
    };
    log.0.write(&format!(
        "click: {what} at ({:.0},{:.0}){outcome}\n",
        p.x, p.y
    ));
}
