# Deckbound — Role-Card Redesign (proposal under exploration)

> **Status: speculative — under active exploration, NOT canon.** A proposal to restructure role
> identity around a small, **scarce, shared pool of role cards** unlocked by clearing levels. This
> doc tracks the idea, its consequences, and the open decisions. Nothing here binds until it
> graduates to the Charter / Spec via the [source-of-truth](canon/0-source-of-truth.md) protocol.
> Before adoption it **must clear the computability invariants** (Spec §0) — see §5.

---

## 1. The proposal — three constraints

1. **Scarce, shared, level-gated pool.** There is **one copy of each role card**, and **one role
   card per (role, level)**. Beating level *N* of a track unlocks that role's cards for levels
   `1..N`. With 5 roles × 5 levels that is **exactly 25 role cards / 25 effects.** A reward is a
   *single physical card*, and **the party decides who carries it** — role cards are a shared pool
   the party distributes, not a per-character kit.
2. **One card per role per turn.** A character may play **at most one role card of each role** per
   turn — so it can play several role cards *if they are different roles*. This is the central
   **god-vs-party balance lever** (§3.2).
3. **Sets.** A role-card unlock may be a **set of physical cards representing one effect** — a
   level-1 reward is one simple card; a level-5 reward may be a multi-card set tracking a richer
   effect. So **>25 physical cards, but still 25 effects.**

**Design intent (the designer's framing):** *embrace the constraints*; *more constraints make it
easier to balance*; **without harming optionality** — because "optionality in the presence of a
dominant strategy is an illusion." The goal is to **maximise interesting options**, on the bet that
the *right* constraints do this counter-intuitively, and that one-copy scarcity maps cleanly onto the
physical card-game format.

---

## 2. The current baseline (what exists today)

There is **no "role card" concept** yet; role identity is spread across three card types, unevenly,
with no level structure ([`booklet.ron`](../../../crates/deckbound/data/booklet.ron)):

| Role | Played Action cards | Passive powers | Upgrades (stat) | Total |
| --- | --- | --- | --- | --- |
| **Wall** | Rally, Steel (2) | Phalanx, Bodyguard, Taunt (3) | Bulwark, Ironhide (2) | 7 |
| **Infiltrator** | Bank (1) | Blitz, Shadowstep, Backstab (3) | Edge, Shadow (2) | 6 |
| **Artillery** | Barrage, Suppress (2) | Longshot (1) | Volley, Munitions (2) | 5 |
| **Controller** | Confuse, Slow, Dread (3) | — (0) | Hex, Curse (2) | 5 |
| **Support** | Ward, Mend, Haste (3) | — (0) | Vigor, Grace (2) | 5 |

Plus a generic Gold upgrade (Training) and two orphan Action cards (Cleave, Sunder). Today
progression rewards **stat Upgrades bought with currency** (§8.3); the *playable* abilities live on
pre-built specialists and are **not** a progression reward. The redesign would **re-type** rewards
from stat attachments into playable role cards and **regularise** the 1–3 / 0–3 / 2 patchwork into a
flat **5 × 5 grid** — the redesign normalises the pool (~5.6 cards/role today → exactly 5), it does
not inflate it.

---

## 3. Consequences

### 3.1 Scarcity + a shared pool (constraint 1)

- **Allocation becomes a real decision.** "Who gets the Wall level-3 card?" is a genuine
  assignment / knapsack choice each reward. Scarcity (one copy) **forbids stacking** — no "five of
  the best card" — so it caps power and *creates* options instead of a dominant pile. This is the
  "constraints → options" bet in its cleanest form.
- **The party's total power is a clean budget.** Total role-card power = a function of *levels
  cleared*, shared and distributed — **independent of party size.** That is the structural backing
  for Spec §8.5's "a solo god ≈ a full party in total power": every party holds the *same* pool as it
  clears; only the *distribution* and *play rate* differ (§3.2).
- **Build space:** the build is now "which of the ≤25 unlocked cards each character holds." Bigger
  than today's 11 upgrades, but **bounded** — at most 25 binary owns × *K* characters. Stays inside
  Spec §0.1 **iff assignment is monotone** (see §5).

### 3.2 One card per role per turn — the god/party lever (constraint 2)

This is the load-bearing constraint. Trace per-turn **role-card throughput** across the party-size
spectrum (full 25-card pool):

- **5 specialists:** each holds its one role's 5 cards, plays ≤1/turn → **≈5 role cards/turn**, spread
  across **5 bodies / 5 positions** (resilience + coverage).
- **1 god (holds all 25):** ≤1 *per role* → a ceiling of 5/turn — **but one body sits in one
  position.** The three **positional** roles (Wall = Vanguard, Infiltrator = Skirmisher,
  Artillery = Reserve, §8.5's `3 + 2`) are **mutually exclusive for a single body**, so the god can
  realistically play **one positional + the two position-agnostic effect roles (Support, Controller)
  ≈ 3 role cards/turn**, chosen as the best 3 of 25.

So the constraint produces a **non-dominant tradeoff**, not a winner:

| | role cards / turn | bodies | the trade |
| --- | --- | --- | --- |
| 5 specialists | ~5 | 5 | throughput + resilience + coverage; but each body is stuck with its 5 |
| 1 god | ~3 (positional limit) | 1 | flexibility / quality (best-of-25 each turn), fewer bodies to keep alive; lower throughput, one fragile point of failure |

Two things fall out:
- **Positional coherence reins in the god *for free*** — no explicit "gods are nerfed" rule; a body
  can't be in three positions, so the `3 + 2` split caps concentrated play emergently. (#9 / #10: a
  rule that falls out of the fiction.)
- **The spectrum interpolates smoothly** (2–4 characters: fewer bodies, more roles each), so
  party-size choice is a real depth/breadth fork (#2), and whether `god par ≈ party par` becomes a
  *measurable* balance target (candidate **BI-3**, §7) — exactly what the per-turn cap exists to make
  tunable.

### 3.3 Sets — depth without build-space blow-up (constraint 3)

- **"25 effects, >25 cards" is the computability-preserving framing.** A set must be **atomic in the
  build** — unlocked and owned as *one* of the 25 binary effects — so the build space stays
  25-dimensional regardless of physical card count.
- **The multi-card complexity is combat-*runtime* state, not build state.** A level-5 set's several
  cards track in-battle stages/charges via the §5 zone machine (facing = state); that state **resets
  each battle** (§0.1 "no carried combat state"), so it never enters the campaign's carried build.
  Sets buy *tactical* depth on the cheap, paid inside the combat oracle, not the campaign search.
- **The guardrail (see §5):** a set is never *partially* owned, and its components must not combine
  *multiplicatively* with the other 24 effects — that would reintroduce the build-space explosion
  §0.1 forbids. Internal richness, yes; cross-effect multiplication, no.

### 3.4 Scaling cards & the card taxonomy (new-effect ↔ modifier) — *candidate*

A track's five levels needn't be five independent effects; a level can **escalate** an earlier one —
*L1 firebolt · L2 +damage · L3 +extra target · L4 overcharge (spend a turn for a bigger hit).* This
adds a coherent power-fantasy, but risks **diminishing card variety** (the designer's stated worry).

**Card taxonomy** (resolves the collision with constraint 2 — see below):
- **Base** — *played*; the track's core effect(s).
- **Modifier** — *passive*; owned → auto-applies to its base (the "scaling" cards). **Rides free**,
  not counted by the per-turn cap. *(Maps onto today's passive-power vs played-action split.)*
- **Mode** — *played*; an alternative / charged version (overcharge), mutually exclusive with the
  base that turn.

**The collision, and why the taxonomy fixes it.** If a base *and* its modifier were both *played*
cards of the same role, constraint 2 ("one role card per role per turn") would forbid playing both —
the chain couldn't fire. Making modifiers **passive** dissolves it: the per-turn cap counts **Base +
Mode** plays; modifiers ride along.

**The dial.** *New-effect vs modifier* is the **variety ↔ escalation** knob, tunable per track. "Variety"
splits two ways: modifiers cut *effect* variety but can add *decision* variety (when to overcharge,
how to sequence). **The collapse is measurable** — a modifier-heavy design shows up as low strategic
dimensionality / few clusters in the §6 toolkit, so the ratio is set with *evidence*, not gut.

**The tradeoff (not strictly better or worse).** A scaling chain **keeps a track's cards together**
(a +damage modifier is dead without its base), so you assign the *whole chain* to one character.
- *Cost:* less of constraint 1's "who gets this card" allocation freedom, and less effect variety.
- *Gain:* coherence + escalation fantasy, **and more computable** — you assign ~5 chains, not ~25
  loose cards, shrinking the assignment space (§5).

Hold this as a **per-track option**, not a global commitment.

---

## 4. Why this serves the design philosophy

- **Constraints create options.** One-copy scarcity (no stacking) + the per-role-per-turn cap (no
  spamming one role) + atomic sets (no combo-multiplication) each *remove* a dominant pattern, and a
  removed dominant pattern is where real choice lives ("optionality under a dominant strategy is an
  illusion"). The interesting decisions — *who carries which scarce card, how to spread roles across
  bodies, which 3 of your 25 to fire this turn* — exist **because** of the constraints.
- **Easier to balance.** Exactly **25 effects** on a flat 5 × 5 grid is a far smaller, more legible
  tuning surface than today's uneven action/power/upgrade patchwork — fewer knobs, clearer symmetry
  (Spec §0.3 / the par solver tunes 25 effects against the invariants).
- **Maps to the physical format.** A finite 25-card pool, one physical copy each, physical
  hand-off of the reward, and multi-card sets for complex effects are all native to a card game made
  of cards (#7).

---

## 5. Computability check (must pass before adoption — Spec §0)

| Spec §0.1 invariant | Does the redesign hold it? |
| --- | --- |
| No RNG / hidden info in the core | Yes — role cards are deterministic effects; unlock is gated by *clearing*, not a draw. |
| Foes a fixed environment | Unaffected. |
| Battles stateless `f(build, place)` | **Yes, iff** a set's multi-card tracking is *combat-runtime* state that resets each battle (§3.3). |
| **Builds monotone / additive / order-independent** | **Holds either way.** The owned-card set only *grows* (monotone). The card→character **assignment** is fully captured by the *current* state (Markovian) regardless of history — so even **free reassignment does not break §0.1** (an earlier mis-read; corrected 2026-06-19). What §0.1's "no swap" actually forbids is a **resource-refunding** swap — sell-back / oscillation that makes a *budget* path-dependent — which this isn't (no resource is refunded; cards only accrue). If anything, free reassignment is *more* computable: the assignment stops being *carried* state (re-chosen per battle) and the campaign build collapses to the owned-set. |
| Bounded horizon / branching | Build space grows (≤25 owns × *K* characters) but stays bounded; watch it against the §4 budget test. |

**Bottom line:** the redesign is computability-safe **provided** sets are atomic and
non-multiplicative (§3.3). The **permanent-vs-reassignable** choice is a *gameplay* call (§6), **not** a
computability one. Several decisions *reduce* the build space rather than grow it: **eliminating
currency** (§6) drops the balance-recompute state; **scaling chains** (§3.4) keep a track's cards
together; and **permanent** assignment, if chosen for gameplay reasons, is at worst neutral here.

---

## 6. Decisions

### Settled (designer, 2026-06-19)

- **What "a role" means → the role cards assigned to it.** A character *is* its assigned role cards;
  "role" is emergent, not a label. A god = a character holding cards across all five tracks. With
  **permanent** assignment, roles only *accrete* (gain, never lose) — monotone and §0.1-safe.
- **Stats → a bundled `(role card, generic card)` pair.** Each level-clear yields a **pair**: a
  *role* card (identity / effect — the option layer) and a *generic* card (stats — the survivability
  layer the glass cannons need). This **eliminates the generic *currency***: Gold stops being a
  currency and becomes the paired stat-card track.
- **Economy → eliminate the middle man.** Direct unlock (clear level → reward pair); no
  earn-currency-then-buy step. *Simpler and more computable* (no currency-balance state).
  **The substantive decision survives:** depth-vs-breadth (which roles to invest in) lives in
  **routing** — which levels you clear — and currency was only the middleman expressing it; "whether
  to spend" was a *dominated* (fake) decision anyway. **The one real thing removed** is §8.3
  **co-location / "sharing as logistics"** (earn here, spend there, pool across the party) — and the
  **designer judges that a win** (it was bookkeeping / more to track, not fun; *decided 2026-06-19*).
  So currency removal is **pure gain**, not a tradeoff.
- **Reassignment → permanent (for gameplay, not computability).** *Correction (2026-06-19):* free
  reassignment does **not** break §0.1 (§5) — the assignment is Markovian, and freely re-choosable, it
  would just stop being carried state. Permanent is chosen because it makes **"who gets this scarce
  card" a weighty, irreversible decision** — the interesting constraint we want; free reassignment
  would make allocation trivial and gut it.
- **Pair splitting → no (bundle).** Bundling keeps each role a **self-contained package**: a role's
  *effects* and the *survivability to use them* grow together, so a fragile role-investment is never
  left a glass cannon by someone else taking its generic cards. Splitting would license a
  **"stat-mule"** decoupling (all stats on one body, all effects on others), severing the role ↔
  survivability coupling and adding balance surface; it also doubles the assignment space.

### Still open

- **Scaling vs independent (per track, §3.4):** the **new-effect ↔ modifier ratio** — the
  variety↔escalation dial. Set it with the §6 dimensionality/clustering analysis, not by gut.
- **Per-level complexity curve:** how fast do sets grow (level 1 simple → level 5 a set)? The
  tactical-depth dial and the combat-oracle-cost dial at once.
- **Positional coherence:** is "one body, one position blocks Wall+Infiltrator same turn" an explicit
  rule or purely emergent from positioning? (Prefer emergent, §3.2.)
- **What happens to Gold-as-role?** With the generic *currency* gone, is there still a generic
  *track* (the stat cards), and does the "5 roles + generic" count (Spec §8.5) restate as "5 role
  tracks + 1 generic stat track"?

---

## 7. Relationship to canon (what it would touch)

- **Spec §8.3** (currency & loot) and **§8.5** (progression & roles) — re-typed rewards; possibly the
  economy step. **§5** (zones) — sets track via the zone machine.
- **Spec §0.1** — the assignment-monotonicity and atomic-set guardrails (§5) are how it stays in the
  computable core.
- **Charter #2 / #4 / #11** — depth/breadth fork; balance-by-team budget; computability.
- **Balance invariants** — refines BI-1 / BI-2; introduces a candidate **BI-3** (*god par ≈
  best-party par; party sizes interpolate*) that the per-turn cap is *designed to make true* and the
  solver would verify.

**See also:** [computability-and-balance.md](computability-and-balance.md) · [Spec
§0 / §8](canon/2-spec/README.md) · [balance-invariants.md](balance-invariants.md) ·
[progression-design.md](progression-design.md) (the economy this would re-type).
