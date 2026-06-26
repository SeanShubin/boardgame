# Automated balance testing — roadmap & gap map

> **Index, staged 2026-06-26.** Ties together the balance-testing effort: what's **planned** (the
> sibling `needs-merge` docs) and what **remains**. **Promotion target: `computability-and-balance.md`
> §10** (the par-tooling runbook — the authoritative deferred-build plan); the conceptual halves go to
> `docs/game-theory/`. Legend: ✅ planned · ◐ partial · ⬜ unplanned.

## Instruments (the engine layer)
- ✅ **Battle solver** (Task 1) — `perfect-solver-plan.md`. Exact optimal battle play / battle-par /
  win-reachability. Ratified.
- ◐ **Encounter suite** (Task 2) — sketched in `role-weight-balance-testing.md` (the profile-relative
  section). **Remains:** the full per-role **niche ledger** — for each lever, its *profile*
  (Anchor/Striker/Multiplier) + *intended domain* + *baseline that should lose* + *exclusivity*
  (which other roles must fail to clear it), plus a **difficulty-scaling axis** and a **coverage
  completeness** check.
- ⬜ **Run-level (campaign) solver** — the **outer** planning problem: par-in-Days over routes / builds /
  battles (§0.1 state = positions / cleared-set / builds / Day), using the **battle solver as a
  memoizable oracle**. This is the actual *whole-game* core-balance instrument (§0.3 step 1 is about the
  **run**, not one battle); the battle solver is its inner loop. **The biggest unplanned piece.**

## Measurements (what we compute with the instruments)
- ✅ **Role-weight / marginal contribution** — `role-weight-balance-testing.md` (LOO → pairwise synergy →
  Shapley, profile-relative, dominance ≠ soloable). **Remains:** the computation **harness wiring**
  (the graded-metric plumbing + report format).
- ◐ **Par** — *battle*-par falls out of the solver's graded objective. **Remains:** **campaign par-in-Days**
  (needs the run-level solver) + **par-target assertions** (§8.2 golf; §0.4 "winnable within the horizon").
- ⬜ **Closure check** — *"no unnamed strategy dominates the interesting set"* (§0.3 step 1 — the **dual of
  necessity**). Search the build × play space for a line that beats par or makes the interesting set
  redundant. **Unplanned, and a first-class gate** (the half that catches degenerate *strategies*, where
  necessity catches dead *mechanics*).
- ◐ **Necessity (§6.1) at the MECHANIC level** — roles are partially covered (the suite niches).
  **Remains:** a required-to-win scenario for **every** ability / keyword / phase-rule, the **dependency
  graph** (each scenario forces exactly one new mechanic → topological sort = test order = tutorial
  order), and the **coverage ledger / cut list**.

## Luck layers & build space (§0.3 step 2 + progression)
- ⬜ **Luck-layer isolation** — each hidden/random layer (blind bid, randomized creature decks, location
  fog, event deck, threat deck) shown **neutral / non-dominant in isolation** before composition
  (§0.2 / §0.3). Unplanned.
- ◐ **Build-space balance** — dominant-build / upgrade-path search (monotone dominance pruning, §0.1);
  progression closure. Partially: the `reference.rs` lattice. **Remains:** generalize beyond the lattice.

## Harness / process
- ◐ **Assertion vs diagnostic split** — CI **gates** (regression assertions — §6.1's double-payoff: a
  retune that lets a naive line win **breaks the build**) vs on-demand **sweeps** (today's `balance.rs`
  `probe_*` `#[ignore]` reports). **Remains:** decide which checks are gates, plus the **solver's
  performance budget** (the par-tooling must be fast enough to run).
- ⬜ **Tuning loop** — seed numbers → tune against the suite/solver → human gate (we deferred numbers:
  "propose seeds, tune via balance tests"). The workflow itself is unplanned.

## Suggested order
1. **Tier 1 — completes the role-weight goal you started from:** the **encounter suite** (Task 2) + the
   **marginal-contribution harness**. Both ride the (planned) battle solver; nothing else is needed for
   "does each role pull its weight."
2. **Tier 2 — the core-balance program (§0.3 step 1):** **closure check** + **mechanic-level necessity**
   + the **run-level solver** (the big one; unlocks campaign par).
3. **Tier 3 — separable luck (§0.3 step 2) + build space.**
4. **Tier 4 — harness/CI split + the tuning loop** (threads through all).

All of this is the surface of the `computability-and-balance.md` §10 par-tooling runbook; the **battle
solver is its computational core**, and everything else is a consumer or a sibling instrument. Fold these
slices into §10 when the spec-sync clears.
