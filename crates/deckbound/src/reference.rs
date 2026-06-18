//! `reference-scenario.md` — the **reference scenario**: a diagnostic **A / B / C / final** lattice,
//! built as data and **maintained as a test**. Each location probes a balance property, and the
//! invariant evaluator checks the gating holds:
//!
//! - **A** clearable from a clean slate.
//! - **B[p]** (one per path) builds path *p*.
//! - **C[p]** clearable **iff** B[p] was built (path *p* is necessary and sufficient).
//! - **Final** clearable **iff** all paths are covered.
//!
//! *First-pass:* the clear-model is **analytical** — a party's currency-power (treasure earned) vs a
//! per-location **demand** — not a full battle playthrough (that needs the human-emulating AI,
//! roadmap). So this fixture catches *structural / gating* regressions today; a combat-resolving
//! evaluator can replace [`clearable`] later without changing the lattice.

use crate::currency::{Coins, Currency, balance};
use crate::encounter::{EncounterCard, RosterEntry};
use crate::form::StatCard;
use crate::world::{Coord, Layout, Location, Run};

/// What a location demands to be cleared (analytical clear-model).
#[derive(Clone, Copy, Debug)]
enum Demand {
    /// Clearable from a clean slate (A, B[p]).
    Free,
    /// Needs ≥ `amount` of one currency — the reward a built path yields (C[p]).
    Counter(Currency, i64),
    /// Needs ≥ `amount` of **every** path currency (the final).
    AllPaths(i64),
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

/// A placeholder encounter for a location (combat resolution is deferred; this is well-formed data).
fn make_encounter(name: &str, currency: Currency) -> EncounterCard {
    EncounterCard {
        name: name.into(),
        currency,
        strategy: "aggressor".into(),
        foes: vec![RosterEntry {
            creature: "Husk".into(),
            from_level: 1,
            base: 1,
            growth: 1,
        }],
        scaling: StatCard {
            body: 2,
            ..Default::default()
        },
    }
}

/// Build the reference scenario over `paths` (the progression paths — currently the five roles).
/// Layout (grid): A at col 0; B[p] at col 1; C[p] at col 2; final at col 3 — connected so every
/// location is reachable from A. Payout = a cleared location yields its currency × its `max_level`.
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
    encounters.push(make_encounter("A", Currency::Gold));
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
        encounters.push(make_encounter(&format!("B[{}]", p.label()), p));
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
        encounters.push(make_encounter(&format!("C[{}]", p.label()), p));
        demands.push(Demand::Counter(p, level as i64)); // B[p] at max yields exactly `level`
    }

    // Final — needs all paths covered (B *and* C cleared → 2×level per path).
    let final_index = locations.len();
    locations.push(Location {
        name: "Final".into(),
        coord: Coord::new(3, 0),
        currency: Currency::Gold,
        max_level: level,
    });
    encounters.push(make_encounter("Final", Currency::Gold));
    demands.push(Demand::AllPaths(2 * level as i64));

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

/// Is a `demand` met by a party that has `earned` (and `spent`) the given currency, over `paths`?
fn clearable(demand: Demand, earned: &[Coins], spent: &[Coins], paths: &[Currency]) -> bool {
    match demand {
        Demand::Free => true,
        Demand::Counter(c, amount) => balance(c, earned, spent) >= amount,
        Demand::AllPaths(amount) => paths.iter().all(|&p| balance(p, earned, spent) >= amount),
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
    let spent: Vec<Coins> = Vec::new();
    let payout = |idx: usize| {
        Coins::new(
            s.run.locations[idx].currency,
            s.run.locations[idx].max_level,
        )
    };

    // --- structural ---
    if s.b_index.len() != n || s.c_index.len() != n {
        v.push("expected one B and one C per path".into());
    }
    for (i, &p) in s.paths.iter().enumerate() {
        if s.run.locations[s.b_index[i]].currency != p {
            v.push(format!("B[{}] mints the wrong currency", p.label()));
        }
        if s.run.locations[s.c_index[i]].currency != p {
            v.push(format!("C[{}] mints the wrong currency", p.label()));
        }
    }
    if s.run.objective != s.final_index {
        v.push("the objective is not the final location".into());
    }
    if reachable(&s.run).iter().any(|&r| !r) {
        v.push("some location is unreachable from A".into());
    }

    // --- gating (analytical) ---
    if !clearable(s.demands[s.a_index], &[], &spent, &s.paths) {
        v.push("A is not clearable from a clean slate".into());
    }
    for (i, &b) in s.b_index.iter().enumerate() {
        if !clearable(s.demands[b], &[], &spent, &s.paths) {
            v.push(format!(
                "B[{}] not clearable clean-slate",
                s.paths[i].label()
            ));
        }
    }
    // C[p]: NOT clearable without path p (no coverage leak); clearable after building B[p].
    for (i, &c) in s.c_index.iter().enumerate() {
        if clearable(s.demands[c], &[], &spent, &s.paths) {
            v.push(format!(
                "C[{}] is clearable WITHOUT building its path (coverage leak)",
                s.paths[i].label()
            ));
        }
        let after_b = [payout(s.b_index[i])];
        if !clearable(s.demands[c], &after_b, &spent, &s.paths) {
            v.push(format!(
                "C[{}] is NOT clearable even after building B[{}]",
                s.paths[i].label(),
                s.paths[i].label()
            ));
        }
    }
    // Final: clearable with full coverage; NOT with a path missing.
    let mut full: Vec<Coins> = Vec::new();
    full.extend(s.b_index.iter().map(|&b| payout(b)));
    full.extend(s.c_index.iter().map(|&c| payout(c)));
    if !clearable(s.demands[s.final_index], &full, &spent, &s.paths) {
        v.push("Final is NOT clearable with full coverage".into());
    }
    if n > 0 {
        let missing: Vec<Coins> = full
            .iter()
            .copied()
            .filter(|c| c.currency != s.paths[0])
            .collect();
        if clearable(s.demands[s.final_index], &missing, &spent, &s.paths) {
            v.push("Final is clearable with a path missing (a path is redundant)".into());
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
}
