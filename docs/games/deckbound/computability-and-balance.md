# Deckbound — Computability & Balance (a design discipline)

> **Why this document exists.** Deckbound has a deterministic skeleton we intend to *use*:
> to compute par, to prove the game is beatable, and to balance it objectively. That only
> works if future design choices keep the skeleton computable. This is the detailed
> elaboration of **Charter north star #11** (and it serves #2 and #4). If you — human or AI
> — are about to add a mechanic that touches **randomness, foe behaviour, carried state,
> build growth, or the day clock**, run the checklist in §3 first.
>
> This is a **design-intent** document. It traces to the Charter; it is not the Spec. When
> a system here gets a Spec section, its invariants should graduate into that section's
> **GUARANTEES** (the Spec owns mechanical invariants). See §8.
>
> **▶ Resuming the build?** Par tuning is **deferred until the mechanics are vetted** (the
> designer's call). When you return to build the measurement tools, tune the cards, or design the
> combat algorithms, **start at [§10 — Resuming: the deferred build plan](#10-resuming--the-deferred-build-plan)**: it is the runbook (order of work, locked decisions, the questions to ask first, code/doc entry points, and the definition of done).

---

## 1. The intention

We deliberately built a game with two layers (Charter #2): a **tactical** layer that is
small and near-solvable, and a **strategic** layer that, *for the player*, is judgment
under uncertainty. Turn off the optional Clash module and open the creature decks and the
map, and a third thing appears underneath: a **deterministic, perfect-information,
single-agent, bounded** game. We want to exploit that, for five purposes:

1. **Compute par** objectively, and surface the most straightforward solution.
2. **Balance for variety** — ensure a range of *interesting* strategies all win in par.
3. **Balance roles, cards, and creatures.**
4. **Balance the costs of cards.**
5. Run all of the above fast, via a **near-optimal deterministic combat resolver**.

None of these is possible if the skeleton stops being computable. So computability is not
a nice-to-have; it is a **standing constraint on the design**, and this document is its
detailed statement so we do not forget *why* a given restriction is there.

---

## 2. Why it is computable today (the structural facts)

These are the facts that make par feasible. Each is also an invariant to protect (§3).

- **Deterministic.** With the Clash module off, nothing in the canonical mode consumes the
  RNG — combat resolution, the creature AI, encounter building, and the world layout are all
  pure functions of state. (In the code, the *only* RNG consumer is the Clash creature's
  move-pick.) The seed becomes a no-op.
- **Perfect information.** Open the encounter draw decks and reveal the face-down location
  cards and there is no hidden information. (Determinism and perfect information are
  distinct: a shuffled-but-unrevealed deck is deterministic yet hidden. "Open" is what buys
  perfect information.)
- **Single-agent — this is the big one.** The creatures run a **fixed, deterministic
  policy**; they do not optimise against your specific plan. So the campaign is not a
  two-player game — it is **one agent planning against a fixed environment**. "Optimal
  strategy" means an **optimal plan** (the action sequence that wins in fewest days), not a
  game-theoretic equilibrium. This is why it is categorically easier than chess: chess is
  `∃ my move ∀ your move …` (minimax); this is `∃ a sequence of my moves` (a graph search).
- **Battles are stateless in *combat*; the campaign's carried state is the *build*.** Each `Enter`
  rebuilds fresh actors from `base + upgrades`, days reset the tokens, and a win clears the location
  in one fight — so **no wounds or buffs persist**, and combat outcome is a **memoizable oracle**
  `clears(build, place) → win/lose (+ margin)`. But the build *does* persist: **progression** is
  the campaign's carried state (clears → currency → upgrades → a stronger build). Combat
  statelessness ≠ campaign statelessness.
- **Progression's trajectory-diversity collapses onto a small state set — *if* builds stay
  monotone.** Characters evolve along many *trajectories* (who specialises when, in what order), but
  because upgrades are permanent, additive, and **order-independent** (§5.5), those trajectories
  collapse onto the same build *states* — par searches **states `(positions, cleared, builds, day)`,
  not histories** — and monotonicity makes **dominance pruning** valid (an earlier or superset build
  dominates). So the diversity is free *to the search*; it's where the *interesting* strategy variety
  lives, not a cost. **This is contingent on the §3 build invariants** — respec, order-dependent, or
  multiplicative upgrades make the build path-dependent and the search explodes. Progression is the
  dimension that spends the computability budget; guard it.
- **The campaign is a routing + build optimisation.** Given the oracle, the campaign reduces
  to: clear locations (each gated by needing the right build) to earn currency to buy
  upgrades to unlock harder locations, minimising days, subject to one move and one fight per
  day. A small, bounded, enumerable state space — far smaller than chess's `~10⁴⁴`.

**Consequence.** Par (min days) is a finite shortest-path / optimisation, decidable in
principle and, for the reference scenario, *feasibly* so (full search, or A* with a
"days-remaining" heuristic, with each `Enter` expanding into an embedded combat search).

---

## 3. The invariants to protect — and a design-review checklist

Each invariant is paired with **what breaks it** and **why it matters**. A mechanic that
breaks one must be confined to an optional mode (Clash, Versus) or explicitly bounded.

| Invariant | Breaks it | Why it matters |
| --- | --- | --- |
| **No RNG / hidden info in the canonical mode** | unseeded randomness; a hidden deck the par mode can't read | randomness turns par into an *expectation over* a distribution → you're computing equilibria, not plans |
| **Foes are a fixed environment, not an adversary** | AI that searches/reacts against the player's specific plan; learning foes | turns single-agent planning into two-player minimax → the chess cliff (PSPACE/EXPTIME, the alternating-quantifier blow-up) |
| **Battles near-stateless: `f(build, place)`** | carried wounds, persistent buffs, consumables, fatigue, deck-thinning that persists | the oracle becomes `f(build, place, history)`; the planner must drag an HP/resource vector across battles → state-space explosion |
| **Builds monotone, additive, order-independent** | removable/swappable upgrades; order-dependent or *multiplicative* combos | breaks monotone pruning ("more is better") and balloons the reachable build set (this is *why* the aspect/chord combo layer is deferred — see [future-possibilities](future-possibilities.md)) |
| **Bounded horizon, modest branching, terminating combat** | unbounded productive loops (infinite farming); large continuous boards; many simultaneous independent choices; no round cap | unbounded horizon makes "min days" ill-posed; high branching/horizon makes the search infeasible |

**The review checklist.** For any new mechanic, ask:

1. Does it add **randomness or hidden information** to the canonical mode? (If yes → confine
   to the Clash, or make it open.)
2. Does it make **foes adaptive** to the player's plan? (If yes → it's a two-player feature;
   keep it out of the canonical PvE skeleton.)
3. Does it add **state carried between battles**? (If yes → is it small, discrete, and
   bounded? An unbounded or high-dimensional carry breaks the oracle.)
4. Does it break **build monotonicity / order-independence** (removal, swaps, multiplicative
   combos)? (If yes → expect a combinatorial blow-up; bound it or defer it.)
5. Does it raise the **horizon or branching factor** materially, or allow **unbounded loops**?
   (If yes → bound it.)

If any answer is "yes" and the mechanic is *not* confined to an optional mode or bounded,
you are **spending the computability budget**. That is allowed — but do it **on purpose**,
and update this document and the Charter when you do.

---

## 4. The computability budget is a test, not a guideline

"Feasibly computable" should be **enforced**, the way the encyclopedia counts and the
combat bands already are. When the par solver exists (§8), wire it as a **regression test**:
the reference scenario must solve within a fixed **state / time budget**. A change that
quietly breaks an invariant in §3 will blow the budget and **fail the build**, instead of
silently making balance unverifiable. This converts the discipline from an aspiration into a
gate.

---

## 5. Par is policy-relative (read before quoting any par number)

There is **no objective par independent of how well combat is played.** Par exists only
relative to a combat resolver `P`, so every par number must be stamped **"par under `P`."**

- A **weak** `P` (e.g. the current greedy policy) under-plays, so marginal builds read as
  losses, so the planner over-invests in upgrades, so **par looks harder than it is.** The
  *sign* of your balance error is set by the oracle's weakness — never tune against a weak
  oracle.
- **Certify, don't assert, "near-optimal."** Build a slow but exact per-battle search (the
  hero's decision tree vs the fixed foes — single-agent, so a plain memoizable search) as
  **ground truth**. Bound the fast policy's error against it on a *sample* of `(build, place)`
  pairs, then run the fast policy at scale. That gives purpose #5 a known error bar.

---

## 6. The balancing method — human taste in, objective measurement out

The par solver is the **instrument**; the designer supplies the **taste**. The loop:

1. **Label strategies.** You (human) define a set of **interesting** strategies and a set of
   **boring** ones. A strategy must be expressible as a **constraint the solver optimises
   within** (e.g. "only Iron-path upgrades", "Support carries the gates"), so `par(strategy)`
   is the best days achievable under that constraint.
2. **Require the ordering.** Tune the `booklet.ron` numbers (purposes #3, #4) until the
   **interesting** strategies tie **near par** (within a chosen tolerance `ε` days) and the
   **boring** ones are **strictly worse**.
3. **The closure check — the part hand-enumeration cannot give you.** Your labels only
   constrain the strategies you *named*. The solver computes the **global** par over *all*
   strategies and verifies **no unnamed strategy beats the interesting set.** If one does,
   it's a degenerate exploit you didn't think to label — found for you by the determinism.
   *Without this check, "balanced" is unproven.*

Refinements that keep this honest:

- **Two axes, not one: `(par, robustness)`.** Two strategies can tie on par yet differ
  wildly in how punishing they are — one a knife-edge only an expert hits, the other
  forgiving. Measure the **near-par basin** (how fast par degrades under a noisier /
  suboptimal policy), so a strategy that is "on par for a bot" is not mistaken for an
  accessible one. (This is the same policy-relativity as §5, applied to robustness.)
- **"Boring" means degenerate, not simple.** Boring = dominant / decision-free / exploit.
  Do **not** let it slide into "low-complexity", or you will over-nerf elegant, clean
  strategies. Short is not the same as boring.
- **Let the solver discover candidates.** Run it bidirectionally: it finds the actual
  par-frontier and **clusters** near-par solutions; *you* label each cluster
  interesting / boring / exploit; then tune until the labels match the ordering. Stronger
  than enumerating blind, because it surfaces the strategies (and dominators) you would miss.

**Why this defuses the two stock risks.** It can't ship a *flat, boring* design because
`interesting > boring` is required by construction. It can't ship one *balanced for a bot*
because **you** supply the labels — the bot only measures. The closure check is what makes
it *sufficient* rather than merely well-shaped.

**The concrete targets live in a registry.** The specific, checkable balance properties the tuned
numbers must satisfy — each an instance of "interesting beats boring" or "interesting on par" — are
catalogued in [balance-invariants.md](balance-invariants.md). When the solver lands, each becomes an
assertion it runs (§4), so a retune that breaks one fails the build.

---

## 7. What is allowed to break the rules (quarantined modes)

These modes intentionally leave the computable skeleton — that is their job, and it's fine.
The canonical analysis mode simply has them **off**.

- **Clash module ON** — a hidden, simultaneous RPS-plus-magnitude exchange with a randomised
  creature deck. This reintroduces genuine RNG and **mixed strategies / Nash equilibria**. It
  is Charter #2's "computable *tactics*", confined to the single exchange.
- **Versus (PvP)** — both sides human: the real **two-player adversarial** game, the actual
  chess-analogue, where minimax hardness returns.

Keeping these *optional and confined* is precisely what lets the PvE skeleton stay
single-agent and deterministic.

---

## 8. How this graduates

- **Into the Spec — done (the cross-cutting core).** The separability contract is now binding as
  **[Spec §0](canon/2-spec/README.md#0-the-deterministic-core--separable-balance-)** (*The
  deterministic core — separable balance*): §0.1 the core is computable, §0.2 luck is a separable
  layer, §0.3 separable balance — each as RULE / WHY / GUARANTEES. The Spec owns those mechanical
  invariants; **this document owns the *why* and the cross-cutting discipline** (the §3 checklist,
  §5 policy-relativity, the §6 method).
- **Into the Spec — per system.** As each individual system is worked, it should *also* restate the
  slice of §0 it upholds as a local GUARANTEE (e.g. the §4 battle section: "with Clash off, the
  outcome is a pure function of the two Forms and the encounter"). §0 is the cross-cutting
  statement; the per-section GUARANTEES are its local witnesses.
- **Into code.** The par solver / balance harness is a **future build** — a new Rust crate or
  an `examples/` program reusing `deckbound::solver` and the campaign's legal-action API,
  **never an ad-hoc script** (per the repo guardrail). When it lands, add the §4 budget test
  to CI and make the guide honest by comparing it to the computed par. **The full runbook for this
  deferred build is [§10](#10-resuming--the-deferred-build-plan).**

---

## 9. The one-line test

> Before any new mechanic: *does this keep the canonical mode deterministic, single-agent,
> near-stateless between battles, monotone in builds, and bounded?* If not — is it confined
> to an optional mode, or explicitly bounded? If you can't answer yes, you are spending the
> computability budget. Spend it on purpose, and write it down here.

---

## 10. Resuming — the deferred build plan

**Status: deferred.** Par tuning waits until the designer has **vetted the mechanics** (a while).
When the human says *"build the measurement tools / tune the cards / design the algorithms,"* this
is the runbook. Read it, read the §10.4 context, then ask the §10.3 questions **before** writing
code.

### 10.1 The three workstreams, in dependency order

Build the instrument before its consumers.

1. **Measurement — the par solver (MVP first).** A **new Rust crate or `examples/` program** (never
   an ad-hoc script — repo guardrail) over `deckbound::solver` and the campaign's `Game` API. Combat
   oracle = the existing `solver::auto_resolve` (greedy, Clash off). Planner = Dijkstra / A* over
   campaign states `(positions, cleared-set, builds, Day)` (Spec §0.1), minimising Days, calling the
   oracle at each `Enter`, with dominance pruning on monotone builds. **Output:** par + a witness
   path + the near-par solution set. First payoff: does the **guide** equal par?
2. **Algorithms — strengthen the combat oracle (the "near-optimal substitute", §5).** Add a slow
   **exact** per-battle search (the hero's decision tree vs the fixed foes — single-agent,
   memoizable per `(build, encounter)`) as ground truth; then a **fast, certified** near-optimal
   policy, error-bounded against the exact search on a sample. Fix this as the canonical resolver
   `P`; every par is **"par under `P`."**
3. **Tuning — the balance loop (§6).** With trustworthy par: express each strategy as a solver
   **constraint**, then tune `booklet.ron` numbers so *interesting* strategies tie near par, *boring*
   ones are strictly worse, and the **closure check** passes (no unnamed dominator). Verify the
   [balance-invariants](balance-invariants.md) registry (BI-1, …). Add the **`(par, robustness)`**
   axis (the near-par basin).
4. **Lock it in.** Wire the solver as the **budget regression test** (§4): the reference scenario
   solves within its state / time budget, or the build fails. Reconcile the guide to computed par.

### 10.2 Locked decisions — do not re-litigate

- The **computable core + separability** is canon: **Spec §0**, **Charter #11**.
- **Par is policy-relative** (§5) — always stamp "par under `P`".
- **Balance method** = interesting > boring + the **closure check** + `(par, robustness)`; the human
  labels, the solver measures (§6).
- **Build invariants** (monotone / additive / order-independent; no carried *combat* state — the
  build is the carried state) are GUARANTEES (Spec §0.1).
- **Numbers are AI-seeded, human-tuned** in `booklet.ron`; never tune a number in the same breath as
  a rule (`0-source-of-truth.md`).
- The **reference scenario is the harness** (`reference-scenario.md`); the guide win (~19 Days) is the
  current par *upper bound*.

### 10.3 Open questions — ask these (batched) at kickoff

- **Par objective:** min Days only, or lexicographic — Days, then fewest upgrades / closest-to-guide?
  This defines the "most straightforward solution" tie-break.
- **Packaging:** a `par-solver` crate, or `examples/par.rs` in `deckbound`? Name it.
- **State key & pruning:** the canonical key for `(positions, cleared, builds, Day)` and the
  dominance rules (earlier / superset build dominates).
- **Strategy constraints:** the language for expressing a named strategy as a search restriction (§6).
- **Tolerances:** `ε` for "on par"; the robustness / near-par-basin metric.
- **Budget thresholds:** the `N` states / `T` seconds for the §4 CI test.
- **Policy rollout:** greedy MVP first, then the certified fast policy; the certification sample size.

### 10.4 Entry points (where the context lives)

**Code (`crates/deckbound/src/`):**
- `solver.rs` — `auto_resolve`, `greedy`: the oracle substrate (Clash-off, deterministic).
- `campaign.rs` — `CampaignState`, `Campaign::{legal_actions, apply, suggest, view}` (the planner
  API), `reference_campaign` (the start state), test `the_guide_wins_the_reference_run` (par baseline).
- `reference.rs` — `check_invariants`, `check_combat_bands`, `reference_scenario` (the harness + gates).
- `game.rs` — `battle_state` (headless battle for the oracle), `nav_level`/`session_key` (state shape).
- `data/booklet.ron` — the numbers to tune (Step 3 only).

**Docs (`docs/games/deckbound/`):** this doc (the discipline) · **Spec §0** (the binding contract) ·
`balance-invariants.md` (the targets) · `reference-scenario.md` (the harness) · Charter **#2 / #4 / #11**
(intent) · `progression-design.md` (the economy / build space the planner searches).

### 10.5 Definition of done

- Par solver computes **par + witness + near-par diversity** for the reference scenario, within the §4
  budget, **wired as a CI test**.
- The combat policy `P` is **fixed and certified** (error-bounded vs the exact search).
- The **balance loop** runs green: interesting tie near par; boring strictly worse; closure check
  passes; **BI-1** (and any further registry invariants) verified.
- The **guide is reconciled** to computed par.

### 10.6 Constraints that protect future-you

- **Build against the `engine::Game` trait + the campaign API + `booklet.ron` data — not hardcoded
  rules.** The designer is revising mechanics now; a tool that reaches the game only through the trait
  and the data survives those revisions (rules change behind the trait; numbers change in data).
- **Do not tune numbers until the designer says the mechanics are vetted.** The measurement tools and
  the algorithms (Steps 1–2) are mechanics-agnostic and *may* be built earlier on request; **Step 3
  (tuning) waits for the explicit go.**

---

**See also:** [Charter](canon/1-charter.md) (#2, #4, #11) · the
[Spec](canon/2-spec/README.md) · [reference-scenario](reference-scenario.md) (the par
target) · [future-possibilities](future-possibilities.md) (the deferred combo layer, and why).
