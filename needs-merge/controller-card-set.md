# Controller (Bone) card-set — rebalanced for fear-as-control

**Status:** proposal for ratification (2026-06-21). Suggestions only — **numbers and card names are
seeds; human disposes.** Authored to `needs-merge/` so the card-design instance merges it into
`booklet.ron` (avoid a collision). Implements **Charter #13** (Controller deals *no direct damage*) and
the **§2.2 fear-as-control** rewrite (fear → Freeze / Shaken / Rout, never Body).

Companion to the §8.6 **Controller-lock** rebalance (the reference-scenario / `balance.rs` work, another
instance's lane) — the two are partners: this fixes the *cards*, that fixes the *scenarios*. Doing one
without the other will **not** turn the two red balance tests green.

---

## 1. The principle — two disable axes, fear is the spine

A Controller has exactly two ways to take a foe out of the fight, and a good set carries **both**:

- **Will-break (Dread).** Fear past the foe's Resolve breaks its will — **Freeze** (>R, lose action) →
  **Shaken** (>2R, +can't defend) → **Rout** (>3R, +driven to Reserve). **Scales with Dread, gated by
  Resolve.** The answer to *low-Resolve* foes; useless against the brave.
- **Hard-lock (flat status).** `Slow` (drain action economy) / `Disarm` (can't play role cards) —
  applied **regardless of Resolve**. The answer to *fear-immune (high-Resolve)* foes.

The current track has **three overlapping economy drains** (Slow −Speed, Confuse −Tempo, *and* the new
fear-Freeze all = "fewer enemy actions") and **buries fear at L2**. The fix: make fear the through-line
from L1, keep **one** consolidated hard-lock, drop the overlap.

---

## 2. The proposed track

| L | Card | Effects (seed) | Axis | Stat bundle (seed) |
|---|---|---|---|---|
| **1** | **Dread** | `Damage(fear, power 2)` | will-break — *the signature, from L1* | `dread 2` |
| **2** | **Shackle** | `Slow(speed 2)` + `Disarm` | **hard-lock** (Resolve-proof) — collapses old Slow + Confuse | `dread 1, resolve 1` |
| **3** | **Terror** | `Damage(fear, power 4)`, reach (2,2) | will-break — *escalate the tier* | `dread 2` |
| **4** | **Curse** | passive: +1 debuff target (unchanged) | AoE widener | `dread 1, resolve 1` |
| **5** | **Unmake** | `Damage(fear, power 4)`, ×3 targets, Spend | **mass will-break** capstone | `dread 2` |

Change-from-current: **Slow→Dread** at L1 (fear anchors the role); **Confuse→Shackle** at L3→L2
(consolidate the two economy drains + Disarm into one Resolve-proof hard-lock); **Terror** keeps its slot
but is now **pure fear** (its explicit `Stagger` already removed — redundant with Freeze); **Curse** and
**Unmake** unchanged. `Confuse` is retired (its `Confuse(tempo)` + `Disarm` fold into Shackle).

### Per-card rationale
- **L1 Dread** — a cheap single fear; against a typical foe its low pile lands **Freeze** (lose action).
  The Controller's *opening* move now expresses its identity.
- **L2 Shackle** — the **flat** lever: it works on the brave foe that shrugs off fear. One card replaces
  the redundant Slow + Confuse, and folding `Disarm` in gives it a clear job: *shut down an ability foe.*
- **L3 Terror** — heavier fear (higher pile → **Shaken/Rout** as Dread grows); the will-break escalates
  with investment (#5 doom-to-mastery).
- **L4 Curse** — unchanged; the AoE widener that turns single debuffs into line-wide control.
- **L5 Unmake** — the capstone: mass fear, so a deep Controller can **Freeze/Shaken** a whole rank at
  once (with Curse, wider still). Note the dominance lever below.

---

## 3. The power dial & balance levers

- **Dread curve (seed):** +2 / +1 / +2 / +1 / +2 across L1–L5 → maxed **Dread ≈ 8**. That sets how high
  the fear tier climbs: vs Resolve R, Dread 8 lands Freeze at R≥8, Shaken at R 4–7, **Rout at R<3**.
- **Foe Resolve is the counter-dial.** Against **Resolve-0** foes (today's reference creatures) *any*
  fear → **Rout** (full disable) — and with Curse + Unmake a maxed Controller **Routs the whole line**.
  That is the  dominance risk: total disable, even without kills, trivializes a fight. **Balance fix lives
  in the scenarios, not the cards:** most foes should carry **Resolve** so fear tiers *down*, and the
  Controller's §8.6 **lock** is a deliberately **low-Resolve, high-threat** foe only fear can shut down.
- **Mass control gate:** keep AoE fear at the **capstone** (L5) and the widener as a **modifier** (L4),
  so line-wide disable is an end-state investment, not a turn-one default.

---

## 4. Case classification & open calls

- **Retiring `Confuse`, consolidating into `Shackle`** — *case 1* (removes redundancy) but changes
  content; **numbers human-tuned.**
- **Moving fear to L1 (Slow→Dread)** — *case 1* (re-arrangement; intent unchanged — still "the unmaker").
- **Card names** (`Dread`, `Shackle`) — flavor seeds; rename freely.
- **The dominance lever** (foe Resolve / the lock scenario) — *case 3 / balance*, **human + the
  `balance.rs` instance's lane.** This proposal only commits the *cards*.

**Open for the human:** (1) the two-axis split (1 hard-lock card enough, or keep two?); (2) the Dread
curve / fear `power` numbers; (3) names. Once ratified, implementation is a `booklet.ron` edit
(coordinated with the card-design instance) — no engine change (the fear-as-control engine is already
landed in `stats.rs`/`combat.rs`).
