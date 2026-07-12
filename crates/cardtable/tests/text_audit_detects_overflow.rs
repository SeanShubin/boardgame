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
