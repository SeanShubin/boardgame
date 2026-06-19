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

### 3.3 Sets — bigger rewards, free for the planner (constraint 3)

**The principle: the build-space dimension is the number of *atomic rewards*, not the card count.** A
reward is *deterministic and atomic* — clearing (role Y, level X) yields a *fixed* bundle of `Z`
cards; you don't choose what's in it. So the campaign planner's state is **`(progress grid,
assignment)`** — *which `(role, level)` clears you've made* + *which character holds each reward-set*
— and **neither term depends on `Z`.** Ten cards in a reward or one, it's still "Y-X cleared, assigned
to C": one progress increment, one assignment. **`Z` is free for the planner** ("25 effects, *>*25
cards", generalised to heterogeneous bundles).

- **A reward-set may be heterogeneous** — role-effect cards (played; combat-runtime), **stat** cards
  (additive into the build's block — §5.5, commutative), and **passive** modifiers (self-contained
  upgrades to that set's effect). The *mix* is a balance dial, **not** a compute cost.
- **The only real cost of large `Z` is the *combat oracle*** — more cards to play means a richer
  per-battle search — and it is **bounded and combat-internal** (resets each fight, §0.1). So **`Z` is
  free for the campaign planner, paid by the combat oracle**: the practical limit on reward size comes
  from keeping *per-battle* complexity reasonable (the set-complexity curve, §3.4 / §6), never from a
  campaign-side blow-up. A 20-card reward wouldn't explode par; it'd just make each fight heavy.

**Three guardrails keep `Z` free (see §5):**
1. **Atomic in acquisition** — you get the *whole fixed set*; no *drafting* a sub-selection (a choice
   within the reward would branch the build space).
2. **Atomic in assignment** — the *whole set* goes to one character; no splitting across the party
   (the generalisation of the pair-splitting "no", §6 — splitting makes the assignment space grow
   with `Z`).
3. **Self-contained** — a set's cards do not *multiplicatively combine* with other sets' cards.
   Internal richness, yes; cross-reward multiplication, no.

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

### 3.5 Physical representation — cards print their provenance (and that *enforces* the rules)

Atomic sets mean each card must print **which set it came from** — and that turns three abstract
guardrails into self-enforcing physical facts (#7 cards-only, #9 rules ride on the card). Each card
shows, in corner real-estate card games already use:

1. **Set provenance — `(role, level)`** (a role colour/icon — the five currency colours — plus a level
   numeral, e.g. *Artillery · III*): which reward it came from.
2. **Intra-set index — `n / M`** (only when a reward is multi-card): so a set reads as *complete* and
   stays *together*.
3. **Card type** — Base / Modifier / Mode / **Stat** (§3.4): how to use it, no lookup.

*(The existing `CardView` already has the slots — `type_line` carries "Artillery · III · Modifier", the
corner badge the index — so the catalog renderer needs no new structure.)*

**Why it's a feature, not a cost.** Every card in a set prints its provenance **including the stat
cards** — a *+2 Body* card from the Artillery-3 reward reads *Artillery · III · Stat*, its **provenance
is the set even though its effect is generic.** So:
- **Atomic assignment** and the **no-split / no-stat-mule** decision (§6) become *self-enforcing at the
  table* — you can't quietly siphon every stat card onto one unkillable body, because each is stamped
  with the role-set it travels with. The "stats stay coupled to role identity" rule lives on the card,
  not a tracking sheet.
- **One-copy scarcity is legible** — exactly one card is stamped *Artillery · III*.

The one genuine constraint: a card is **bound to its provenance** (no repurposing it as a free generic)
— but that is already the design (scarce, set-bound), so it costs nothing new.

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

### Settled since (2026-06-19)

- **Per-role cap = per round** (D1, §8). **Positional coherence = positional cards require their
  position**, for now (D2, §8). **Gold** → a generic **Stat layer** bundled with every reward, *not* a
  currency or a sixth role (§8.5 draft).
- **Per-level power is monotone** (a §8.3 GUARANTEE: deeper = at least as powerful, #5).
  **Complexity is a *lever*, not the intent** — a level expresses its higher power by a bigger number
  *or* a richer effect, the designer's call.

### Still open (content / balance dials, not rules)

- **Scaling vs independent (per track, §3.4):** the **new-effect ↔ modifier ratio** — the
  variety↔escalation dial. Set it with the §6 dimensionality/clustering analysis, not by gut.
- **How each level cashes out its power** — number vs richer effect (also the combat-oracle-cost dial,
  §3.3). Power is fixed by the Spec; *how* it's spent is a `booklet.ron` call.

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

---

## 8. Proposed Spec graduation — Phase 0 (DRAFT for review)

> **Not yet canon.** This is the proposed text to graduate into the [Spec](canon/2-spec/README.md)
> (§8.3 / §8.5 / §5 / §4), in RULE / WHY / GUARANTEES form so it's drop-in ready. **Code stays put** —
> on graduation those sections become `🟡 seeded · migration pending` (the code is then a defect to fix
> over Phases 1–4). **Scope: campaign-first** (the combat *scenarios* keep their pre-built kits for
> now). Sign off (and resolve the two decisions below) → I commit it to the canonical Spec.

### Two decisions — resolved (2026-06-19)

- **D1 — the per-role cap's "turn" unit = *per round*.** ✅ A character may play **one role card of
  each role per lane round** (§4).
- **D2 — positional coherence = *positional cards require their position*** (for now). ✅ Wall /
  Infiltrator / Artillery cards are playable only from the matching §4 position (Vanguard /
  Skirmisher / Reserve); Support / Controller cards are position-agnostic — capping the god
  *emergently* (one body, one position). *Flagged for later exploration* (alternatives: an explicit
  cap, or no coherence) → [future-possibilities](future-possibilities.md) when revisited.

### §8.3 → **Rewards & role cards** (replaces "Currency & loot")

**RULE.** Clearing **level X of role-track Y** unlocks the **reward** for `(Y, X)`: a fixed, **atomic
set** of cards — role-effect card(s), a bundled generic **Stat** card, and any passive **Modifier** —
**one physical copy each** (scarce). The **party assigns the whole set, permanently, to one
character.** 5 tracks × 5 levels = **25 rewards**. **No currency:** clearing *is* the unlock (clear
level N ⇒ levels 1..N of that track).

**WHY.** One-copy scarcity (no stacking) + atomic permanent assignment make *"who carries this"* a
weighty choice (#2 opportunity cost; #4 team balance); the shared pool is a **party-size-independent
power budget** (#4: god ≈ party-total). Direct unlock drops the currency middleman — the depth/breadth
choice lives in **routing** — and keeps the build a §0.1 *no-path-dependent-budget* function of
clears + assignment.

**GUARANTEES.**
- Total reward power = a function of **levels cleared**, shared and distributed — party-size-independent.
- A reward is **atomic** — acquired and assigned as one unit, never sub-drafted or split — so the
  build-space dimension is the **count of rewards, not cards** (§0.1).
- **No currency, no path-dependent budget** (§0.1): the build is *which rewards are owned and who holds
  each*, a function of clears + assignment, not order/route; assignment is **permanent** (no sell-back).
- **One physical copy per reward**; every card prints its `(role, level)` provenance, so scarcity and
  atomic assignment are legible / self-enforcing (§3.5).
- **Power is monotone in level** — within a track, a deeper reward is *at least as powerful* as a
  shallower one (the doom-to-mastery curve, #5). **Complexity is an optional lever for expressing that
  power, never the intent**: a higher level may be a bigger number *or* a richer (multi-card) effect —
  the designer's `booklet.ron` call, not a requirement.

### §8.5 → **Progression & roles** (revised)

**RULE.** A character **is its assigned role cards** — "role" is *emergent*, not a label, and roles
only **accrete** (permanent). The five **role tracks** are the §4 triangle's **`3 + 2`**
(Wall / Infiltrator / Artillery + Controller / Support); a generic **Stat layer** is **bundled into
every reward** (the old generic *currency*, Gold, is gone — it is now a stat-card pairing, not a
currency). Party size sets the spectrum: many bodies → specialists (one track each); few → multi-track;
one → a **god** spanning all five.

**WHY.** Characters are deliberately unbalanced; coverage comes from the **team and scenario** (#4).
Role-as-assigned-cards makes "god ≈ party" concrete (the *same* pool, distributed); the per-role play
cap (§4) is what equalises their throughput. Depth-vs-breadth stays the uncomputable strategic fork
(#2).

**GUARANTEES.**
- A character's roles = its assigned role-card tracks; **accretes** (monotone, §0.1).
- **Stats are bundled with role rewards** — the survivability to *use* a role grows *with* the role; no
  free-floating generic stat pool (no "stat-mule").
- Five role tracks (the `3 + 2`); the generic is a **stat layer**, not a currency.

### §5 → **the role-card taxonomy** (addition)

**RULE.** A role card is exactly one of: **Base** — played from Hand, the track's core effect (normal
§5.3 zone behaviour); **Modifier** — *passive*, lives in Active, auto-applies to its Base (the scaling
card), never separately played; **Mode** — *played*, an alternative / charged Base (e.g. spend a turn
for a bigger effect), mutually exclusive with the Base that round; **Stat** — a Form attachment
(§2.3 stats-as-deck), contributes stats, not played.

**WHY.** The taxonomy makes scaling cards coexist with the §4 per-role cap (Modifiers ride free;
Base + Mode count). It maps onto the existing passive-power vs played-action split.

**GUARANTEES.** A set's cards are **self-contained** — Modifiers/Stats apply *within* their set; **no
cross-reward multiplicative combo** (§0.1).

### §4 → **the role-card play rule** (addition)

**RULE.** A character may play **at most one role card of each role per round** (so it may play several
if they are *different* roles). **[D2]** A **positional** role card (Wall / Infiltrator / Artillery) is
playable only from the matching §4 position; **effect** cards (Support / Controller) are
position-agnostic.

**WHY.** The per-role cap is the **god-vs-party lever** (#4 god ≈ party): a god holds every track but,
being one body in one position, plays ~one positional + the two effect cards per round, while a
5-specialist party plays ~5 across five bodies — a **throughput tradeoff, not dominance**. Positional
coherence reins the god in **emergently** (#9 — one body, one position).

**GUARANTEES.** Per-role-per-round cap; a positional card requires its position. Neither party size
dominates on raw card throughput (the budget #4 / candidate **BI-3** the solver verifies).

### Deferred to content / balance (not Spec rules)

- The **new-effect ↔ modifier ratio** per track (§3.4) — a `booklet.ron` authoring + §6-measured dial.
- **How** each level expresses its (Spec-guaranteed) higher power — a bigger number vs a richer
  multi-card effect. *Power is the intent (a §8.3 GUARANTEE); complexity is just one lever for it, and
  also the combat-oracle-cost dial (§3.3).*

### Migration status (what graduation makes "pending")

On graduation, §8.3 / §8.5 / §5 / §4 are `🟡 seeded · migration pending`. The code is then a defect to
fix in: **(1)** drop the currency economy + add the role-card data model & permanent atomic assignment;
**(2)** the per-role-per-round cap + positional metadata in combat; **(3)** author the 25 sets;
**(4)** rebuild `reference.rs`. (Per the [§10 runbook](computability-and-balance.md) discipline, code
follows Spec, not the reverse.)
