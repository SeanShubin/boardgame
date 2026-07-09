//! Golden-master (characterization) tests that pin CURRENT behavior at the pure-state surfaces.
//!
//! Rendering is a pure function of the `Tableau`, so pinning the `Tableau` (and, later, combat
//! outcomes) pins on-screen behavior without a GPU. Determinism (everything flows from a seed) makes
//! the goldens stable. Regenerate deliberately with `BLESS=1 cargo test -p characterization`; without
//! `BLESS` the tests compare against the committed goldens and fail on any drift.

use cardtable_combat::{begin_manual_combat, resolve_encounter};
use cardtable_model::{Node, PileId, Recipe, Tableau, sample_table};
use std::path::PathBuf;

/// **Byte tier** — the full internal `Tableau` RON. Frozen for phases that preserve the construction
/// path (P3 purge, P4/P5 moves, P6 rename); see plan §12.
fn assert_golden(name: &str, actual: &str) {
    assert_golden_file(&format!("{name}.ron"), actual);
}

/// **Behavioral tier** — the rendered projection (structure + content + interactivity, no geometry).
/// Stable across construction-path changes, so it is the acceptance criterion for the reunification
/// phases (P1/P2); see plan §12.
fn assert_behavior(name: &str, actual: &str) {
    assert_golden_file(&format!("{name}.behavior.txt"), actual);
}

/// Compare `actual` against the committed golden file, or (with `BLESS=1`) rewrite it.
///
/// This is test infrastructure, not an ad-hoc script: the bless path only runs when the operator
/// sets `BLESS`, and the default path is a pure comparison that fails loudly on drift or a missing
/// golden. Goldens live beside this crate under `golden/` and are checked into the repo.
fn assert_golden_file(filename: &str, actual: &str) {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("golden");
    path.push(filename);

    if std::env::var_os("BLESS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
        eprintln!("blessed golden: {}", path.display());
        return;
    }

    let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!("missing golden `{filename}` at {} — run `BLESS=1 cargo test -p characterization` to create it", path.display())
    });
    // Normalize line endings on both sides: git may rewrite the committed goldens to CRLF on
    // checkout (Windows/autocrlf), while the freshly-serialized string is always LF. Behavior
    // drift shows up as content differences, never as an EOL artifact.
    let norm = |s: &str| s.replace("\r\n", "\n");
    assert_eq!(
        norm(actual),
        norm(&expected),
        "golden `{filename}` drifted (behavior changed)"
    );
}

/// Project a `Tableau` to its **rendered** form: the recursive zone tree (structure + order) with each
/// card's visible face and interactivity. Deliberately excludes everything the reunification path
/// legitimately reconstructs differently while preserving what the player sees and clicks:
///
/// - **geometry** (positions / sizes) — player-controlled drag state;
/// - **arrangement** (`layout`: Free / Grid / List / Rows) — presentation the renderer applies, which
///   `from_table_view` defaults rather than authoring;
/// - **model mechanisms** (`projection`, `reflects`) — the Inn's projection and character-deck links are
///   cardtable-model tricks the reunified emitter reimplements (inline cards; deckbound-internal state);
/// - **card-vs-sub-zone interleave** — the seam's `ZoneView` holds cards and sub-zones in *separate*
///   lists, so `from_table_view` always emits a pile's cards before its sub-piles. We therefore render
///   cards first, then sub-zones (card order and sub-zone order each preserved), canonicalizing an
///   interleave the split-seam construction path can't reproduce.
///
/// What remains — nesting, card/sub-zone order, card face (title/type/detail/panel/qty), and
/// `actionable` — is stable across construction paths, which is the point of the behavioral tier
/// (plan §12). Deterministic by construction: `children()` is an ordered `Vec`.
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

/// Serialize a value to a **canonical** pretty RON snapshot.
///
/// `Tableau` stores `piles`/`cards` in `HashMap`s, whose iteration order is randomized per process,
/// so the raw serde output is non-deterministic across runs. We sort every map by key to get a
/// stable, drift-sensitive snapshot — any real value change still shows, but random map ordering
/// no longer does. This canonicalizes the *witness*, not the model: the product's on-disk save
/// format stays exactly as it is (a separate, out-of-scope observation).
fn to_golden_ron<T: serde::Serialize>(value: &T) -> String {
    let raw = ron::ser::to_string_pretty(value, ron::ser::PrettyConfig::new()).unwrap();
    let parsed: ron::Value = ron::from_str(&raw).expect("re-parse own RON as ron::Value");
    let canon = canonicalize(parsed);
    ron::ser::to_string_pretty(&canon, ron::ser::PrettyConfig::new()).unwrap()
}

/// Recursively sort every map by the serialized form of its key. Sequences (order is meaningful)
/// and leaves pass through unchanged.
fn canonicalize(value: ron::Value) -> ron::Value {
    use ron::Value;
    match value {
        Value::Map(map) => {
            let mut entries: Vec<(Value, Value)> =
                map.into_iter().map(|(k, v)| (k, canonicalize(v))).collect();
            entries.sort_by(|(a, _), (b, _)| {
                ron::to_string(a).unwrap().cmp(&ron::to_string(b).unwrap())
            });
            let mut sorted = ron::Map::new();
            for (k, v) in entries {
                sorted.insert(k, v);
            }
            Value::Map(sorted)
        }
        Value::Seq(items) => Value::Seq(items.into_iter().map(canonicalize).collect()),
        Value::Option(Some(inner)) => Value::Option(Some(Box::new(canonicalize(*inner)))),
        leaf => leaf,
    }
}

/// The opening world the product ships. Pins the entire starting `Tableau`.
#[test]
fn golden_sample_table() {
    let table = sample_table();
    assert_golden("sample_table", &to_golden_ron(&table));
    assert_behavior("sample_table", &behavior(&table));
}

// --- world navigation + fight setup (public-API replicas of cardtable-combat's test helpers) ------

/// A top-level deck by label.
fn top(t: &Tableau, label: &str) -> PileId {
    let root = t.root_id();
    t.pile(root)
        .unwrap()
        .subpiles()
        .into_iter()
        .find(|&p| t.pile(p).unwrap().label == label)
        .unwrap_or_else(|| panic!("top-level deck `{label}`"))
}

/// A place by label inside the Locations grid.
fn place(t: &Tableau, label: &str) -> PileId {
    let locations = top(t, "Locations");
    t.pile(locations)
        .unwrap()
        .subpiles()
        .into_iter()
        .find(|&p| t.pile(p).unwrap().label == label)
        .unwrap_or_else(|| panic!("location `{label}`"))
}

fn marksman() -> Recipe {
    Recipe {
        stats: [4, 4, 1, 2, 2],
        ability: "Stand-Off".into(),
    }
}
fn executioner() -> Recipe {
    Recipe {
        stats: [6, 3, 1, 1, 1],
        ability: "Alpha Strike".into(),
    }
}

/// Equip hero #0 at the inn with `recipe`, then march its position copy to `dest`. Mirrors
/// `cardtable-combat`'s private `station_at`, using only the public model API.
fn station_at(t: &mut Tableau, recipe: Recipe, dest: &str) -> PileId {
    let heroes = top(t, "Heroes");
    let stats = top(t, "Stats");
    let numbers = top(t, "Numbers");
    let abilities = top(t, "Abilities");
    let progress = top(t, "Progress");
    let ashfen = place(t, "Ashfen Crossing");
    let name = t
        .card(t.content_cards(heroes)[0])
        .unwrap()
        .name()
        .to_string();
    t.equip_character(
        &name, &recipe, heroes, stats, numbers, abilities, ashfen, progress,
    )
    .unwrap();
    let position = t
        .content_cards(ashfen)
        .into_iter()
        .find(|&c| {
            let k = t.card(c).unwrap();
            k.card_type() == "hero" && k.front_title() == name
        })
        .expect("the stationed hero's position copy");
    let dst = place(t, dest);
    t.move_character(position, dst, progress).unwrap();
    dst
}

/// Auto-combat: outcome + resulting `Tableau` (which includes the virtual combat-log narration).
fn auto_golden(name: &str, recipe: Recipe, dest: &str, seed: u64) {
    let mut t = sample_table();
    let place = station_at(&mut t, recipe, dest);
    let outcome = resolve_encounter(&mut t, place, seed);
    assert_golden(
        name,
        &format!("outcome: {outcome:?}\n---\n{}", to_golden_ron(&t)),
    );
    assert_behavior(
        name,
        &format!("outcome: {outcome:?}\n---\n{}", behavior(&t)),
    );
}

/// Manual-combat: the greedy-driven mutation stream + resulting `Tableau` (arena + folded state).
fn manual_golden(name: &str, recipe: Recipe, dest: &str, seed: u64) {
    let mut t = sample_table();
    let place = station_at(&mut t, recipe, dest);
    let bestiary = top(&t, "Bestiary");
    let root = t.root_id();
    let arena = t.add_pile(root, "Arena").unwrap();
    let mut combat =
        begin_manual_combat(&mut t, place, arena, bestiary, seed).expect("a fight begins");
    let mut muts = Vec::new();
    let outcome = combat.run_to_end_auto(|m| muts.extend_from_slice(m));
    let mut head = format!("outcome: {outcome:?}\nmutations:\n");
    for m in &muts {
        head.push_str(&format!("  {m:?}\n"));
    }
    head.push_str("---\n");
    assert_golden(name, &format!("{head}{}", to_golden_ron(&t)));
    assert_behavior(name, &format!("{head}{}", behavior(&t)));
}

/// The answering kit clears the Coil at two seeds — pins the win outcome + log narration + folded table.
#[test]
fn golden_auto_marksman_cinderwatch_seed1() {
    auto_golden(
        "auto_marksman_cinderwatch_seed1",
        marksman(),
        "Cinderwatch Keep",
        1,
    );
}
#[test]
fn golden_auto_marksman_cinderwatch_seed7() {
    auto_golden(
        "auto_marksman_cinderwatch_seed7",
        marksman(),
        "Cinderwatch Keep",
        7,
    );
}
/// The wrong kit loses — foes remain, "Defeat" logged. Pins the loss path.
#[test]
fn golden_auto_executioner_cinderwatch_seed1() {
    auto_golden(
        "auto_executioner_cinderwatch_seed1",
        executioner(),
        "Cinderwatch Keep",
        1,
    );
}
/// Manual greedy path: instantiates real foe cards, drives the resumable resolver, folds back.
#[test]
fn golden_manual_marksman_cinderwatch_seed7() {
    manual_golden(
        "manual_marksman_cinderwatch_seed7",
        marksman(),
        "Cinderwatch Keep",
        7,
    );
}

/// A scripted interaction transcript over the pure model: drill into zones, cycle a card's size,
/// select it, drill back out — snapshotting the `Tableau` after each step.
#[test]
fn golden_interaction_transcript() {
    let mut t = sample_table();
    let mut log = String::new();
    let mut blog = String::new();
    let step = |t: &Tableau, label: &str, log: &mut String, blog: &mut String| {
        log.push_str(&format!("== {label} ==\n{}\n", to_golden_ron(t)));
        blog.push_str(&format!("== {label} ==\n{}\n", behavior(t)));
    };

    let locations = top(&t, "Locations");
    t.focus(locations).unwrap();
    step(&t, "focus Locations", &mut log, &mut blog);

    let ashfen = place(&t, "Ashfen Crossing");
    t.focus(ashfen).unwrap();
    step(&t, "focus Ashfen", &mut log, &mut blog);

    if let Some(&card) = t.content_cards(ashfen).first() {
        t.cycle_card_size(card).unwrap();
        step(&t, "cycle first card size", &mut log, &mut blog);
        t.select(card).unwrap();
        step(&t, "select first card", &mut log, &mut blog);
    }

    let root = t.root_id();
    t.focus(root).unwrap();
    step(&t, "focus root", &mut log, &mut blog);

    assert_golden("interaction_transcript", &log);
    assert_behavior("interaction_transcript", &blog);
}

/// The emitter's opening world (new game, empty party) — a fresh golden, not `sample_table`. The emitter
/// reproduces the shipped static world *except the Inn*, which is now the functional **equip view** (real
/// hero/kit cards with pairings) rather than `sample_table`'s projection stub. `sample_table.behavior.txt`
/// stays as the old-path witness until P2.4 deletes that path.
///
/// (Earlier this asserted the emitter's full view equalled `sample_table` verbatim — proven through P2.2b,
/// in git history; the emitter now intentionally diverges at the Inn as it becomes interactive.)
#[test]
fn emitter_world_view() {
    use cardtable_model::from_table_view;
    use contract::Game;

    let game = deckbound_cardtable::CardTableWorld;
    let view = game.view(&game.new_game(1, 1), None);
    assert_behavior("emitter_world", &behavior(&from_table_view(&view)));
}

/// P2.3.1d: equip flows through the seam. The inn offers each un-recruited hero as a card that **pairs
/// onto** a kit; applying the `Equip` action recruits the character, which then shows in the inn and is no
/// longer offered to equip. Re-equipping a recruited hero is rejected.
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

/// P2.3.0: the emitter's fight resolution matches the old `cardtable-combat` path's outcome for the same
/// kit + location + seed. Both delegate to deckbound's deterministic resolver, so outcome-parity holds by
/// construction — this pins it. (Outcome-parity is the P2.3 acceptance criterion; the arena presentation
/// is authored fresh on top of this and blessed separately.)
#[test]
fn emitter_fight_outcomes_match_the_old_path() {
    let cases = [
        ("Marksman", marksman(), "Cinderwatch Keep", 1u64),
        ("Marksman", marksman(), "Cinderwatch Keep", 7),
        ("Executioner", executioner(), "Cinderwatch Keep", 1),
    ];
    for (kit, recipe, location, seed) in cases {
        // Old path: station the hero on the sample table, then resolve the encounter.
        let mut t = sample_table();
        let place = station_at(&mut t, recipe, location);
        let old = resolve_encounter(&mut t, place, seed);

        // New path: the emitter resolves the kit vs the location's encounter directly.
        let (won, _log) = deckbound_cardtable::resolve_fight(kit, location, seed)
            .expect("the location has an encounter");
        let new = if won {
            cardtable_combat::CombatOutcome::Win
        } else {
            cardtable_combat::CombatOutcome::Loss
        };

        assert_eq!(
            old, new,
            "outcome parity for {kit} @ {location} seed {seed}"
        );
    }
}
