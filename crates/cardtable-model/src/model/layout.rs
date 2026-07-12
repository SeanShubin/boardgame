//! **Pure 2-space layout** — a card's on-felt footprint `(width, height)` as a function of its
//! [`Size`](super::Size) and its content (how many detail / panel lines it carries). No Bevy, no rendering,
//! no measurement: the model *computes* where a card's box is, and the renderer sizes its card node to
//! exactly this footprint (a pass-through), so the whole layout - positions, sizes, overlaps - is known
//! without ever rendering. This is the model owning "where the cards are in 2-space".
//!
//! Each content line is drawn as **one** line (no wrap, horizontal overflow clips), so a card's height is an
//! exact function of its line count - never dependent on text wrapping or font measurement. Width is fixed
//! per size. The constants here are the single source of truth: the renderer reads the footprint, it does
//! not define its own card sizes.

use super::{Pos, Size};

/// Card width for the compact (name-only) form and for decks / location cards.
pub const SMALL_W: f32 = 120.0;
/// Card height for the compact form (fixed; the name strip clips a long title).
pub const SMALL_H: f32 = 96.0;
/// Card width for a full card face (name + detail lines).
pub const MEDIUM_W: f32 = 200.0;
/// Card width for a document-sized panel (a combat log, docs).
pub const LARGE_W: f32 = 320.0;
/// A large panel caps here and scrolls; its footprint never exceeds this height.
pub const LARGE_MAX_H: f32 = 360.0;

/// The diagonal step between the layers of a deck chip's stack - each deeper card peeks out this far along
/// the left and bottom edges, hinting at depth. The renderer draws exactly this offset.
pub const STACK_OFFSET: f32 = 2.0;
/// The most stack layers a deck chip draws (deeper decks stop growing, so a huge deck's chip stays bounded).
pub const MAX_STACK: usize = 10;

/// Vertical chrome: padding (top+bottom) plus border (top+bottom) around a card's content.
const V_CHROME: f32 = 2.0 * 10.0 + 2.0 * 2.0;
/// The gap the renderer puts **between** every stacked child in a card (`row_gap`). Each child past the
/// first adds one of these, so the height must budget for them or the last line clips.
const ROW_GAP: f32 = 4.0;
/// The name/title line's height (the `FONT_HEAD` line). Held a little above the raw font line box so text
/// never clips against a rounding-down of the renderer's metrics.
const TITLE_H: f32 = 24.0;
/// The **type badge** row Medium cards carry under the title (its padding + `FONT_BADGE` line). A Medium
/// card is assumed to have one; a card with no type just wears the extra slack (better than clipping).
const BADGE_H: f32 = 18.0;
/// One content line (a detail or panel line, `FONT_BODY`), drawn no-wrap so it is exactly one line tall.
const BODY_LINE_H: f32 = 17.0;

/// A card's on-felt footprint `(width, height)` in logical px, from its size and content line counts. The
/// renderer draws the card at exactly this size (content clips to fit), so this is authoritative for layout.
///
/// The height mirrors the renderer's column exactly: outer chrome, then each stacked child (title, the
/// Medium type badge, then one line per content line) plus the `row_gap` **between** them. Miss the badge or
/// the gaps and the last line clips - which is the whole point of computing it here rather than guessing.
pub fn footprint(size: Size, detail_lines: usize, panel_lines: usize) -> Pos {
    match size {
        Size::Small => Pos {
            x: SMALL_W,
            y: SMALL_H,
        },
        // Medium: title, type badge, then the detail lines - each child after the title preceded by a gap.
        Size::Medium => Pos {
            x: MEDIUM_W,
            y: V_CHROME
                + TITLE_H
                + ROW_GAP
                + BADGE_H
                + detail_lines as f32 * (BODY_LINE_H + ROW_GAP),
        },
        // Large: title then the panel lines (no badge), capped - the overflow scrolls in the renderer.
        Size::Large => Pos {
            x: LARGE_W,
            y: (V_CHROME + TITLE_H + panel_lines as f32 * (BODY_LINE_H + ROW_GAP)).min(LARGE_MAX_H),
        },
    }
}

/// A **deck chip's** footprint `(width, height)` in logical px: a Small card wearing a stack of `count`
/// offset layers. Each layer past the first steps the box `STACK_OFFSET` further along both axes, capped at
/// `MAX_STACK` layers, so the chip's box is the **union of its stacked cards** - the deck's drop target. An
/// empty deck (count 0) is a single Small card. Computed purely from the physical card count; no rendering.
pub fn chip_footprint(count: usize) -> Pos {
    let depth = count.clamp(1, MAX_STACK);
    let spread = (depth - 1) as f32 * STACK_OFFSET;
    Pos {
        x: SMALL_W + spread,
        y: SMALL_H + spread,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chip_grows_with_the_stack_then_caps() {
        // Empty / single-card decks are a bare Small card.
        assert_eq!(chip_footprint(0), Pos { x: 120.0, y: 96.0 });
        assert_eq!(chip_footprint(1), Pos { x: 120.0, y: 96.0 });
        // Each extra card steps the box out by STACK_OFFSET on both axes...
        assert_eq!(
            chip_footprint(4),
            Pos {
                x: 120.0 + 3.0 * STACK_OFFSET,
                y: 96.0 + 3.0 * STACK_OFFSET
            }
        );
        // ...until MAX_STACK, past which the chip stops growing.
        let capped = chip_footprint(MAX_STACK);
        assert_eq!(chip_footprint(MAX_STACK + 50), capped);
        assert_eq!(
            capped,
            Pos {
                x: 120.0 + (MAX_STACK - 1) as f32 * STACK_OFFSET,
                y: 96.0 + (MAX_STACK - 1) as f32 * STACK_OFFSET
            }
        );
    }

    #[test]
    fn small_is_a_fixed_box_regardless_of_content() {
        // A small card is name-only; its box never depends on detail/panel counts.
        assert_eq!(footprint(Size::Small, 0, 0), Pos { x: 120.0, y: 96.0 });
        assert_eq!(footprint(Size::Small, 9, 9), Pos { x: 120.0, y: 96.0 });
    }

    #[test]
    fn medium_grows_one_line_at_a_time_deterministically() {
        let zero = footprint(Size::Medium, 0, 0);
        let three = footprint(Size::Medium, 3, 0);
        assert_eq!(zero.x, 200.0, "medium width is fixed");
        assert_eq!(three.x, 200.0);
        assert_eq!(
            three.y - zero.y,
            3.0 * (BODY_LINE_H + ROW_GAP),
            "each detail line adds exactly one line plus its gap"
        );
    }

    #[test]
    fn medium_budgets_for_title_badge_and_gaps() {
        // A zero-detail Medium card is still tall enough for the title, its gap, and the type badge - so a
        // one-line card never clips its badge.
        assert_eq!(
            footprint(Size::Medium, 0, 0).y,
            V_CHROME + TITLE_H + ROW_GAP + BADGE_H
        );
    }

    #[test]
    fn large_caps_and_reads_panel_lines() {
        assert_eq!(footprint(Size::Large, 0, 2).x, 320.0);
        // A very long panel is capped (it scrolls in the renderer), never taller than the cap.
        assert_eq!(footprint(Size::Large, 0, 1000).y, LARGE_MAX_H);
    }
}
