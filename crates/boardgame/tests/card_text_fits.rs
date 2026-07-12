//! **The text-fit check, as part of the build.** Every card in the real opening board, drawn at all three
//! render sizes, must fit inside the footprint the model computed for it — no clipped text.
//!
//! Why this has to *render*: the model sizes a card from a **line count** times per-line constants
//! (`cardtable_model::layout`), which cannot know the font's true metrics. So the box is a prediction. This
//! test renders the cards headlessly and measures what the text actually needs, which turns "some card's text
//! is cut off" from something you notice by eye into something that fails the build. It catches both sides:
//! a content change (a longer stat line, one more detail row) *and* a constant change.
//!
//! When it fails, `cargo run -p cardtable --example card_gallery` shows the same audit visually — the
//! offending cards are framed in red.

use cardtable::audit_card_text;
use deckbound_board::sample_table;

#[test]
fn every_card_text_fits_its_footprint() {
    let overflows = audit_card_text(&sample_table());
    assert!(
        overflows.is_empty(),
        "{} card(s) have text that spills outside the box the model computed for them - it would be \
         clipped. Fix the content, or the per-line constants in `cardtable_model::layout`:\n{}",
        overflows.len(),
        overflows
            .iter()
            .map(|o| format!(
                "  [{:<6}] {:?}  +{:.0}px wide, +{:.0}px tall",
                o.size, o.card, o.over_x, o.over_y
            ))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
