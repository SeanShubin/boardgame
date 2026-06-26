# Perfect battle solver — design plan (Task 1)

> **Design plan, staged 2026-06-26.** A solver that computes **exact optimal battle play** (no
> heuristic). **Promotion target: `computability-and-balance.md` §10** (the deferred par-tooling
> runbook; home is the `deckbound::solver` crate per §8). Pairs with `automated-optimal-battle-play.md`
> (why it's tractable) and `role-weight-balance-testing.md` (the strong policy this provides). No code
> yet — a spec-sync is in flight; this is the plan to build once it settles.

## Core framing — a finite-horizon single-agent MDP, not a game
A PvE battle is **not** a two-player game: creatures run a **fixed, non-adaptive instinct** (§0.1) — a
fixed-policy *environment*. So perfect play is **exact backward induction over a finite horizon**, no
equilibrium, no heuristic:
- **Luck OFF** (open creature commit, deterministic — §0.2): collapses to **reachability** — "does there
  exist a player line to a win-leaf within the round cap?" Pure existential search (no min/adversary
  nodes), the §0.4 "winnable within the horizon?" boolean. Optimal policy = the winning line.
- **Luck ON** (creature bid/decks hidden, RNG on): a **finite-horizon MDP** — creature's fixed
  distribution + RNG draws are **chance nodes**; solve for **max expected value** via backward induction
  / value iteration (expectimax). Exact *iff* the distributions are finite/enumerable (they are — §0.1).
- **PvP is the only place minimax/mixed-Nash returns** — quarantined (§7), out of scope.

Operating envelope: run under **`Ruleset::analysis()`** (§0.4 — 5-round horizon, ≤5 roster types,
swarm-as-one). That bound is what makes the state space **finite and exactly searchable**; live play's
unbounded Ruleset is not the solver's concern.

## What the search branches on (decision points)
Per round, the player commits at three nodes; everything else (Breach/Reckoning/Lull) resolves
automatically from commitments:
1. **Standoff bid** — positioning (each hero Vanguard/Rearguard) × group partition × which `Standing`
   abilities to cast. (In PvE the creature's bid is fixed/known or a known distribution — a fixed input,
   not a co-decision.)
2. **Fray commit** — the *set* of (actor → ability → target) plays + defensive responses, bounded by Tempo.
3. **Volley commit** — free Vanguards' charges/flanks + targets, instant re-fires, the rear's pre-empt answers.

**§1.9 order-independence is the key lever:** within a phase the player commits a **set**, not a
sequence — so the search branches on *subsets of commitments*, not permutations (a factorial reduction).

## State, transition, leaf
- **State** = the `State` struct's combat fields (per-actor Health, Tempo, tokens, position, lock/charge/
  deferred status, per-phase pile, phase, round). Needs a **canonical, hashable encoding** for a
  **transposition table** (combat is a memoizable oracle — §0.1).
- **Transition** = the existing resolver (`combat.rs`: `fray_clash` / `resolve_volley` / `resolve_breach`
  / `resolve_reckoning` / `tally` / `clear_phase_piles`). The solver *applies committed actions and reads
  the next state* — it does not reimplement combat.
- **Leaf** = terminal by rule (foes dead → win; party dead or round cap → loss/draw). **Exact value, no
  evaluation heuristic** (§0.4) — the whole point of the bounded horizon.

## Objective (lexicographic; configurable)
Primary **win/loss** (reachability — "winnable within the horizon?"), tiebroken by **fewer rounds**
(the par metric), then **fewest party characters downed** (party preservation — losing a whole unit ≫
chip damage), then **more Health remaining** (survival margin). So the "optimal line" is *also* the par
line, and among par lines the solver prefers the one that keeps the most bodies standing. Swap the objective for the **graded balance metrics** (rounds-to-clear, difficulty frontier)
that `role-weight-balance-testing.md` needs — same search, different leaf value.

## The hard part — branching factor (the one real risk)
Per-phase commitment sets could be large (power-set of legal plays). Finiteness is guaranteed (§0.4);
**speed** is the risk. Levers (all preserve exactness):
- **Order-independence** (§1.9) — commit sets, not sequences.
- **Tempo budget** — bounds actions/actor/round.
- **Transposition table** — memoize canonical states (collapses transpositions).
- **Dominance pruning** (§0.1 monotonicity) — prune *provably* dominated commitments only (so it stays
  perfect). E.g., a superset of beneficial buffs / a strictly-stronger target.
- **Symmetry** — swarm-as-one + identical-unit canonical ordering (§0.4).
- **Greedy as move-ordering oracle** — try the greedy policy's move first to find a winning line early
  (early cutoff for the boolean objective); does **not** compromise exactness, only speed.
**Validate the branching factor empirically on real encounters in Phase B** — the reference campaign
resolves in ~3 rounds under greedy (§0.4 note), so depth is small in practice; confirm width is too.

## Build phases (incremental, each verifiable)
- **A — legal-action enumerator + canonical state hash.** Reuse `game.rs` action routing for legality;
  add a per-phase commitment-set generator and a hashable state key. (No search yet.)
- **B — reachability search, luck-off, boolean objective** + transposition table. **Validate on toy
  known-answer scenarios** (hand-computed winnable/unwinnable battles). Invariant: optimal ≥ greedy's
  result always.
- **C — graded objectives** (rounds-to-clear / frontier) via backward induction with the lexicographic value.
- **D — luck-on expectimax** — chance nodes over creature fixed distributions + RNG; exact value iteration
  over the finite horizon.
- **E — perf + wiring** — dominance/symmetry pruning if B/C show width pressure; expose the API to the
  par-tooling and the role-weight measurement (this *is* their strong policy).

## API (sketch)
`solve(party, encounter, ruleset, objective) -> { value, optimal_line }` in `deckbound::solver`
(replaces/augments `greedy`). `optimal_line` = the perfect-play trace (the par line; also a readable
transcript and the strong policy for role-weight).

## Correctness
Toy known-answer scenarios; determinism (seeded → identical); the **optimal ≥ greedy** invariant; and
(later) mutual cross-check with the encounter suite (Task 2) — the suite stress-tests the solver, the
solver validates the suite's niches.

## Ratified design calls (2026-06-26)
1. **Objective** — lexicographic **win → fewer rounds → fewest characters downed → most Health
   remaining**. (Downs outrank Health: losing a whole unit ≫ chip damage.)
2. **First cut** — **luck-off deterministic only** (Phases A–C); defer luck-on expectimax (D).
3. **Pruning** — start with **provably-exact** levers only (transposition + order-independence); add
   dominance pruning **only if** Phase B shows width pressure.
