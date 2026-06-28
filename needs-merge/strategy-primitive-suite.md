# The strategy-primitive suite — deriving roles from the rules

> **Design framework, 2026-06-26.** A principled way to build the encounter suite and *test the role
> roster itself*: one **extreme, symmetric** encounter per simple strategy the rules imply; the set of
> non-fungible **counters** the solver confirms *is* the role roster. Supersedes the hand-picked suite.
> **Promotion target: `computability-and-balance.md` §10 + `docs/game-theory/`.** Built on the role-weight
> harness (`role-weight-harness-built.md`) and the fungibility finding (`lock-exclusivity-finding.md`).

## The core idea
A balanced game should have, for **every simple strategy the rules make possible**, *some* answer in the
party's design — and the **extreme** of that strategy should make *exactly one* answer viable. We don't
test context-dependent / adaptive play (that's PvP); we test the **pure strategy primitives** the
mechanics imply, each pushed until only its intended counter survives. Symmetry is soft: every enemy
strategy has *an* answer in the party's space, but heroes are unique by design (the party doesn't mirror
a horde body-for-body — its symmetric answer to breadth would be a **summon**, not a hero-horde).

## Why "to the extreme" is the load-bearing word
Every fight is two simultaneous races: the **survival race** (don't die first) and the **damage race**
(kill them first). Roles are *methods* of winning a race, and a role reads **fungible** whenever its race
can be won another way:
- **Survival** ← prevent the hit (Wall) · heal it (Support) · suppress enemy offense (−Cadence/−Might).
- **Damage** ← deliver to the target (reach / range / AoE) · amplify through defenses (−Toughness/−Finesse / burst).

At normal difficulty these substitute freely (this is the fungibility we measured). **The extreme is the
difficulty at which only one method survives**: a spike too big to heal forces *prevent*; a bulwark too
tough to burst forces *strip*; a blitz too fast to out-heal forces *suppress*. The extreme isn't "harder"
— it's "kills every substitute but one."

## The candidate roster the rules imply: a symmetric 3 + 3
| Survival race (don't die)                                       | Damage race (kill them)                                           |
| --------------------------------------------------------------- | ----------------------------------------------------------------- |
| **Prevent** — absorb the spike → **Wall**                       | **Reach** — get to the protected target → **Infiltrator**         |
| **Heal** — out-sustain the grind → **Support**                  | **Range/breadth** — hit the distant/many → **Artillery**          |
| **Suppress** — blunt enemy offense (−Cadence/−Might) → *Warden* | **Strip** — crack enemy defense (−Toughness/−Finesse) → *Breaker* |

The current **five** = this six with the **Controller** bundling *Warden* + *Breaker* (its −stat kit spans
both halves). So the open question "is the count right?" has a concrete test: if a **Cadence-extreme**
independently forces −Cadence **and** a **Toughness-extreme** independently forces −Toughness, the count
is **six**; if survival/burst keep substituting for one half even at the extreme, it's **five** (or the
redundant half is cut). **Vitality is the one stat with no isolating extreme** — a pure HP sponge is always
"bring more damage" — so we should *not* build a role around it (the data already showed this).

## Verified mechanics (so the extremes are built right)
- **Toughness** = the **per-phase pile wall** (melee + ranged). A hit flips `floor(pile / Toughness)`
  Health cards; **overflow within a phase is wasted**. So at Toughness above any single hit's Might, *no
  hit ever flips a card* → only **Sunder** (−Toughness) opens it. `raw = eff_might + card_power`; max raw
  burst = Assassinate (power 9, one-shot). → a **Toughness-above-burst** foe forces Strip.
- **Finesse** = the **ranged evade contest** (`cards × Finesse` must strictly exceed the attacker's
  volley). **Mark** (−Finesse) makes a defender easier to hit *and* weakens its own ranged pressure —
  dual-use. → an **evasion-extreme** foe (high Finesse) blanks ranged unless Marked.
- **Vitality** = Health-card count (no special counter). **Cadence** = action/Tempo economy; **Mire**
  (−Cadence) cuts how often a unit acts. **Might** = strike magnitude; **Defang** (−Might) blunts it.

## Positioning — two wrong models, then the actual rule (no spec change needed)
First draft proposed "cover blocks ranged line-of-sight" (redundant — ranged already fires *at* the
Vanguard, not past it). Second draft proposed "slip past the lock" — **also wrong: the lock was never the
barrier.** The actual rule, verified in `combat::compute_locks`: **you are locked iff *you* attacked an
enemy Vanguard that survived; being *struck* never locks you** (only attacking locks). So the Infiltrator
slips by simply **not engaging** in the Fray — it dodges/eats the front's blows, strikes nothing, stays
**free**, and charges the Rearguard in the Volley. **The code already matches the intended spec; no rule
change is needed.**

The slip is **richly kit-supported — force not fiat**: Silver's **Smoke** (the rear cannot pre-empt the
next charge), **Shadowstep** (win the tie when slipping past an interceptor), **Slip Strike**'s **Shove**
(no melee strike-back), **Blitz** (first slip each round is free), plus Cadence/Finesse-heavy stats (Wisp:
Cadence 7, Finesse 5). Others *may* charge too, but without these they eat the pre-empt or can't afford the
tempo. **The lever exists; the encounter just has to *demand* it.**

### Why the Infiltrator niche didn't flip Silver — wrong encounter, not a missing rule
The screened-backline fights were **out-survivable**: Wall/Support tanked the backline, so the party never
*needed* to cross. The correct Infiltrator-necessity niche is a backline **lethal enough to defeat max
survival** (you *must* kill it, can't tank it) **whose pre-empt kills any crosser lacking the slip kit** →
only Silver reaches and kills it alive. Build *that* and the slip is non-fungible. **Caveat to verify:**
that the solver actually *exercises* the slip optimally (declines the Fray engagement, uses Smoke/
Shadowstep) — if a niche built this way still won't flip Silver, the gap is in the **Infiltrator's card
kit**, not the rules: its abilities may need redesign to make crossing-and-killing something only it can do
(e.g. tempo/Finesse advantages on the cross that no other role can match), per the force-not-fiat charter.

## The method: let the harness discover the roster
Build the isolating extremes + the minimal content each requires, run them through the winnability-flip
harness, and read off the boundaries instead of guessing:
- exactly one role flips → that responsibility is real and non-fungible (a role);
- none flips (while hard) → fungible primitive (like Vitality) → a role is redundant / mis-drawn;
- key + extras flip → an overlap to resolve with a rule (e.g. slip-past for reach).

### Content the extremes need (emergent, not fiat — real targets, not immunity keywords)
- **Toughness-above-burst** foe (forces Strip / tests Breaker).
- **Cadence-extreme** foe (forces Suppress / tests Warden — the Controller-split question).
- **Evasion-extreme** foe (tests Mark / ranged).
- enemy **healer** + enemy **buffer** creatures (give Reach/Range/Strip non-fungible targets — you can't
  out-damage a heal, you must remove it).
- optional **summon** card (the party's symmetric answer to breadth).

## Open decisions for the human
1. **Controller = one role or two?** (Warden vs Breaker — the 5-vs-6 question; the extremes decide.)
2. **Adopt slip-past-a-standing-front?** (the reach-exclusivity lever; engine change.)
3. **Summon** as an Artillery sub-card, a new role, or not at all.
