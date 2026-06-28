# Power catalog rewrite — the 25 under the new rules

> **✅ MERGED INTO `role-card-redesign.md` §10 (2026-06-25).** The catalog, axes, retire/convert list,
> token vocabulary, and §6.1 necessity audit below were folded into the design canon (`§10`, `§10.6`,
> and the §8 Migration-status). **`booklet.ron` is NOT updated** — it's blocked on the schema/code
> migration (new `Card.cast`/`resolve`, token effect-kinds, one-shot flag, per-phase piles); see
> §8 item (5) there. This file is **retained as the rationale + necessity-audit record.**
>
> _Original framing (for the record):_ Companion to `powers-redesign-form-abilities.md` (merged). Rebuilds the
> **25 role cards** (5 roles × 5 levels) for the new framework: **cast/resolve** timing, the
> **per-phase accumulator**, **tempo-gated / no-Spend** abilities, the **scaling axes**
> (Magnitude / Breadth / Duration / Cost), and the new **utility-card** design space.
>
> **Hard constraint (this pass):** *every effect is tracked with a **card or token** — never human
> memory* (§5.1 cards-only). Each persistent / charging / accumulating state below names the physical
> token that carries it.
>
> **Merge target:** `role-card-redesign.md` §10 + `crates/deckbound/data/booklet.ron` (numbers are
> human-tuned — shown here as *(appendix)* illustrations only). Schema per entry:
> `reach · cast · resolve · target · cost · effect`.

---

## 0. Cross-cutting changes (what the rules force on the old set)

**Retired — cannot survive:**
- **Disarm** ("cannot play role cards"). Abilities are **Form cards, immune to Disrupt, never Spent**
  (§5.2) — nothing to knock Down. *(Cleansing a **utility token** is a different, new effect.)*
- **Hard action-bans** (Stagger "loses its action," Menace). Blanket "you don't act" is **fiat**
  (wins by banning, not out-playing). Survives **only** as targeted **non-lethal disrupt of a
  *deferred* spell** (§4.6).
- **"Free action" riders** (Blitz "first slip free"). Violates *Tempo is the only currency* → becomes
  a **Cost-axis discount** (a flat reduction, no per-use memory), never a literal free action.
- **Execute / "down regardless of health"** (Assassinate M4). Violates the **single kill-condition**
  (§2.2 — you die exactly one way, the pile empties). Reframed as **overwhelming Magnitude** (a burst
  big enough to empty a Rearguard's pool in one phase), not a fiat bypass.

**Converted:**
- **All "zone: Spend" capstones** → **one-shots** (flip-face-down-for-combat) *or* **charge-gated**
  (limited by the charge investment, §1 below). Spend doesn't exist for abilities.
- **Control = stat-drop**, gradable, **min-1 floored** (§2.2 / Charter #13). −Cadence / −Finesse /
  −Tempo via tokens; never a binary lock.

**Reshaped by the per-phase accumulator:**
- **Low-power AoE** weakens (each target's pile must clear Toughness *in one phase*) → its niche is
  **anti-group / Hoard** (AoE hits every member, §4.5).
- **Burst** strengthens (burst beats chip).
- **+Toughness buffs** become strong (Toughness is now a **per-phase wall**) — the Wall's new spine.

**Newly enabled (utility cards):** persistent stat-drops, DoT, charge-up, reflect (thorns),
cover/redirect, regen, zones, deferred wind-ups — all in §1.

---

## 1. Utility-card vocabulary (every new token, and what tracks it)

Each is a **physical token / card** on the table — the state is *seen*, never remembered.

| Token          | Carries                                                                       | Sits on                           | Resolves / ticks                                | Used by                |
| -------------- | ----------------------------------------------------------------------------- | --------------------------------- | ----------------------------------------------- | ---------------------- |
| **Guard**      | +Toughness while present                                                      | the holder                        | passive (raises the per-phase wall)             | Wall                   |
| **Cover**      | redirect: single-target damage on the covered ally spills to the holder first | the covered ally                  | at damage resolution (§4.5)                     | Wall                   |
| **Charge**     | +1 step of magnitude when consumed                                            | the **ability card** (Active)     | placed one round, consumed a later round (§5.4) | Infiltrator, Artillery |
| **Mark**       | −Finesse (min 1) while present                                                | the marked foe                    | passive stat-drop                               | Controller             |
| **Mire**       | −Cadence (min 1) while present                                                | the mired foe                     | passive stat-drop                               | Controller             |
| **Burn** (DoT) | deal Might, −1 token/round                                                    | the burning foe                   | **Reckoning** pile, each round                  | Artillery              |
| **Thorns**     | reflect Might to any attacker                                                 | the warded ally                   | when the ally is attacked (attacker's pile)     | Support                |
| **Smoke**      | one **uncontested** slip (interceptor can't bid)                              | the Infiltrator (consumed on use) | when slipping / charging                        | Infiltrator            |

*(One-shot use is tracked by the **ability card flipped face-down**; the per-phase pile is tracked by
**cards in the pile zone**. So nothing — charges, debuffs, DoT clocks, reflects, regen — lives in a
player's head.)*

**Note — charges & §5.4 / §2.1.** Charge-up rides the existing **§5.4** Charge mechanic (Lasting
cards consumed for ×magnitude; "burst is paid for by the setup Rounds"). Charges persist across the
Lull as **visible cards**, so they honor cards-only (§5.1); they are a *second within-battle
quantity*, which lightly stretches §2.1's "one maintained meter" — **acceptable because it is
physical, not mental** (flag for ratify).

---

## 2. Wall (Iron) — *hold the front, shield the back* · axes: **Cost + Breadth**

New spine: braces raise **Toughness** (the per-phase wall), and protection is a **trackable cover
token**, not a remembered promise.

- **L1 — Brace** (Base). `— · standing · on-cast · self · N Tempo · buff(token)`. Place a **Guard
  token** → **+Toughness** this round. **Axis: Cost** (more Tempo → bigger guard). *(appendix: +2 T)*
- **L2 — Phalanx** (Modifier, rides free). The group's **Guard tokens pool** — every member shares the
  Wall's guard (a superb summed wall, §4.5). **Axis: Breadth.**
- **L3 — Aegis** (Base). `— · standing · on-cast · ally · N Tempo · cover(token)`. Assign a **Cover
  token** to a chosen ally → single-target damage on that ally **spills to the Wall first** (extends
  §4.5 spillover to a non-adjacent ally — *protect the back*). **Axis: Breadth** (cover more allies).
- **L4 — Shield Sweep** (Base). `melee · strike · on-cast · **area** · N Tempo · Might`. A **melee
  AoE** — strike every enemy body in the **front rank** (hits each, §4.5). The Wall's one offense.
  **Axis: Breadth.** *(your melee+AoE)*
- **L5 — Last Stand** (one-shot, flip-for-combat) **+ Taunt** (Modifier). *Last Stand:* a **1-Health
  floor** this round — damage that would down the holder leaves it at 1 (a *number*, not immunity;
  it's fully exposed, just not killable this round). *Taunt:* chargers/attacks are pulled to this Wall
  first (the modifier card is the standing rule — no per-instance memory). **Axis: Cost.**

*Retired:* round-only Guard-as-Tempo (now Guard-as-Toughness token); nothing else — Wall maps cleanly.

---

## 3. Infiltrator (Silver) — *slip the wall, gut the back* · axes: **Magnitude + Breadth**

The slip/charge **is** the breach mechanic: `cast: strike, resolve: breach` (declared in the Volley,
lands in the Breach). Signature utility: the **charge-up** (the patient assassin).

- **L1 — Slip Strike** (Base). `melee · strike · **breach** · single · N Tempo · Might + shove`. The
  charge blow; **shove** pushes the foe out of melee (positional — tracked by position). **Axis: Magnitude.**
- **L2 — Smoke** (Base + **Smoke token**). `— · strike · on-cast · self · N Tempo · enabler(token)`.
  Place a **Smoke token**; spend it to make one slip/charge **uncontested** — the interceptor cannot
  bid to stop it (a guaranteed breach). It does **not** stop the rear's Volley pre-empt, so it stays
  bounded (token-limited + Tempo-paid, never blanket immunity). **Axis: Breadth** (more tokens = more
  guaranteed slips). *(replaces Blitz — a necessity-passing "must," not a discount.)*
- **L3 — Shadowstep** (Modifier, rides free). **Win ties** when evading past an interceptor (helps the
  weakest-link evade, §4.5). The modifier card is the rule; applied at contest resolution.
- **L4 — Coiled Strike** (Base + **Charge tokens**). Spend a round to place **Charge tokens** on this
  ability; a later strike **consumes them for +Might each** (§5.4). **Axis: Magnitude** (via time
  investment — burst paid for by the setup round). *(your charge-up example)*
- **L5 — Assassinate** (one-shot, flip-for-combat). `melee · strike · breach · single · N Tempo · **big**
  Might`. An overwhelming burst vs an enemy **Rearguard** — enough to empty a caster's pool in one
  phase (kills by Magnitude, **not** by execute-bypass). Lethal when stacked with Coiled Strike.
  **Axis: Magnitude.**

*Retired:* Blitz (free/discount slip — failed the necessity test, replaced by Smoke); execute/"down
regardless" (→ burst, single kill-condition).

---

## 4. Artillery (Brass) — *ranged burst, AoE, and the slow shell* · axes: **Magnitude + Breadth**

Burst is king now; AoE is the anti-group tool; and Artillery owns **DoT** (it deals damage) and the
**deferred** wind-up.

- **L1 — Bolt** (Base). `ranged · strike · on-cast · single · N Tempo · Might`. Single-target burst.
  **Axis: Magnitude.**
- **L2 — Volley** (Base). `ranged · strike · on-cast · **area** · N Tempo · Might`. AoE — every body in
  a rank/group (anti-group, §4.5). **Axis: Breadth.**
- **L3 — Incendiary** (Base + **Burn tokens**). `ranged · strike · on-cast · single/area · N Tempo ·
  **DoT**`. Place **Burn tokens** → each **Reckoning**, deal Might into the target's Reckoning pile and
  remove one (caster-independent once applied). **Axis: Duration.** *(DoT lives here — it's damage.)*
- **L4 — Longshot** (Modifier, rides free). This Rearguard's ranged fire may target the **enemy
  Rearguard** directly (the sanctioned sniper exception to derived-targeting). The modifier card is the
  standing rule.
- **L5 — Bombardment** (Base + **Charge tokens**, **deferred**). Charge over a round (Charge tokens),
  then release a **`resolve: reckoning`** AoE scaled by charges — a siege wind-up that's **disruptable**
  (kill the Artillery in the Breach → it fizzles). **Charge-gated, not one-shot** (the charge
  investment is the limiter). **Axes: Magnitude + Breadth.** *(charge-up × deferred × AoE)*

*Retired:* Spend-zone Bombardment (→ charge-gated deferred). *Moved in:* DoT (was mis-filed as a
Controller idea; it deals damage, so it's Artillery's).

---

## 5. Controller (Bone) — *break them without a scratch* · axes: **Breadth + Duration**

Position-agnostic, but offensive debuffs are **ranged attacks** (Rearguard-cast, evadable, §4.4).
**No damage, ever** — only **stat-drops** (min-1 floored) and **disrupt**, all on tokens.

- **L1 — Mark** (Base). `ranged · strike · on-cast · single · N Tempo · persistent(token)`. **Mark
  token** → **−Finesse (min 1)** while marked, whole-combat. Evadable. **Axis: Breadth** (more marks).
- **L2 — Mire** (Base). `ranged · strike · on-cast · single · N Tempo · persistent(token)`. **Mire
  token** → **−Cadence (min 1)**, shrinking the foe's Tempo pool. **Axis: Duration.** *(replaces
  Shackle — keeps the Slow stat-drop, drops the dead Disarm)*
- **L3 — Hex** (Base). `ranged · strike · on-cast · **area** · N Tempo · persistent`. Marks every body
  in a target rank (§4.5 AoE). **Axis: Breadth.** *(replaces Terror's AoE)*
- **L4 — Curse** (Modifier, rides free). Each Controller card you play hits **+1 additional foe**.
  **Kept** (M5) — it *is* the Breadth axis as a passive.
- **L5 — Silence** (one-shot, flip-for-combat). Cancel/delay a foe's **deferred** spell at the
  Reckoning (legit non-lethal disrupt, §4.6) and **rout** several foes (Vanguard→Rearguard — positional,
  tracked by position). **Axis: Breadth.** *(Unmake off the Spend zone; its old Stagger reframed as
  legitimate disrupt, not a blanket lock)*

*Retired:* Stagger-as-total-lock and Disarm (both dead); standalone Shove/Rout folded into L5.

---

## 6. Support (Salt) — *no blade of its own* · axes: **Magnitude + Breadth**

Buffs/heals are `cast: standing` (resolve before damage, §1.9). Cannot deal damage **directly** — but
**reflected** damage is the attacker's own doing, which opens **Thorns**.

- **L1 — Haste** (Base). `— · standing · on-cast · ally · N Tempo · buff`. +Tempo to an ally this
  round. **Axis: Magnitude/Breadth.**
- **L2 — Empower** (Base, Lasting). `— · standing · on-cast · party · N Tempo · buff(Lasting)`. +Might
  to the party this round. **Axis: Breadth.**
- **L3 — Thorns** (Base + **Thorns token**). `— · standing · on-cast · ally · N Tempo · reflect(token)`.
  Place a **Thorns token** on an ally; when that ally is **attacked**, the attacker takes reflected
  Might into **its own** pile. Support's only "offense" — and it's the *attacker* hurting itself, so it
  doesn't count as Support dealing damage. **Axis: Magnitude.** *(your thorns idea)*
- **L4 — Mend** (Base). `— · standing · on-cast · ally · N Tempo · heal`. Restore Health cards to the
  most-wounded ally (burst rescue; the restored Health cards are their own tracker — no token).
  **Axis: Magnitude.** *(restored — same-round single-ally rescue Regen/Sanctuary couldn't cover.)*
- **L5 — Sanctuary** (one-shot, flip-for-combat). Empower **+** Haste **+** Mend the **whole party**
  (`target: all allies`). **Axis: Breadth.**

*Retired:* Spend-zone Sanctuary (→ one-shot); Bolster (folded — Empower + Mend cover it). *Added:*
Thorns (indirect offense, clean). *Parked:* Regen / `resolve: lull` heal-over-time — no current slot,
but the schema window stays available if a 6th Support effect is ever wanted.

---

## 7. Decisions made this pass — flag for ratify

1. **Assassinate: execute → overwhelming burst** (preserves the single kill-condition, §2.2). The old
   "down regardless of health" (M4) is dropped as a fiat second kill-condition.
2. **DoT is Artillery, not Controller.** Controller never deals damage; DoT does. Controller's
   persistence is **stat-drop tokens** only.
3. **Blitz: free action → flat Cost discount** (no "first per round" memory; honors Tempo-only).
4. **Not every capstone is a one-shot** — **Bombardment is charge-gated** instead (the charge
   investment is its limiter). Variety in limiter type, both card-tracked.
5. **Charges persist across the Lull as cards** (rides §5.4) — a second within-battle quantity that
   lightly stretches §2.1's "one maintained meter," accepted because it is **physical, not mental**.

## 8. Resolved mechanics (token lifecycle & the floor)

- **Token cleanup.** A token returns to its owner's supply when **its bearer dies** or at **combat
  end**; a freed token may be **re-applied** (re-cast, paying Tempo). The Breadth pool is a
  redeployable resource; a dead target never strands the investment. *(Card-tracked: the token
  physically returns to supply.)*
- **`min 1` stacking.** Each stat floors **independently at 1**, so a maximally Marked+Mired foe has
  Finesse 1 **and** Cadence 1 → Tempo = `1 × 1 = 1` (one feeble action, **never zero**): no debuff
  stack can lock a foe (force-not-fiat, enforced by the floor). **Same-stat stacking saturates at the
  floor** (a second Mark on a floored foe is wasted) — which is *why* the Controller's axis is
  **Breadth, not depth**: the floor itself forces spreading over piling.

## 9. Necessity audit (§6.1 removal test — one-line "required-to-win" per power)

✓ earns it · ⚠ overlap to differentiate · ✂ no scenario found (cut candidate).

- **Wall** — Brace ✓ · Phalanx ✓ · Aegis ✓ · Shield Sweep ✓⚠(*vs* Volley: melee/front-only) ·
  Last Stand+Taunt ✓
- **Infiltrator** — Slip Strike ✓ · **Smoke ✓** (guaranteed uncontested slip — raw Tempo can't buy
  it) · Shadowstep ✓ · Coiled Strike ✓⚠ · Assassinate ✓⚠(*vs* Coiled)
- **Artillery** — Bolt ✓ · Volley ✓ · Incendiary ✓ · **Longshot ✓** (only no-breach back-reach) ·
  Bombardment ✓⚠(*vs* Volley)
- **Controller** — Mark ✓ · Mire ✓ · Hex ✓ · Curse ✓ · **Silence ✓** (only stop for an unreachable
  deferred bomb)
- **Support** — Haste ✓ · Empower ✓ · **Thorns ✓** (only Support offense) · Mend ✓ (same-round
  single-ally rescue) · Sanctuary ✓

**Findings:**
1. **Blitz cut → Smoke** *(resolved 2026-06-25)* — replaced by the uncontested-slip token, which passes
   the test.
2. **Mend restored** *(resolved 2026-06-25)* — swapped in for Regen at Support L4; Regen parked (§6).
3. **Differentiate** Coiled vs Assassinate (setup-repeatable vs no-setup one-shot) and Bombardment vs
   Volley (charged-deferred vs cheap-instant) — keep both, distinction explicit. *(open: confirm in numbers)*

## 10. Open / next

- **Numbers → `booklet.ron`** (human-tuned), then the **scaling curves** (rationale doc §8):
  shared-per-axis default, per-power override only where justified.
- **Full §6.1 scenarios** — promote each audit line to a runnable naive/keyed pair; topo-sort into the
  dependency graph (doubles as tutorial order). Pending the par-solver / combat-lab build.
