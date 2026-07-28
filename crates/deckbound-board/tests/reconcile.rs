//! A software update meets a live session: reconcile a save written by "old code" against the pristine
//! table the current build constructs. The player's arrangement and progress survive; every card's
//! content is the current build's. This is the gate for the load path `boardgame::persistence` runs.

use cardtable_model::{Board, CardKind, Face, PileId, reconcile};
use deckbound_board::sample_table;

/// The root subpile labeled `label`.
fn root_pile(b: &Board, label: &str) -> PileId {
    b.pile(b.root_id())
        .expect("root")
        .subpiles()
        .into_iter()
        .find(|&p| b.pile(p).is_some_and(|q| q.label == label))
        .unwrap_or_else(|| panic!("no root pile labeled {label}"))
}

/// The subpile of `parent` labeled `label`.
fn sub_pile(b: &Board, parent: PileId, label: &str) -> PileId {
    b.pile(parent)
        .expect("parent")
        .subpiles()
        .into_iter()
        .find(|&p| b.pile(p).is_some_and(|q| q.label == label))
        .unwrap_or_else(|| panic!("no sub-pile labeled {label}"))
}

#[test]
fn a_session_survives_an_update_with_fresh_rules_text() {
    // The session, as saved by the "old build".
    let mut saved = sample_table();
    let rules = root_pile(&saved, "Rules");
    let phases = sub_pile(&saved, rules, "Phases");

    // The player parked a phase card out on the table; the old build had written different text on it.
    let phase_card = *saved
        .content_cards(phases)
        .first()
        .expect("the Phases deck has cards");
    let phase_title = saved.card(phase_card).unwrap().name().to_string();
    saved
        .set_card_detail(phase_card, vec!["OLD RULES TEXT".into()])
        .unwrap();
    let root = saved.root_id();
    let at = saved.pile(root).unwrap().children().len();
    saved.move_card(phase_card, root, at).unwrap();

    // The old build also carried a rules card the update has since retired.
    let ghost = saved
        .add_card(
            rules,
            Face::Up {
                title: "Retired Rule".into(),
            },
            None,
        )
        .unwrap();
    saved.set_card_type(ghost, "rule").unwrap();

    // A day passed...
    let progress = root_pile(&saved, "Progress");
    let events = root_pile(&saved, "Events");
    saved.advance_day(progress, events).unwrap();
    assert_eq!(saved.current_day(progress), 1);

    // ...and a battle was won at Ashfen Crossing, leaving its record (as `arena::record_outcome` does).
    let ashfen = sub_pile(&saved, root_pile(&saved, "Locations"), "Ashfen Crossing");
    let victory = saved.add_pile(ashfen, "Victory").unwrap();
    let log = saved
        .add_card(
            victory,
            Face::Up {
                title: "Round 1".into(),
            },
            None,
        )
        .unwrap();
    saved.set_card_kind(log, CardKind::Virtual).unwrap();
    saved.set_card_type(log, "log").unwrap();
    saved
        .set_card_panel(log, vec!["Raider fells the Husk.".into()])
        .unwrap();

    // The update loads the save: reconcile it against the table the CURRENT code builds.
    let out = reconcile(sample_table(), &saved);

    // The parked phase card is still parked at the root, but says what the current build says.
    let fresh = sample_table();
    let fresh_detail = {
        let phases = sub_pile(&fresh, root_pile(&fresh, "Rules"), "Phases");
        let card = fresh
            .content_cards(phases)
            .into_iter()
            .find(|&c| fresh.card(c).unwrap().name() == phase_title)
            .expect("the phase card exists in the current build");
        fresh.card(card).unwrap().detail().to_vec()
    };
    let parked = out
        .pile(out.root_id())
        .unwrap()
        .cards()
        .into_iter()
        .find(|&c| out.card(c).unwrap().name() == phase_title)
        .expect("the parked phase card is still at the root");
    assert_eq!(
        out.card(parked).unwrap().detail(),
        fresh_detail.as_slice(),
        "the parked card carries the current build's text, not the save's"
    );

    // The retired card is gone, wherever it sat.
    let all_cards: Vec<_> = {
        let mut piles = vec![out.root_id()];
        let mut cards = Vec::new();
        while let Some(p) = piles.pop() {
            let pile = out.pile(p).unwrap();
            cards.extend(pile.cards());
            piles.extend(pile.subpiles());
        }
        cards
    };
    assert!(
        all_cards
            .iter()
            .all(|&c| out.card(c).unwrap().front_title() != "Retired Rule"),
        "a card the update no longer provisions cannot come back"
    );

    // The day survived: the clock still reads day 1.
    assert_eq!(out.current_day(root_pile(&out, "Progress")), 1);

    // The Victory record survived verbatim.
    let ashfen = sub_pile(&out, root_pile(&out, "Locations"), "Ashfen Crossing");
    let victory = sub_pile(&out, ashfen, "Victory");
    let log = *out
        .pile(victory)
        .unwrap()
        .cards()
        .first()
        .expect("the record holds its round log");
    assert_eq!(out.card(log).unwrap().kind(), CardKind::Virtual);
    assert_eq!(
        out.card(log).unwrap().panel(),
        ["Raider fells the Husk.".to_string()]
    );

    // Conservation: the physical total is exactly what the current build provisions.
    assert_eq!(
        out.physical_card_count(out.root_id()),
        sample_table().physical_card_count(sample_table().root_id()),
        "reconcile neither mints nor destroys physical cards"
    );
}
