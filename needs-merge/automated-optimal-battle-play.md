# Automated optimal (perfect) battle play — why it's tractable

> **Design note, staged 2026-06-26 (discussion record).** Whether/how to compute **perfect gameplay**
> for a battle automatically, given the bounded horizon and the *optional* luck/hidden-info layers.
> **Promotion target: `docs/game-theory/`** (sit beside `measurement-mechanics.md`; pairs with
> `role-weight-balance-testing.md` — this is the **strong policy** that measurement needs). Captured in
> `needs-merge/` per the parallel-instance convention while a spec-sync runs; promote, don't lose.

## The crux — PvE is single-agent, so the blind bid is *benign*
Perfect play in a 2-player **simultaneous-hidden** game = a **mixed-strategy Nash equilibrium** (hard:
you must randomize to stay unexploitable). A PvE battle **is not that game**: creatures run a **fixed,
non-adaptive instinct** (§0.1) — an *environment*, not a best-responder.

- **Hidden info only bites when the opponent exploits it.** A fixed creature cannot react to your blind
  bid, so you never need to randomize to be unexploitable → **the optimal PvE policy is PURE
  (deterministic)**, even with the blind bid.
- A battle is therefore a **single-agent bounded planning problem**, not a game:
  - **Luck/hidden OFF** (open the creature's commit — §0.2 is a switch): deterministic,
    perfect-information → **exact bounded backward induction**. Pure perfect play, **no evaluation
    heuristic** — exactly what §0.4's bounded envelope was built for.
  - **Luck/hidden ON** (creature's bid concealed): choose against the creature's **known fixed
    distribution** → **expectimax** over it + backward induction → exact *expected*-optimal, still pure,
    still single-agent.
- True Nash/mixed hardness returns **only in PvP/Versus** (both adaptive) — quarantined (§7), off in tuning.

## Why it's tractable (exact, small)
- **5-round horizon + roster cap** (§0.4, swarm-as-one) → tiny state space; leaves terminal-by-rule
  (draw on cap) → backward induction exact, no heuristic.
- **Order-independence within a phase** (§1.9) → you commit a *set* of actions, not an ordering →
  collapses the within-phase branching a naive turn-by-turn tree would suffer.
- **The blind bid is a small finite choice** — positioning constrained (melee→Vanguard, ranged→Rearguard),
  few sensible groupings → a modest extra branch at the Standoff, evaluated like any decision.

## The blindness, precisely
"Blind" matters only if the opponent's choice depends on info hidden from you AND it adapts. In PvE:
- creature bid **open** ⇒ pure planning (you see it);
- creature bid **hidden** ⇒ expectimax over its *known fixed* distribution (no strategic uncertainty —
  just an expectation).
Your own bid being hidden from the creature is irrelevant: the creature doesn't adapt, so it can't
punish you. Hence **pure** optimal play suffices in PvE; mixing is a PvP-only need.

## Build path
Implement the exact bounded search in **`deckbound::solver`** (`computability-and-balance.md` §8),
two modes: (a) luck-off deterministic backward induction (cleanest for tuning); (b) luck-on expectimax
over the creatures' fixed instinct distributions. Prereq for (b): each creature's bid is a *defined*
fixed/stochastic policy (§0.1 already mandates this). Reuses the existing resolver (`combat.rs`) as the
state-transition; replaces/augments the greedy.

## Bonus — it unlocks honest role-weight measurement
This optimal policy is the **strong policy** `role-weight-balance-testing.md` requires: it closes the
**policy-relativity** pitfall (the thing that made the Controller read as dead weight under greedy —
same cards, weak policy, wrong verdict). **Build the optimal solver once; it serves both** perfect-play
tuning and honest marginal-contribution / Shapley role numbers.

## Summary
Perfect PvE battle play is a **pure, exact, bounded (expecti)max search** — easy *because the adversary
was made optional*. The blind bid adds a small branch, not game-theoretic hardness, since a fixed
instinct can't punish a hidden commit. Equilibrium/mixing only ever appears in PvP.
