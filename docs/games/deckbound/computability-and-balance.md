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
- **Battles are near-stateless functions of `(build, place)`.** Each `Enter` rebuilds fresh
  actors from `base + upgrades`, days reset the tokens, and a win clears the location in one
  fight. So no damage is carried between battles — the only thing that flows is the
  **economy** (clears → currency → upgrades → a stronger build). Combat outcome is therefore
  a **memoizable oracle** `clears(build, place) → win/lose (+ margin)`.
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
  to CI and make the guide honest by comparing it to the computed par.

---

## 9. The one-line test

> Before any new mechanic: *does this keep the canonical mode deterministic, single-agent,
> near-stateless between battles, monotone in builds, and bounded?* If not — is it confined
> to an optional mode, or explicitly bounded? If you can't answer yes, you are spending the
> computability budget. Spend it on purpose, and write it down here.

---

**See also:** [Charter](canon/1-charter.md) (#2, #4, #11) · the
[Spec](canon/2-spec/README.md) · [reference-scenario](reference-scenario.md) (the par
target) · [future-possibilities](future-possibilities.md) (the deferred combo layer, and why).
