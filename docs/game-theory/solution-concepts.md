# Solution Concepts — how to actually solve each kind of game

`game-classification.md` tells you which **cell** a game (or one layer of it) is in. This document gives
the **solution method** for each cell, ordered from cheapest (single-agent search) to hardest
(mixed-strategy equilibrium), plus the special tools for environment randomness and for teams. The
golden rule: **classify first, then take the cheapest method that fits** — solving a single-agent puzzle
as if it were an adversarial game wastes enormous effort *and* computes the wrong object.

---

## 1. Single-agent / fixed-environment — search & dynamic programming

When no one adapts to you (a puzzle, or fixed/scripted opponents), optimal play is a **plan**:

- **Reachability (boolean objective).** If the goal is just *win-or-not* within a horizon, the question
  is "does a winning line exist?" — an existential graph search (BFS / DFS / IDA\*), no values needed.
  The optimal policy is any winning line.
- **Shortest-path / least-cost (graded objective).** If you also minimize a cost (turns, resources,
  days), it is a shortest-path problem — **Dijkstra / A\*** with an admissible heuristic.
- **Backward induction over a finite horizon.** Solve a bounded sequential problem from the leaves
  back: a leaf's value is **terminal by rule** (nothing to estimate), and each interior node takes its
  best child. With a hard horizon whose leaves are terminal, this is **exact — no evaluation heuristic**
  (the usual source of "strong but not perfect").
- **Dynamic programming / memoization (transposition tables).** When the same state recurs via different
  move orders, **hash a canonical encoding of the state and memoize.** When commitments within a step
  are **order-independent**, you branch on *sets* rather than *sequences* — a factorial reduction.
- **Dominance pruning.** When "more is strictly better" (monotonicity), prune *provably* dominated
  options without losing exactness.

**Exactness condition:** the state space must be **finite** (bounded horizon, bounded branching).
Bounding an otherwise-open game — a turn/round cap, a roster cap, symmetry reduction — is the standard
lever that turns "computable in principle" into "computable in practice."

---

## 2. Stochastic but non-adaptive — expectation over chance nodes

Add environment randomness (dice, a draw from a *known, fixed* distribution) but still **no adaptive
opponent.** The tree gains **chance nodes**, and you maximize **expected value**:

- **Expectimax / finite-horizon value iteration** — at your nodes take the max; at chance nodes take the
  probability-weighted average; recurse to terminal leaves. Exact **iff** the distributions are
  finite / enumerable.
- This is a **Markov Decision Process (MDP)**: states, actions, a (possibly stochastic) transition, a
  reward — solved by value / policy iteration, or by finite-horizon backward induction.
- **Random ≠ adversarial.** A fixed random environment is *not* a min player; you never need to be
  *unexploitable* against dice. (Only if an *adaptive* opponent also moves do you get
  **expectiminimax** — chance **and** min **and** max nodes.)

---

## 3. Two-player adaptive, perfect information — minimax

Ordered moves, both adapt, nothing hidden: **minimax / alpha-beta** over the game tree. Exact only to
the depth searched; real games (chess, Go) are far too deep, so practical engines apply a **heuristic
evaluation** at a cutoff — hence "strong, not perfect." The hardness is structural: the `∃∀∃∀…`
alternation is what places these games (PSPACE / EXPTIME-hard) **categorically above** single-agent
planning.

---

## 4. Simultaneous and/or hidden, adaptive — mixed-strategy equilibrium

When players commit **simultaneously** (or with hidden information) and each can **exploit the other's
tendencies,** a deterministic choice is exploitable. The solution is a **Nash equilibrium**, in general
a **mixed strategy** — a probability distribution over choices that no opponent can punish.

- **Zero-sum + simultaneous = a matrix game,** solvable *exactly* by **linear programming** (the
  minimax theorem). For a symmetric, balanced counter system the game value is 0 and the equilibrium is
  **uniform** (see `hierarchy-of-concerns.md` §3, `measurement-mechanics.md` Measure 3).
- Bounding a hidden, simultaneous **sub-game** down to a small matrix makes even this exactly solvable
  by LP, so it can be **embedded inside** a larger backward induction (solve the little matrix game at
  each node, then propagate its value).

### When must you actually randomize? — the *value of unpredictability*

Mixing buys exactly one thing: **unexploitability against an opponent who can read and punish you.**
Therefore:

- **Against a fixed / non-adaptive environment, a pure (deterministic) optimal strategy always
  suffices** — there is no one to hide from. (This is why the single-agent cell of §1 never needs
  mixing, *even when your own choices are hidden from a fixed opponent* — a fixed opponent cannot use
  them.)
- **Mixing is required only when the opponent is adaptive *and* the timing / information lets them
  exploit a pattern.**

Define the **value of unpredictability** = (equilibrium value vs an adaptive opponent) − (value of the
best deterministic line). It is **exactly zero** when the opponent cannot adapt, and grows with how
badly a correct read can punish you. This yields a precise statement for tuning with a deterministic
solver `P`:

- `P` measures the true value **exactly** wherever unpredictability is worth nothing — e.g. against
  fixed AI, the bulk of single-player balance. There it is *not an approximation; it is the answer.*
- `P` **mis-rates** options only in the hidden-simultaneous-vs-adaptive layer — **under**-rating options
  whose worth *is* being unreadable (feints, bluffs, mixed positioning) and **over**-rating pure
  counters to a now-predictable foe. The error is **option-dependent, not a constant offset.**
- The penalty for being read is a **tuning dial**: keep a wrong read a *modest* swing and the gap stays
  small (the deterministic proxy stays faithful); make a wrong read *catastrophic* and the game
  *demands* mixing, and `P` diverges. Validate by solving the small mixed sub-game in isolation (it's a
  tiny matrix game — §4 LP) and comparing its value to what `P` scores.

---

## 5. Cooperative / teams — marginal contribution & the Shapley value

When the question is "how much does a *member* contribute to a *team's* outcome," the non-cooperative
tools above don't apply. Use marginal contribution and the Shapley value — see
`cooperative-and-marginal-value.md`.

---

## Picking the method (summary)

| The situation                                            | Method                                                       | Exact?                  |
| -------------------------------------------------------- | ----------------------------------------------------------- | ----------------------- |
| No adaptive opponent · win/lose                          | reachability search (BFS/DFS/IDA\*)                          | yes (if finite)         |
| No adaptive opponent · minimize a cost                   | Dijkstra / A\*                                               | yes (if finite)         |
| No adaptive opponent · bounded sequential                | backward induction + memoization + dominance pruning        | yes (if finite)         |
| + environment randomness                                 | expectimax / MDP value iteration                            | yes (if distrib. finite)|
| Adaptive opponent · perfect info · sequential            | minimax / alpha-beta                                         | only to search depth    |
| Adaptive opponent · simultaneous / hidden · zero-sum     | mixed-strategy Nash via linear programming                  | yes (matrix game)       |
| Adaptive opponent · sequential **with** chance           | expectiminimax                                              | yes (if finite)         |
| A member's value within a team                           | Shapley value                                               | yes (if `v` computable) |

**See also:** `game-classification.md` (classify before solving) · `measurement-mechanics.md` (the
counter-system LP and the balance measures) · `cooperative-and-marginal-value.md` (teams).
