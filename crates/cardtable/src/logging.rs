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

use bevy::picking::events::{Click, DragDrop, DragStart, Pointer};
use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use std::collections::HashMap;
use std::sync::Mutex;

use cardtable_model::{CardId, Node as TableNode, PileId, Tableau};

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
            .add_systems(Update, (log_physical, log_view, log_layout))
            .add_observer(log_pickup)
            .add_observer(log_drop)
            .add_observer(log_click);
    }
}

// ---- physical-cards.log: the card tree + transitions ---------------------------------------------

/// One card in a physical snapshot: where it lives (a `/`-joined pile-label path) and which way it faces.
#[derive(Clone, PartialEq)]
struct CardState {
    path: String,
    name: String,
    face_up: bool,
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
                cards.insert(
                    *cid,
                    CardState {
                        path: path.clone(),
                        name: card.front_title().to_string(),
                        face_up: !card.is_face_down(),
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
                out.push_str("(reordering — no card changed pile or face)\n");
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
    nodes: Query<(&Movable, &ComputedNode, &UiGlobalTransform)>,
    table: Res<Table>,
    log: Res<UiLog>,
    mut last_frame: Local<String>,
    mut last_logged: Local<String>,
) {
    // Every positioned felt element (card *and* pile) with its settled box (top-left + size) in logical
    // pixels, ordered top-to-bottom then left-to-right. Position + size are exact, so any overlap or
    // inter-element gap is computable from this — the layout is fully reconstructable without rendering.
    let mut boxes: Vec<(String, f32, f32, f32, f32, String)> = nodes
        .iter()
        .filter_map(|(movable, cn, gt)| {
            let (name, zoom) = match movable.0 {
                TableNode::Card(cid) => {
                    let card = table.0.card(cid)?;
                    (card.front_title().to_string(), format!("{:?}", card.size()))
                }
                TableNode::Pile(pid) => (format!("[{}]", table.0.pile(pid)?.label), "-".into()),
            };
            let sf = cn.inverse_scale_factor;
            let size = cn.size() * sf;
            let tl = gt.translation * sf - size * 0.5;
            Some((name, tl.x, tl.y, size.x, size.y, zoom))
        })
        .collect();
    boxes.sort_by_key(|b| (b.2 as i32, b.1 as i32));

    let cards_block: String = boxes
        .iter()
        .map(|(name, x, y, w, h, zoom)| {
            format!("  {name} @ ({x:.0},{y:.0}) size ({w:.0}x{h:.0}) zoom {zoom}")
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Overlaps — the visible result of the push/shove (`separate`) logic: with it working, nothing should
    // overlap. Each entry gives the overlap depth in pixels; "none" means the push settled everything clear.
    let mut overlaps = Vec::new();
    for i in 0..boxes.len() {
        for j in (i + 1)..boxes.len() {
            let (ni, ax, ay, aw, ah, _) = &boxes[i];
            let (nj, bx, by, bw, bh, _) = &boxes[j];
            let ox = (ax + aw).min(bx + bw) - ax.max(*bx);
            let oy = (ay + ah).min(by + bh) - ay.max(*by);
            if ox > 0.5 && oy > 0.5 {
                overlaps.push(format!("  OVERLAP: {ni} & {nj} by ({ox:.0}x{oy:.0})"));
            }
        }
    }
    let overlap_block = if overlaps.is_empty() {
        "  overlaps: none (push settled all clear)".to_string()
    } else {
        format!("  overlaps: {}\n{}", overlaps.len(), overlaps.join("\n"))
    };

    let snapshot = format!("{cards_block}\n{overlap_block}");
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
        .unwrap_or_else(|| "?".into())
}

/// Log a card pick-up (drag start) with its pointer position.
fn log_pickup(
    on: On<Pointer<DragStart>>,
    cards: Query<&CardRef>,
    table: Res<Table>,
    log: Res<UiLog>,
) {
    if let Ok(cref) = cards.get(on.event().entity) {
        let p = on.event().pointer_location.position;
        log.0.write(&format!(
            "pick up: {} at ({:.0},{:.0})\n",
            card_name(&table.0, Some(cref)),
            p.x,
            p.y
        ));
    }
}

/// Log a drop with the dragged card, what it landed on, and the pointer position.
fn log_drop(
    on: On<Pointer<DragDrop>>,
    cards: Query<&CardRef>,
    piles: Query<&PileDropZone>,
    table: Res<Table>,
    log: Res<UiLog>,
) {
    let event = on.event();
    let Ok(dragged) = cards.get(event.event.dropped) else {
        return;
    };
    let onto = if let Ok(target) = cards.get(event.entity) {
        format!("card {}", card_name(&table.0, Some(target)))
    } else if let Ok(zone) = piles.get(event.entity) {
        let label = table
            .0
            .pile(zone.0)
            .map(|p| p.label.clone())
            .unwrap_or_default();
        format!("pile [{label}]")
    } else {
        "felt".into()
    };
    let p = event.pointer_location.position;
    log.0.write(&format!(
        "drop: {} onto {onto} at ({:.0},{:.0})\n",
        card_name(&table.0, Some(dragged)),
        p.x,
        p.y
    ));
}

/// Log a click on a card with its pointer position.
fn log_click(on: On<Pointer<Click>>, cards: Query<&CardRef>, table: Res<Table>, log: Res<UiLog>) {
    if let Ok(cref) = cards.get(on.event().entity) {
        let p = on.event().pointer_location.position;
        log.0.write(&format!(
            "click: {} at ({:.0},{:.0})\n",
            card_name(&table.0, Some(cref)),
            p.x,
            p.y
        ));
    }
}
