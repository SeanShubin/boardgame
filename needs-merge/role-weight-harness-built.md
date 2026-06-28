# Role-weight harness — built, and what the solver's practical reach turned out to be

> **Built + empirical findings, 2026-06-26.** Implements the Tier-1 marginal-contribution instrument
> from `role-weight-balance-testing.md`, riding the landed solver. The build surfaced hard limits on the
> *graded* solver's reach that reshaped the instrument. **Promotion target: `computability-and-balance.md`
> §10** (the par-tooling runbook) + `docs/game-theory/`. Pairs with `lock-exclusivity-finding.md`.

## What got built (`crates/deckbound`)
- **`solver::solve_within(heroes, foes, seed, ruleset, max_nodes) -> Solution`** — a *budgeted* graded
  solve mirroring `winnable_within`; `solve` now delegates with `MAX_NODES`. Returns `overflowed = true`
  when the budget is hit (the value is then a **lower bound at the cut**). This operationalizes the
  **searchability-bound** design signal (raise budget / shrink encounter / accept knowingly).
- **`balance::role_weight_report(seed, budget)`** — marginal **necessity** across an encounter suite:
  for each encounter, is the full-kit party (one specialist per reward suit) winnable, and does removing
  each role's **kit** (body kept — LOO-by-*swap*, isolating mechanics from headcount) make it
  unwinnable? A FLIP = NECESSARY there. Rides `winnable_within`. Probe: `probe_role_weight`.
- **`balance::battle_par_report(seed, budget)`** — graded par/downed/Health on the small 3-hero §8.6
  lock parties (baseline vs +role), where the Anchor's contribution is legible. Probe: `probe_battle_par`.
- A **scaled suite** (armored front / screened backline / swarm / Toughness wall / mixed / lethal volley),
  sized to a *five-body* party's edge (the §8.6 bands were 3-hero and a 5-body party trivially won them).

## The reach finding (reshaped the instrument)
- **Graded `solve` cannot short-circuit** — to prove a value *optimal* it must explore the whole tree.
  A 5-hero full-kit graded solve **overflows even 300K nodes** (≈15 min, *every* verdict budget-limited);
  a budget-limited graded value's rounds/downed/Health are an **artifact of where the search stopped and
  are not comparable across parties** (a first run had the Wall reading "HURTS everywhere" purely from
  truncation depth). Even at **3 heroes**, graded par mostly overflowed 1M nodes (≈4 min).
- **`winnable_within` short-circuits on the first win** → cheap and reliable for the boolean flip,
  *regardless* of budget. So: **marginal necessity rides winnability; graded par is reserved for small
  decisive fights.** Full-kit graded *par* is out of practical reach — a real limit on the solver as a
  balance instrument, not a tuning miss.
- Winnability only signals at the **edge** of winnability, so the suite must be tuned to the party's edge
  (a 5-body party trivially wins under-sized encounters → no flip). This is the roadmap's
  "difficulty-scaling axis," now confirmed as **necessary**, not optional.

## First real signal (scaled suite, seed 1, 400K budget, ~76 s)
| Role                  | necessary / redundant | reading                                                           |
| --------------------- | --------------------- | ----------------------------------------------------------------- |
| **Iron (Wall)**       | 2 / 4                 | **load-bearing** — NECESSARY in *mixed threat* + *lethal volley*  |
| **Salt (Support)**    | 2 / 4                 | **load-bearing** — NECESSARY in the same two (budget-limited `?`) |
| Silver / Brass / Bone | 0 / 6                 | **no winnability flip** — value is graded/niche, not winnability  |

- **The Anchor's necessity surfaces on the *survival* axis** (lethal encounters) — the one axis a win/loss
  boolean can see. This directly answers the founding worry ("don't mis-measure the Wall as
  over/under-valued"): measured on lethality, the Wall is load-bearing exactly where it should be.
- **Support** is likewise necessity-provable (sustain in lethal fights), confirming the "universal
  solvent" reading from `lock-exclusivity-finding.md` — here it's a *virtue*, legibly load-bearing.
- **Silver/Brass/Bone show no winnability flip** in this suite: a 5-body party brute-forces these without
  their specific kit (even the Toughness wall is cracked by volume without Bone's Sunder). Their value is
  **graded efficiency** (fewer rounds/downed) or a **narrow niche** — and graded efficiency is exactly
  what's intractable at full scale. **This is the open measurement gap.**

## Open gap → next
The offense levers' (Infiltrator / Artillery / Controller) weight is *graded*, and graded par doesn't
scale to a full party. Options to close it (next session):
1. **Niche encounters tuned to the party's edge** where each offense lever flips *winnability* (the
   screened-backline lever is the proven template — `lock-exclusivity-finding.md`); a per-lever
   **difficulty ramp** that finds the foe-count band where the full party transitions win→loss.
2. **Reduced-party graded par** (2–3 heroes) where `solve_within` completes un-flagged, measuring
   par/downed/Health deltas — the `battle_par_report` path (note: even 3-hero locks often overflow 1M).
3. Treat the budget-limited `?` flips (Support) as needs-decision signals; confirm at higher budget.

Then **Task 6 — variety checks** (`variety-as-a-balance-objective.md`): combo premium + anti-spam, which
also ride winnability/party-restriction and inherit the same edge-tuning requirement.
