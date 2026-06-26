# Powers as Form abilities — the cast/resolve timing model

> **✅ MERGED INTO CANON 2026-06-25.** This design has been folded into the Spec
> (`docs/games/deckbound/canon/2-spec/README.md`): **§5.2** (abilities are Form cards — tempo-gated,
> no Spend), **§4.4** (casting no longer Spends; repeatable, Tempo-bounded), **§4.6**
> (`cast`/`resolve` supersedes instant/deferred; the **per-phase accumulator** RULE), plus the
> consistency fix to **§2.2** ("the round's pile" → "the phase's pile"). This file is **retained as
> the design-rationale record** — the derivation, the Fray/Volley merge analysis, and the worked
> examples — not as a pending action. **Still outstanding (not yet canon):** the combat-log rewrite
> (§7) and the `booklet.ron` Toughness re-tune (human-tuned numbers).
>
> _Original framing (for the record):_ authoritative powers/Form design; **merge targets** §5.2 /
> §4.4 / §4.6; promotion notes in §7.
>
> **Supersedes:** the `instant | deferred` trait (everywhere, incl.
> `log-driven/combat-logs/card-combat-all-mechanics.md`); the §4.4/§5.3 "casting **Spends** the
> card" rule *for abilities*; the per-round accumulator pile (now per-phase).

---

## 1. Architecture — abilities are Form cards, tempo-gated

**RULE.** Every **ability lives on the Form** (§5.2) — your permanent build, face-up in **Active**.
A Form ability is **permanent** (never Spends, immune to Disrupt, never leaves the table), **open
information** (never drawn — no card-draw RNG over your own capabilities), and a **passive
enabler**: having it means you *may* use it, **repeatably**, gated by **one cost only — Tempo**.
There is **no exhaustion on abilities** — using one does **not** flip it face-down. The lone
exception is an explicit **one-shot**, which limits itself by **flipping face-down for the whole
combat** (never resets).

- **Tempo is the only action currency** (Cadence × Finesse, a fixed per-round pool). The **blind
  bid** remains the sole hidden commit: **Form open, bid hidden.**
- **Utility cards are bookkeeping tokens**, not a second action resource and not a draw. They
  physically track the **persistent state** a complex power imposes (a mark, poison stacks, a
  zone). Their **count is Form-derived** (set by the ability's level), so they carry no hidden
  information and no RNG.
- **A "power" and an "attack" are the same object** — a card with
  `(reach, cast, resolve, target, cost, effect)`. A power is just an attack with richer trait
  values (area, a deferred resolve, a buff or lingering effect instead of plain Might). One schema;
  attacks and powers are a spectrum of trait richness.

**WHY.** Putting abilities on the Form keeps the PvE core **deterministic and perfect-information**
(§0.1): your capabilities are never a draw, so there is no RNG over your own kit. Gating by **Tempo
alone** (not a Spend/exhaust clock) makes an ability a clean *repeatable enabler* — its only limit
is the conserved, party-size-invariant Tempo pool (§4.4), so **god ≈ party** still falls straight
out of the tempo economy. Dropping per-ability exhaustion removes a second clock the player would
have to track; the one-shot's **flip-for-combat** is the minimal, fully-visible "once per combat"
limiter — no charges, no hidden timer (§5.1 cards-only).

**GUARANTEES.**
- Abilities are **Active, permanent, open**; they never Spend and are immune to Disrupt (§5.2).
- An ability is bounded **only** by Tempo (and **evade**, for offensive ones) — never by a
  per-card cooldown. A **one-shot** bounds itself by flipping face-down for the combat.
- Capabilities are **never drawn** — no RNG over your own kit (§0.1 preserved).

---

## 2. The round — six phases, five resolution gates

A combat round runs the §4.6 fixed sequence. For ability authoring, read it as a sequence of
**gates**: at each gate some abilities **choose targets** and inject effects; the gate then
**resolves** (flips, then finalizes deaths).

| #   | Phase         | New targeting here?                                                                                                           | What accumulates                                   | Resolution **at the boundary**                                      | What the gate **orders**                         |
| --- | ------------- | ----------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------- | ------------------------------------------------------------------- | ------------------------------------------------ |
| 1   | **Standoff**  | **Yes** — buffs/braces pick **own side** (auto-land, no contest); bid revealed, positions lock                                | buff effects (temp Tempo, +stat)                   | buffs take hold                                                     | braces/buffs active **before** the Fray          |
| 2   | **Fray**      | **Yes** — front picks **enemy front**; instant ranged picks enemy front                                                       | Might → each target's Fray pile; Tempo contests    | flips at Toughness; **deaths finalize → breach list fixed**         | **who is freed** to charge                       |
| 3   | **Volley**    | **Yes** — free Vanguards pick **enemy rear** (charge) or flank; rear picks the **chargers** (answer); instant ranged re-fires | charge/answer Might → Volley piles; Tempo contests | rear's answers resolve; **deaths finalize**                         | **pre-empt** — a charger can die before it lands |
| 4   | **Breach**    | **No** — chargers land on targets chosen in the Volley                                                                        | surviving charge Might → rear's Breach pile        | flips; **deaths finalize → casters killed before Reckoning**        | **disrupt** — no caster, no spell                |
| 5   | **Reckoning** | **No** — deferred / DoT resolve on targets chosen at cast                                                                     | deferred-area Might & DoT ticks → Reckoning pile   | flips; deaths finalize                                              | last damage of the round                         |
| 6   | **Lull**      | **No**                                                                                                                        | —                                                  | refresh: Tempo resets, temp Tempo expires, Health persists, round++ | round boundary                                   |

**Three targeting moments, five resolution gates.** Targets are chosen only in the **Standoff,
Fray, and Volley**. After the Volley **no new target is ever chosen** — the **Breach and Reckoning
are resolution-only gates** that cash out choices made earlier. There are **five resolution gates**
(Standoff…Reckoning); **four resolve damage** (Fray/Volley/Breach/Reckoning), the **Standoff
resolves buffs**. The **Lull is the refresh**, not a resolution gate.

**Two senses of "resolve" — only one is a gate.**
1. **Flip** — a pile crossing Toughness flips one Health card. **Continuous**: it happens the
   instant a hit lands, in any phase.
2. **Death-finalization** — a body at 0 Health is **removed only at the phase boundary** (§1.3: it
   still lands every blow it committed in that phase). **This** is the gate. A death at gate *N*
   silences anything that unit had pending at gate > *N* — every preclusion rule (pre-empt,
   disrupt) is just this.

---

## 3. The accumulator is per-phase

**RULE.** Each phase owns a **per-target accumulator pile**. A landed hit adds **Might** to the
pile of its **resolve phase** (§4); when the pile clears **Toughness**, one Health card flips and
the **overflow is wasted**. **Every pile wipes at its own phase boundary** — sub-threshold damage
does **not** carry to the next phase. **Health (flipped cards) persists** within the fight (the one
maintained meter, §2.1); only the sub-threshold pile is ephemeral.

- **Damage lands in the pile named by `resolve`** (§4). So two effects that **share a resolve
  phase stack in the same pile** — a deferred bomb and a poison tick both land in the **Reckoning**
  pile and can jointly cross Toughness even if neither would alone. Additive and order-independent
  (§0.1 — a combo is diverse effects in a phase, never a multiplying chain).

**WHY.** **Tabletop legibility:** with per-phase wipe, **no pile-number ever crosses a phase
boundary**, so a human never tracks an accumulator across the round — they tally within a phase and
clear it. This is the §2.1 "one maintained meter" (Health) made strict: the *only* number carried
between phases is Health.

**GUARANTEES.**
- No accumulator state crosses a phase boundary; only Health persists.
- An effect's Might always banks into the pile of its **resolve** phase; piles are recomputable
  from the cards committed to that phase.

**Balance consequences (tune deliberately).**
- **Toughness becomes a per-phase wall** — it effectively "regenerates" each phase, so
  high-Toughness bodies get markedly harder to flip.
- **Burst / focus-fire within a phase ≫ chip spread across phases.** Sub-threshold damage is lost
  at every boundary, so concentrating enough Might in *one* phase is decisive. Low-Might attackers
  (the Hoard) must **gang in a single phase** (they already do, in the Fray).
- **More waste ⇒ Toughness numbers likely come *down*** (`booklet.ron`) to keep flips achievable —
  unless burstier combat is the intent.
- **Theme cost (accepted):** a wound chipped across a round "heals" between phases. Traded knowingly
  for the tracking win.

---

## 4. The cast/resolve schema — *replaces `instant | deferred`*

**RULE.** The timing of an ability is **two fields**, over **named cast windows** and **named
resolution gates**:

```
reach    : melee (front / charge)  |  ranged (any lane)
cast     : standing                -- the Standoff (own-side buffs / braces; auto-land)
         | strike                  -- the STRIKE WINDOW: the Fray AND the Volley
                                      (a card usable in one strike window is usable in both)
resolve  : on-cast                 -- lands in the pile of the phase it was used (Fray or Volley)
         | breach                  -- lands in the Breach pile (a charge)
         | reckoning               -- lands in the Reckoning pile (a deferred spell / a DoT tick)
target   : single  |  area (every body in the target group / rank)
cost     : N Tempo  |  0 + one-shot (flips face down for the combat)
effect   : Might damage  |  buff (e.g. temporary Tempo)  |  persistent effect (needs a utility token)
```

**Defaults:** `cast: strike`, `resolve: on-cast` — a plain attack writes neither.

**Legal targets are derived, never enumerated on the card.** A card declares only its **window**;
*what it may hit* in a given phase comes from **reach** (melee = Vanguard/front, ranged = any lane,
§4.2) **+ breach state** (the front shields the rear until cracked; the rear is reachable only by a
**freed** charger). So "enemy front in the Fray, enemy rear-via-charge in the Volley" is **never**
on the card — the targeting rules read it off positions.

**The disruption window is `resolve − cast`,** measured in gates:
- `resolve: on-cast` → window **0** → **undisruptable** (protected by §1.3) — the old "instant."
- `resolve` a later gate → a positive window = the gates **strictly between** cast and resolve =
  exactly where a death can silence it — the old "deferred." **Author's dial:** *choose `on-cast`
  when the effect should be guaranteed (a trade); choose a later gate when it should be disruptable
  — the further out, the longer it is exposed.*

**WHY.** Two open fields generalize the binary enum and make the **disruption window an explicit,
computable property of the card** (§4.6 PRINCIPLE: phases exist only to order a death against a
later effect — now a one-line authoring rule). Naming the **Strike window** removes the need to
repeat `{Fray, Volley}` on every attack and encodes "every Fray card is a Volley card" as a
definition. Deriving targets from reach + breach state keeps a card from carrying positional logic
it would otherwise duplicate.

**GUARANTEES.**
- `instant` ≡ `resolve: on-cast`; `deferred` ≡ `resolve: reckoning`. No expressive loss.
- A **Strike-window** card is usable in **both** the Fray and the Volley (Tempo permitting — it
  is repeatable, §1).
- Effect Might always lands in the pile of its **resolve** gate (§3).
- The **charge** falls into the schema (`cast: strike, resolve: breach`), confirming `breach` is a
  first-class resolve gate; its Volley-restriction comes from needing a **freed** unit, not from
  the card.

**Common abilities under the schema.**

| Ability                           | reach          | cast     | resolve   | note                             |
| --------------------------------- | -------------- | -------- | --------- | -------------------------------- |
| Punch / Throw Rock (plain attack) | melee / ranged | strike   | on-cast   | the default                      |
| Charge (melee breakthrough)       | melee          | strike   | breach    | Volley-only via *freed* state    |
| Throw Bomb (deferred area)        | ranged         | strike   | reckoning | disrupt window = Breach          |
| Hunter's Mark (apply)             | ranged         | strike   | on-cast   | then persists via token          |
| Poison (apply)                    | ranged         | strike   | on-cast   | then ticks in the Reckoning pile |
| Rallying Cry (one-shot buff)      | —              | standing | on-cast   | flips face-down for the combat   |

---

## 5. Worked examples

### 5.1 Hunter's Mark — a persistent debuff
- **Schema:** `ranged · strike · on-cast · single · N Tempo · persistent`. On your **Form**:
  `Hunter's Mark`; in supply: one or more **mark** tokens (count = ability level).
- **Cast & land:** in a **strike window**, spend Tempo → a **ranged attack** to *land* the mark;
  the target may **evade** it like any ranged shot (§4.2). It resolves **on-cast** (no
  disruption window — once it lands, the token is on the target).
- **Effect (persistent):** while marked, the target has **−2 Finesse, minimum 1**, until [the card
  states — e.g. end of combat]. This is a **stat-drop read off the token**, not a damage track
  (§2.2 / Charter #13) — Hunter's Mark has **no tick**.
- **Scaling:** higher levels grant **more mark tokens** → more simultaneous marked targets.

Two anti-fiat properties: **landing the mark is itself an attack** (earned, not decreed, and
evadable); the **`min 1` floor** lets a debuff grind but never **zero out** — force-not-fiat,
enforced by a number.

### 5.2 Poison — a damage-over-time
- **Schema:** `ranged · strike · on-cast · single · N Tempo · persistent (DoT)`.
- **Cast & land:** apply via the normal ranged contest (evadable), then **place 3 poison markers**
  on the target.
- **Tick:** at each **Reckoning**, **remove 1 marker and deal 3 Might into the target's Reckoning
  pile.** The markers are both the **state** (how much poison is left) and the **clock** (it ends
  when they run out).
- **Caster-independence (derived, not a new rule):** once **applied**, the token is on the target,
  so the DoT **keeps ticking even if the caster dies** — contrast a *held* deferred attack, which
  is dropped if its caster dies before the Reckoning. The discriminator is simply *"has it landed
  yet?"* — a deferred attack's Cast→Land gap spans the Breach (disruptable); an applied DoT has
  already landed.

### 5.3 Rallying Cry — a one-shot pre-combat buff
- **Schema:** `— · standing · on-cast · party · 0 + one-shot · buff`. On your **Form**:
  `Rallying Cry`.
- **Cast:** in the **Standing window** (the Standoff); it **auto-lands** (a buff — no contest) and
  **flips face down for the whole combat** (never resets).
- **Effect (buff):** each ally gains **+1 temporary Tempo this round**; temporary Tempo **expires
  at the Lull** (it never refreshes).
- **No utility token** — the effect is instantaneous; the **face-down flip is its own
  bookkeeping**.

Patterns to lift: **one-shot via flip-face-down-for-combat** generalizes to any once-per-combat
ability (no charges, no timers); **temporary Tempo** is the minimal buff (more of an existing
resource, not a new stat); it is **load-bearing** in the worked log (a C3 unit spends its temp
Tempo to land the disrupting Breach blow).

---

## 6. Open questions (flagging, not deciding)

*(Resolved and removed: the deferred-cast-moment dial — dissolved into the per-card `cast` window;
the instant/deferred enum — replaced by cast/resolve; the per-round-vs-per-phase accumulator —
**per-phase**; per-ability Spend — **removed, tempo-gated**.)*

- **Effect lifetime convention** — should a persistent effect default to **whole-combat** (like
  Health) or **per-round** (like the pile/Tempo), with the card overriding? Pick the default.
- **Token cleanup** — do markers return to supply when the target dies / combat ends? Does a marked
  target dying **free the mark** for re-use this combat (interacts with scaling)?
- **Defending an area application** — when an *area persistent* effect lands on a group, does it
  follow weakest-link evade / sum-block like other ranged attacks (§4.5), and does it touch every
  member (like AoE) or spill?
- **Necessity test (§6.1)** — each new power must ship with a scenario it is **required** to win
  (naive line provably loses, keyed line wins). A power with no such scenario is **fiat or
  redundant** — cut it.
- **Power scaling (§8)** — *"all powers scale to level"* via typed axes (Magnitude / Breadth /
  Duration / Cost); curve granularity **decided 2026-06-25**: shared-per-axis default + per-power
  override on demand, on the base-2 rail.

---

## 7. Promotion notes — what changes where

- **§5.2 (Form vs Action):** add that **abilities are Form cards** — Active, permanent, never
  Spend, immune to Disrupt, **tempo-gated with no exhaustion** (one-shots flip for the combat).
  Adopt §1 here.
- **§4.4 (the ability layer):** **remove "casting Spends the card"** for abilities; an ability is a
  **repeatable** enabler bounded by Tempo (+ evade for offense). Keep the Rearguard-cast /
  offensive-spell-is-a-ranged-attack rule.
- **§4.6 (the six phases):** **replace the instant/deferred RULE** with the **cast/resolve schema**
  (§4 here); add the **per-phase accumulator** RULE (§3 here, supersedes "the pile carries within a
  round, wipes at the Lull"); add the **derived-targeting** note. Keep all six phases / five gates.
- **§2.2 / §3 numbers:** revisit **Toughness** values in `booklet.ron` for the per-phase wall (§3
  balance notes).
- **`card-combat-all-mechanics.md` and other logs:** re-express `instant`/`deferred` as
  `cast/resolve`, and the per-round pile as **per-phase** (the legend's "carries within a round,
  wipes at the Lull" line in particular).

---

## 8. Power scaling — the level axes 🟡 *(direction 2026-06-25; curve granularity **decided** 2026-06-25)*

**Direction.** Every power **scales to level**, but along a **small set of typed axes** — *not* one
universal magnitude curve. A power is tagged with a **primary scaling axis**; its level picks a point
on that axis's curve.

**The axes.**
- **Magnitude** — Might per hit (damage powers). Gated by the **per-phase Toughness wall** (§3): a
  step function — `base × f(L)` must clear Toughness *within its resolve phase* to flip, so leveling
  buys **breakpoints**, not smooth growth.
- **Breadth** — targets / tokens / extra foes (Mark's marks, AoE rank, a Curse's +1 foe). The
  **debuff axis** — forced here because the **`min 1` floor** (force-not-fiat) caps magnitude, so a
  debuff can only grow by hitting *more bodies*, never *harder toward a lock*.
- **Duration** — ticks / rounds (DoT markers, lingering zones). The time axis.
- **Cost / efficiency** — the **Tempo** paid (or effect-per-Tempo). Must move with the others, or
  leveling just inflates the one-pool economy (§4.4).

*(Binary effects — execute, "cannot fall," disrupt — scale on **availability / breadth**, never
magnitude.)*

**Why this makes the rewrite/rebalance easier.**
- **Fewer free parameters** — tune a base + a curve per axis, not N loose numbers per power → a far
  smaller space for the par-solver (`computability-and-balance.md` §10 / §0.3).
- **Monotonicity for free** — level-scaling makes every power monotone, so **dominance pruning**
  ("higher level dominates lower, all else equal") is valid — the very §0.1 invariant the computable
  core already relies on.
- **Progression is one atom** — a level rides the **base-2 denomination** encoding (§2.5): "level up
  a power" = "add a denomination card," the same rail stats use (§8.5). No new machinery.
- **Comparable units** — a common level scale lets the **closure check** and the **necessity test**
  (§6.1) compare powers on one axis instead of bespoke per-power judgment.

**DECIDED 2026-06-25 — curve granularity: shared curve per axis (default) + per-power override on demand.**
- **Shared-per-axis** *(the default)* — all Magnitude powers ride one `f_mag(L)`, all Breadth powers
  one `f_brd(L)`, etc. Fewest dials, maximal comparability, trivially monotone; a power's identity
  lives in its **traits + Cost**, never a private damage number (extends the stat-collapse philosophy).
- **Per-power override** *(deliberate exception)* — a power may name its own curve **only where its
  identity genuinely demands it**. The extra dial is **paid for on purpose, never taken by default**;
  when used, its monotonicity is re-checked individually.
- **Encoding:** rides the **base-2 denomination** rail (§2.5) — the shared structure is "level adds a
  denomination"; the default is a uniform per-level increment per axis, an override sets a power-specific
  base/increment. This is the parameterization the eventual par-solver consumes
  (`computability-and-balance.md` §10).
