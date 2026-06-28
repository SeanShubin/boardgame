# Classifying Games — the axes that decide how to analyze one

Before measuring or balancing a game, **classify it.** The analysis method, the meaning of "optimal
play," and even whether a clean optimum *exists* are all decided by a few independent axes. The other
counter-system documents in this directory treat one specific cell of this space — two-player,
simultaneous, zero-sum, single-shot (Rock-Paper-Scissors and its kin). This document maps the *whole*
space so you know which tools apply, and so you don't reach for an equilibrium when a plain search would
do (or vice versa).

---

## The axes

1. **Number of decision-makers.**
   - **Single-agent (one optimizer).** One actor chooses; everything else is a fixed environment.
     *There is no opponent.* This includes any game played against **fixed, non-adaptive opponents** —
     they are part of the environment, not players, because they do not choose *in response to you*.
   - **Two-player.** Two actors whose payoffs interact.
   - **n-player.** Three or more; coalitions become possible (see cooperative game theory).

2. **Adaptivity of the others — the axis that matters most.** Even with "opponents" on the board, what
   counts is whether they **respond to your specific choice.** A **fixed policy** (scripted AI, a fixed
   distribution, a deck shuffled the same way regardless of what you do) is an *environment*, not an
   adversary — and it collapses a "game" down to single-agent planning. An **adaptive** opponent (one
   who best-responds, learns, or reads you) is a true adversary and brings the hard machinery.

3. **Information.**
   - **Perfect vs imperfect.** Can each decision-maker see the full state when choosing? Hidden cards,
     fog, and face-down commitments make it imperfect.
   - **Determinism is a *separate* axis from information.** A shuffled-but-unrevealed deck is
     *deterministic* (its order is already fixed) yet *hidden*. "Open it" buys perfect information;
     "consume no randomness" buys determinism. Conflating the two is a common analysis error.

4. **Timing.**
   - **Simultaneous — a matrix game.** All commit at once; no one sees the others first. Counter systems
     live here.
   - **Sequential — an extensive-form game (a tree).** Moves are ordered; later movers may observe
     earlier moves.

5. **Sum / alignment.**
   - **Zero-sum / strictly competitive** — one party's gain is another's loss.
   - **General-sum** — payoffs can be partly aligned.
   - **Cooperative** — players form coalitions and share a joint payoff; the question becomes *how much
     each member contributes* (see `cooperative-and-marginal-value.md`).

---

## The key reductions (why classification pays off)

- **Fixed opponents ⇒ single-agent planning, not a game.** This is the highest-value reclassification.
  If the others don't adapt, there is no equilibrium to compute — "optimal play" is just the best
  **plan**: `∃ a sequence of my moves that wins`, an optimization / graph search. Contrast a true game:
  `∃ my move ∀ their move ∃ my move …` — alternating quantifiers, the source of minimax (chess/Go)
  hardness. The fixed-opponent case is **categorically easier**. (Methods: `solution-concepts.md` §1.)
- **Simultaneous + adaptive + hidden ⇒ you must randomize.** Only when an opponent can both *observe and
  punish* your tendencies does a *pure* strategy become exploitable; then the solution is a
  **mixed-strategy equilibrium** (counter systems; bluffing in poker).
- **Stochastic but non-adaptive ⇒ expectation, not equilibrium.** Randomness from the *environment*
  (dice, a known shuffle distribution) is handled by averaging over **chance nodes** (expectimax / MDP
  value), not by game-theoretic reasoning. **Random ≠ adversarial.**

---

## Where common games sit

| Game                       | Players | Others adaptive? | Information     | Timing              | Sum         | "Optimal play" is…          |
| -------------------------- | ------- | ---------------- | --------------- | ------------------- | ----------- | --------------------------- |
| Solitaire / a puzzle       | 1       | — (none)         | perfect         | sequential          | —           | a winning plan (search)     |
| A PvE level vs scripted AI | 1\*     | no (fixed)       | perfect if open | sequential          | —           | a winning plan (search)     |
| Chess / Go                 | 2       | yes              | perfect         | sequential          | zero-sum    | a minimax strategy          |
| Rock-Paper-Scissors        | 2       | yes              | imperfect       | simultaneous        | zero-sum    | a mixed equilibrium         |
| Poker                      | 2+      | yes              | imperfect       | sequential          | ~zero-sum   | a mixed equilibrium         |
| Backgammon                 | 2       | yes              | perfect         | sequential + chance | zero-sum    | an expectiminimax strategy  |
| A co-op team's worth       | n       | —                | —               | —                   | cooperative | each member's Shapley value |

\* single-agent **precisely because** the AI is fixed; make the AI adapt to the player's plan and the
same level becomes a two-player game with all the attendant hardness.

---

## Why this matters for design and measurement

The cell you are in dictates the instrument:

- a **single-agent** cell is **solvable exactly** (search / dynamic programming) and yields an objective
  par;
- a **simultaneous-adaptive** cell needs **equilibrium** computation and yields a *mixed* strategy;
- a **cooperative** cell needs **marginal-contribution** accounting (Shapley), not a strength score.

Picking the wrong instrument measures the wrong quantity — e.g. computing a "Nash equilibrium" for what
is really a single-agent puzzle, or scoring a co-op support role by its *solo* strength. A game with
multiple layers can sit in **different cells at different layers**; analyze each layer in its own cell
(the counter-system version of this point is `nested-counter-systems.md`).

**See also:** `solution-concepts.md` (how to solve each cell) · `hierarchy-of-concerns.md` +
`measurement-mechanics.md` (the simultaneous zero-sum counter-system cell, in depth) ·
`cooperative-and-marginal-value.md` (the cooperative cell).
