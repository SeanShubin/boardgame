//! **The text-fit guard must actually guard.** `audit_card_text` renders cards headlessly and reports the
//! ones whose text spills outside the box the model computed. If that harness ever silently measured
//! *nothing* — no camera, no font, no layout — it would cheerfully report "no overflows" and the build would
//! stay green while clipped text shipped. That failure mode is invisible, so it gets its own test: give the
//! audit a card whose title cannot possibly fit, and it must say so.

use cardtable::audit_card_text;
use cardtable_model::{Board, Face};

#[test]
fn the_audit_detects_text_that_cannot_fit() {
    let mut board = Board::new();
    let root = board.root_id();
    let deck = board.add_pile(root, "Samples").expect("root exists");
    let id = board
        .add_card(
            deck,
            Face::Up {
                title: "W".repeat(120), // no card is 120 wide characters
            },
            None,
        )
        .expect("deck exists");
    board.set_card_type(id, "hero").expect("card just added");

    let overflows = audit_card_text(&board);

    assert!(
        !overflows.is_empty(),
        "a 120-character title cannot fit on a card, so the audit must flag it. Reporting no overflow \
         means the harness measured nothing - and every other text-fit check is worthless."
    );
}

/// **A clipped detail LINE must be flagged too.** A NoWrap detail line wider than the card is sized to the
/// card and its glyphs are clipped - the node itself never spills past the box, so a spill-only check misses
/// it entirely. That is exactly the bug that shipped truncated Rules-card text while the audit stayed green.
/// The natural-width check must catch it.
#[test]
fn the_audit_detects_a_detail_line_that_clips_without_spilling() {
    let mut board = Board::new();
    let root = board.root_id();
    let deck = board.add_pile(root, "Samples").expect("root exists");
    let id = board
        .add_card(
            deck,
            Face::Up {
                title: "Short".into(), // the title fits; only the detail line overflows
            },
            None,
        )
        .expect("deck exists");
    board.set_card_type(id, "hero").expect("card just added");
    board
        .set_card_detail(id, vec!["W".repeat(120)]) // one detail line far wider than any card
        .expect("card just added");

    let overflows = audit_card_text(&board);

    assert!(
        overflows
            .iter()
            .any(|o| o.card == "Short" && o.over_x > 1.0),
        "a 120-character detail line is clipped by the card (it never spills), so only the natural-width \
         check can catch it - and it must: {overflows:?}"
    );
}
