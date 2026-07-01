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

| Invariant                                                 | Breaks it                                                                                                                      | Why it matters                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **No RNG / hidden info in the canonical mode**            | unseeded randomness; a hidden deck the par mode can't read                                                                     | randomness turns par into an *expectation over* a distribution → you're computing equilibria, not plans                                                                                                                                                                                                                                                                                                                                                                                                                        |
| **Foes are a fixed environment, not an adversary**        | AI that searches/reacts against the player's specific plan; learning foes                                                      | turns single-agent planning into two-player minimax → the chess cliff (PSPACE/EXPTIME, the alternating-quantifier blow-up)                                                                                                                                                                                                                                                                                                                                                                                                     |
| **Battles near-stateless: `f(build, place)`**             | carried wounds, persistent buffs, consumables, fatigue, deck-thinning that persists                                            | the oracle becomes `f(build, place, history)`; the planner must drag an HP/resource vector across battles → state-space explosion                                                                                                                                                                                                                                                                                                                                                                                              |
| **Builds monotone, additive, order-independent**          | **resource-refunding** swaps (sell-back / oscillation → a *path-dependent budget*); order-dependent or *multiplicative* combos | breaks monotone pruning ("more is better") and balloons the reachable build set (this is *why* the aspect/chord combo layer is deferred — see [future-possibilities](future-possibilities.md)). **Precise:** the killer is a **path-dependent resource/budget**, *not* movement per se — freely **rearranging already-owned, monotone assets** (e.g. reassigning a card between characters) keeps the build Markovian and is fine; it's the *refund/consume-recover* that makes a budget history-dependent and kills dominance |
| **Bounded horizon, modest branching, terminating combat** | unbounded productive loops (infinite farming); large continuous boards; many simultaneous independent choices; no round cap    | unbounded horizon makes "min days" ill-posed; high branching/horizon makes the search infeasible                                                                                                                                                                                                                                                                                                                                                                                                                               |

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
  pairs, then run the fast policy at scale. That gives purpose #5 a known error bar. (The detailed
  plan for this exact search is §10.7.)

### 5.1 Deterministic-proxy fidelity — when "par under `P`" equals the human answer (and when it can't)

A natural hope: tune the game against a **100% deterministic** `P` and trust it matches what skilled,
theory-of-mind humans would find. The precise statement: **"par under `P`" differs from the true
human / equilibrium value by exactly the *value of unpredictability* in the game.** Where mixing is worth
nothing, the deterministic number is **not an approximation — it is the answer.** *(The game-agnostic
form of this — pure vs mixed play, and when a deterministic solver is exact — is
[`docs/game-theory/solution-concepts.md`](../../game-theory/solution-concepts.md) §4.)*

- **PvE balance is a maximization, so the deterministic answer is *exact*, not approximate.** Against
  fixed, non-reading instinct foes (§7 / Spec §7), the player's best line **is** the value — there is no
  equilibrium to approximate, no mixed strategy a human could add. So for **raw / resource balance** (an
  over-efficient card, a dominant role, a degenerate build) `P` is a faithful detector, full stop. This is
  the bulk of balance, and the hope holds *exactly* there.
- **Faithful to raw strength; blind to the mind-game layer.** Residual unpredictability-value lives in
  exactly two places: the **Clash** (already quarantined off, §7) and — in the *base* game — the
  **per-round blind bid of positions** (Spec §4: a hidden, simultaneous sub-game). A deterministic,
  predictable `P` is exploitable there, and the mis-rating is **option-dependent, not a constant offset**:
  an option whose value is *being unreadable* (a feint, a bluffed position) is **under-rated**, while a
  pure counter to a predictable foe is **over-rated**. So the blind-bid layer is the one place imbalance
  can hide from `P`, and it distorts *relative* balance — not just the absolute number.
- **"Non-degenerate `P`" means *near-optimal*, not merely rule-complete (cf. §5).** Using every rule a
  human could only guarantees `P` can *reach* a state — not that it *finds* the exploiting line. A weak `P`
  gives **false negatives** (real imbalance, undetected). Same rules ≠ same skill; **certify**
  near-optimality (§5), don't assume it.

**Don't assume the gap is negligible — measure it, and tune it.**

- **Solve the blind-bid sub-game in isolation.** It is tiny and finite (positions × group assignments).
  Compute its equilibrium value and compare to what `P` scores: a small gap *certifies* the deterministic
  proxy; a large one localises where mixing matters.
- **The mispredict penalty is a tuning dial.** If guessing the enemy's positions wrong is a *modest* swing
  (the front still functions, the back isn't instantly lost), unpredictability is worth little and `P`
  stays faithful; if a wrong read is *catastrophic*, the game **demands** mixing and `P` diverges. Keep the
  swing modest to keep the proxy honest.
- **Audit unpredictability-dependent options against a *reading* opponent**, not just the deterministic
  foe — that small set is exactly what `P` mis-rates.

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

**Exploratory analysis toolkit — which question → which method.** The solver emits a huge,
*noise-free* strategy dataset; analyse it to **summarise the computed structure for human judgment**,
not to *infer hidden causes* (the game is white-box — you can compute why a build wins). The
structure that matters is **interactions and thresholds**, so prefer methods that model those over
linear-correlation methods:

| Question (the "I don't know what I'm looking for") | Method                                                                                    |
| -------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| How many real strategic axes / what's the shape?   | PCA / SVD (linear first pass) → **UMAP / t-SNE** for the nonlinear manifold               |
| What are the archetypes?                           | Clustering (HDBSCAN / hierarchical) · **Archetype Analysis** (the literal "pure corners") |
| Which cards are redundant / substitutes?           | Co-occurrence & substitution across *winning* builds                                      |
| Which `booklet.ron` numbers move balance?          | **Global sensitivity analysis** (Sobol / Morris)                                          |
| What drives par?                                   | Gradient-boosted trees + **SHAP** (captures interactions / thresholds)                    |
| Dominant line? interesting on par?                 | The cluster-then-label loop + the **closure check** above — the solver, not a statistic   |

*Caveat on factor analysis.* Classical **EFA is a poor fit** here: it models linear, Gaussian,
*noisy* indicators of latent traits, but this data is **deterministic** (no error variance → Heywood /
degenerate; PCA dominates EFA), **threshold-y and interaction-driven** (linear correlation is blind to
the gating / combo structure that matters most), and **white-box** (compute the latents, don't infer
them). One apt use: PCA / FA on the **card × stat *design* matrix** (continuous) to check that the
card set spans the intended axes — i.e. that the roles actually separate (a check on BI-1's premise).

**The concrete targets live in a registry.** The specific, checkable balance properties the tuned
numbers must satisfy — each an instance of "interesting beats boring" or "interesting on par" — are
catalogued in [balance-invariants.md](balance-invariants.md). When the solver lands, each becomes an
assertion it runs (§4), so a retune that breaks one fails the build.

### 6.1 The necessity test — every mechanic must earn a scenario it is required to win

The **closure check** (step 3 above) is one half of "the rules are right": it proves **no unintended
*strategy* wins**. The **necessity test** is the other half — it proves **every intended *mechanic*
matters** — and it is just as executable.

**RULE.** For each mechanic `M`, build a scenario with two lines: a **naive line** that ignores `M` and a
**keyed line** that uses it. The scenario is valid iff the naive line **provably loses** and the keyed
line **wins**. Run both through the solver / combat-lab and assert `naive = loss ∧ keyed = win`.

**Why it is the dual of the closure check.** Closure: *no unnamed strategy beats the intended set* —
catches degenerate **strategies**. Necessity: *every mechanic has a scenario unwinnable without it* —
catches fiat / redundant **mechanics**. Together they bound the design from both sides: nothing
unintended wins, and nothing intended is dead weight.

**Necessity as an audit (the removal test, made runnable).** A mechanic for which **no** such scenario can
be built is **suspect** — either **fiat** (it "wins" only by banning the alternative, not by being
out-played) or **redundant** (another mechanic already forces the same line). Delete `M`: if some scenario
flips from forced-loss to winnable, `M` was load-bearing there; if none does, `M` earns nothing. This is
the emergent-not-fiat removal test executed, not argued.

**The double payoff — one artifact, three uses.**

- a **regression assertion** (like the §4 budget / BI checks): a retune that lets a naive line win
  **breaks the build**, flagging that a mechanic silently stopped mattering;
- a **player tutorial**: the same forced scenario teaches `M` with clean **credit assignment** (you cannot
  win without the insight) — the deferred tutorial-series plan
  ([tutorial-design](../../../log-driven/brainstorming/tutorial-design.md)) *is* this suite, read for
  teaching instead of testing;
- a **coverage ledger**: the mechanics with **no** passing scenario are the live audit / cut list.

**Ordering.** Mechanics compose, so the scenarios form a **dependency graph**: a scenario may *use*
already-tested mechanics as prerequisites but must **force exactly one new** one. A topological sort gives
both the test order and the tutorial order at once.

### 6.2 Class discovery — generate-and-test for a balanced ecology

The two checks above tune a **named** roster. The dual question is generative: *does a balanced roster even
exist in the design space, or must classes be hand-propped?* Once **capabilities are cards** (the base
strike included — its range and area are read from the strike card, §4.3 / [[role-card-redesign]]), a class
is nothing but **a strike card from the matrix (melee/ranged × single/aoe) plus a stat allocation**, and its
**role emerges** from range+stats (never a free input). So the whole class space is finite and enumerable —
and every candidate is **built and fought on the real engine** (no parallel sim to drift, §11).

**Capability budgeting — range is free, area is paid.** The two capability axes are *not* symmetric.
**Melee↔ranged is a positional tradeoff** priced **structurally by §4.2** (ranged is safe but evadable and
needs a living screen; melee is the shield with the reflexive trade but can't reach the back) — neither
strictly beats the other, so it costs **no** points. **Area is different:** an AoE strike hits the *whole*
enemy group, unevadable, past the bodyguard, with no Might cut — at equal stats it is **weakly dominant**
(never worse, often much better). A free strictly-better capability is a degeneracy, so the area card costs
**`K` of the stat budget**: an AoE class gets `BUDGET − K` points. This is capabilities-as-cards taken to its
conclusion — a capability you can't out-trade is a capability you must *pay* for.

**METHOD.** Enumerate every candidate (the 4 capability cells × all allocations of that cell's budget),
field each as a **grouped party** (NvN — AoE's advantage *only exists against a group*, so a 1v1 sweep
cannot price it), **round-robin** them (both side-assignments, to cancel side bias), and **scan `K`**. At
each cost read the **tournament's shape**:

- **No dominant class** — none beats the entire field (a strictly-best class is a balance failure);
- **No dead class** — none loses to the entire field (dead weight);
- **AoE win% ≈ single win%** — the area price is right when the AoE cells no longer out-perform the single
  cells (at `K=0` they do, by construction); the smallest `K` that levels them is the capability's cost;
- **A non-transitive RPS cycle** `A ▸ B ▸ C ▸ A` spanning **distinct cells and roles** — positive evidence
  the field has *no* strict pecking order (a total order is the failure mode: someone is best).

This is the **ecology-level** analogue of the §6 closure check (no dominant *strategy*) and the §6.1
necessity test (no dead *mechanic*): here it is no dominant / dead **class**, a priced area capability, and a
live counter-cycle.

**TOOL.** `cargo run -p deckbound --example discover` runs the NvN sweep on the real engine and scans the AoE
cost. First result (8-point budget, 3v3, each stat ≥ 1): the AoE cells' mean win-rate falls **61% → 57% →
49% → 34%** as `K` goes `0 → 1 → 2 → 3` while the single cells hold ~**40–50%** — so **area is worth ~2 stat
points**. At `K=2` AoE (49%) ↔ single (50%) level out, **0 dominant, 0 dead**, and a 3-cell / 3-role RPS
cycle survives (e.g. melee·single Outrider ▸ ranged·single Rearguard ▸ melee·aoe Vanguard). The sweep also
searches for a **4-cell Hamiltonian cycle** — one class per cell forming a directed loop `A ▸ B ▸ C ▸ D ▸ A`
(a Hamiltonian cycle ⟹ the quartet is strongly connected, Moon's theorem, so *no* cell dominates or is
dominated) — the stronger witness that **all four** capability cells coexist; at `K=2` one exists (e.g.
melee·single Outrider ▸ ranged·single Rearguard ▸ ranged·aoe Rearguard ▸ melee·aoe Vanguard). So meaningful
balanced classes emerge from combining range × shape × stats *once area is priced* — they need not be
fiat-authored. The per-cell exemplars are candidates for the hand-tuned roster, then locked by §6 / §6.1.

> **The asymmetric, two-population successor to this section** — party compositions *vs* creature
> compositions as the rows and columns of a matrix game, dual-tuned via double-oracle / EGTA, with RPS
> read off as the matrix's cyclic component — is written up as a deferred build in
> [§10.8](#108-the-composition-metagame--dual-tuning-parties-vs-creatures-the-game-theory).

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
   `P`; every par is **"par under `P`."** **Detailed runbook: §10.7 (the exact battle solver).**
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

### 10.7 The exact battle solver — perfect PvE combat play (the §10.1-step-2 oracle, detailed)

> **STATUS — built 2026-06-26 (Phases A–C + E luck-off; D deferred).** Implemented in
> [`crate::solver`](../../../crates/deckbound/src/solver.rs) (`solve`, `winnable`, `Solution`), alongside
> the greedy `auto_resolve`/`greedy` it augments. It is a **memoized backward-induction search over the
> existing `Game` loop**: the foe AI runs *inside* `apply` (`foe_fray`/`foe_volley`) and `legal_actions`
> only ever offers the committing side — **always the heroes in PvE** — so every hero action has a single
> successor and the search is single-agent (no minimax). The engine sequences a phase's commitments as
> single-action steps, so order-independence (§1.9) is collapsed by the **transposition table** keyed on a
> canonical `state_key`, not a separate set-enumerator. **Verified:** toy known-answers (winnable /
> not-winnable-without-damage) and the **optimal ≥ greedy** invariant on every small campaign scenario
> (`solver::tests`); `cargo test probe_solver -- --ignored --nocapture` prints per-scenario verdict / par /
> node counts.
>
> **Empirical width (the §10.7 "validate the branching factor" step):** 2v2 encounters are tiny (Ward 84
> nodes / par 1; Hold & Rain 5 342 / par 2). **Reachability (`winnable`) scales to the full roster** — the
> 11-unit "The Five" resolves quickly — because of the **early-cutoff + symmetry pruning + greedy
> move-ordering** in `Reach`. The **graded `solve`** (battle-par) is exact and cheap on small/medium
> rosters but expensive on the largest *distinct*-hero scenarios (5 unique heroes ⇒ a 2⁵ position space ×
> per-phase plays with **no symmetry to collapse**); a node budget (`MAX_NODES`) makes it return
> `overflowed` rather than hang. **Remaining levers (future perf, exactness-preserving):** **dominance
> pruning** (the real fix for the distinct-hero graded case) and **full swarm canonicalization** of the
> `state_key` (merge permutations of identical units — helps graded par on swarms like the six-Husk
> Swarm). The load-bearing instrument the consumers need — boolean reachability / the difficulty frontier —
> already scales; graded par is the refinement.

**Ratified 2026-06-26.** The detailed runbook for **step 2** above: replace the greedy combat oracle
(`solver::auto_resolve`) with one that computes **exact optimal battle play** (no heuristic). It *is* the
"slow but exact per-battle search" §5 calls for as ground truth — and because the **analysis envelope**
(Spec §0.4) bounds the battle, it is *exactly searchable*, so the same search doubles as the **certified
canonical resolver `P`** at analysis scale. It is also the **strong policy** the role-weight /
marginal-contribution measurement depends on: it closes the **policy-relativity** pitfall (§5 / §5.1) — the
thing that once made the Controller read as dead weight under greedy (same cards, weak policy, wrong
verdict).

**Why it's tractable — PvE is a finite-horizon single-agent MDP, not a game.** A PvE battle is *not* a
two-player game: creatures run a **fixed, non-adaptive instinct** (Spec §0.1) — a fixed-policy
*environment*, not a best-responder. So perfect play is **exact backward induction over a finite
horizon**: no equilibrium, no heuristic. The **blind bid** (Spec §4) is therefore *benign* — a fixed foe
cannot react to your hidden commit, so you never need to randomize to stay unexploitable ⇒ **the optimal
PvE policy is pure (deterministic)**. True minimax / mixed-Nash hardness returns **only in PvP** (both
sides adaptive), quarantined (§7) and out of scope. Two modes:

- **Luck OFF** (open creature commit, deterministic — Spec §0.2): collapses to **reachability** — "does
  there exist a player line to a win-leaf within the round cap?" Pure existential search (no adversary
  nodes), the Spec §0.4 "winnable within the horizon?" boolean. Optimal policy = the winning line.
- **Luck ON** (creature bid/decks hidden, RNG on): a finite-horizon **MDP** — the creature's *known fixed*
  distribution + RNG draws are **chance nodes**; solve for **max expected value** by expectimax / value
  iteration over the bounded horizon. Exact *iff* the distributions are finite/enumerable (they are —
  Spec §0.1). Still pure, still single-agent (no strategic uncertainty — just an expectation).

Operating envelope: run under **`Ruleset::analysis()`** (Spec §0.4 — 5-round horizon, ≤5 roster types,
swarm-as-one). That bound is what makes the state space finite and exactly searchable; live play's
unbounded `Ruleset` is not the solver's concern.

**Decision points (what the search branches on).** Per round the player commits at three nodes;
everything else (Breach / Reckoning / Lull) resolves automatically (Spec §4.6):

1. **Standoff bid** — positioning (each hero Vanguard/Rearguard) × group partition × which `Standing`
   abilities to cast. (The creature's bid is a fixed/known input or a known distribution — not a
   co-decision.)
2. **Fray commit** — the *set* of (actor → ability → target) plays + defensive responses, bounded by Tempo.
3. **Volley commit** — free Vanguards' charges/flanks + targets, instant re-fires, the rear's pre-empt answers.

**Spec §1.9 order-independence is the key lever:** within a phase the player commits a *set*, not a
sequence — so the search branches on *subsets of commitments*, not permutations (a factorial reduction).

**State / transition / leaf.**

- **State** = the `State` struct's combat fields (per-actor Health, Tempo, tokens, position, lock/charge/
  deferred status, per-phase pile, phase, round). Needs a **canonical, hashable encoding** for a
  **transposition table** (combat is a memoizable oracle — Spec §0.1).
- **Transition** = the existing resolver (`combat.rs`: `fray_clash` / `resolve_volley` / `resolve_breach`
  / `resolve_reckoning` / `tally` / `clear_phase_piles`). The solver *applies committed actions and reads
  the next state* — it does not reimplement combat.
- **Leaf** = terminal by rule (foes dead → win; party dead or round cap → loss/draw). **Exact value, no
  evaluation heuristic** (Spec §0.4) — the whole point of the bounded horizon.

**Objective (lexicographic; configurable).** Primary **win/loss** (reachability), tiebroken by **fewer
rounds** (the battle-par metric), then **fewest party characters downed** (losing a whole unit ≫ chip
damage), then **most Health remaining** (survival margin). So the "optimal line" is also the battle-par
line, and among par lines keeps the most bodies standing. Swap the leaf value for the **graded balance
metrics** (rounds-to-clear, difficulty frontier) the role-weight measurement needs — same search,
different leaf value. *(This is **battle** par; the **campaign** objective — min Days — is the separate
§10.3 open question for step 1.)*

**The one real risk — branching factor.** Finiteness is guaranteed (Spec §0.4); **speed** is the risk
(per-phase commitment sets are a power-set of legal plays). Levers, all **exactness-preserving**:

- **order-independence** (Spec §1.9) — commit sets, not sequences;
- **Tempo budget** — bounds actions/actor/round;
- **transposition table** — memoize canonical states (collapses transpositions);
- **dominance pruning** (Spec §0.1 monotonicity) — prune *provably* dominated commitments only (e.g. a
  superset of beneficial buffs / a strictly-stronger target), so it stays perfect;
- **symmetry** — swarm-as-one + identical-unit canonical ordering (Spec §0.4);
- **greedy as a move-ordering oracle** — try greedy's move first to find a winning line early (boolean
  cutoff); speed only, never correctness.

**Validate width empirically on real encounters in Phase B** — the reference campaign resolves in ~3
rounds under greedy (Spec §0.4 note), so depth is small in practice; confirm width is too.

**Build phases (incremental, each verifiable).**

- **A — ✅ legal-action enumerator + canonical state hash.** `combat_actions` (legal moves minus the
  `ToMenu` escape) + `state_key` (round/phase + per-actor mutable state + the round plan; tokens and the
  attacked-map are sorted so orderings canonicalize). The engine *is* the per-phase set-sequencer.
- **B — ✅ reachability search, luck-off, boolean objective** + transposition table. `winnable` / `Reach`,
  with early cutoff + greedy move-ordering. Validated on toy known-answers and the **optimal ≥ greedy**
  invariant.
- **C — ✅ graded objectives** via backward induction with the lexicographic value (`solve` → `Solution`
  `{ win, rounds, downed, health, line }`). Swap the leaf value for rounds-to-clear / the difficulty
  frontier the role-weight measurement needs.
- **D — ⬜ luck-on expectimax** (deferred per the ratified first cut) — chance nodes over creature fixed
  distributions + RNG; exact value iteration over the finite horizon.
- **E — ◐ perf + wiring.** Done: **symmetry pruning** (collapse interchangeable identical-foe targets) +
  the boolean early-cutoff, which make reachability scale to the full roster; API exposed
  (`deckbound::{solve, winnable, Solution}`) as the strong policy for the par-tooling / role-weight /
  encounter-suite consumers. **Remaining:** **dominance pruning** and **full swarm canonicalization** of
  the state key, for graded par on the largest (distinct-hero / deep-swarm) encounters.

**API (sketch).** `solve(party, encounter, ruleset, objective) -> { value, optimal_line }` in
`deckbound::solver` (replaces/augments `greedy`). `optimal_line` = the perfect-play trace (the battle-par
line; also a readable transcript and the strong policy for role-weight).

**Correctness.** Toy known-answer scenarios; determinism (seeded → identical); the **optimal ≥ greedy**
invariant; and (later) mutual cross-check with the encounter suite (the suite stress-tests the solver, the
solver validates the suite's niches).

**Ratified design calls (2026-06-26) — do not re-litigate.**

1. **Objective** — lexicographic **win → fewer rounds → fewest characters downed → most Health remaining**
   (downs outrank Health: losing a whole unit ≫ chip damage).
2. **First cut** — **luck-off deterministic only** (Phases A–C); defer luck-on expectimax (D).
3. **Pruning** — start with **provably-exact** levers only (transposition + order-independence); add
   dominance pruning **only if** Phase B shows width pressure.

### 10.8 The composition metagame — dual-tuning parties vs creatures (the game theory)

**Status: deferred (the natural successor to §6.2).** §6.2 discovers a balanced *class* ecology by a
symmetric class-vs-class round-robin. The dual, harder question the designer actually wants is
**asymmetric and two-population:** given the whole design space, does a set of **party compositions** and
a set of **creature compositions** exist that *hard-counter each other in an RPS-style cycle* — and can we
find those "interesting" pairings without brute force? The full space (5 stats × melee/ranged ×
single/aoe × party grouping × in-combat decisions, on both sides) is uncountable by enumeration. It is not
uncountable by **game theory**: what looks impossible is a stack of three nested games, only the outermost
of which we search.

**The three nested games (only the inner one is already built).**

1. **Combat (inner).** Fixed party vs fixed creature group → who wins. This is the **exact solver**
   (`deckbound::{solve, winnable}`, §10.7). It emits a *scalar* — win / margin / rounds-to-clear. Perfect
   PvE play is the whole point: the cell value must be the *skill ceiling*, not a heuristic, or the
   metagame above measures the AI's mistakes instead of the design.
2. **Composition (middle).** Rows = party comps, columns = creature comps, each cell = the solved combat
   value. This is a **zero-sum matrix game**; von Neumann **minimax** applies verbatim. "Balance" here is
   *not vibes* — it is measurable structure on this matrix (below).
3. **Parameter tuning (outer).** Choosing stat costs, the AoE price `K` (§6.2), range/shape availability,
   creature budgets — so that the matrix at layer 2 has the shape we want. This is the only layer a human
   taste-judgment enters; it is a **coevolutionary** search (below).

**What "balanced + interesting" means precisely on the layer-2 matrix.** Fill the cells with the solver,
then read the matrix as a game, not a spreadsheet:

- **Dominance** — a party (or creature) comp that is weakly dominated appears in *no* equilibrium: dead
  design weight. Killing dominated comps is the rigorous form of §6.2's "no dead class," lifted to
  compositions.
- **Equilibrium support** — solve the matrix game (an LP for zero-sum) for its mixed-strategy Nash; the
  **number of comps with positive probability** is the diversity metric. A flat pecking order collapses to
  support 1; a healthy metagame has broad support.
- **Exploitability** — how much a best-responder gains against a fixed strategy; low across the board ⇒ no
  degenerate meta.
- **RPS = the cyclic component of the matrix.** Any pairwise-outcome matrix splits into a **transitive
  part** (a global power ranking — this *is* power creep / a pecking order) plus a **cyclic/intransitive
  part** (genuine A ▸ B ▸ C ▸ A) plus a harmonic remainder. The formal tools: **combinatorial Hodge
  decomposition** (Jiang–Lim–Yao–Ye, *Statistical ranking and combinatorial Hodge theory*) and, for games
  specifically, **Candogan–Menache–Ozdaglar–Parrilo, *Flows and Decompositions of Games* (2011)** —
  potential (transitive) + harmonic (RPS-like) + nonstrategic. Czarnecki et al., *Real World Games Look
  Like Spinning Tops* (2020) is the geometry we're aiming the design at: a modest transitive axis wrapped
  in a *fat cyclic disk* of viable counters. So **"find the interesting hard counters that pop out" =
  maximize the cyclic energy of the composition matrix while keeping the transitive component small.**
  That is a *number* to tune against, and it is the same object §6.2 already gropes at by hand with its RPS
  cycle and its Moon's-theorem Hamiltonian witness (a Hamiltonian cycle ⟹ strongly connected ⟹ nonzero
  cyclic energy).

**Searching it without brute force — the designer's own instinct, named.** "Start with a sample party and
creature set, run a subset, tune, expand" is a published method:

- **Empirical Game-Theoretic Analysis (EGTA)** (Wellman) — never enumerate the space; sample a *restricted*
  set of comps, fill that submatrix with the solver, solve it, treat it as an approximation of the true
  metagame. §6.2's round-robin tournament *is* an EGTA payoff matrix already.
- **Double Oracle / PSRO** (McMahan; Lanctot et al., DeepMind) — iterate: (1) compute the Nash equilibrium
  over the current pool, (2) use the solver as a **best-response oracle** to find the strongest *new* party
  vs the current creature meta, and the strongest new creature vs the current party meta, (3) add both,
  repeat. Provably converges to the full-game equilibrium while only ever touching a tiny submatrix. This
  is the principled engine for **"dual-tuning party vs creature compositions simultaneously,"** and the
  exact combat solver (§10.7) is exactly the oracle it needs.

**The dual tuning itself is competitive coevolution — with a known failure mode.** Tuning both populations
against each other is **coevolution**, whose classic hazard is **Red Queen dynamics**: both sides chase
each other in a cycle and *relative* progress masquerades as *absolute* balance, or the search loses
gradient and oscillates. Standard fixes, all worth building in: keep a **Hall of Fame / Nash memory**
(evaluate each candidate against a frozen historical archive, not just today's opponent), and make the
**equilibrium** the evaluation target rather than the latest opponent — which is precisely what PSRO does,
and why it is preferred over naive coevolution.

**Design caution — counter hardness is a knob, not a goal.** Deterministic 100/0 counters maximize cyclic
energy but degrade the game to a pre-combat guessing match: whoever reads the opponent wins and in-combat
skill (layer 1) goes vestigial. Softer counters (≈65/35) keep the cycle *and* preserve mixed strategies
and live decisions. This is the same tension §5.1 and §7 already fence off: **the solver measures the raw
transitive/structural axis exactly and is blind to the blind-bid / Clash layer** — so it is the right
instrument for the matrix's *structure* (steps below) but it will not price the mind-game for you. Balance
in isolation is a floor, not the target (§11).

**Concrete build (a new `examples/` consumer, or an extension of `discover` — never an ad-hoc script, repo
guardrail).**

1. Seed a pool of hand-built party comps and creature comps.
2. Fill the matrix with `deckbound::solve` (exact value per cell).
3. Solve the zero-sum matrix game (LP) → equilibrium, support size, exploitability.
4. Run the Hodge / Candogan split → report **transitive vs cyclic energy**.
5. **Double-oracle expand:** the solver returns the best-response party to the creature meta and vice
   versa; add both; repeat until no profitable deviation.
6. **Tune the layer-3 parameters** (stat costs, the §6.2 AoE price `K`, creature budgets) to push the
   objective: *shrink the transitive axis, grow the cyclic disk, raise support, kill dominated comps.*

The result generalizes §6.2 from "does a balanced *roster* exist" to "does a balanced *matchup ecology*
exist, and where are its hard counters" — with the solver as the oracle throughout and the human owning
only the objective (§11).

---

## 11. Division of labour — human and AI, with the solver

The instrument changes who does what. The binding short form is in
[`0-source-of-truth.md`](canon/0-source-of-truth.md) ("Division of labour — with the solver"); this is
the detailed version.

**The principle.** *You (human) own the **objective** and the **taste**; I (AI) own the
**optimisation** against it.* This **refines** the canon's "AI seeds numbers, human tunes": once an
objective is **computable and agreed** (the par solver + the balance invariants), tuning *to* it is
optimisation, not judgment — so it becomes AI-ownable, *proposed for ratification*. What stays yours
is everything the objective can't see.

**The net catches *broken*, not *unfun*.** The instrument runs on the **deterministic core**, so it
is objective about **structural balance** — dominance, dead / dominated options, redundancy,
strategy-space collapse, unbeatable or trivial gates, role non-separation, budget breaks — and
**blind** to fun, feel, pacing, theme, ergonomics, and the *experience* of the luck / hidden-info
layers it strips away. "Balanced in isolation" (neutral EV / no dominant exploit) is a **floor** a
luck layer must clear, not the point — a coin-flip clears it and is miserable.

**What AI is empowered to handle (given the tools):**

- **Build and run the instrument** — solver, sensitivity analysis, clustering / manifold exploration,
  the budget test.
- **Detect and flag structural pathologies** — dominant strategies (closure check), dead / dominated
  options, redundant or substitutable cards, dimensionality collapse, unbeatable / free gates, roles
  that don't separate, computability-budget breaks.
- **Tune numbers to the stated objective + invariants** — propose `booklet.ron` configurations that
  hit the par targets, BI-1, diversity, and the no-dominator check, *for ratification* (it's
  optimisation now, not taste).
- **Generate candidates and surface structure** — draft cards / mechanics for keep-or-cut; cluster the
  strategy space into archetypes and hand them over for labelling.
- **Implement, test, verify, keep the canon coherent** — code, the spec-sync discipline, regenerated
  projections.
- **Certify the instrument's internals** — e.g. bound the combat policy `P`'s error vs the exact search.

**What still needs the human (the tools are blind to these):**

- **Author the objective and the taste** — what "balanced / interesting / fun / good" *means*; the
  balance invariants; the interesting-vs-boring labels; the Charter north stars. The solver enforces;
  you author.
- **Judge fun and feel** — decision density, tension, drama, pacing, the doom-to-mastery curve,
  ergonomics — mostly subjective, and mostly in the *played* (luck / hidden-info) game the core never
  sees.
- **Design the luck layers as *experience*** — inventing a luck mechanic that is *fun* (and judging its
  drama) is a creative act; "neutral and non-dominant" is a bar it must clear, not its purpose.
- **Judge model faithfulness — when to trust the instrument** — does core-balance transfer to the
  played game? is `P` human-like? does the featurisation capture what matters? are these the right
  invariants? (The guard against "balanced for a bot.")
- **Decide what exists, and any intent change** — what's in / out, theme, identity, any Charter or
  Spec-WHY change (the canon already reserves intent to the human).
- **Ratify** the AI's tuned configs and flagged findings — the final taste pass, especially the fun
  check the solver can't make.

**The one-line test.** *Is it checkable against a stated, computable objective?* If yes → AI can own
it (measure, tune, flag, implement). If it **defines** the objective, judges **fun / feel /
faithfulness**, or changes **what the game is** → human.

**The duty you can't delegate.** Because AI optimises *confidently* to whatever objective it is given,
an **incomplete objective yields a confidently balanced-but-soulless result.** So the human's residual
role includes **vigilance** — noticing when "passes every check" still isn't the game you want. The net
frees you from playtesting to find what's **broken**; not from playing to feel what's **alive**.

---

**See also:** [Charter](canon/1-charter.md) (#2, #4, #11) · the
[Spec](canon/2-spec/README.md) · [reference-scenario](reference-scenario.md) (the par
target) · [future-possibilities](future-possibilities.md) (the deferred combo layer, and why) · the
general game theory in [`docs/game-theory/`](../../game-theory/README.md) (single-agent planning,
solution concepts, the value of unpredictability, cooperative/Shapley).
