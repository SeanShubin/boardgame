//! Golden-master (characterization) tests that pin CURRENT behavior at the pure-state surfaces.
//!
//! Rendering is a pure function of the `Tableau`, so pinning the `Tableau` (and, later, combat
//! outcomes) pins on-screen behavior without a GPU. Determinism (everything flows from a seed) makes
//! the goldens stable. Regenerate deliberately with `BLESS=1 cargo test -p characterization`; without
//! `BLESS` the tests compare against the committed goldens and fail on any drift.

use cardtable_combat::{begin_manual_combat, resolve_encounter};
use cardtable_model::{PileId, Recipe, Tableau, sample_table};
use std::path::PathBuf;

/// Compare `actual` against the committed golden `<name>`, or (with `BLESS=1`) rewrite it.
///
/// This is test infrastructure, not an ad-hoc script: the bless path only runs when the operator
/// sets `BLESS`, and the default path is a pure comparison that fails loudly on drift or a missing
/// golden. Goldens live beside this crate under `golden/` and are checked into the repo.
fn assert_golden(name: &str, actual: &str) {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("golden");
    path.push(format!("{name}.ron"));

    if std::env::var_os("BLESS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
        eprintln!("blessed golden: {}", path.display());
        return;
    }

    let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!("missing golden `{name}` at {} — run `BLESS=1 cargo test -p characterization` to create it", path.display())
    });
    // Normalize line endings on both sides: git may rewrite the committed goldens to CRLF on
    // checkout (Windows/autocrlf), while the freshly-serialized string is always LF. Behavior
    // drift shows up as content differences, never as an EOL artifact.
    let norm = |s: &str| s.replace("\r\n", "\n");
    assert_eq!(
        norm(actual),
        norm(&expected),
        "golden `{name}` drifted (behavior changed)"
    );
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
    let mut out = format!("outcome: {outcome:?}\nmutations:\n");
    for m in &muts {
        out.push_str(&format!("  {m:?}\n"));
    }
    out.push_str("---\n");
    out.push_str(&to_golden_ron(&t));
    assert_golden(name, &out);
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
    let step = |t: &Tableau, label: &str, log: &mut String| {
        log.push_str(&format!("== {label} ==\n{}\n", to_golden_ron(t)));
    };

    let locations = top(&t, "Locations");
    t.focus(locations).unwrap();
    step(&t, "focus Locations", &mut log);

    let ashfen = place(&t, "Ashfen Crossing");
    t.focus(ashfen).unwrap();
    step(&t, "focus Ashfen", &mut log);

    if let Some(&card) = t.content_cards(ashfen).first() {
        t.cycle_card_size(card).unwrap();
        step(&t, "cycle first card size", &mut log);
        t.select(card).unwrap();
        step(&t, "select first card", &mut log);
    }

    let root = t.root_id();
    t.focus(root).unwrap();
    step(&t, "focus root", &mut log);

    assert_golden("interaction_transcript", &log);
}
