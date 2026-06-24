# Measurement Mechanics for Counter Systems

This document provides concrete algorithms and formulas for measuring each property in the hierarchy of concerns. Each section includes the input data model, the computation, and how to interpret the result.

---

## Data Model

Before measuring anything, represent the counter system as one of two structures:

**Binary win matrix** (for discrete, deterministic systems like RPS):
```
W[i][j] = 1  if element i beats element j
W[i][j] = 0  if element i loses to element j
W[i][j] = -1 if undefined (no matchup; illegal in a complete tournament)
```
Diagonal (i=j) is undefined (elements don't play themselves).

**Weighted effectiveness matrix** (for graded systems like Pokémon):
```
E[i][j] = damage multiplier when type i attacks type j
         Typical values: 0 (immune), 0.25, 0.5, 1, 2, 4
```

All measurements below apply to one or both forms.

---

## Measure 1: Condorcet / Copeland Score

**Purpose:** Detect dominant elements. The most important measurement.

**Algorithm:**
```
For each element i:
    wins[i]   = count of j where W[i][j] = 1
    losses[i] = count of j where W[i][j] = 0
    copeland[i] = wins[i] - losses[i]
```

**Interpretation:**
- `copeland[i] = n-1` — element i is a Condorcet winner (fatal)
- `copeland[i] = 0` for all i — perfect balance
- Distribution of Copeland scores shows which elements are over/under-powered

**Red flags:**
- Any score of n-1 (Condorcet winner)
- High variance in scores
- Scores clustered into two groups (dominant tier vs. everything else)

**Example — RPS:**
```
Rock:     beats Scissors, loses to Paper     → copeland = 0
Paper:    beats Rock, loses to Scissors      → copeland = 0
Scissors: beats Paper, loses to Rock         → copeland = 0
```
Perfect: all zeros.

**Example — hypothetical broken 3-element system (A beats B, A beats C, B beats C):**
```
A: copeland = 2  (Condorcet winner — fatal)
B: copeland = 0
C: copeland = -2
```

---

## Measure 2: Regularity Score (Out-Degree Variance)

**Purpose:** Quantify how evenly wins are distributed.

**Algorithm:**
```
out_degree[i] = sum of row i in W  (count of elements i beats)
mean_degree   = sum(out_degree) / n
              = (n-1)/2  (always, in a complete tournament)

variance = sum((out_degree[i] - mean_degree)^2) / n
```

**Interpretation:**
- `variance = 0` — perfectly regular
- Higher variance = more imbalanced
- For a complete tournament, theoretical max variance occurs when one element beats all (out-degree n-1) and one loses all (out-degree 0)

**Normalized version:**
```
max_variance = ((n-1)^2 + (n-1)^2) / n  (rough upper bound for one dominant, one submissive)
regularity_score = 1 - (variance / max_variance)
```
Scores closer to 1.0 are better.

**Extended for weighted systems:**
Instead of binary wins, use effective win weight:
```
win_weight[i][j] = log2(E[i][j])  for Pokémon-style multipliers
                 = -1 for immune (special case)
```
Compute variance of row sums. This captures that 2x matchups are softer advantages than 4x.

---

## Measure 3: Nash Equilibrium Distance

**Purpose:** Determine the optimal mixed strategy and how far it deviates from uniform. This is the strategic ground truth.

**Setup:** Frame the counter system as a zero-sum game. Build payoff matrix M:
```
M[i][j] =  1  if i beats j
M[i][j] = -1  if i loses to j
M[i][j] =  0  if draw
```

**Computing Nash Equilibrium via Linear Programming:**

Player 1 wants to maximize their guaranteed payoff v. They choose a mixed strategy p = (p₁, p₂, ..., pₙ) where pᵢ ≥ 0 and Σpᵢ = 1.

```
Maximize:  v
Subject to:
    For each element j: sum_i(p[i] * M[i][j]) >= v
    sum_i(p[i]) = 1
    p[i] >= 0  for all i
```

In a symmetric zero-sum game (which a balanced counter system is), the solution satisfies p* = p (both players use the same strategy) and v* = 0.

**Interpretation:**
```
nash_distance = ||p* - (1/n, 1/n, ..., 1/n)||₂
              = sqrt(sum((p*[i] - 1/n)^2))
```
- `nash_distance = 0` — perfectly balanced; uniform play is optimal
- `nash_distance > 0` — some elements should be played more than others
- `p*[i] >> 1/n` — element i is underplayed at equilibrium if opponents play uniformly (i.e., it is stronger than average)

**Without a linear program solver**, a shortcut for regular tournaments: if all Copeland scores are 0, the Nash equilibrium is provably uniform. You can confirm balance without solving the LP.

**Empirical Nash estimation from tournament data:**
```
If you have win-rate data instead of a clean win matrix:
    W_empirical[i][j] = observed win rate of i against j
    
Substitute into LP above, treating win rate as payoff.
Nash distance from empirical data = measure of real-world balance.
```

---

## Measure 4: Strong Connectivity (SCC Count)

**Purpose:** Detect if the system fragments into independent sub-games.

**Algorithm:** Tarjan's Strongly Connected Components — O(V + E)

```
Procedure SCC(graph):
    index_counter = 0
    stack = []
    lowlink = {}
    index = {}
    on_stack = {}
    sccs = []
    
    For each node v not yet visited:
        strongconnect(v)
    
    Function strongconnect(v):
        index[v] = lowlink[v] = index_counter++
        stack.push(v)
        on_stack[v] = true
        
        For each (v → w) in graph:
            if w not visited:
                strongconnect(w)
                lowlink[v] = min(lowlink[v], lowlink[w])
            elif on_stack[w]:
                lowlink[v] = min(lowlink[v], index[w])
        
        if lowlink[v] == index[v]:  // v is root of an SCC
            scc = []
            while true:
                w = stack.pop()
                on_stack[w] = false
                scc.append(w)
                if w == v: break
            sccs.append(scc)
    
    return sccs
```

**Interpretation:**
- `len(sccs) = 1` — system is one cohesive game (ideal)
- `len(sccs) > 1` — system is fragmented; each SCC is an independent sub-game
- SCCs sorted by size reveal which sub-game is dominant

**In a complete tournament (every pair has an edge):** Tarjan always returns one SCC — by definition, a complete tournament is strongly connected. So this measure becomes relevant when the counter system is *not* a complete tournament (missing matchups, ties, or partial systems).

---

## Measure 5: Hamiltonian Cycle Detection

**Purpose:** Verify elegant learnability — that a single cyclic ordering captures the whole system.

**Algorithm:** Backtracking DFS — feasible for n < 20

```
Procedure find_hamiltonian_cycle(W, n):
    path = [0]  // start at node 0
    visited = {0}
    
    return search(path, visited)

Procedure search(path, visited):
    if len(path) == n:
        // Check if last node beats first node (closes the cycle)
        return W[path[-1]][path[0]] == 1
    
    current = path[-1]
    for next_node in range(n):
        if next_node not in visited and W[current][next_node] == 1:
            path.append(next_node)
            visited.add(next_node)
            if search(path, visited):
                return path  // found it
            path.pop()
            visited.remove(next_node)
    
    return None
```

**Interpretation:**
- Returns a cycle if one exists; None if not
- For regular tournaments on odd n, a cycle is guaranteed (König's theorem)
- If a cycle exists, you can describe the whole system as: "each element beats the k elements after it in this ordering" — a dramatic reduction in cognitive load

**Practical application:** Once the cycle is found, verify if the remaining matchups follow a uniform pattern (each element also beats elements at positions +2, +3, etc. from it in the cycle ordering). If yes, the entire system can be stated as one rule plus one cycle.

---

## Measure 6: Cognitive Load Quantification

**Purpose:** Measure the learning burden imposed by the system.

**Base formula:**
```
raw_load = n * (n - 1) / 2   // distinct pairwise matchups
```

**Reduction factors:**

*Hamiltonian cycle reduction:* If a Hamiltonian cycle exists and all other matchups follow a uniform offset pattern:
```
cycle_reduced_load = n   // just learn the cycle order, one rule
```

*Thematic inference reduction:* Matchups inferable from real-world logic (Fire burns Grass) require less explicit memorization. Hard to quantify precisely, but empirical studies of Pokémon suggest players learn ~40–50 of 153 matchups naturally through theme, then rely on pattern or trial-and-error for the rest.

*Symmetry reduction:* If the system has rotational symmetry (A→B→C→A, B→C→A→B, C→A→B→C are all equivalent), effective load is reduced by the symmetry group size.

**Learnability index (informal):**
```
learnability = 1 / (raw_load / reduction_factor)
```
Higher is more learnable. RPS: 1/(3/3) = 1.0. RPSLS: 1/(10/5) = 0.5. Pokémon: 1/(153/50) ≈ 0.33.

---

## Measure 7: Pokémon-Style Extended Analysis

For weighted, non-binary counter systems, standard tournament measures need adaptation.

**Attack breadth** — how many types does a given type threaten:
```
attack_score[i] = count of j where E[i][j] > 1   // types i hits super-effectively
attack_reach[i] = sum_j(log2(E[i][j]))            // log-weighted to penalize 0x immunities
```

**Defensive resilience** — how well a type survives incoming attacks:
```
defense_score[i]   = count of j where E[j][i] < 1  // types i resists
vulnerability[i]   = count of j where E[j][i] > 1  // types i is weak to
net_balance[i]     = defense_score[i] - vulnerability[i]
```

**System balance:**
```
balance_variance = variance(net_balance across all types)
```
Lower = more balanced. Perfect = 0 (all types equally threatened and threatening).

**Immunity penalty:**
Immunities (E[i][j] = 0) are structural breaks. Count them:
```
immunity_count = count of (i,j) pairs where E[i][j] = 0
```
Ideal = 0. Each immunity is a hard-counter relationship that breaks tournament structure and typically benefits the immune type disproportionately.

---

## Quick Reference: Which Measure for Which Question

| Question | Measure |
|---|---|
| "Is one element dominating play?" | Copeland scores |
| "Are all elements equally viable?" | Nash distance from uniform |
| "Is the system structurally balanced?" | Out-degree variance |
| "Does the system hang together as one game?" | SCC count |
| "Can a player learn this system efficiently?" | Hamiltonian cycle + cognitive load |
| "How bad are the hard counters in a weighted system?" | Immunity count + attack/defense balance |
