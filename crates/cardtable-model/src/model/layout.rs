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

/// Vertical chrome: padding (top+bottom) plus border (top+bottom) around a card's content.
const V_CHROME: f32 = 2.0 * 10.0 + 2.0 * 2.0;
/// The name/header strip above the content lines.
const HEADER_H: f32 = 24.0;
/// One content line (a detail or panel line), drawn no-wrap so it is exactly one line tall.
const LINE_H: f32 = 18.0;

/// A card's on-felt footprint `(width, height)` in logical px, from its size and content line counts. The
/// renderer draws the card at exactly this size (content clips to fit), so this is authoritative for layout.
pub fn footprint(size: Size, detail_lines: usize, panel_lines: usize) -> Pos {
    match size {
        Size::Small => Pos {
            x: SMALL_W,
            y: SMALL_H,
        },
        Size::Medium => Pos {
            x: MEDIUM_W,
            y: V_CHROME + HEADER_H + detail_lines as f32 * LINE_H,
        },
        Size::Large => Pos {
            x: LARGE_W,
            y: (V_CHROME + HEADER_H + panel_lines as f32 * LINE_H).min(LARGE_MAX_H),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            3.0 * LINE_H,
            "each detail line adds exactly one line of height"
        );
    }

    #[test]
    fn large_caps_and_reads_panel_lines() {
        assert_eq!(footprint(Size::Large, 0, 2).x, 320.0);
        // A very long panel is capped (it scrolls in the renderer), never taller than the cap.
        assert_eq!(footprint(Size::Large, 0, 1000).y, LARGE_MAX_H);
    }
}
