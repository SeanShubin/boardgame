# Combat reference — how to read the card-combat logs

The legend and rules digest for every log here (`designer/card-combat-all-mechanics.md`,
`designer/card-combat-round-4v4.md`, `designer/card-combat-round-breach.md`,
`designer/mechanic-validation-six-phase.md`, and the `tutorials/` series).
Canon source: Spec §4.6 (the six phases, cast/resolve, the per-phase accumulator), §2.1–§2.2
(stats, Health), §3.4 (defender responses), §1.3/§1.9 (death-timing, order-independence).

These logs model the **bare combat engine**. Out of scope (canon, but not shown): armor /
damage-types (gear), the `persist` / `cleave` keywords, and role-power cards. "Abilities" here
are the four bare stand-ins (Punch / Throw Rock / Throw Bomb / Rallying Cry).

---

## 1. The five stats

| Stat          | Letter | What it is                                                                                                                  |
| ------------- | ------ | --------------------------------------------------------------------------------------------------------------------------- |
| **Might**     | M      | Damage magnitude of a landed hit. Pours into the target's pile. **Finesse-blind** — the wound size is Might, never the bid. |
| **Vitality**  | V      | The **number of Health cards** you have.                                                                                    |
| **Toughness** | T      | The **flip threshold** — a Health card flips once its pile reaches T.                                                       |
| **Cadence**   | C      | The **number of Tempo cards** you have — i.e. actions per round.                                                            |
| **Finesse**   | F      | The **value of each Tempo card** in a contest. A bid is `cards × F`; F is also your evade strength.                         |

**Derived pools.**
- **Health** = `V` cards, each absorbing `T` damage before it flips ⇒ effective HP = **V × T**.
  Health is the **one maintained meter**: flipped cards **persist** across phases *and* rounds.
- **Tempo** = `C` cards, each worth `F` ⇒ bidding power = **C × F** per round. Tempo **refreshes
  each round** and is **shared across all six phases** (it does **not** refresh between phases).

---

## 2. The Tempo contest, and the three responses

Every attack is a **bid**: `cards × Finesse` of Tempo. To avoid it, the **defender must
*strictly beat* the bid** — **a tie lands the hit.** Standing still and *soaking* a blow are
free; only *acting* (attacking or actively defending) spends Tempo.

A defender picks one of three responses (§3.4):

| Response                                 | Cost                      | Result                                                             |
| ---------------------------------------- | ------------------------- | ------------------------------------------------------------------ |
| **AVOID** (block / dodge / slip / evade) | Tempo to **beat** the bid | Blow whiffs; no damage                                             |
| **STRIKE-BACK** (counter)                | Tempo + a melee attack    | **Mutual** — the blow still lands **and** you deal your Might back |
| **EAT**                                  | nothing                   | Take the Might; deal nothing back (conserve Tempo)                 |

---

## 3. Damage and the per-phase accumulator pile

A landed hit adds **Might** to the target's **pile**. When the pile reaches **Toughness**, one
Health card **flips** and the **overflow is wasted** (pile resets to 0).

- The pile is **per phase, per target** (`P n/T` = `n` banked toward Toughness `T`). It
  **carries across actions within a phase** and **wipes at that phase's boundary.** **Only Health
  crosses a boundary** — sub-threshold damage never carries to the next phase.
- A hit banks into the pile of its **resolve phase** (§4). Two effects that share a resolve
  phase **stack in the same pile** (e.g. a Reckoning bomb + a poison tick can jointly cross
  Toughness). Additive and order-independent — never a multiplying chain.

---

## 4. Positions, reach, and the per-unit lock

- **Positions:** **Vanguard** (front, exposed) and **Rearguard** (back, shielded). A back is
  shielded by its own front and is reached only when that front is broken.
- **Reach:** **melee** strikes the front (or charges/flanks); **ranged** fires from any lane.
  Reach influences where it's wise to stand but doesn't dictate it.
- **Per-unit lock (set by the Fray):** a Vanguard is **LOCKED** while an enemy Vanguard *it
  attacked* is still alive; **FREE** if every Vanguard it struck is dead, or it attacked none.
  **Only attacking locks** — being struck, blocking, or evading a ranged shot never locks you.
  A **FREE** Vanguard may, in the Volley, **charge** the enemy Rearguard or **flank** a surviving
  enemy Vanguard. A partial front-break leaks the freed killers through while the rest stay pinned.

---

## 5. The six phases

A round runs this fixed sequence. Targets are chosen only in the **Standoff, Fray, and Volley**;
the **Breach and Reckoning are resolution-only** (they cash out earlier choices).

| #   | Phase         | What happens                                                                                                                                                                                                 |
| --- | ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | **Standoff**  | Blind bid revealed; positions lock; **Standing** effects (buffs / braces) auto-land (no contest).                                                                                                            |
| 2   | **Fray**      | Front clash — melee **and** `on-cast` ranged **and** defenses resolve **simultaneously**. Deaths here (melee *or* ranged) **fix the breach list** (who is freed).                                            |
| 3   | **Volley**    | Free Vanguards **charge** the enemy rear (or **flank**); the rear answers **first** — counter-fire / strike-back / dodge resolve **before** the charge blow (**pre-empt**). On-cast ranged may re-fire here. |
| 4   | **Breach**    | Surviving chargers' blows land (`resolve: breach`). A breacher can **kill a held caster before the Reckoning** → **disrupt**.                                                                                |
| 5   | **Reckoning** | `resolve: reckoning` attacks (held area / DoT) resolve **last**. A caster killed earlier never casts — its held attack **fizzles**.                                                                          |
| 6   | **Lull**      | **Refresh:** Tempo resets, temporary Tempo expires, **Health persists**, round++. (Piles already wiped at each phase boundary.)                                                                              |

**The governing PRINCIPLE (§4.6).** *Within* a phase, resolution is **order-independent** — every
committed strike and defense lands together, **including the blows of a body that dies that phase**
(§1.3). The **only** reason to split into phases is **timing between them:** a unit **dead at a
phase boundary takes no further action**, so a death can **preclude** a later phase but never reach
back. **Pre-empt, disrupt, and flank-intercept are all this one rule.** To decide a new timing
question: *same phase if two effects should trade (both land); ordered phases if one death should
silence the other.*

---

## 6. The cast / resolve timing model

An ability has two timing fields:

- **`cast`** — *when you may invoke it:* `standing` (the Standoff) or `strike` (the **Strike
  window = the Fray or the Volley**).
- **`resolve`** — *when its effect lands.* A card carries one **intrinsic** value: `on-cast`
  (immediately, in the phase cast) or `reckoning` (**held** to resolve last). The third value,
  **`breach`, is *derived*, not authored on the card.** It is the timing a **melee** attack takes
  when used as a **charge** — a *free* Vanguard, targeting the enemy **rear**, in the Volley; the
  gap-crossing defers the blow to the Breach so the rear can **pre-empt** the crosser. So
  `resolve: breach` is a property of **melee charging the rear** (reach + breach-state — *targeting
  is derived, not enumerated*), shared by **every** melee ability, never specific to one card. The
  same melee attack used as a **flank** (a surviving enemy *front*, no gap) stays `on-cast` — a
  trade that lands in the Volley.

Old terms → new: **instant** = `resolve: on-cast`; **slow / deferred** = `resolve: reckoning`;
**a charge** = `resolve: breach`. Because a `strike`/`on-cast` ability isn't phase-locked, the
**same card can fire in both the Fray and the Volley** (Tempo permitting) — "fires in any Strike
phase" just falls out.

**The ability schema:** `(reach · cast · resolve · target · cost · effect)`. A **power and an
attack are the same object** — a power is just an attack with richer trait values (area, a
deferred resolve, a buff or lingering effect instead of plain Might).

---

## 7. Forms, abilities, and resources

- **Abilities live on the Form** (your permanent build) as **passive enablers**: having one means
  you *may* use it, **repeatably**, gated by **one cost only — Tempo**. Abilities are **open
  information**, **never drawn** (no RNG over your own kit), and **never Spend / exhaust**.
- A **one-shot** limits itself by **flipping face down for the whole combat** (never resets).
- **Tempo is the only action currency**; the **blind bid is the only hidden commit** ("Form open,
  bid hidden").
- **Utility cards** are **bookkeeping tokens** for *persistent* effects (a mark, poison stacks) —
  not a resource and not a draw; their count is Form-derived. (None of the bare four needs one.)

**The four bare abilities used in the logs:**

```
Punch        reach: melee  · cast: strike   · resolve: on-cast · single · 1 Tempo · Might
Throw Rock   reach: ranged · cast: strike   · resolve: on-cast · single · 1 Tempo · Might
Throw Bomb   reach: ranged · cast: strike   · resolve: reckoning · AREA · 2 Tempo · Might to ALL in the
             target group — a HELD wind-up; releases at the Reckoning but DROPPED if the thrower dies first
Rallying Cry                 cast: standing · resolve: on-cast · party · 0 Tempo, ONE-SHOT (flips face
             down for the whole combat) · gives each ally +1 temporary Tempo this round
```

*`breach` is not a card field: **any** melee attack — Punch included — resolves at the **Breach**
instead of on-cast when used as a **charge** on the enemy rear (§6). That's a function of
melee + targeting the rear, not of the card.*

---

## 8. Groups and Hoards

- **Group** `{A = B}` — bodies joined into one formation entity, sharing a declared spill order.
- **Sum-to-block** (melee AVOID): the group **pools** Tempo — the **sum** of members' bids must
  beat the attacker. One Tempo per participating member. *(A strong wall.)*
- **Weakest-link-to-evade** (ranged AVOID): **every** member must beat the bid **alone**; the
  **weakest member gates** the whole group's evade. *(A poor dodger.)*
- **Spillover:** single-target damage hits the **declared-first** member; overflow **past a kill**
  carries to the next in order.
- **AREA (AoE):** touches **every** member of the target group **simultaneously** — *not*
  spillover.
- **Hoard ⟨n⟩:** a built-in group of **n one-Health bodies** (each its own Tempo); behaves as a
  group, dying one body per landed hit.

---

## 9. Key interactions (all corollaries of the §4.6 PRINCIPLE)

- **Pre-empt** — the Volley resolves before the Breach, so the rear's answer (counter-fire /
  strike-back / dodge) can **kill a charger before its blow lands.**
- **Disrupt** — a breacher's Breach blow resolves before the Reckoning, so **killing a held
  caster fizzles its attack.** (Non-lethal stagger/silence effects, a later layer, do the same
  without a kill.)
- **Flank intercepts** — a flank is a **trade** (adjacent melee, both land), but resolving **in
  the Volley** means a flank-**kill** silences a flanked charger's own Breach charge.
- **§1.3 (within a phase)** — a body reduced to 0 Health is removed only **at the phase boundary**;
  it still **lands every blow it committed** that phase.
- **Force-not-fiat** — you reach a back by **winning**, never by decree; **every position dies to
  enough Might/Tempo**. No hard immunity; debuffs floor at **min 1**.

---

## 10. Lifetimes — what crosses what

| Thing                            | Lives until                                                           |
| -------------------------------- | --------------------------------------------------------------------- |
| **Health** (flipped cards)       | **Persists** across phases *and* rounds (the one maintained meter)    |
| **Tempo**                        | **Refreshes each round**; shared across all six phases within a round |
| **Accumulator pile**             | **Wipes at each phase boundary** (per-phase; only Health crosses)     |
| **Temporary Tempo** (from buffs) | **Expires at the Lull** (never refreshes)                             |
| **A one-shot ability**           | **Down for the whole combat** once used                               |
| **The battle**                   | Up to **5 rounds**, or until a side is dead                           |

---

## 11. Board notation

Logs are a **state machine**: a layout, the actions (with targets), then the new layout.

| Symbol                                | Meaning                                                                                                |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| `M# V# T# C# F#`                      | the five stats on a stat line, e.g. `M3 V4 T3 C4 F3`                                                   |
| `reach (role)`                        | `melee` / `ranged` (a body may carry both = **multi-reach**); a role/flavor note                       |
| `h[..]` / `h[X.]`                     | **Health** pool — `.` fresh card, `X` flipped (lost) card                                              |
| `t[..]` / `t[X.]`                     | **Tempo** pool — `.` fresh, `X` spent (refreshes each round)                                           |
| `P n/T`                               | **accumulator pile** — `n` banked toward Toughness `T` (per-phase)                                     |
| `+•` / `+×`                           | a **temporary** Tempo card — unspent / spent (expires at the Lull)                                     |
| `{A = B}`                             | a joined **group** (declared spill order, first-named first)                                           |
| `Mob⟨n⟩`                              | a **Hoard** of `n` one-Health bodies                                                                   |
| `✗`                                   | down / dead                                                                                            |
| `·LABEL`                              | a state tag — `·FREE`, `·LOCKED`, `·charging`, `·braced`, `·reckoning AoE` …                           |
| `Held` (queue line)                   | committed `resolve: reckoning` attacks awaiting the Reckoning                                          |
| `1×F3 = 3`                            | a bid: `cards × Finesse`                                                                               |
| `Might 3 → pile 3 ≥ T2 → FLIP`        | damage resolution: Might into the pile, crossing Toughness, flipping a card                            |
| `AVOID` / `STRIKE-BACK` / `EAT`       | the defender's chosen response                                                                         |
| `[Vanguard] / [Rearguard]`, side rows | the **2-D table** — laid out **per side** in two labelled rows; **1-D decks** are the hidden blind bid |

---

*Conventions across the four logs vary slightly (`[ ]`/`[X]` vs `.`/`X`; whether the `P n/T`
pile is printed or kept inline as "pile 3 ≥ T2"). Each log restates its own legend up top; this
file is the union.*
