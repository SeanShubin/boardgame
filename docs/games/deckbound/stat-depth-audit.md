# Stat-Depth Audit — does every stat earn its slot?

> **Historical (promoted from `needs-merge/`, 2026-06-21).** The findings here have **shipped**:
> Spirit→**Dread** (now wired), Support's missing stat → **Inspiration**, Pierce/**Daring** kept,
> **Keystone cut**; the foundational principle became **Charter #12** + **Spec §8.6**. Kept as the
> design-reasoning record — non-authoritative, canon wins.

**Status:** design pass (suggestions only — 2026-06-21). No code or canon edited. A separate
instance merges `needs-merge/`. Per `0-source-of-truth.md`: AI proposes, human disposes —
**especially on numbers**, and every intent-level call is surfaced as **case 3** explicitly.

**Method.** Each stat is judged on **depth** (does it create real tradeoffs, or is it just
bigger=better?), **distinctness** (mechanically its own thing, or redundant?), and **complexity
cost** (what a player must learn/track). The set is then read through three lenses the brief
supplied: **channel-symmetry** (Body ↔ Fear), **reset-clocks** (§2.7), and **the 3+2 role set**
(§8.5). Ground truth: `stats.rs`, `form.rs`, `combat.rs`, `booklet.ron`, and the rules-tour
transcript.

**The one-line headline.** The set is **well-chosen** — almost every stat is the mechanical
substrate of one of the five role-suits, so it is re-derivable from intent (§10). But it has **one
dead member (Spirit), one unexercised member (Keystone), one strictly-dominated member (Pierce),
and one structural asymmetry (the inner channel has no pool/per-encounter layer).** Fix Spirit and
the set becomes genuinely minimal-complete for the 3+2 roles.

---

## Per-stat verdict table

| Stat (deck × suit)           | Depth                  | Distinct?              | Cost                      | Verdict                       | Why (one line)                                                                                                                                                       |
| ---------------------------- | ---------------------- | ---------------------- | ------------------------- | ----------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Strike** · Strike·Power    | med                    | yes (the attack atom)  | low                       | **KEEP**                      | The spine of offense; every blow's base. Currently *overloaded* — it also powers Fear (see Spirit).                                                                  |
| **Spirit** · Spirit·Power    | **none (unwired)**     | **no (dupes Strike)**  | **pure cost**             | **WIRE or REMOVE**            | Summed, displayed, scaled per-level, **bought by the Bone track — yet never consumed.** The Controller's headline offense stat does nothing.                         |
| **Pierce** · Pierce·Power    | low                    | weak (anti-Armor only) | low                       | **REWORK / KEEP-conditional** | Point-for-point **strictly dominated by Strike**; earns a slot only if priced cheaper, as the dedicated armor-counter enabling low-Strike anti-tank builds.          |
| **Speed** · Tempo·Quantity   | **high**               | yes                    | med                       | **KEEP**                      | The deep benchmark: one pool pays offense *and* defense, so every Tempo spend is a live allocation.                                                                  |
| **Drive** · Tempo·Power      | low *(today)*          | yes (grade vs count)   | **high (one-place stat)** | **KEEP but FLAG**             | The genuine count×grade partner to Speed, but it bites in *exactly one sub-phase* (a crossing) — a comprehension tax, and shallow under the v1 single-card resolver. |
| **Health** · Body·Quantity   | high                   | yes (the only pool)    | the budgeted meter        | **KEEP**                      | The one maintained meter (§2.1); per-encounter attrition. Load-bearing.                                                                                              |
| **Toughness** · Body·Power   | high                   | yes (clock vs Armor)   | low                       | **KEEP**                      | The bar — "few big hits" axis, per-round clock. Non-redundant with Armor by clock + with Health by suit.                                                             |
| **Armor** · Guard·type·Power | high                   | yes (per-hit, typed)   | **high (×8 types)**       | **KEEP, note type-tax**       | "Many small hits" axis + called-shots from typing. The 8 damage types are the real cost multiplier.                                                                  |
| **Ward** · Ward·type·Power   | med                    | yes (inner cut)        | med                       | **KEEP, simplify**            | Mirrors Armor on the Fear channel, but the inner channel has ~one live type — the typed `BTreeMap` buys little; a scalar would do.                                   |
| **Resolve** · Power          | low                    | yes (inner bar)        | low                       | **KEEP, the shallow channel** | A threshold with no pool behind it → binary/swingy (Resolve 0 ⇒ any Fear is instant-lethal). The inner channel's depth gap.                                          |
| **Keystone** · categorical   | **none (unexercised)** | n/a (marker)           | latent                    | **ACTIVATE or SHELVE**        | **Never set to anything but Body in any card/actor (0 in data).** Pure latent complexity; its `Aspect::Mind` branch is dead.                                         |

---

## Remove / Merge

### 1. Spirit — **WIRE it (recommended)**, or REMOVE it. *(the headline)*

**Finding (ground truth).** `offense.spirit` is summed (`form.rs:70`), scaled per encounter level
(`encounter.rs:82`), displayed in the transcript and stat sheet — and **bought by the Bone/Controller
reward track** (`booklet.ron`: `Reward(track: Bone … stat:(spirit: 2))`, levels 1/3/5). But it is
**never read in combat.** Both attack paths — `base_strike` (weapon) and `play_card`'s
`Effect::Damage` — compute force as `offense.power + card_pow (+ power_bonus)`, *regardless of damage
type*. So a **Fear** attack scales off **Strike**, not Spirit. In the transcript, Hex (Strike 1,
**Spirit 8**) casts Dread (fear 4) for `1 + 4 = 5` — its Spirit 8 contributes **zero**. Hex's entire
identity — terrifying but physically feeble — is mechanically inert; the Bone track is **selling a
dead stat**.

**RULE (the fix — wire it).** An **inner (Fear) attack's base force = the attacker's Spirit**
(+ the card's power), exactly as an **outer attack's base force = Strike**. The damage type already
selects the channel (`DamageType::channel()`); it should also select **which offense stat** supplies
the raw: outer → `power`, inner → `spirit`.

- **WHY.** Restores §2.2's stated invariant that the two channels are **structurally parallel**:
  `attack: Strike ↔ Spirit`. It un-overloads Strike (which currently silently does both jobs), makes
  the Bone track's investment real, and preserves the *fear-specialist* archetype (high Spirit / low
  Strike = an inner-channel cannon who is an outer-channel pushover — the analog of Strike/Body
  specialization).
- **GUARANTEES preserved.** Channel parallelism (§2.2), called-shots (type still picks the channel),
  commutative Form (unchanged).
- **Case classification: 1 (mechanics-fix).** The code fails to consume a stat the Spec says exists;
  the fix keeps the WHY and the "both channels parallel" GUARANTEE. Safe to propose. *(Note: it is
  also a **balance** change — Hex's fear output jumps from 5 to ~12 — so the **number** side is
  human-tuned. The wiring is case 1; the resulting magnitudes are "AI proposes, human disposes.")*

**Alternative — REMOVE Spirit.** Let one **Strike** stat feed both channels; the weapon/card's damage
*type* alone decides Body-vs-Fear. Cheaper (one fewer stat), and the called-shot model already gives
the inner channel its distinctness on *defense*.
- **Cost:** collapses the **fear-vs-physical attack-specialization axis** — a Controller can no longer
  be offensively strong on the inner channel while weak on the outer. The asymmetry "two defenses,
  one attack stat" replaces §2.2's clean parallel.
- **Case classification: 3 (intent change).** This abandons the §2.2 parallel-attack symmetry — the
  human's call, not a silent mechanics fix. **Surfaced.**

> **Disposition needed (human):** wire Spirit (keep the symmetry, Controller becomes real) **or**
> remove it (one attack stat, simpler, lose the inner-offense archetype). Either way the current
> state — *a stat that is sold, displayed, and inert* — is a defect and should not persist.

### 2. Pierce — **rework or justify-by-price**, watch for merge into Strike.

**Finding.** Net Body damage through one hit = `raw − max(0, armor − precision)`.
- `+1 Strike` → `+1` through **always** (any armor level).
- `+1 Precision` → `+1` through **only while** `precision < armor`, else **0**.

So **Strike weakly dominates Precision point-for-point**, and both scale identically across many hits
(Armor is per-hit, so no clock distinction rescues Pierce). Pierce earns its slot **only** if it is
**cheaper per point than Strike** (then the tradeoff is "cheap-conditional vs expensive-universal"),
or as a deliberate **designer lever**: it lets Armor be set high enough to matter without being
unbeatable, and lets a **low-Strike anti-tank** build exist (Sear's Pierce 2). At equal price it is a
strictly-worse Strike.

- **Verdict:** KEEP-conditional. **Either** confirm Pierce is priced below Strike in the reward
  economy (making the conditional-vs-universal tradeoff real), **or** fold it into Strike and express
  "armor-bypass" as a card keyword (e.g. *Sunder*) rather than a standing stat.
- **Case classification:** repricing = **case 1**; removing the stat = **case 3** (drops armor-bypass
  as a standing build axis). **Surfaced.**
- **Symmetry note (feeds the Add section):** Pierce bypasses **Armor** (outer cut) but there is **no
  inner mirror** that bypasses **Ward**. Pierce is an outer-channel special case.

### 3. Keystone — **activate with a creature, or shelve.**

**Finding.** `defense.keystone` decides which loss is lethal (Body / Mind / Spirit). In **all of
`booklet.ron` it is never set** — every actor uses the Body default (grep: 0 keystone occurrences).
Its `Aspect::Spirit` branch is reachable only by data that doesn't exist; its `Aspect::Mind` branch
is **dead** (`is_down` → `false`; the Mind channel was removed 2026-06-20).

- **Verdict:** the *idea* is good (an **incorporeal foe** — a wraith you defeat only by breaking its
  will, immune to physical death — is exactly the kind of emergent variety §6 wants). But an
  **unexercised** categorical is latent complexity and, by §10, an **unmotivatable** stat *as
  shipped* (nothing in play re-derives it). Choose: **(a) activate** — print at least one
  Spirit-keystone creature so the stat earns its slot, or **(b) shelve** — remove it from the live
  model until such a scenario is wanted (YAGNI).
- **Case classification: 3 (intent).** Keystone is part of the §2 defense model; cutting or
  activating it is a design choice about whether the game wants non-Body kill-conditions. **Surfaced.**

### 4. (Code hygiene, not a stat) — `Aspect::Mind` is a vestige.

The Mind/Confusion channel was removed 2026-06-20, but `Aspect::Mind` survives in `stats.rs` with a
dead `is_down` arm, and `DamageType::Confusion` (0 uses in data) is now routed into the Fear channel.
**Case 1 cleanup** — flag for the code/spec-sync pass; not part of the stat roster.

---

## Add / Gaps — driven by the symmetry + clock lenses

The §2.2 parallel-channel table, filled in against ground truth:

| layer                    | outer (Body) | inner (Fear)          | status                                                  |
| ------------------------ | ------------ | --------------------- | ------------------------------------------------------- |
| **attack**               | Strike ✅     | Spirit ⚠️ **unwired** | asymmetric *today* (Strike serves both) — see Remove §1 |
| **cut** (per-hit)        | Armor ✅      | Ward ✅                | symmetric (Ward thin in practice)                       |
| **bar** (per-round)      | Toughness ✅  | Resolve ✅             | symmetric                                               |
| **pool** (per-encounter) | Health ✅     | **— none —**          | **asymmetric by design** — see below                    |
| **cut-bypass** (offense) | Pierce ✅     | **— none —**          | asymmetric (no Ward-bypass) — minor                     |

### Gap A — the inner channel has no pool / no per-encounter layer.

By the **clock lens** (§2.7), the Body channel stacks three clocks — Armor (per-hit) → Toughness
(per-round) → **Health (per-encounter)**. The Fear channel has only **two** — Ward (per-hit) →
Resolve (per-round bar). It has **no per-encounter layer**: nothing inner persists across rounds
(the fear pile clears at round end; only the ScaredToDeath bleed touches the permanent Body pool).
This is *why* Resolve feels shallow — the inner channel is a binary threshold with no graceful
degradation, where the Body channel has a whole pool to chew through.

Two readings:

- **(a) Accept the asymmetry (recommended).** §2.2 *intends* only Body to have a pool, and §2.1's
  **single maintained meter** is the load-bearing comprehensibility rule — a second pool ("Trauma")
  would be a **second tracked number**, breaking §2.1. So the missing inner pool is **guarded, not an
  oversight**: Fear is a *transient per-round status spike* (you panic, then recover), Body is
  *per-encounter attrition*. This is motivated and re-derivable — keep it.
- **(b) Add a per-encounter inner layer.** If the human wants the inner channel to have real depth, the
  lens says the missing stat is a **per-encounter fear accumulator** (a "Dread"/"Trauma" track that
  carries across rounds, raising break risk). **But this costs the §2.1 single-meter guarantee** — it
  is a **second maintained meter**. **Case 3 (intent).** Only worth it if the inner channel is meant
  to be a co-equal attrition axis rather than a spike. **Surfaced — recommend NOT doing it** unless the
  Controller archetype proves too swingy in play.

A **cheaper middle option** (no new meter): give Resolve a small *pool-like* cushion via the
**clock**, e.g. let the fear pile decay by a fixed amount at round end instead of clearing fully — a
"nerves settle slowly" rule. That adds inner-channel depth without a second tracked number. **Case 1**
(a clock tweak inside the existing Resolve stat). Offered as the low-cost path if (a) feels too binary.

### Gap B — no Ward-bypass (minor, do not fill).

Pierce bypasses Armor; nothing bypasses Ward. By symmetry one *could* mint a "Fear-pierce," but the
inner channel is barely typed and Ward is lightly used — a Ward-bypass would be complexity with no
demand. **Recommendation: leave empty.** It mainly confirms Pierce is an outer-channel special case
(reinforcing Remove §2's "is Pierce general enough to be a standing stat?").

### No new *offensive* dimension is missing.

Offense covers magnitude (Strike), channel (Strike/Spirit), bypass (Pierce), breadth (Speed), and
crossing-grade (Drive). That spans the gauntlet's actual decisions. The only offensive defect is the
**dead Spirit**, not a missing axis.

---

## Is the set minimal-complete? — the 3+2 role view (§8.5)

Map each stat to the role-suit that owns it:

| Suit / Role              | Owns (stats)                            | Healthy?                                                   |
| ------------------------ | --------------------------------------- | ---------------------------------------------------------- |
| **Iron / Wall**          | Body, Toughness, Armor, Resolve(+Rally) | ✅ the Body-pool defensive cluster — coherent               |
| **Silver / Infiltrator** | Speed, Drive                            | ✅ the Tempo (breadth) cluster — coherent                   |
| **Brass / Artillery**    | Strike, Pierce                          | ⚠️ Strike is the real axis; **Pierce is a thin appendage** |
| **Bone / Controller**    | **Spirit**, Resolve                     | ❌ **its headline offense stat (Spirit) is dead**           |
| **Salt / Support**       | Resolve, Body, Speed (force-multiplier) | ✅ cross-cutting buffs — coherent                           |

**Judgment: the set is *nearly* minimal-complete, and well-motivated** — almost every stat is the
mechanical substrate of a role, so it passes §10 (re-derivable from the role's job, not arbitrary).
It is **not arbitrary**; it is a role-driven set with three blemishes:

1. **Spirit is dead** → the Controller role is **offensively hollow**. This is the one hole that
   actually breaks completeness: a whole role-suit's primary offense stat does nothing. **Wiring
   Spirit is the single change that makes the set genuinely complete.**
2. **Keystone sits *outside* the 3+2** — no role needs it, and no data uses it. Either give it a home
   (an incorporeal-creature scenario) or shelve it.
3. **Pierce is redundant-leaning** within Brass — Strike already covers Artillery offense; Pierce
   earns its keep only as a priced-cheaper armor-counter.

**If the human wires Spirit, shelves (or activates) Keystone, and reprices-or-folds Pierce, the
roster is minimal-complete for the 3+2 roles** — every remaining stat creates a real tradeoff (Speed's
allocation, Toughness-vs-Armor's hit-shape, Health's attrition) and is re-derivable from a role's
intent. The deepest stats (Speed, Health, Toughness, Armor) are unimpeachable; the shallow ones
(Drive's one-place rule, Resolve's binary bar) are *motivated* shallow, not arbitrary — flagged, not
condemned.

---

## Every case-3 (intent) call, collected for the human

These are **not** mechanics fixes — they change what the game is trying to do. Listed so none is
smuggled in:

1. **Remove Spirit** (vs wire it) — abandons §2.2's parallel-attack symmetry and the fear-specialist
   archetype. *(Wiring it is case 1; removing is case 3.)*
2. **Remove vs activate Keystone** — decides whether the game wants non-Body kill-conditions
   (incorporeal foes) at all.
3. **Remove Pierce** (vs reprice) — drops armor-bypass as a standing build axis.
4. **Add a per-encounter inner layer (Trauma)** — would break §2.1's single-maintained-meter rule;
   only if the inner channel is meant as co-equal attrition. *(The Resolve-decay middle option is
   case 1 and needs no intent change.)*

**Recommended disposition (one analyst's view, human disposes):** **wire Spirit** (case 1, restores
the Controller and the channel symmetry); **shelve Keystone** until a scenario wants it (or print one
Spirit-keystone creature); **reprice Pierce** below Strike or fold it to a card keyword; **accept the
inner-pool asymmetry** (it's guarded by §2.1) — reach for the Resolve-decay tweak only if play proves
the inner channel too swingy. Numbers throughout stay human-tuned.
