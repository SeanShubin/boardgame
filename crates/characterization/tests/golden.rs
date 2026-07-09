//! Behavioral golden-master tests for the **card-table world emitter** (`deckbound-cardtable`).
//!
//! Each test drives the emitter's `Game` (view / legal_actions / apply), inflates the view to a `Tableau`
//! via `from_table_view`, and asserts on a **behavioral projection** — the rendered zone tree with each
//! card's face + interactivity, but no geometry. Rendering is a pure function of the `Tableau`, so this
//! pins on-screen behavior without a GPU; determinism (everything flows from a seed) keeps it stable.
//! Regenerate the golden with `BLESS=1 cargo test -p characterization`.
//!
//! (The old-path witness — the `sample_table` / `cardtable-combat` goldens + the byte tier — was retired
//! in P3 when that path was deleted; it lives in git history.)

use cardtable_model::{Node, PileId, Tableau};
use std::path::PathBuf;

/// Compare `actual` against the committed behavioral golden `<name>.behavior.txt`, or (with `BLESS=1`)
/// rewrite it. Test infrastructure, not an ad-hoc script: the bless path only runs when the operator sets
/// `BLESS`; the default path is a pure comparison that fails loudly on drift or a missing golden.
fn assert_behavior(name: &str, actual: &str) {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("golden");
    path.push(format!("{name}.behavior.txt"));

    if std::env::var_os("BLESS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
        eprintln!("blessed golden: {}", path.display());
        return;
    }

    let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!("missing golden `{name}` at {} — run `BLESS=1 cargo test -p characterization` to create it", path.display())
    });
    // Normalize line endings: git may rewrite the committed golden to CRLF on checkout; the fresh string
    // is always LF. Behavior drift shows up as content differences, never as an EOL artifact.
    let norm = |s: &str| s.replace("\r\n", "\n");
    assert_eq!(
        norm(actual),
        norm(&expected),
        "golden `{name}` drifted (behavior changed)"
    );
}

/// Project a `Tableau` to its **rendered** form: the recursive zone tree (structure + order) with each
/// card's visible face and interactivity. Deliberately excludes everything the reunification path
/// legitimately reconstructs differently while preserving what the player sees and clicks:
///
/// - **geometry** (positions / sizes) — player-controlled drag state;
/// - **arrangement** (`layout`: Free / Grid / List / Rows) — presentation the renderer applies;
/// - **model mechanisms** (`projection`, `reflects`) — the emitter reimplements these as inline cards;
/// - **card-vs-sub-zone interleave** — `ZoneView` splits cards and sub-zones, so `from_table_view` emits
///   a pile's cards before its sub-piles; we canonicalize by rendering cards first, then sub-zones.
///
/// What remains — nesting, card/sub-zone order, card face (title/type/detail/panel/qty), `actionable`, and
/// pairings — is stable across construction paths. Deterministic: `children()` is an ordered `Vec`.
fn behavior(t: &Tableau) -> String {
    let mut out = String::new();
    render_pile(t, t.root_id(), 0, &mut out);
    out
}

fn render_pile(t: &Tableau, pid: PileId, depth: usize, out: &mut String) {
    let pile = t.pile(pid).unwrap();
    let indent = "  ".repeat(depth);
    out.push_str(&format!("{indent}[{}]\n", pile.label));
    // Cards first (in order)...
    for node in pile.children() {
        let Node::Card(cid) = node else { continue };
        let c = t.card(*cid).unwrap();
        let face = if c.is_face_down() {
            "«down»".to_string()
        } else {
            c.front_title().to_string()
        };
        let mut line = format!(
            "{indent}  - {face} | type={:?} | qty={}",
            c.card_type(),
            c.quantity()
        );
        if let Some(a) = c.actionable {
            line.push_str(&format!(" | act={a}"));
        }
        if !c.detail().is_empty() {
            line.push_str(&format!(" | detail={:?}", c.detail()));
        }
        if !c.panel().is_empty() {
            line.push_str(&format!(" | panel={:?}", c.panel()));
        }
        if let Some(k) = c.pair_key() {
            line.push_str(&format!(" | pair_key={k}"));
        }
        if !c.pairings().is_empty() {
            line.push_str(&format!(" | pairs={:?}", c.pairings()));
        }
        out.push_str(&line);
        out.push('\n');
    }
    // ...then nested sub-zones (in order).
    for node in pile.children() {
        if let Node::Pile(child) = node {
            render_pile(t, *child, depth + 1, out);
        }
    }
}

/// The emitter's opening world (new game, empty party). The emitter reproduces the shipped static world
/// *except the Inn*, which is the functional **equip view** (real hero/kit cards with pairings).
#[test]
fn emitter_world_view() {
    use cardtable_model::from_table_view;
    use contract::Game;

    let game = deckbound_cardtable::CardTableWorld;
    let view = game.view(&game.new_game(1, 1), None);
    assert_behavior("emitter_world", &behavior(&from_table_view(&view)));
}

/// Equip flows through the seam. The inn offers each un-recruited hero as a card that **pairs onto** a kit;
/// applying `Equip` recruits the character, which then shows in the inn and is no longer offered to equip.
#[test]
fn emitter_equip_recruits_a_character() {
    use cardtable_model::from_table_view;
    use contract::Game;

    let game = deckbound_cardtable::CardTableWorld;
    let mut world = game.new_game(1, 1);

    // "Offered to equip" = the inn's un-recruited hero card, distinguished from the Heroes bank copy by
    // its pairings (bank heroes carry none).
    let offered = |w: &deckbound_cardtable::World| {
        behavior(&from_table_view(&game.view(w, None)))
            .contains("Vael Thornbrand | type=\"hero\" | qty=1 | pairs=")
    };

    assert!(
        offered(&world),
        "the inn offers Vael to equip before recruiting"
    );

    game.apply(
        &mut world,
        &deckbound_cardtable::Action::Equip { hero: 0, kit: 2 },
    )
    .expect("equip is legal");

    let after = behavior(&from_table_view(&game.view(&world, None)));
    assert!(
        after.contains("Vael Thornbrand · Marksman | type=\"character\""),
        "the recruited character shows in the inn"
    );
    assert!(!offered(&world), "the inn no longer offers Vael to equip");

    assert!(
        game.apply(
            &mut world,
            &deckbound_cardtable::Action::Equip { hero: 0, kit: 1 }
        )
        .is_err(),
        "re-equipping a recruited hero is rejected"
    );
}

/// March flows through the seam. A recruited character (stationed at the inn) can march to any other
/// location — a pairing onto the destination Location card; `apply(March)` re-stations it. Verified via
/// `legal_actions` (robust to view nesting).
#[test]
fn emitter_march_moves_a_character() {
    use cardtable_model::from_table_view;
    use contract::Game;
    use deckbound_cardtable::Action;

    let game = deckbound_cardtable::CardTableWorld;
    let mut world = game.new_game(1, 1);
    game.apply(&mut world, &Action::Equip { hero: 0, kit: 2 })
        .unwrap();

    // Recruited at the inn (Ashfen = idx 4): can march elsewhere, not to where it already is.
    let acts = game.legal_actions(&world);
    assert!(
        acts.iter().any(|a| matches!(
            a,
            Action::March {
                character: 0,
                location: 1
            }
        )),
        "can march to Cinderwatch from the inn"
    );
    assert!(
        !acts.iter().any(|a| matches!(
            a,
            Action::March {
                character: 0,
                location: 4
            }
        )),
        "can't march to the inn it already occupies"
    );

    // March to Cinderwatch Keep (idx 1).
    game.apply(
        &mut world,
        &Action::March {
            character: 0,
            location: 1,
        },
    )
    .unwrap();
    let acts = game.legal_actions(&world);
    assert!(
        !acts.iter().any(|a| matches!(
            a,
            Action::March {
                character: 0,
                location: 1
            }
        )),
        "no longer marchable to its current location"
    );
    assert!(
        acts.iter().any(|a| matches!(
            a,
            Action::March {
                character: 0,
                location: 4
            }
        )),
        "can march back to the inn"
    );

    // Exactly one character in the world; a bad character index errors.
    let proj = behavior(&from_table_view(&game.view(&world, None)));
    assert_eq!(
        proj.matches("type=\"character\"").count(),
        1,
        "exactly one character in the world"
    );
    assert!(
        game.apply(
            &mut world,
            &Action::March {
                character: 9,
                location: 0
            }
        )
        .is_err(),
        "marching a non-existent character errors"
    );
}

/// The Fight action auto-resolves through the seam. A character stationed where an encounter waits pairs
/// onto it to fight; `apply(Fight)` resolves via deckbound (outcome-parity), and on a win the encounter
/// clears — its card is gone, a Victory combat-log is left, and it can't be fought again.
#[test]
fn emitter_fight_clears_an_encounter() {
    use cardtable_model::from_table_view;
    use contract::Game;
    use deckbound_cardtable::Action;

    // Seed 7: Marksman clears Cinderwatch Keep (the Coil).
    let game = deckbound_cardtable::CardTableWorld;
    let mut world = game.new_game(7, 1);
    game.apply(&mut world, &Action::Equip { hero: 0, kit: 2 })
        .unwrap();
    game.apply(
        &mut world,
        &Action::March {
            character: 0,
            location: 1,
        },
    )
    .unwrap();

    let before = behavior(&from_table_view(&game.view(&world, None)));
    assert!(
        before.contains("The Coiled Sentry | type=\"encounter\""),
        "the encounter is present before the fight"
    );
    assert!(
        game.legal_actions(&world)
            .iter()
            .any(|a| matches!(a, Action::Fight { character: 0 })),
        "the character is offered a fight"
    );

    game.apply(&mut world, &Action::Fight { character: 0 })
        .expect("the fight resolves");

    let after = behavior(&from_table_view(&game.view(&world, None)));
    assert!(
        !after.contains("The Coiled Sentry | type=\"encounter\""),
        "the cleared encounter card is gone"
    );
    assert!(
        after.contains("Victory | type=\"log\""),
        "a Victory combat-log is left"
    );
    assert!(
        !game
            .legal_actions(&world)
            .iter()
            .any(|a| matches!(a, Action::Fight { character: 0 })),
        "no more fight offered once cleared"
    );
    assert!(
        game.apply(&mut world, &Action::Fight { character: 0 })
            .is_err(),
        "fighting a cleared encounter errors"
    );
}

/// The interactive arena drives deckbound's resumable battle. Opening it takes over the felt (the view
/// becomes the Arena — combatants with Health + the running log); stepping advances the fight; when it
/// ends it folds back to the world (Victory log + cleared encounter).
#[test]
fn emitter_arena_plays_a_fight_to_completion() {
    use cardtable_model::from_table_view;
    use contract::Game;
    use deckbound_cardtable::Action;

    let game = deckbound_cardtable::CardTableWorld;
    let mut world = game.new_game(7, 1);
    game.apply(&mut world, &Action::Equip { hero: 0, kit: 2 })
        .unwrap();
    game.apply(
        &mut world,
        &Action::March {
            character: 0,
            location: 1,
        },
    )
    .unwrap();

    // Open the arena at Cinderwatch (the Coil).
    game.apply(&mut world, &Action::Arena { character: 0 })
        .expect("the arena opens");
    let arena = behavior(&from_table_view(&game.view(&world, None)));
    assert!(arena.contains("[Arena]"), "the arena takes over the felt");
    assert!(arena.contains("The Coil"), "the foe is shown in the arena");

    // Step to completion — while fighting, StepArena is the only legal move.
    let mut guard = 0;
    while game
        .legal_actions(&world)
        .iter()
        .any(|a| matches!(a, Action::StepArena))
    {
        game.apply(&mut world, &Action::StepArena)
            .expect("the arena steps");
        guard += 1;
        assert!(guard < 1000, "the fight must terminate");
    }

    // Folded back: Marksman won, arena closed, encounter cleared, a Victory log left.
    let after = behavior(&from_table_view(&game.view(&world, None)));
    assert!(
        !after.contains("[Arena]"),
        "the arena closes when the fight ends"
    );
    assert!(
        after.contains("Victory | type=\"log\""),
        "the win folds back as a Victory log"
    );
    assert!(
        !after.contains("The Coiled Sentry | type=\"encounter\""),
        "the cleared encounter is gone"
    );
}

/// Per-blow **player choices** drive the fight. The arena renders the current hero decision as answerable
/// `choice` cards (each a clickable action); playing via those explicit answers (never the greedy
/// StepArena) carries the fight to completion and folds a combat-log back.
#[test]
fn emitter_arena_player_choices_play_a_fight() {
    use cardtable_model::from_table_view;
    use contract::Game;
    use deckbound_cardtable::Action;

    let game = deckbound_cardtable::CardTableWorld;
    let mut world = game.new_game(7, 1);
    game.apply(&mut world, &Action::Equip { hero: 0, kit: 2 })
        .unwrap();
    game.apply(
        &mut world,
        &Action::March {
            character: 0,
            location: 1,
        },
    )
    .unwrap();
    game.apply(&mut world, &Action::Arena { character: 0 })
        .unwrap();

    // The arena shows the decision and clickable choice cards bound to legal actions.
    let arena = behavior(&from_table_view(&game.view(&world, None)));
    assert!(
        arena.contains("type=\"decision\""),
        "a hero decision is shown"
    );
    assert!(
        arena.contains("- Strike ") && arena.contains("type=\"choice\" | qty=1 | act="),
        "answerable, clickable choices are shown"
    );

    // A "player" that always takes the first explicit answer (never StepArena) — drives via real choices.
    let mut guard = 0;
    while game
        .legal_actions(&world)
        .iter()
        .any(|a| matches!(a, Action::StepArena))
    {
        let acts = game.legal_actions(&world);
        let choice = acts
            .iter()
            .find(|a| !matches!(a, Action::StepArena))
            .expect("a hero decision always offers at least one answer while the arena is up")
            .clone();
        game.apply(&mut world, &choice).expect("the choice applies");
        guard += 1;
        assert!(guard < 2000, "the player-driven fight must terminate");
    }

    // The fight completed through explicit choices; a combat-log folded back (win or loss).
    let after = behavior(&from_table_view(&game.view(&world, None)));
    assert!(
        !after.contains("[Arena]"),
        "the arena closes when the player-driven fight ends"
    );
    assert!(
        after.contains("type=\"log\""),
        "a combat-log is folded back to the world"
    );
}
