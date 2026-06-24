# Keeping counter systems interesting

How to build matchup systems (rock-paper-scissors and beyond) that stay
interesting, and how to objectively detect the boring cases — derived from the
four-stat combat model.

## The four stats

The whole discussion uses only four numbers per character:

- **Mitigation `M`** — damage deducted from every single strike.
- **HP `H`** — total damage we can take.
- **Strikes `N`** — how many times we can strike per turn.
- **Strike damage `D`** — how much each strike does.

Duel model: effective damage per strike is `max(0, D_att − M_def)`, so

```
DPS(A→B) = N_A · max(0, D_A − M_B)
TTK(A→B) = H_B / DPS(A→B)        (fewer turns to kill = winner)
```

## Result 1 — mitigation is the only thing that makes RPS possible

Drop mitigation (`M = 0` everywhere). Then A beats B iff

```
H_A · N_A · D_A  >  H_B · N_B · D_B
```

Every character collapses to a **single scalar `Q = H·N·D`**. A single number is a
total order — strictly transitive — so **rock-paper-scissors is impossible**
with only HP, strikes-per-turn, and damage-per-strike.

Mitigation rescues it because `max(0, D − M)` is **non-linear**: the same armor
that barely dents a big hit *erases* a small one. That threshold is the entire
mechanism for intransitivity.

### A concrete RPS loop

| Character              | `M` | `H` | `N` | `D` |
| ---------------------- | :-: | :-: | :-: | :-: |
| **Tank** (armor)       | 4   | 12  | 1   | 4   |
| **Swarm** (volume)     | 0   | 20  | 4   | 2   |
| **Bruiser** (big hits) | 0   | 16  | 1   | 8   |

- **Tank → Swarm:** 4 dps, 5 turns. **Swarm → Tank:** `max(0,2−4)=0` → never. **Tank wins.**
- **Bruiser → Tank:** 4 dps, 3 turns. **Tank → Bruiser:** 4 dps, 4 turns. **Bruiser wins.**
- **Swarm → Bruiser:** 8 dps, 2 turns. **Bruiser → Swarm:** 8 dps, 2.5 turns. **Swarm wins.**

Bruiser → Tank → Swarm → Bruiser. Each beats exactly one and loses to one.
(Caveats: the simultaneous-TTK model ignores turn order/rounding — re-check close
legs against your initiative rule; and the Swarm→Tank `0` is a *hard wall*, which
we formalize next.)

## Result 2 — "hard wall" and "degenerate" are two different things

The informal complaints split into **two independent layers**:

> **Direction** = *who* wins (the matchup graph) — governs global structure.
> **Margin** = *by how much* (the edge weight) — governs hard vs soft.

### Layer A — margin (per leg): the texture

Score each matchup by its **margin** (the TTK ratio):

```
margin(A,B) = max(TTK(A→B), TTK(B→A)) / min(TTK(A→B), TTK(B→A))   ∈ [1, ∞]
```

| Phrase             | Margin      | Feel                                                  |
| ------------------ | ----------- | ----------------------------------------------------- |
| **Coin-flip**      | ≈ 1         | no real counter; mush                                 |
| **Edge / lean**    | finite, > 1 | *soft counter* — favored, underdog can still steal it |
| **Wall / lockout** | ∞           | *hard counter* — cannot win                           |

The wall has a crisp signature: `margin = ∞` **iff** the floor binds in one
direction, i.e. `D_loser ≤ M_winner` ⇔ `DPS = 0` ⇔ `TTK = ∞`. A second symptom:
when the floor binds, the outcome becomes **insensitive to three of the four
stats** — a healthy leg depends on all eight numbers, a walled leg on two.

### Layer B — direction: the structure

The wall is **separable** from RPS — you don't need any binding floor to get a
cycle. Working strictly in the *unclipped* regime (`D_att > M_def` for every
pair), A beats B iff

```
f(A,B) = a_A·(D_A − M_B) − a_B·(D_B − M_A) > 0,    where  a_X = H_X · N_X
```

An explicit wall-free cycle (smallest `D−M` is 0.5, nothing clips):

| Char | `H` | `N` | `D` | `M` | `a = H·N` |
| ---- | :-: | :-: | :-: | :-: | :-------: |
| A    | 10  | 1   | 8   | 1   | 10        |
| B    | 20  | 1   | 3   | 3   | 20        |
| C    | 30  | 1   | 3.5 | 2   | 30        |

`f(A,B)=+10`, `f(B,C)=+5`, `f(C,A)=+15` → A→B→C→A with **every margin finite**.
So "is it a wall" and "is it genuine RPS" are orthogonal.

### What governs cycle existence: a determinant

Summing the legs around the loop:

```
f(A,B) + f(B,C) + f(C,A) = Σ
Σ = M_A(a_B − a_C) + M_B(a_C − a_A) + M_C(a_A − a_B)
```

- **Strike damage `D` cancels** — it only tunes per-leg margins, never creates or
  orients a cycle. The cycle lives entirely in the `(a, M) = (H·N, M)` interaction.
- **A cycle forces `Σ ≠ 0`** (sign of `Σ` = cycle direction).

`Σ` is (twice) the signed area of the triangle formed by plotting each character
as a point `(H·N, M)`:

> **RPS ⇔ the characters are non-collinear as points `(H·N, M)`.**
> Triangle orientation = cycle direction; triangle area = margin headroom.

Special cases that are collinear (→ `Σ = 0` → total order):
- **No mitigation** (`M` all equal) → horizontal line. (This is the `Q = H·N·D` result.)
- **Equal bulk** (`H·N` all equal) → vertical line.
- You need `H·N` **and** `M` to vary non-proportionally — genuine 2D spread.

## Scaling past 3: vocabulary and detectors

Past three options the matchup graph is a **tournament** (one directed edge per
pair). The boring cases all have names and objective tests.

### Texture-boring (margin layer)

- **All coin-flips** → counters don't matter; flavorless.
- **All walls** → "did you bring the counter? y/n" — brittle, no play.
- Healthy texture: **mostly edges, a few walls for flavor.**

### Structure-boring (direction layer) — use strongly-connected components

Compute the **SCCs** of the matchup graph. An SCC is a **tier** that cycles
internally; every tournament's SCCs always stack into a strict top-to-bottom
order, so any roster is "a **ladder of roundtables**."

| Phrase                              | Definition                    | Test                                                  |
| ----------------------------------- | ----------------------------- | ----------------------------------------------------- |
| **Roundtable** (good)               | one big SCC                   | whole roster is one component                         |
| **Pecking order / ladder** (boring) | transitive, strictly rankable | all SCCs singletons / no 3-cycle / scores `0,1,…,n−1` |
| **Boss** (boring)                   | beats everyone                | a row of all-wins (out-degree `n−1`)                  |
| **Doormat** (boring)                | loses to everyone             | a row of all-losses (in-degree `n−1`)                 |
| **Reskin / clone** (boring)         | same matchups as another      | two identical rows                                    |
| **Tier**                            | one SCC among several         | a multi-node component                                |

The number to watch as you scale: **how many SCCs, and how big.** All singletons
= pure ladder (worst). One SCC of size `n` = everyone interesting (best). A few
fat tiers stacked = a tiered metagame, often what you actually want.

Payoff: **any strongly-connected roster of ≥3 options is pancyclic** — every
option sits on a cycle of *every* length. "One roundtable" guarantees no option
is dead weight.

### Detection recipe

1. Build the matrix: for each pair, sign of the margin → an arrow.
2. **Flag clones** — identical rows.
3. **Flag bosses / doormats** — all-win / all-loss rows.
4. **Run SCCs** — many singletons means you built a ladder, not a counter system.
5. **Within each SCC, inspect margins** — all walls = brittle; all coin-flips =
   mush; a mix = good.

### Optional deeper knob: rank

Take the **rank of the skew-symmetric margin matrix** (`f(A,B) = −f(B,A)`). It is
the `n`-option generalization of the `Σ` determinant:

- rank 0 → single scalar `Q` / total order,
- rank 2 → one **clock** (every counter explained by position on one wheel),
- higher even rank → genuinely independent counter-axes.

Low rank isn't necessarily boring (a clock can be elegant), but it tells you how
much real variety you have versus how much is one wheel restated.

## Carry-home phrases

- **Margin / texture:** *coin-flip vs edge vs wall* (soft vs hard counter).
- **Direction / structure:** *roundtable vs ladder*, with *bosses*, *doormats*,
  and *reskins* as the failure modes.
- **The design target:** one big roundtable of soft edges, with a few decisive
  walls for flavor.
