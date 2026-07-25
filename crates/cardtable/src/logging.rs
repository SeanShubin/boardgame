//! **Debug logs** that mirror the three layers (plan §0), so a play session can be read back and
//! checked. Three files (native only — no filesystem on the web):
//!
//! - `physical-cards.log` — the **physical model**: the conserved card tree as an indented hierarchy
//!   (each card's face up/down), alternating with the **transitions** (what moved / flipped / appeared /
//!   vanished) between one state and the next. Lets a human confirm each state transition by hand.
//!   *Truncated on launch.*
//! - `ui-state.log` — the **UI model + IO**: which view (zone) is entered, the settled layout of each
//!   card on that view (position, size, zoom), and every pick-up / drop / click with its pointer
//!   position. Lets a reader reconstruct exactly how the table was interacted with. *Truncated on launch.*
//! - `combat-log.log` — **just the combat-log area**: the running transcript of exactly what the player read
//!   there, and nothing else. *Truncated at the start of each battle*, so it always holds the last fight.
//! - `frame-time.log` — the **frame-rate monitor**: a per-second FPS pulse that stays quiet while the app runs
//!   smoothly and records every slowdown (`SLOW` windows, immediate `STALL` freezes), so "the app got slow"
//!   can be checked against numbers. *Truncated on launch.* See [`FrameLog`].
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
use crate::{CardRef, Dragging, Movable, PileDropZone, SceneRegion, Table, TileCard};

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
pub(crate) struct UiLog(Log);

impl UiLog {
    /// Append a diagnostic line to `ui-state.log` (native only; a no-op on the web). Lets other systems drop
    /// a note into the same stream as the click log, so a gesture and its outcome sit side by side.
    pub(crate) fn note(&self, text: &str) {
        self.0.write(text);
    }
}

/// `combat-log.log` — **only** what the combat-log area shows the player, and truncated at the start of each
/// battle, so it always holds the last fight and nothing else. Unlike the other two it is not a launch-time
/// log: it is re-created when a fight opens.
#[derive(Resource)]
struct CombatLog(Mutex<Option<std::fs::File>>);

impl CombatLog {
    /// Begin a battle: throw away the previous one.
    fn restart(&self) {
        if cfg!(target_arch = "wasm32") {
            return;
        }
        if let Ok(mut guard) = self.0.lock() {
            *guard = std::fs::File::create("combat-log.log").ok();
        }
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

// ---- frame-time.log: the frame-rate monitor -----------------------------------------------------

/// A frame slower than this (30 fps) counts as a **hitch**.
const HITCH_MS: f64 = 33.3;
/// A one-second window whose average frame rate falls below this is flagged even without a single big hitch -
/// the "everything got sluggish" case, as opposed to an isolated stutter.
const FPS_FLOOR: f64 = 50.0;
/// A single frame at least this long is a visible **freeze** - logged the instant it lands, not at window end.
const STALL_MS: f64 = 250.0;
/// How often a smooth (no-drop) window still prints a heartbeat, so a quiet log is "no drops", not "monitor died".
const HEARTBEAT_S: f64 = 30.0;

/// `frame-time.log` - the **frame-rate monitor**. It watches the real frame clock and stays quiet while the
/// app runs smoothly, calling out every slowdown so a "the app got slow" report can be checked against numbers
/// instead of a feeling: a single frame over [`STALL_MS`] is logged the instant it happens (a visible freeze);
/// each one-second window with any hitch (> [`HITCH_MS`]) or a sub-[`FPS_FLOOR`] average is logged `SLOW`;
/// otherwise a heartbeat every [`HEARTBEAT_S`] prints the smooth baseline. (Native only; a no-op on the web.)
#[derive(Resource)]
struct FrameLog {
    log: Log,
    /// The startup frame's delta is a load spike, not a drop; skip it and start the first window after it.
    started: bool,
    /// Real-clock seconds at the current window's start, and at the last line written (for the heartbeat).
    window_start: f64,
    last_report: f64,
    /// This window's accumulators: frames seen, hitches (>= HITCH_MS), worst single frame, total time.
    frames: u32,
    slow: u32,
    worst_ms: f64,
    sum_ms: f64,
}

/// Watch the real frame clock; record drops to `frame-time.log` (see [`FrameLog`]). Uses `Time<Real>` so a
/// paused or time-scaled virtual clock could never hide or fake a slowdown.
fn log_frame_time(time: Res<Time<bevy::time::Real>>, mut fl: ResMut<FrameLog>) {
    let now = time.elapsed().as_secs_f64();
    if !fl.started {
        fl.started = true;
        fl.window_start = now;
        fl.last_report = now;
        fl.log.write(&format!(
            "frame-time.log - frame-rate monitor (native only)\n\
             thresholds: hitch >= {HITCH_MS:.0} ms (30 fps), window floor {FPS_FLOOR:.0} fps, \
             stall >= {STALL_MS:.0} ms; heartbeat every {HEARTBEAT_S:.0}s\n\
             a quiet log means no drops - only SLOW / STALL lines are problems.\n\n"
        ));
        return; // skip the startup frame's delta (a load spike, not a drop)
    }

    let dt_ms = time.delta().as_secs_f64() * 1000.0;

    // A single freeze is an event in its own right: log it the instant it lands, not folded into a window.
    if dt_ms >= STALL_MS {
        fl.log.write(&format!(
            "[t={now:8.1}s] STALL  one frame {dt_ms:.1} ms (~{:.0} fps)\n",
            1000.0 / dt_ms.max(0.001)
        ));
    }

    fl.frames += 1;
    fl.sum_ms += dt_ms;
    if dt_ms > fl.worst_ms {
        fl.worst_ms = dt_ms;
    }
    if dt_ms >= HITCH_MS {
        fl.slow += 1;
    }

    let span = now - fl.window_start;
    if span >= 1.0 {
        let fps = fl.frames as f64 / span;
        let avg = fl.sum_ms / f64::from(fl.frames.max(1));
        let problem = fl.slow > 0 || fps < FPS_FLOOR;
        if problem || now - fl.last_report >= HEARTBEAT_S {
            let (tag, worst, slow, frames) = (
                if problem { "SLOW" } else { "ok  " },
                fl.worst_ms,
                fl.slow,
                fl.frames,
            );
            fl.log.write(&format!(
                "[t={now:8.1}s] {tag}  {fps:5.1} fps  avg {avg:5.1} ms  worst {worst:6.1} ms  hitches {slow}/{frames}\n"
            ));
            fl.last_report = now;
        }
        fl.window_start = now;
        fl.frames = 0;
        fl.slow = 0;
        fl.worst_ms = 0.0;
        fl.sum_ms = 0.0;
    }
}

/// Records the two debug logs. Added by the product; native-only file output.
pub struct LoggingPlugin;

impl Plugin for LoggingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PhysicalLog(Log::create("physical-cards.log")))
            .insert_resource(UiLog(Log::create("ui-state.log")))
            .insert_resource(CombatLog(Mutex::new(None)))
            .insert_resource(FrameLog {
                log: Log::create("frame-time.log"),
                started: false,
                window_start: 0.0,
                last_report: 0.0,
                frames: 0,
                slow: 0,
                worst_ms: 0.0,
                sum_ms: 0.0,
            })
            .add_systems(
                Update,
                (
                    log_physical,
                    log_view,
                    log_layout,
                    log_scene,
                    mirror_scene,
                    mirror_screen,
                    log_combat,
                    drain_drop_trace,
                    log_frame_time,
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
    /// The modal scene's regions. Not cards, not decks, not drop-zones - so until now, invisible to the very
    /// log built to answer "is there room for this?".
    regions: Query<
        'w,
        's,
        (
            &'static SceneRegion,
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

    // The scene's regions, with the room each has left. `free` is the slack INSIDE the region below its last
    // child - which is the number the question "can another panel fit here?" actually turns on.
    let mut regions: Vec<(String, f32, f32, f32, f32)> = q
        .regions
        .iter()
        .map(|(r, cn, gt)| {
            let (center, half) = crate::node_box(cn, gt);
            let (size, tl) = (half * 2.0, center - half);
            (r.0.to_string(), tl.x, tl.y, size.x, size.y)
        })
        .collect();
    regions.sort_by_key(|r| (r.2 as i32, r.1 as i32));
    let regions_block: String = regions
        .iter()
        .map(|(n, x, y, w, h)| format!("  {n} @ ({x:.0},{y:.0}) size ({w:.0}x{h:.0})"))
        .collect::<Vec<_>>()
        .join("\n");

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
            // A card cascaded from a **nested** pile onto its parent's card is likewise an intentional
            // stack: a hero seated in a cell's Seat sub-pile is drawn on the cell's encounter. Exempt a
            // parent/child pile pair too (the drop target still surrounds the whole cascade).
            if let (Some(pa), Some(pb)) = (a.pile, b.pile)
                && (table.0.pile(pa).and_then(|p| p.parent()) == Some(pb)
                    || table.0.pile(pb).and_then(|p| p.parent()) == Some(pa))
            {
                continue;
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

    // The scene's regions come FIRST: on the combat screen they *are* the layout, and none of the felt's cards
    // are even on it. "Is there room for another panel?" is a question about these boxes and nothing else.
    let scene_block = if regions_block.is_empty() {
        String::new()
    } else {
        format!("  scene regions:\n{regions_block}\n")
    };
    let snapshot = format!("{scene_block}{cards_block}\n{overlap_block}{zones_block}");
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
    choices: Query<&crate::ChoiceControl>,
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
    } else if let Ok(c) = choices.get(entity) {
        // The scene's decision buttons (a hero's order menu). These were previously UNLOGGED, so a dropped
        // or double-needed choice click left no trace - exactly the "I clicked and nothing happened" case.
        format!("choice #{} [choice]", c.0)
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

/// A screen box for overlap testing: its title, the pile it stacks in (same-pile cards stack intentionally),
/// its identity, and its rect (top-left + size, logical px). Pure data - no ECS - so [`screen_overlaps`] is
/// unit-testable.
#[derive(Clone)]
struct ScreenBox {
    title: String,
    stack: Option<PileId>,
    /// The parent of this box's stack pile, if any - so the overlap check can tell a **nested** cascade (a
    /// hero seated in a cell's Seat sub-pile, drawn on the cell's encounter) from a real spill.
    parent_stack: Option<PileId>,
    id: u64,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

/// Every pair of boxes that overlap and are NOT an intentional same-pile stack, as
/// `(title_a, title_b, overlap_w, overlap_h)`. The pure core of the screen snapshot's overlap check - the
/// never-overlap invariant made checkable from the boxes alone (the geometry tenet: a settled layout clips,
/// it never overlaps).
fn screen_overlaps(boxes: &[ScreenBox]) -> Vec<(String, String, f32, f32)> {
    let mut out = Vec::new();
    for i in 0..boxes.len() {
        for j in (i + 1)..boxes.len() {
            let (a, b) = (&boxes[i], &boxes[j]);
            if a.id == b.id || (a.stack.is_some() && a.stack == b.stack) {
                continue; // the same card twice, or an intentional stack
            }
            // A card cascaded from a nested pile onto its parent's card is also intentional: a seated hero
            // (in a cell's Seat sub-pile) drawn on the cell's encounter. Exempt a parent/child stack pair.
            if (a.stack.is_some() && a.stack == b.parent_stack)
                || (b.stack.is_some() && b.stack == a.parent_stack)
            {
                continue;
            }
            let ox = (a.x + a.w).min(b.x + b.w) - a.x.max(b.x);
            let oy = (a.y + a.h).min(b.y + b.h) - a.y.max(b.y);
            if ox > 0.5 && oy > 0.5 {
                out.push((a.title.clone(), b.title.clone(), ox, oy));
            }
        }
    }
    out
}

/// The rendered elements [`mirror_screen`] reads - the actual UI render tree, so the snapshot describes what
/// the screen shows rather than what the model intends.
#[derive(SystemParam)]
struct ScreenQuery<'w, 's> {
    /// Every text node the renderer spawned, with its box - the complete "what text did we attempt to draw"
    /// set, pre-wrapping (the string is the unwrapped source). Covers titles, badges, prompts, button
    /// labels, the log - everything with words, on any screen.
    texts: Query<
        'w,
        's,
        (
            &'static Text,
            &'static ComputedNode,
            &'static UiGlobalTransform,
            Option<&'static bevy::ui::CalculatedClip>,
        ),
    >,
    /// Every rendered table card (felt or a Virtual readout).
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
    /// Every rendered modal tile (a combat tile carries `TileCard`, not `CardRef`).
    tiles: Query<
        'w,
        's,
        (
            Entity,
            &'static TileCard,
            &'static ComputedNode,
            &'static UiGlobalTransform,
        ),
    >,
}

/// **The generic screen-description snapshot** - `screen.txt`, rewritten whenever the settled screen changes,
/// on EVERY screen (felt or the modal fight). It is the present-tense answer to "what is on the screen right
/// now", built by reading the actual UI RENDER TREE (the same nodes the GPU draws), not the model - so
/// nothing that is drawn as a card or a piece of text escapes it:
///
/// - **TEXT** - every string the renderer attempted, with its box, in reading order. Pre-wrapping (the source
///   string), because wrapping is a visual artifact, not content; the text we *tried* to draw is what
///   matters.
/// - **CARDS** - every card / tile with its box and z (render order), and any **effect** applied to it (the
///   targeting ring, the commanded highlight). Effects are recorded as ASSIGNMENTS (*which* cards carry them)
///   and never as animation frames: the marching-dots ring is drawn by [`animate_target_rings`] and is
///   deliberately absent here; this file just says the ring is *on* The Wall and The Sniper.
/// - **OVERLAPS** - since every card has a well-defined box, any two that overlap (and are not an intentional
///   same-pile stack) are flagged from the boxes alone. In a settled layout there must be none.
///
/// It is the present tense; the append history stays in `ui-state.log`. (Native only.) The guarantee this
/// reaches - "nothing rendered that is not described" - is observational: it reads the render tree, so any
/// node with content is captured. Pure decoration (a background panel with no text) is out of scope, as are
/// visual artifacts (wrapping, fonts, the animation frames themselves).
fn mirror_screen(
    q: ScreenQuery,
    table: Res<Table>,
    scene: Res<SceneState>,
    dragging: Res<Dragging>,
    windows: Query<&bevy::window::Window, With<bevy::window::PrimaryWindow>>,
    mut last_frame: Local<String>,
    mut last_written: Local<String>,
) {
    if cfg!(target_arch = "wasm32") {
        return;
    }

    // The logical viewport size, so margins (e.g. the gap between the log and the bottom edge) are computable
    // from the file: gap = viewport height - element bottom.
    let viewport = windows
        .single()
        .map(|w| {
            format!(
                "viewport: {:.0} x {:.0} (logical px)
",
                w.width(),
                w.height()
            )
        })
        .unwrap_or_default();

    // The view header: the drilled-into zone, and whether the modal fight is up.
    let focus = table.0.focus_id();
    let view = table
        .0
        .pile(focus)
        .map(|p| p.label.clone())
        .unwrap_or_default();
    let header = match &scene.0 {
        Some(s) => format!("{viewport}screen: MODAL - {}", s.heading),
        None => format!("{viewport}screen: felt - view [{view}]"),
    };

    // ---- effect assignments (which cards carry which cue), from the scene - never the animated dots. ----
    let mut ring: Vec<String> = Vec::new();
    let mut commanded: Vec<String> = Vec::new();
    if let Some(s) = &scene.0 {
        let mut note = |t: &cardtable_model::Tile| match t.highlight {
            cardtable_model::Highlight::Targeted => ring.push(t.title.clone()),
            cardtable_model::Highlight::Active => commanded.push(t.title.clone()),
            _ => {}
        };
        match &s.body {
            cardtable_model::SceneBody::Lanes(lanes) => {
                for lane in lanes {
                    for t in lane.left.iter().chain(lane.right.iter()) {
                        note(t);
                    }
                }
            }
            cardtable_model::SceneBody::Rows(rows) => {
                for row in rows {
                    for t in &row.tiles {
                        note(t);
                    }
                }
            }
        }
    }

    // ---- the card / tile boxes (id, title, box, z, stack), for the geometry + overlap pass. ----
    struct Box {
        title: String,
        id: CardId,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        pile: Option<PileId>,
    }
    let title_of = |id: CardId| {
        table
            .0
            .card(id)
            .map(|c| c.front_title().to_string())
            .unwrap_or_else(|| format!("#{}", id.0))
    };
    let mut boxes: Vec<Box> = Vec::new();
    let mut push_box = |id: CardId, cn: &ComputedNode, gt: &UiGlobalTransform| {
        let (center, half) = crate::node_box(cn, gt);
        let tl = center - half;
        boxes.push(Box {
            title: title_of(id),
            id,
            x: tl.x,
            y: tl.y,
            w: half.x * 2.0,
            h: half.y * 2.0,
            pile: table.0.pile_of(id),
        });
    };
    for (_, cref, cn, gt) in q.cards.iter() {
        push_box(cref.0, cn, gt);
    }
    for (_, tile, cn, gt) in q.tiles.iter() {
        push_box(tile.0, cn, gt);
    }
    boxes.sort_by_key(|b| (b.y as i32, b.x as i32));

    let effect_of = |b: &Box| -> String {
        let t = &b.title;
        if ring.contains(t) {
            "  <targeting-ring>".to_string()
        } else if commanded.contains(t) {
            "  <commanded>".to_string()
        } else {
            String::new()
        }
    };

    // ---- drop cues: which cards are pickable-for-a-game-action, and (mid-drag) which cards the held one may
    // legally land on. This is the "which drop targets are activated" a headless reader needs - the same
    // predicates the green glow is painted from (`is_game_movable` / `can_drop_on_card`), as data.
    let movable: Vec<&str> = boxes
        .iter()
        .filter(|b| crate::is_game_movable(&table.0, b.id))
        .map(|b| b.title.as_str())
        .collect();
    let dragged = dragging.0.and_then(|n| n.card());
    let drop_block = {
        let held = dragged
            .and_then(|d| boxes.iter().find(|b| b.id == d))
            .map(|b| b.title.as_str())
            .unwrap_or("(nothing held)");
        let targets: Vec<&str> = dragged
            .map(|d| {
                boxes
                    .iter()
                    .filter(|b| b.id != d && crate::can_drop_on_card(&table.0, d, b.id))
                    .map(|b| b.title.as_str())
                    .collect()
            })
            .unwrap_or_default();
        let list = |v: &[&str]| {
            if v.is_empty() {
                "(none)".to_string()
            } else {
                v.join(", ")
            }
        };
        format!(
            "  movable (game action): {}\n  held: {held}\n  drop-target cards: {}",
            list(&movable),
            list(&targets)
        )
    };
    let cards_block: String = boxes
        .iter()
        .map(|b| {
            format!(
                "  {} @ ({:.0},{:.0}) size ({:.0}x{:.0}){}",
                b.title,
                b.x,
                b.y,
                b.w,
                b.h,
                effect_of(b)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // ---- overlaps: any two boxes that overlap and are not the same intentional stack. ----
    let cells: Vec<ScreenBox> = boxes
        .iter()
        .map(|b| ScreenBox {
            title: b.title.clone(),
            stack: b.pile,
            parent_stack: b
                .pile
                .and_then(|p| table.0.pile(p))
                .and_then(|z| z.parent()),
            id: b.id.0,
            x: b.x,
            y: b.y,
            w: b.w,
            h: b.h,
        })
        .collect();
    let overlaps: Vec<String> = screen_overlaps(&cells)
        .into_iter()
        .map(|(a, b, ox, oy)| format!("    ERROR overlap: {a} & {b} by ({ox:.0}x{oy:.0})"))
        .collect();
    let overlap_block = if dragging.0.is_some() {
        "  overlaps: (drag in progress - transient overlap allowed)".to_string()
    } else if overlaps.is_empty() {
        "  overlaps: none".to_string()
    } else {
        format!(
            "  ERROR: {} settled overlap(s):\n{}",
            overlaps.len(),
            overlaps.join("\n")
        )
    };

    // ---- every text node, in reading order (top-to-bottom, left-to-right). ----
    let mut texts: Vec<(i32, i32, String, f32, f32)> = q
        .texts
        .iter()
        .filter_map(|(t, cn, gt, clip)| {
            let s = t.0.trim().to_string();
            if s.is_empty() {
                return None;
            }
            let (center, half) = crate::node_box(cn, gt);
            // Honor an ancestor's overflow clip: text laid out fully OUTSIDE its clip rect is not on screen
            // (e.g. a bottom-anchored log's oldest lines that overflow above their box), so it is not
            // described. This keeps the snapshot to what is VISIBLE, not the pre-clip layout.
            if let Some(clip) = clip {
                let (cmin, cmax) = (clip.clip.min, clip.clip.max);
                let (nmin, nmax) = (center - half, center + half);
                let outside =
                    nmax.x <= cmin.x || nmin.x >= cmax.x || nmax.y <= cmin.y || nmin.y >= cmax.y;
                if outside {
                    return None;
                }
            }
            let tl = center - half;
            Some((tl.y as i32, tl.x as i32, s, half.x * 2.0, half.y * 2.0))
        })
        .collect();
    texts.sort_by_key(|t| (t.0, t.1));
    let text_block: String = texts
        .iter()
        .map(|(y, x, s, w, h)| format!("  {s:?} @ ({x},{y}) size ({w:.0}x{h:.0})"))
        .collect::<Vec<_>>()
        .join("\n");

    let effects_block = {
        let mut lines = Vec::new();
        if !ring.is_empty() {
            lines.push(format!("  targeting-ring on: {}", ring.join(", ")));
        }
        if !commanded.is_empty() {
            lines.push(format!("  commanded: {}", commanded.join(", ")));
        }
        if lines.is_empty() {
            "  (none)".to_string()
        } else {
            lines.join("\n")
        }
    };

    let snapshot = format!(
        "{header}\n\nEFFECTS (assignments, not animation frames):\n{effects_block}\n\nDROP CUES:\n{drop_block}\n\nCARDS ({} on screen):\n{cards_block}\n{overlap_block}\n\nTEXT ({} strings attempted):\n{text_block}\n",
        boxes.len(),
        texts.len(),
    );

    // Write only a SETTLED frame (this frame equals the last) that differs from what is on disk - so the file
    // holds a stable arrangement, never a mid-animation one, and is not rewritten every frame.
    if snapshot == *last_frame && snapshot != *last_written {
        let _ = std::fs::write("screen.txt", &snapshot);
        *last_written = snapshot.clone();
    }
    *last_frame = snapshot;
}

/// **The current-screen snapshot** - `ui-scene.txt`, rewritten whenever the modal scene changes, so "what
/// does the screen look like RIGHT NOW" is always answerable from one file, completely and unambiguously:
/// every tile with its named attention state, every choice with its status, the controls, the prompt. The
/// append history lives in `ui-state.log`; this file is the present tense. (Native only.)
fn mirror_scene(scene: Res<SceneState>, mut last: Local<String>) {
    if cfg!(target_arch = "wasm32") {
        return;
    }
    let text = match &scene.0 {
        Some(s) => s.describe(),
        None => "(no modal scene - the felt is showing)
"
        .to_string(),
    };
    if *last != text {
        let _ = std::fs::write("ui-scene.txt", &text);
        *last = text;
    }
}

/// Log the **modal scene** — the combat screen — whenever its text changes (the append HISTORY, in
/// `ui-state.log`; the present-tense snapshot is [`mirror_scene`]'s `ui-scene.txt`). Everything the screen
/// says is written here: which phase and step each track is on, the prompt, the decision being asked for
/// (with each option's consequence, or the reason it is barred), and the combat log lines themselves.
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
    // The body: every tile with its badges, whether it is highlighted as a legal target, and whether a tap
    // would actually do anything. Without this the log says which *phase* you are in but not what is *on the
    // board*, so a report like "it says no targets chosen while there are none to choose" cannot be checked -
    // whether a target existed is exactly the fact in dispute.
    let tile_line = |t: &cardtable_model::Tile, group: &str| {
        let badges: Vec<&str> = t.badges.iter().map(|b| b.text.as_str()).collect();
        format!(
            "  [{group}] {}  {:?}{}{}  | {}\n",
            t.title,
            t.highlight,
            if t.tappable { " tappable" } else { "" },
            if t.draggable { " draggable" } else { "" },
            badges.join(" / ")
        )
    };
    match &s.body {
        cardtable_model::SceneBody::Rows(rows) => {
            for r in rows {
                for t in &r.tiles {
                    out.push_str(&tile_line(t, &r.label));
                }
            }
        }
        cardtable_model::SceneBody::Lanes(lanes) => {
            for l in lanes {
                for t in l.left.iter().chain(l.right.iter()) {
                    out.push_str(&tile_line(t, &l.label));
                }
            }
        }
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

/// Write **the combat-log area** — exactly the lines the player reads there, nothing else — to
/// `combat-log.log`, cleared at the start of each battle so it always holds the last fight.
///
/// The scene's log is a *snapshot* of the current step, not a running transcript: it is rebuilt from the board
/// every frame and replaced wholesale as the fight walks on. So the running transcript has to be assembled
/// here, by appending each state as it appears.
///
/// **Only committed states.** The log area also previews what you are *staging* ("Raider may strike: The Wall
/// (not aimed yet)"), and that churns with every tap. A transcript full of half-formed plans is noise, so a
/// block is written only when the fight's **walk position** moves - which is exactly what a Commit does, and
/// the only thing that does. The log at that moment describes what the commit resolved. Everything staged and
/// then re-staged in between is left out, as it should be: it never happened.
fn log_combat(
    scene: Res<SceneState>,
    log: Res<CombatLog>,
    mut last: Local<String>,
    mut in_fight: Local<bool>,
) {
    let Some(s) = &scene.0 else {
        // The arena is gone: the fight ended (or was left). Keep the file - it IS the last battle.
        if *in_fight {
            log.write("\n-- the fight is over --\n");
            *in_fight = false;
            last.clear();
        }
        return;
    };
    if !*in_fight {
        log.restart(); // a new battle: the previous one is no longer "the last battle"
        *in_fight = true;
        last.clear();
    }

    // Where the fight stands. The panel's own title now says it ("Round 1 - Clash - Strike"), so this file is
    // a **literal transcript of what the player read** - no re-stating the position in a different format that
    // could disagree with the screen. It changes only on a Commit (or a Back, which rewinds one), never on
    // staging a plan.
    if s.log_title == *last {
        return;
    }
    *last = s.log_title.clone();

    if s.log.is_empty() {
        return; // nothing happened here - so the log area is empty, and so is this
    }
    let mut out = format!("\n{}\n", s.log_title);
    for line in &s.log {
        out.push_str(&format!("{line}\n"));
    }
    log.write(&out);
}

#[cfg(test)]
mod tests {
    use super::{ScreenBox, screen_overlaps};
    use cardtable_model::PileId;

    fn b(title: &str, stack: Option<u64>, x: f32, y: f32, w: f32, h: f32) -> ScreenBox {
        ScreenBox {
            title: title.into(),
            stack: stack.map(PileId),
            parent_stack: None,
            id: (x as u64) * 100_003 + (y as u64),
            x,
            y,
            w,
            h,
        }
    }

    /// Disjoint boxes overlap nothing; a genuine spill is flagged with its overlap size; two cards of the
    /// SAME pile are an intentional stack and are not flagged.
    #[test]
    fn screen_overlaps_flags_only_real_spills() {
        // Two decks side by side, clear of each other.
        let clear = vec![
            b("[A]", None, 0.0, 0.0, 100.0, 100.0),
            b("[B]", None, 120.0, 0.0, 100.0, 100.0),
        ];
        assert!(
            screen_overlaps(&clear).is_empty(),
            "disjoint boxes: no overlap"
        );

        // A card spilling onto another deck - a real error.
        let spill = vec![
            b("The Wall", Some(1), 0.0, 0.0, 100.0, 100.0),
            b("[Bestiary]", None, 80.0, 20.0, 100.0, 100.0),
        ];
        let found = screen_overlaps(&spill);
        assert_eq!(found.len(), 1, "the spill is flagged");
        assert_eq!(
            (found[0].2, found[0].3),
            (20.0, 80.0),
            "with its overlap size"
        );

        // Two cards of the SAME pile (a location's characters) - an intentional stack, not an error.
        let stack = vec![
            b("Raider", Some(7), 0.0, 0.0, 100.0, 100.0),
            b("Marksman", Some(7), 10.0, 10.0, 100.0, 100.0),
        ];
        assert!(
            screen_overlaps(&stack).is_empty(),
            "same-pile cards stack intentionally"
        );

        // A seated hero (its Seat sub-pile, id 8) cascaded on its cell's encounter (the cell, id 7): the
        // Seat's parent IS the cell, so this nested cascade is intentional, not a spill.
        let cascade = vec![
            b("The Keep Duelist", Some(7), 0.0, 0.0, 100.0, 100.0),
            ScreenBox {
                parent_stack: Some(PileId(7)),
                ..b("Raider", Some(8), 0.0, 26.0, 100.0, 100.0)
            },
        ];
        assert!(
            screen_overlaps(&cascade).is_empty(),
            "a hero seated on its encounter is a nested cascade, not a spill"
        );
    }
}
