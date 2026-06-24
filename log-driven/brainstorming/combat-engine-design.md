# Combat engine — locked design (v1 melee, greenfield)

The implementation target for a melee combat engine that simulates matchups and
emits a directory of balance reports. **Greenfield**: it lives entirely inside the
`quantity / magnitude / flippable-card` pattern and deliberately does *not* import
the game spec's vocabulary (Tempo/Daring/crossing/Dread/Ward/Resolve). It may
later influence the spec, not the other way around.

Companions: `keeping-systems-interesting.md` (the matchup theory),
`character-catalog.md` (the roster fixture — note: needs a revision pass, see end).

## Principles (locked)

- **No a-priori budget.** Builds are not cost-equalized. A maxed specialist is
  simply, vastly more powerful in its domain. Balance is *measured*, never assumed.
- **Vertical scaling = god-like.** Investing deeper in a domain raises its numbers
  without bound; a maxed specialist dominates its domain by astronomical margins.
- **Horizontal scaling = variety (multiclass).** Branching into more
  damage-types/domains buys matchup *coverage* at the cost of peak.
- **Both endgames supported:** a god with everyone's power, and a balanced party
  that only wins with all parts covered.
- **Power-fantasy invariant:** every bad matchup is overcomable with sufficient
  stats — **no permanent walls**, only walls relative to the current numbers.
- **Identity invariant:** a class behaves the same as its stats grow (scaling
  multiplies its profile; it never mutates into a different archetype).

## The card primitive

Every stat is `(quantity, magnitude)`, both default 1, realized as `quantity`
cards that flip when a per-card threshold (`magnitude`) is crossed. A character's
entire state is the face-up/down status of its cards.

## v1 melee stat set

| Stat                    | quantity                                 | magnitude                                       |
| ----------------------- | ---------------------------------------- | ----------------------------------------------- |
| **Health**              | # of body cards                          | Toughness — damage a card soaks before flipping |
| **Armor** (per type)    | (only with `brittle`)                    | flat cut subtracted from each incoming strike   |
| **Strike** (per weapon) | —                                        | power per hit                                   |
| **Speed**               | actions per round (cards flipped to act) | initiative grade (tie-break / acts first)       |
| **Pierce**              | —                                        | armor shaved off the defender's Armor           |

Damage-types: **pierce / slash / crush**. Armor is indexed by type; each Strike
has one type. A character may carry multiple weapons (multiple Strike lines).

## Resolution — per round

1. Each combatant has `Speed.quantity` actions; each action is one Strike of
   power `P` on a chosen type `k`. (Spending an action = flipping a Speed card.)
2. **Stage 1 — Armor (hard wall):**
   `bite = max(0, P − max(0, Armor_k − Pierce))`.
   `bite == 0` ⇒ the strike bounces, wasted.
3. **Stage 2 — Toughness (accumulates within the round):** the defender's active
   Health card has a round accumulator `acc`; `acc += bite`. When
   `acc ≥ Toughness`: flip one card; `overflow = acc − Toughness`; set `acc = 0`
   (overflow discarded).
4. **End of round:** discard all un-flipped `acc` (reset to 0). Damage that did
   not flip a card that round is gone.
5. **Death:** all Health cards flipped.
6. **Matchup:** rounds-to-kill `RTK(A→B)` is the one-way grind (may be ∞). Run
   both directions; lower RTK wins; equal → `Speed.magnitude` breaks it; both ∞ →
   draw.

### Keyword deltas (precise)

- **persist** (attacker): `acc` is **not** wiped at end of round — it carries
  across rounds. Lets a low-bite attacker eventually saw through high Toughness.
  *Anti-tank.* (Does nothing against a Stage-1 bounce — armor still hard.)
- **cleave** (attacker): on a flip, instead of discarding overflow, set
  `acc = overflow` and re-check Stage 2, cascading. One bite can flip
  `floor(acc / Toughness)` cards. *Anti-swarm.*
- **brittle** (defender): `Armor_k` is backed by `Armor.quantity` cards; each hit
  on type `k` flips one; at exhaustion `Armor_k → 0` (or steps down one magnitude
  per card). Strong early, erodes under volume. *Trade-off vs flat armor.*

Open keyword tuning (defer to the tool): persist's behavior on a full bounce
(chip or nothing); brittle's exact trigger (per strike vs per damage) and
zero-out vs step-down.

## Scaling model

- **Vertical (a build's own domain):** raise the magnitudes/quantities of its
  signature stats. A Tank raises Health.quantity, Toughness, Armor; a Bruiser
  raises Strike; a Swarm raises Speed.quantity. Profile shape is preserved.
- **Horizontal (multiclass):** add Strike lines / damage-types / a second domain.
  Buys coverage of more matchup legs; with no budget the cost is acquisition, and
  in-model a multiclass simply has more (often individually-smaller) stat lines
  than a pure specialist who poured everything into one.

### Walls are relative — the breach principle

Two wall types, both finite to breach:
- **Stage-1 wall** (`Strike < Armor`): breached by scaling **Strike** (or
  **Pierce**) until `Strike − (Armor − Pierce) > 0`.
- **Stage-2 wall** (a round's accumulation `< Toughness`): breached by scaling
  **Speed.quantity** or **Strike** until one round's bites reach Toughness.

`breach(A→B)` = the minimum scaling delta on some stat of A that flips the leg.
Finite breach for every leg = the power-fantasy invariant, made measurable.

## The matchup engine + report directory

Deterministic from `(roster + rules + reference level + seed)`. No wall-clock.

**Owned output dir:** top-level `combat-reports/`, committed to git, **wiped and
regenerated wholesale every run**; the engine is the sole writer; never
hand-edited. (Committing it is deliberate: re-running after a feature change and
diffing the scorecard is the regression workflow.)

**Reports:**

| File                  | Contents                                                                                                             |
| --------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `index.md`            | scorecard: #viable / #junk / #clones / bosses / doormats / SCC structure / texture histogram / rank                  |
| `matchups.md`         | **iso-level** N×N matrix: per-pair winner, RTK both ways, margin                                                     |
| `breach.md`           | per losing leg, the minimum scaling Δ (and stat) that flips it — sizes every wall, proves finiteness                 |
| `scaling.md`          | per-build level-sweep curves (monotonicity / identity checks) + god (vertical) and party (horizontal) demonstrations |
| `duels/<A>-vs-<B>.md` | turn-by-turn card-flip traces at `one-way.txt` detail                                                                |
| `roster.md`           | the roster as loaded (self-contained report)                                                                         |

**Detectors** (each ↔ a named failure in `keeping-systems-interesting.md`):
dominated/junk, viable/non-covered, boss/doormat, clone, SCC roundtable-vs-ladder,
texture (walls/edges/coin-flips), skew-matrix rank. Plus the scaling-specific
checks: finite-breach-for-every-leg, and per-build monotonic improvement under
self-scaling.

## Architecture (Rust, per repo guardrail)

- Model + resolver + analyzers in `crates/deckbound` (Bevy-free, unit-testable;
  no wall-clock, seed-only).
- Thin example binary `crates/deckbound/examples/combat_report.rs` loads the
  roster and writes `combat-reports/`.
- Roster stored as versioned data (RON/TOML) so it diffs cleanly and stays in
  sync with the catalog doc.

## Deferred

- Exact stat numbers / keyword tuning — found by the tool's sweep, not guessed.
- Inner/fear track and other domains — only after the melee scorecard shows a
  healthy roundtable.
- `character-catalog.md` revision pass: drop the budget `Σ` lines (no budget now)
  and replace `strike-quantity` with `speed` (actions) + a Strike line, per the
  locked action economy.
