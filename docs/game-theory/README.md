# Game Theory for Game Design

This directory collects the game theory used to **analyze and balance** games — what "optimal play"
means, how to compute it, and which structural properties a healthy design must have. The material
divides into a **foundational layer** (classify any game, then pick a solution method) and a
**counter-system layer** (the detailed treatment of one important cell: simultaneous, two-player,
zero-sum systems like Rock-Paper-Scissors).

These documents are deliberately **game-agnostic.** A specific game *applies* them; the application
(and the cross-references back here) lives in that game's own docs — see "How this maps onto Deckbound"
below.

## Read in this order

**Foundations — applies to any game**

1. [`game-classification.md`](game-classification.md) — the axes that classify a game (players ·
   adaptivity · information · timing · sum) and the key reductions (fixed opponents ⇒ single-agent
   planning; simultaneous + adaptive + hidden ⇒ you must randomize). **Classify first — it decides
   everything else.**
2. [`solution-concepts.md`](solution-concepts.md) — how to actually solve each cell: search / backward
   induction / DP (single-agent), expectimax / MDP (environment randomness), minimax (adversarial
   perfect-info), mixed-strategy Nash via LP (simultaneous / hidden), and the **value of
   unpredictability** (when a deterministic solver is exact vs when it mis-rates options).
3. [`cooperative-and-marginal-value.md`](cooperative-and-marginal-value.md) — teams, not opponents:
   marginal contribution & the **Shapley value** for "does this role pull its weight," the specialist
   (high-max / low-average) test, and the pitfalls (policy-relativity, coverage, profiles).

**Counter systems — the simultaneous zero-sum cell, in depth**

4. [`hierarchy-of-concerns.md`](hierarchy-of-concerns.md) — the ordered properties a counter system
   must have (no Condorcet winner → regularity → uniform Nash → strong connectivity → Hamiltonian cycle
   → cognitive load).
5. [`nested-counter-systems.md`](nested-counter-systems.md) — counter systems at multiple levels
   (faction / strategy / unit) and why the design goal differs by level.
6. [`measurement-mechanics.md`](measurement-mechanics.md) — concrete algorithms and formulas for
   measuring each counter-system property.
7. [`examples-done-right.md`](examples-done-right.md) · [`examples-done-wrong.md`](examples-done-wrong.md)
   — worked analyses against the hierarchy.

## How this maps onto Deckbound

The concepts above are general; Deckbound applies them, and the application lives in the Spec and the
computability doc:

- **Single-agent planning** (PvE vs fixed-instinct foes ⇒ a *plan*, not an equilibrium) — Spec §0.1 /
  §0.4; the exact battle solver, `computability-and-balance.md` §10.7.
- **Bounded backward induction / reachability / expectimax** — Spec §0.4 (the analysis envelope); §10.7
  (luck-off = reachability, luck-on = expectimax over fixed creature distributions).
- **Counter systems** — the Clash (Spec §1.0) and the Aggressor ▸ Glass-Cannon ▸ Turtle playstyle RPS
  (Spec §4); analyze with docs 4–6 above.
- **Mixed strategies & the value of unpredictability** — the per-round blind bid (Spec §4); the
  deterministic-proxy fidelity rule, `computability-and-balance.md` §5.1.
- **Cooperative / Shapley** — the role-weight / "does each role pull its weight" measurement
  (`computability-and-balance.md` §10; the marginal-contribution framework).
