//! The renderer's **character palette**: the exact set of characters the UI is allowed to draw, and which the
//! font deck displays. It is printable ASCII plus a small, curated set of typographic symbols (on-theme for a
//! card game). This module is the single source of truth for that set - the one place non-ASCII glyphs are
//! declared - and the `charset` guard test forbids any non-ASCII in app code that is not listed here.
//!
//! Two tests keep "available" honest: [`tests::fonts_cover_palette`] asserts every character here is actually
//! in the bundled fonts' cmaps (so the font deck never shows a glyph the font can't render), and the
//! `tests/charset.rs` guard asserts app code only ever uses characters from this palette.

/// The curated non-ASCII symbols the UI may use. Everything else must be printable ASCII. Adding a symbol
/// here makes it usable *and* makes it appear in the font deck; the coverage test then requires the fonts to
/// have it. Keep grouped by kind for the deck's grid.
pub const SYMBOLS: &[char] = &[
    // arrows
    '\u{2190}', '\u{2191}', '\u{2192}', '\u{2193}', // left up right down
    // card suits
    '\u{2660}', '\u{2665}', '\u{2666}', '\u{2663}', // spade heart diamond club
    // dashes
    '\u{2013}', '\u{2014}', // en em
    // math / relational
    '\u{00D7}', '\u{00F7}', '\u{00B1}', '\u{2264}', '\u{2265}', '\u{2260}', // x / +- <= >= !=
    // dots / bullets
    '\u{00B7}', '\u{2022}', '\u{25CF}',
    '\u{25CB}', // middle-dot bullet filled-circle open-circle
    // marks
    '\u{2713}', '\u{2717}', '\u{2605}', // check cross star
    // misc
    '\u{2026}', '\u{00B0}', // ellipsis degree
];

// Named symbols for use in UI strings. Referencing these (or `\u{...}` escapes) keeps source ASCII - the
// font renders the real glyph - so the "no raw non-ASCII in code" guard stays intact. All are in [`SYMBOLS`],
// so the coverage test guarantees the bundled fonts can draw them.
/// Rightwards arrow, for "a to b" (instead of `->`).
pub const ARROW: char = '\u{2192}';
/// Em dash, for a parenthetical break (instead of ` - `).
pub const MDASH: char = '\u{2014}';
/// Middle dot, a compact field separator (instead of ` | `).
pub const MIDDOT: char = '\u{00B7}';
/// Multiplication sign, for counts like "x3" (instead of `x`).
pub const TIMES: char = '\u{00D7}';
/// Bullet, for a marker/flag (instead of `*`).
pub const BULLET: char = '\u{2022}';

/// The lowest printable ASCII byte (space).
pub const ASCII_LO: u8 = 0x20;
/// The highest printable ASCII byte (`~`).
pub const ASCII_HI: u8 = 0x7E;

/// Every character the UI may draw: printable ASCII (space..=`~`) followed by the curated [`SYMBOLS`].
pub fn available() -> Vec<char> {
    (ASCII_LO..=ASCII_HI)
        .map(|b| b as char)
        .chain(SYMBOLS.iter().copied())
        .collect()
}

/// Whether `c` is in the palette (printable ASCII or a curated symbol) - the allow-list the guard enforces.
pub fn contains(c: char) -> bool {
    (c.is_ascii() && (c as u32) >= ASCII_LO as u32 && (c as u32) <= ASCII_HI as u32)
        || SYMBOLS.contains(&c)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every palette character is actually in the bundled fonts' cmaps (read directly, no rendering). This is
    /// the "the list matches reality" check: the font deck can only offer glyphs the fonts can draw.
    #[test]
    fn fonts_cover_palette() {
        for (name, bytes) in [
            ("DejaVu Sans", crate::DEJAVU_SANS),
            ("DejaVu Sans Mono", crate::DEJAVU_SANS_MONO),
        ] {
            let face = ttf_parser::Face::parse(bytes, 0).expect("bundled font parses");
            let missing: Vec<String> = available()
                .into_iter()
                .filter(|&c| !c.is_whitespace() && face.glyph_index(c).is_none())
                .map(|c| format!("U+{:04X} {c}", c as u32))
                .collect();
            assert!(
                missing.is_empty(),
                "{name} is missing palette glyphs (drop them from palette::SYMBOLS or change the font):\n{}",
                missing.join("\n")
            );
        }
    }
}
