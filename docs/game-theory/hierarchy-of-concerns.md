# Hierarchy of Concerns for Counter Systems

A counter system is a set of elements with defined win/loss relationships between them. Formally it is a **directed graph (tournament)** where an edge A→B means "A beats B." The following properties are ordered by severity of violation — violating a higher-ranked property makes lower-ranked ones irrelevant.

---

## 1. No Condorcet Winner — *Fatal*

**What it is:** A Condorcet winner is an element that beats every other element directly, or that cannot be countered through any chain of play. In graph terms: a node with out-degree n-1, or a node from which every other node is reachable in one hop.

**Formal definition:** Element *c* is a Condorcet winner if ∀ j ≠ c: c beats j.

**Why it is first:** If a Condorcet winner exists, rational players will always choose it. The counter system ceases to function — it becomes a game of "play the winner." No other property can rescue a system with this flaw.

**Example of violation:** Dragon-type Pokémon before Generation 6 approached Condorcet status in competitive play. No common type resisted Dragon; Dragon was only weak to Ice and Dragon itself, both niche or risky. The near-Condorcet status manifested as Dragon-type Pokémon filling 4–6 of the top-8 team slots at major tournaments.

---

## 2. Regularity — *Structural*

**What it is:** Every element beats exactly the same number of other elements and loses to the same number. In graph terms: all nodes have equal out-degree.

**Formal definition:** A tournament is *k-regular* if every node has out-degree k = (n-1)/2.

**The odd-n requirement:** For k to be a whole number, (n-1) must be even, so n must be odd. This is not a convention — it is a mathematical necessity. You cannot construct a perfectly regular tournament on 2, 4, 6, or any even number of elements. This is why 3-element (RPS) and 5-element (RPSLS) systems achieve perfection while 4-element or 6-element systems always have at least one element that beats more than the others.

**Proof sketch:** In a tournament on n nodes, total directed edges = n(n-1)/2. For regularity, each node must have out-degree (n-1)/2. This requires n(n-1)/2 to distribute evenly as n × (n-1)/2, which it always does — but (n-1)/2 must be an integer, requiring n to be odd.

**Why it is second:** Irregularity does not necessarily produce a Condorcet winner, but it guarantees that some elements are strictly better than others in expectation. Players who identify this drift toward the better elements, collapsing the diversity of play.

**Example of violation:** In Pokémon, Ice-type has 4 weaknesses (Fire, Rock, Steel, Fighting) and its attacks are resisted by many types. Steel-type (pre-Gen 6) had 1 weakness (Fighting, Ground) and resisted 10 types. These types have dramatically different out-degrees in any reasonable tournament model.

---

## 3. Nash Equilibrium Equal to Uniform — *Strategic*

**What it is:** In a perfectly balanced system, the optimal mixed strategy for a rational player is to pick every element with equal probability (1/n). Any deviation from uniform in the Nash equilibrium means some elements are strictly more valuable than others.

**Formal definition:** The unique Nash equilibrium of the symmetric zero-sum game defined by the counter system is the uniform distribution p* = (1/n, 1/n, ..., 1/n).

**Relationship to Regularity:** This property is a *consequence* of regularity. A regular tournament always has uniform Nash equilibrium. It is listed separately because the Nash equilibrium is the practical, measurable expression of balance — you can observe deviations in actual play data even when you cannot directly measure out-degrees (e.g., in games with continuous or probabilistic counters).

**Why it is third:** A system can be technically irregular but still have a near-uniform Nash equilibrium if the irregularities cancel out. Conversely, a system might appear structurally balanced but have a non-uniform equilibrium due to second-order effects (e.g., interaction costs, access asymmetries). The Nash equilibrium is the ground truth of strategic balance.

**Example of violation:** In competitive Pokémon formats, team-building data shows that certain types appear on 60–80% of winning teams while others appear on fewer than 5%. This directly reflects a non-uniform Nash equilibrium in the underlying system.

---

## 4. Strong Connectivity — *Coherence*

**What it is:** From any element, you can reach any other element through a chain of counters. In graph terms: the tournament is a single strongly connected component (SCC).

**Formal definition:** For all pairs (i, j), there exists a directed path from i to j in the counter graph.

**Why it is fourth:** If the graph decomposes into multiple SCCs, the system is effectively several independent games. Elements in different SCCs never interact strategically. Players gravitate toward the "best" SCC and ignore the others — a soft form of dominance even without a Condorcet winner.

**Example of violation:** A hypothetical 6-element system where {A, B, C} form one 3-cycle and {D, E, F} form another, with no edges between them. A player choosing from {A, B, C} never has reason to consider {D, E, F} and vice versa. The game fragments.

---

## 5. Hamiltonian Cycle — *Elegance*

**What it is:** A single cyclic ordering of all n elements exists such that each element beats the next one in the sequence (wrapping around). In graph terms: the tournament contains a Hamiltonian cycle.

**Formal definition:** There exists a permutation (e₁, e₂, ..., eₙ) of all elements such that e₁ beats e₂, e₂ beats e₃, ..., eₙ beats e₁.

**Why it is fifth:** This property is not required for balance — a strongly connected regular tournament can exist without a Hamiltonian cycle. But its presence allows players to learn the system as a single memorable cycle rather than a collection of pairwise rules. It is the difference between "Scissors cuts Paper, Paper covers Rock, Rock crushes Scissors" (one sentence) and enumerating all n(n-1)/2 matchups separately.

**Mathematical note:** Every regular tournament on odd n contains a Hamiltonian cycle (this follows from Dirac's theorem adapted for tournaments). So if you achieve properties 1–4, this one comes for free.

**Example:** Rock → Paper → Scissors → Spock → Lizard → Rock is one Hamiltonian cycle in RPSLS (there are others). The entire 5-element system can be stated as this cycle plus one additional rule: each element also beats the element two positions ahead of it.

---

## 6. Cognitive Load — *Practical*

**What it is:** The number of distinct pairwise relationships a player must internalize. For n elements with no ties and no symmetry shortcuts, this is n(n-1)/2.

| Elements (n) | Matchups to learn |
|---|---|
| 3 (RPS) | 3 |
| 5 (RPSLS) | 10 |
| 7 | 21 |
| 9 | 36 |
| 18 (Pokémon types) | 153 |

**Why it is last:** Cognitive load does not affect mathematical balance at all. A 101-element regular tournament is perfectly balanced. But players cannot engage with a system they cannot learn. There is a hard tradeoff: more elements increases strategic richness but increases learning cost quadratically.

**Design implication:** The Hamiltonian cycle property (concern 5) partially mitigates this — a structured cycle reduces n(n-1)/2 facts to ~n facts with a pattern. The Pokémon type chart mitigates it through theming (Fire burns Grass — the multiplier is inferable from the real-world relationship), which trades formal elegance for cognitive accessibility.

---

## Summary Table

| # | Concern | Violation Consequence | Measurable As |
|---|---|---|---|
| 1 | No Condorcet Winner | System collapses to one element | Copeland score distribution |
| 2 | Regularity | Some elements strictly better | Out-degree variance |
| 3 | Uniform Nash Equilibrium | Dominant strategies exist | Distance of p* from uniform |
| 4 | Strong Connectivity | System fragments into sub-games | SCC count |
| 5 | Hamiltonian Cycle | Harder to learn, less elegant | Cycle existence check |
| 6 | Cognitive Load | Players cannot engage | n(n-1)/2 |

---

## A Note on Levels of Analysis

This hierarchy assumes a single-level counter system — one set of elements with one set of win/loss relationships. Complex games contain counter systems at multiple levels simultaneously (faction identity, strategic choices, unit interactions), and the hierarchy applies independently to each level.

Critically, the **design goal differs by level**:

- At the **unit and strategic levels**, the full hierarchy applies and a Hamiltonian cycle is desirable — RPS structures here create depth and variety.
- At the **faction or archetype level**, only concerns 1–3 apply, and the Hamiltonian cycle is not the goal — equality (uniform Nash, no Condorcet winner) is the goal, not a counter cycle between factions.

A faction-level RPS (Faction A beats Faction B beats Faction C) is a design failure: it makes the pre-game faction choice strategically significant, which reduces skill expression and collapses the meta. The RPS structure belongs inside each matchup, not between the participants.

See `nested-counter-systems.md` for the full treatment, including StarCraft as the exemplar of correct multi-level design and Warcraft III / Soulstorm as failures at different levels of the same hierarchy.
