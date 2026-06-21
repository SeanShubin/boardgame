//! `reference-scenario.md` — the **reference scenario**: a diagnostic **A / B / C / final** lattice,
//! built as data and **maintained as a test**. Each location probes a balance property, and the
//! invariant evaluator checks the gating holds:
//!
//! - **A** clearable from a clean slate.
//! - **B[p]** (one per path) builds path *p*.
//! - **C[p]** clearable **iff** B[p] was built (path *p* is necessary and sufficient).
//! - **Final** clearable **iff** all paths are covered.
//!
//! *First-pass:* the clear-model is **analytical** — a party's **role-track coverage** (which tracks
//! it holds rewards in, §8.3) vs a per-location **demand** — not a full battle playthrough (that needs
//! the human-emulating AI, roadmap). So this fixture catches *structural / gating* regressions today;
//! a combat-resolving evaluator can replace [`clearable`] later without changing the lattice.

use crate::currency::Currency;
use crate::encounter::{EncounterCard, RosterEntry};
use crate::form::StatCard;
use crate::world::{Coord, Layout, Location, Run};

/// What a location demands to be cleared (analytical clear-model, in terms of §8.3 reward coverage).
#[derive(Clone, Copy, Debug)]
enum Demand {
    /// Clearable from a clean slate (A, B[p]).
    Free,
    /// Needs the party to hold rewards in track `p` — what a built path yields (C[p]).
    NeedTrack(Currency),
    /// Needs **every** role track covered (the final).
    AllTracks,
}

/// The reference scenario: the lattice over the world map, its per-location encounters and demands,
/// and the indices that name each probe.
pub struct ReferenceScenario {
    pub run: Run,
    pub paths: Vec<Currency>,
    pub encounters: Vec<EncounterCard>,
    demands: Vec<Demand>,
    a_index: usize,
    b_index: Vec<usize>,
    c_index: Vec<usize>,
    final_index: usize,
}

/// An encounter for a location: `count` Husks of the matching `currency`'s threat, fixed-size (no
/// level scaling here — the level dial drives reward; this `count` sets the combat-difficulty band).
fn make_encounter(name: &str, currency: Currency, count: u32) -> EncounterCard {
    EncounterCard {
        name: name.into(),
        currency,
        strategy: "aggressor".into(),
        foes: vec![RosterEntry {
            creature: "Husk".into(),
            from_level: 1,
            base: count,
            growth: 0,
        }],
        scaling: StatCard::default(),
    }
}

/// Build the reference scenario over `paths` (the progression tracks — the five roles).
/// Layout (grid): A at col 0; B[p] at col 1; C[p] at col 2; final at col 3 — connected so every
/// location is reachable from A. Clearing a track-`p` location unlocks that track's rewards (§8.3).
pub fn reference_scenario(paths: &[Currency]) -> ReferenceScenario {
    let level: u32 = 5; // max clear depth of the deep locations (seed)
    let mut locations = Vec::new();
    let mut encounters = Vec::new();
    let mut demands = Vec::new();

    // A — start (generic Gold), shallow.
    let a_index = 0;
    locations.push(Location {
        name: "A — Start".into(),
        coord: Coord::new(0, 0),
        currency: Currency::Gold,
        max_level: 1,
    });
    encounters.push(make_encounter("A", Currency::Gold, 1)); // trivial: a bare Novice clears it
    demands.push(Demand::Free);

    // B[p] — builds path p (clearable clean-slate).
    let mut b_index = Vec::new();
    for (i, &p) in paths.iter().enumerate() {
        b_index.push(locations.len());
        locations.push(Location {
            name: format!("B[{}]", p.label()),
            coord: Coord::new(1, i as i32),
            currency: p,
            max_level: level,
        });
        encounters.push(make_encounter(&format!("B[{}]", p.label()), p, 1)); // builder: bare clears
        demands.push(Demand::Free);
    }

    // C[p] — gate: needs the p-currency that a cleared B[p] yields.
    let mut c_index = Vec::new();
    for (i, &p) in paths.iter().enumerate() {
        c_index.push(locations.len());
        locations.push(Location {
            name: format!("C[{}]", p.label()),
            coord: Coord::new(2, i as i32),
            currency: p,
            max_level: level,
        });
        encounters.push(make_encounter(&format!("C[{}]", p.label()), p, 2)); // gate: bare loses, the p-kit wins
        demands.push(Demand::NeedTrack(p)); // needs the track-p rewards a cleared B[p] yields
    }

    // Final — needs all tracks covered (every B[p] cleared → its track's rewards held).
    let final_index = locations.len();
    locations.push(Location {
        name: "Final".into(),
        coord: Coord::new(3, 0),
        currency: Currency::Gold,
        max_level: level,
    });
    encounters.push(make_encounter("Final", Currency::Gold, 14)); // boss: needs the full party
    demands.push(Demand::AllTracks);

    let run = Run::new(Layout::Grid, locations, final_index, a_index, 1);
    ReferenceScenario {
        run,
        paths: paths.to_vec(),
        encounters,
        demands,
        a_index,
        b_index,
        c_index,
        final_index,
    }
}

/// Is a `demand` met by a party that **covers** `covered` tracks (holds rewards in them), over the
/// scenario's `paths`?
fn clearable(demand: Demand, covered: &[Currency], paths: &[Currency]) -> bool {
    match demand {
        Demand::Free => true,
        Demand::NeedTrack(p) => covered.contains(&p),
        Demand::AllTracks => paths.iter().all(|p| covered.contains(p)),
    }
}

/// Locations reachable from the party's start, by one-space moves over the layout.
fn reachable(run: &Run) -> Vec<bool> {
    let n = run.locations.len();
    let mut seen = vec![false; n];
    let start = run.positions[0];
    seen[start] = true;
    let mut stack = vec![start];
    while let Some(i) = stack.pop() {
        let from = run.locations[i].coord;
        for (j, seen_j) in seen.iter_mut().enumerate() {
            if !*seen_j && run.layout.adjacent(from, run.locations[j].coord) {
                *seen_j = true;
                stack.push(j);
            }
        }
    }
    seen
}

/// Check the diagnostic invariants (`reference-scenario.md`). Returns the list of violations — an
/// empty list means the lattice is well-formed and the gating holds.
pub fn check_invariants(s: &ReferenceScenario) -> Vec<String> {
    let mut v = Vec::new();
    let n = s.paths.len();

    // --- structural ---
    if s.b_index.len() != n || s.c_index.len() != n {
        v.push("expected one B and one C per path".into());
    }
    for (i, &p) in s.paths.iter().enumerate() {
        if s.run.locations[s.b_index[i]].currency != p {
            v.push(format!("B[{}] is the wrong track", p.label()));
        }
        if s.run.locations[s.c_index[i]].currency != p {
            v.push(format!("C[{}] is the wrong track", p.label()));
        }
    }
    if s.run.objective != s.final_index {
        v.push("the objective is not the final location".into());
    }
    if reachable(&s.run).iter().any(|&r| !r) {
        v.push("some location is unreachable from A".into());
    }

    // --- gating (analytical, in §8.3 reward coverage) ---
    if !clearable(s.demands[s.a_index], &[], &s.paths) {
        v.push("A is not clearable from a clean slate".into());
    }
    for (i, &b) in s.b_index.iter().enumerate() {
        if !clearable(s.demands[b], &[], &s.paths) {
            v.push(format!(
                "B[{}] not clearable clean-slate",
                s.paths[i].label()
            ));
        }
    }
    // C[p]: NOT clearable without covering track p (no coverage leak); clearable after building B[p].
    for (i, &c) in s.c_index.iter().enumerate() {
        if clearable(s.demands[c], &[], &s.paths) {
            v.push(format!(
                "C[{}] is clearable WITHOUT building its track (coverage leak)",
                s.paths[i].label()
            ));
        }
        let after_b = [s.paths[i]]; // building B[p] yields track-p reward coverage
        if !clearable(s.demands[c], &after_b, &s.paths) {
            v.push(format!(
                "C[{}] is NOT clearable even after building B[{}]",
                s.paths[i].label(),
                s.paths[i].label()
            ));
        }
    }
    // Final: clearable with full coverage; NOT with a track missing.
    let full: Vec<Currency> = s.paths.clone();
    if !clearable(s.demands[s.final_index], &full, &s.paths) {
        v.push("Final is NOT clearable with full coverage".into());
    }
    if n > 0 {
        let missing: Vec<Currency> = s.paths[1..].to_vec();
        if clearable(s.demands[s.final_index], &missing, &s.paths) {
            v.push("Final is clearable with a track missing (a track is redundant)".into());
        }
    }
    v
}

/// Combat-real difficulty bands (§8.4), via the auto-resolver (Clash off): a bare clean-slate party
/// loses each gate, an appropriately-equipped party wins, and the final needs the full roster.
/// (Currency *affordability* — that p-Upgrades require clearing B[p] — is the analytical check
/// above; this confirms the *difficulty* sits in the right band.)
pub fn check_combat_bands(s: &ReferenceScenario, seed: u64) -> Vec<String> {
    use crate::scenarios::{build_character, build_encounter_foes, rewards_for};
    use crate::solver::auto_resolve;

    let mut v = Vec::new();
    let novice = || build_character("Novice", &[]);
    // A character "invested in track p" holds that track's rewards (what clearing B[p] unlocks).
    let specialist = |p: Currency| build_character("Novice", &rewards_for(p));

    // C[p]: a bare party should lose; a p-equipped specialist should win.
    for (i, &p) in s.paths.iter().enumerate() {
        let enc = &s.encounters[s.c_index[i]];
        if auto_resolve(vec![novice()], build_encounter_foes(enc, 1), seed) != Some(false) {
            v.push(format!(
                "C[{}] too easy — a bare party should lose",
                p.label()
            ));
        }
        if auto_resolve(vec![specialist(p)], build_encounter_foes(enc, 1), seed) != Some(true) {
            v.push(format!(
                "C[{}] too hard — a {}-equipped party should win",
                p.label(),
                p.label()
            ));
        }
    }

    // Final: a full party (one specialist per path) wins; a party missing one path does not.
    let full: Vec<_> = s.paths.iter().map(|&p| specialist(p)).collect();
    let enc = &s.encounters[s.final_index];
    if auto_resolve(full.clone(), build_encounter_foes(enc, 1), seed) != Some(true) {
        v.push("Final too hard — a full party should win".into());
    }
    if full.len() > 1 {
        let short = full[1..].to_vec();
        if auto_resolve(short, build_encounter_foes(enc, 1), seed) == Some(true) {
            v.push("Final too easy — a party missing a path should not win".into());
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The five role currencies — the progression paths the reference scenario is built over.
    fn five_roles() -> Vec<Currency> {
        vec![
            Currency::Iron,
            Currency::Silver,
            Currency::Brass,
            Currency::Bone,
            Currency::Salt,
        ]
    }

    #[test]
    #[ignore]
    fn probe_combat_bands() {
        let s = reference_scenario(&five_roles());
        let v = check_combat_bands(&s, 1);
        println!("combat band violations ({}): {v:#?}", v.len());
    }

    #[test]
    fn reference_scenario_invariants_hold() {
        let s = reference_scenario(&five_roles());
        let violations = check_invariants(&s);
        assert!(
            violations.is_empty(),
            "invariant violations: {violations:?}"
        );
        // Shape: A + 5 B + 5 C + final = 12 locations; every encounter builds real foes.
        assert_eq!(s.run.locations.len(), 12);
        for enc in &s.encounters {
            assert!(!crate::scenarios::build_encounter_foes(enc, 3).is_empty());
        }
    }

    #[test]
    fn a_broken_gate_is_detected() {
        // If C[Iron] demanded nothing (a coverage leak), the evaluator must catch it.
        let mut s = reference_scenario(&five_roles());
        s.demands[s.c_index[0]] = Demand::Free;
        let violations = check_invariants(&s);
        assert!(
            violations.iter().any(|m| m.contains("coverage leak")),
            "evaluator failed to detect the broken gate: {violations:?}"
        );
    }

    #[test]
    fn reference_combat_bands_hold() {
        // The gates hold under REAL combat (Clash off, auto-resolved): a bare party loses each
        // C[p], a path-invested specialist wins it, and the final needs the full roster.
        let s = reference_scenario(&five_roles());
        let v = check_combat_bands(&s, 1);
        assert!(v.is_empty(), "combat band violations: {v:?}");
    }
}
