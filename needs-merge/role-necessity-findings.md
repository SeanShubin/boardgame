# Role necessity — the solver's verdict (2026-06-27)

> **Findings from the §4 spec-sync + enemy-role effort.** The driving question: can *every* party role be
> made **necessary** (removing it flips a winnable fight to a loss), or is there a structural reason one
> can't? Answer below, with the mechanisms built along the way. **Promotion target:
> `computability-and-balance.md` §10 + `docs/game-theory/`.** Pairs with `lock-exclusivity-finding.md`,
> `role-weight-harness-built.md`, `strategy-primitive-suite.md`.

## The verdict
**Necessity is a *survival* property.** Measured at trustworthy (high) budget, only the survival roles
flip a fight from winnable to unwinnable:
- **Wall (Iron)** — necessary against a damage **spike** (lethal volley / mixed-threat).
- **Support (Salt)** — necessary against sustained **attrition** (mixed-threat).

**The three offense roles — Infiltrator (Silver), Artillery (Brass), Controller (Bone) — are fungible for
*winning* everywhere.** Their value is **graded efficiency** (fewer rounds / fewer downed / more Health)
and **variety**, not necessity. Every apparent offense-role flip turned out to be a **budget artifact**:
a low-budget run mislabels a *deep* winning line as "unwinnable"; a high-budget run finds the win and
retracts the flip. (A 300K Silver flip and a 2M Salt flip were both retracted at 12M.)

## Why offense resists necessity (the structural reasons)
- **Reach (Infiltrator).** Reaching the back is fungible with **breaking the front** — any party that can
  fall the screen reaches the back without a slip. The only way to force the slip is an **un-fallable
  screen**, and that creates the **necessity ↔ searchability dilemma** below.
- **Strip (Controller / −Toughness).** The per-phase pile **accumulates across the whole party**, so
  **focus-fire substitutes for Sunder** — high Toughness only slows, never stops (`probe_toughness_extreme`).
- **AoE (Artillery).** Single-target + many bodies clears a horde; AoE is faster, not required.
- **Two "universal solvents":** Support's sustain wins any grind; the Infiltrator's burst kills any
  exposed source — so single exclusive locks overlap (`lock-exclusivity-finding.md`).

## The necessity ↔ searchability dilemma (the deep one)
To make **reach** necessary you need a screen that **cannot fall** (so the party must cross, not break
through). But an un-fallable screen makes the combat **drag to the 5-round cap with no resolution**, and a
**loss-confirmation must search the entire tree** (a win short-circuits; a loss does not). A single
5-hero combat is already **millions of states**; a dragging one **exceeds 12M** (>10 min) and can't be
confirmed. Conversely, any screen thin enough to keep the combat **searchable** is thin enough to **break
through** → reach isn't necessary. **The encounters that force a role are exactly the ones the exact
solver can't resolve.** Adding kill-pressure (a lethal backline) to force fast resolution didn't escape it
— a screen thick enough to block break-through still dragged.

## Mechanisms built along the way (real tactical depth, regardless of the verdict)
All committed, suite green:
- **§4 melee one-contest** (`Guard::Block`) — a defender out-bids `cards × Finesse` to slip a blow; wired
  as a commit-then-resolve Standoff decision the solver explores.
- **Interception** (`combat::intercept`) — a bypassed front Vanguard *strikes the runner*; the crosser
  slips via the same Finesse contest. Not a new mechanic — the existing strike + contest as enemy
  behavior. A wide front drains a crosser slip-by-slip (weakest-link), so the Infiltrator is *more
  efficient* at crossing (just not *uniquely able*).
- **Enemy roles** — a `heal` behavior (the **Mender**) and a high-Finesse **Sentry** interceptor screen.
- **Transition-list architecture** — the round's resolution is an editable `POST_VOLLEY_SCHEDULE` with
  isolated, relocatable accumulator-wipes (the focus-fire / Toughness dial).
- **Budgeted graded solver** (`solve_within`) + a fixed witness; **role-weight necessity harness**.

## Design implication
Stop trying to make offense roles **necessary**; measure them on the axes where they actually differ:
- **Battle par** (graded: rounds / downed / Health) on small, *decisive* fights where the solver resolves
  cleanly — the Infiltrator's slip is a real par/survival improvement (h21→h23 with the contest).
- **Variety** (combo premium, anti-spam — `variety-as-a-balance-objective.md`).
The party is **Wall + Support (necessary survival) + offense specialists (efficiency / variety)**. That is
the honest shape of the roster, and the solver is unambiguous about it once the budget is high enough.
