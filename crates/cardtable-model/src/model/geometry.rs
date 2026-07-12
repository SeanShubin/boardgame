//! Shared box geometry — pure functions over `(pos, size)` rectangles on a surface. Piles and cards
//! both separate against these; nothing here knows about cards, piles, or the tree. Extracted from
//! `model.rs` so the physical/UI split (plan §16) has the geometry (a UI-model concern) in one place.

use super::Pos;

/// The centre point of a box `(pos, size)`.
fn box_center(pos: Pos, size: Pos) -> Pos {
    Pos {
        x: pos.x + size.x * 0.5,
        y: pos.y + size.y * 0.5,
    }
}

/// Clamp a box of `size` at `pos` fully inside `surface` (top-left origin). A box larger than the
/// surface pins to the top-left.
pub(super) fn clamp_box(pos: Pos, size: Pos, surface: Pos) -> Pos {
    let max_x = (surface.x - size.x).max(0.0);
    let max_y = (surface.y - size.y).max(0.0);
    Pos {
        x: pos.x.clamp(0.0, max_x),
        y: pos.y.clamp(0.0, max_y),
    }
}

/// Total area by which a box `(pos, size)` overlaps the `locked` boxes.
fn overlap_area(pos: Pos, size: Pos, locked: &[(Pos, Pos)]) -> f32 {
    let mut total = 0.0;
    for &(lp, lsz) in locked {
        let ox = (pos.x + size.x).min(lp.x + lsz.x) - pos.x.max(lp.x);
        let oy = (pos.y + size.y).min(lp.y + lsz.y) - pos.y.max(lp.y);
        if ox > 0.01 && oy > 0.01 {
            total += ox * oy;
        }
    }
    total
}

/// The position nearest `cur` for a box of `size` that is fully inside `surface` *and* clear of every
/// `locked` box; if none is fully clear, the in-bounds spot of least total overlap. The free region for
/// an axis-aligned box among static boxes is rectilinear, so its nearest point lies on a candidate
/// coordinate line: the box's own coordinate, each locked box's near/far edge in configuration space, or
/// a wall. We test that grid of lines (straight slides *and* go-around-a-corner spots) and keep the
/// clear one closest to where the box already is.
pub(super) fn place_clear_of(cur: Pos, size: Pos, locked: &[(Pos, Pos)], surface: Pos) -> Pos {
    let max_x = (surface.x - size.x).max(0.0);
    let max_y = (surface.y - size.y).max(0.0);
    let cx = cur.x.clamp(0.0, max_x);
    let cy = cur.y.clamp(0.0, max_y);
    let mut xs = vec![cx, 0.0, max_x];
    let mut ys = vec![cy, 0.0, max_y];
    for &(lp, lsz) in locked {
        xs.push(lp.x - size.x); // just left of the locked box
        xs.push(lp.x + lsz.x); // just right of it
        ys.push(lp.y - size.y); // just above it
        ys.push(lp.y + lsz.y); // just below it
    }
    // Never-overlap invariant: candidates may run **off the right / bottom edge** (a box clips rather than
    // overlaps when space is tight), but never off the top / left. Because "just right of the rightmost
    // locked box" is always clear, a non-overlapping position always exists.
    xs.retain(|&x| x >= 0.0);
    ys.retain(|&y| y >= 0.0);

    // Ranked preference: a clear **on-surface** spot beats a clear **off-edge** (clipped) spot beats an
    // overlapping one. On-surface is always preferred, so a box only clips when it genuinely cannot fit
    // without overlapping; the overlapping fallback then never fires - the layout clips, it does not overlap.
    let mut best_on: Option<(f32, Pos)> = None; // clear, on-surface: (dist², pos)
    let mut best_off: Option<(f32, Pos)> = None; // clear, off right/bottom edge: (dist², pos)
    let mut best_any: Option<(f32, f32, Pos)> = None; // overlapping fallback: (overlap area, dist², pos)
    for &x in &xs {
        for &y in &ys {
            let pos = Pos { x, y };
            let overlap = overlap_area(pos, size, locked);
            let dist_sq = (x - cur.x).powi(2) + (y - cur.y).powi(2);
            if overlap <= 0.0 {
                let target = if x <= max_x + 0.01 && y <= max_y + 0.01 {
                    &mut best_on
                } else {
                    &mut best_off
                };
                if target.is_none_or(|(d, _)| dist_sq < d) {
                    *target = Some((dist_sq, pos));
                }
            } else if best_any.is_none_or(|(o, d, _)| overlap < o || (overlap == o && dist_sq < d))
            {
                best_any = Some((overlap, dist_sq, pos));
            }
        }
    }
    best_on
        .or(best_off)
        .map(|(_, p)| p)
        .or(best_any.map(|(_, _, p)| p))
        .unwrap_or(Pos { x: cx, y: cy })
}

/// Lock-as-you-go separation of `boxes` (`(pos, size)`) inside `surface`, pinning `anchor` and shoving
/// the rest clear nearest-first (a wavefront outward from the anchor). Returns each box's settled
/// position, index-aligned with `boxes`. The shared core of `Board::separate` (piles) and
/// `Board::separate` (cards): because each box is placed clear of all already-settled boxes
/// and never disturbed afterward, no two overlap once the space allows it. Terminates in one placement
/// per box.
pub(super) fn separate_boxes(
    boxes: &[(Pos, Pos)],
    anchor: usize,
    surface: Pos,
    pinned: &[(Pos, Pos)],
) -> Vec<Pos> {
    let mut result: Vec<Pos> = boxes.iter().map(|&(p, _)| p).collect();
    if boxes.is_empty() {
        return result;
    }
    // Priority order, lock-as-you-go: whoever is placed first wins its spot; everyone after settles clear.
    let mut locked: Vec<(Pos, Pos)> = Vec::with_capacity(pinned.len() + boxes.len());
    // (1) Pinned fixtures — highest priority. Placed *through the same clear-rule* (not dumped raw), so a
    // lower-priority pinned box yields to a higher one and two pinned boxes can never overlap.
    for &(p, s) in pinned {
        let pos = place_clear_of(p, s, &locked, surface);
        locked.push((pos, s));
    }
    // (2) The anchor (the just-dropped / just-changed box) — clear of the pinned, but nothing else moves it.
    let (anchor_pos, anchor_size) = boxes[anchor];
    let anchor_center = box_center(anchor_pos, anchor_size);
    let anchor_pos = place_clear_of(anchor_pos, anchor_size, &locked, surface);
    result[anchor] = anchor_pos;
    locked.push((anchor_pos, anchor_size));
    // (3) The rest — fan out nearest-first from the anchor, each clear of everything already locked.
    let mut order: Vec<usize> = (0..boxes.len()).filter(|&i| i != anchor).collect();
    order.sort_by(|&i, &j| {
        let di = box_center(boxes[i].0, boxes[i].1).dist_sq(anchor_center);
        let dj = box_center(boxes[j].0, boxes[j].1).dist_sq(anchor_center);
        di.partial_cmp(&dj)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(i.cmp(&j))
    });
    for i in order {
        let pos = place_clear_of(boxes[i].0, boxes[i].1, &locked, surface);
        result[i] = pos;
        locked.push((pos, boxes[i].1));
    }
    result
}
