# Card-system combat log — the breach round (§4.6 six phases, state-machine view)

A worked round that exercises the **§4.6 six phases** — **Standoff → Fray → Volley →
Breach → Reckoning → Lull** — with the **per-unit lock**, the **pre-empt** (the rear
fires *first* at a charger), and **instant cards firing in both the Fray and the
Volley**. Shown as a state machine: **before each phase, the complete physical card
layout** (1-D in a deck, 2-D on the table); the **actions taken, with targets**; then
the **new layout**. Companion to `card-combat-round-4v4.md` (the front-attrition round).
Cards only.

**Reading the table.** Each actor is an identity card in its position; under it sit the
tucked **Vitality/Cadence** stat cards; beside it, the **visible state pools** —
`health` (Vitality cards, flip at Toughness) and `tempo` (Cadence cards, worth
Finesse). In pools: `.` = fresh, `X` = flipped (lost Health / spent Tempo). A
**Deferred** line is a 1-D queue of committed *slow* spells awaiting the Reckoning.

**The six phases.** **Standoff** (reveal + Standing buffs) · **Fray** (front clash —
melee *and* instant ranged, simultaneous; deaths fix the breach list) · **Volley** (free
Vanguards charge; the rear answers **first** — pre-empt) · **Breach** (survivors land
their blows; disrupt the slow caster) · **Reckoning** (deferred spells resolve **last**)
· **Lull** (refresh; Health persists).

---

## Roster (stats shown once; thereafter only the pools change)

```
SIDE A                                    attack-type
  Bram     M2 V6 T4 C2 F3   melee  (tank)
  Torvald  M5 V4 T3 C2 F2   melee  (bruiser)
  Garrick  M4 V4 T3 C4 F4   melee  (the breacher — high Cadence)
  Corvin   M4 V3 T2 C4 F5   ranged · instant  (archer)

SIDE B
  Vesper   M3 V2 T3 C2 F3   melee  (fragile front)
  Sable    M4 V4 T4 C3 F4   melee  (sturdy front)
  Wren     M3 V3 T3 C3 F4   ranged · instant  (archer)
  Robin    M2 V2 T2 C3 F4   ranged · slow  (deferred area-effect caster)
```

---

## Phase 0 — Blind bid (1-D decks, hidden)

Each side stacks its identity deck with position + Join cards, and queues any slow
spell face-down. Hidden until the Standoff.

```
SIDE A deck            SIDE B deck
  [Vanguard]             [Vanguard]
  Bram                   Vesper
  Torvald                Sable
  Garrick                [Rearguard]
  [Rearguard]            Wren
  Corvin                 Robin  (+ slow AoE spell card, face-down)
```

## Phase 1 — The Standoff (reveal; Standing effects land)

Decks reveal into the 2-D table; positions lock. No Standing buffs are bid this round,
so nothing auto-lands — the lines simply face off.

```
[Side A]  Vanguard   Bram   h[......] t[..]    Torvald h[....] t[..]    Garrick h[....] t[....]
          Rearguard  Corvin h[...]   t[....]
[Side B]  Vanguard   Vesper h[..]   t[..]      Sable   h[....] t[...]
          Rearguard  Wren   h[...]  t[...]      Robin  h[..]   t[...]  ·slow caster

Deferred  (empty)
```

A plans to **collapse the gap**: gang Vesper (the fragile front) to *kill her in the
Fray*, which frees whoever struck her to charge in the Volley. B leans on its cannons —
Wren's quick arrows (instant, can fire every phase) and Robin's slow area spell.

---

## Phase 2 — The Fray (front clash: melee + instant ranged, simultaneous)

**Before:** the Standoff table above (all pools fresh).

**Actions (melee and instant ranged all resolve *together*, §1.9):**
```
  Garrick → Vesper   strike ×2 (Might 4 each)   Vesper EATS both → flip, flip → VESPER DOWN
  Torvald → Sable    strike ×2 (Might 5 each)   Sable AVOIDS one (block 1×F4=4 > bid 2), EATS one → flip
  Bram    → Sable    strike (Might 2)           Sable EATS → pile 2 < T4 → no flip (shrugged)
  Sable   → Torvald  strike (Might 4)           Torvald EATS → pile 4 ≥ T3 → flip
  Corvin  → Sable    INSTANT arrow (Might 4)    Sable EATS → 4 ≥ T4 → flip
  Wren    → Garrick  INSTANT arrow (Might 3)    Garrick EATS → 3 ≥ T3 → flip   (front fire on a Vanguard)
  Robin   —          holds (saving Tempo to cast its slow AoE in the Volley)
```

**Breach list fixes now** (on Fray deaths — melee *or* instant ranged): Garrick struck
**Vesper, who died → Garrick is FREE.** Bram & Torvald struck **Sable, who lives →
LOCKED** to her.

> **The lock rule (§4.6, settled).** *Only attacking a body in your way locks you* — to a
> Vanguard **you struck** that is **still alive**. Defending never locks: being struck,
> blocking, and **evading a ranged shot** all leave you free. And if **every** Vanguard
> you struck is dead, you are free. So Sable striking Torvald doesn't lock *Torvald*;
> *his* lock comes from *his* strike on the living Sable. Garrick's only target died → free.

**After:**
```
[Side A]  Vanguard   Bram   h[......] t[X.]    Torvald h[X...] t[XX]    Garrick h[X...] t[XX..]
          Rearguard  Corvin h[...]   t[X...]
[Side B]  Vanguard   (Vesper DOWN — cards removed)   Sable h[XX..] t[XX.]
          Rearguard  Wren   h[...]  t[X..]      Robin  h[..]   t[...]  ·slow caster

Deferred  (empty)
Breach    FREE: Garrick (Vesper dead)     LOCKED: Bram, Torvald (→ Sable, alive)
```

---

## Phase 3 — The Volley (free Vanguards charge; the rear answers *first* — pre-empt)

**At the start of the Volley, B commits its slow spell:** Robin casts its **AoE at A's
Vanguard line**, paying **2 Tempo** up front — the card goes face-up to the **Deferred**
queue and will only resolve in the Reckoning. **A declares its charge:** free Garrick
charges **Robin**.

**Before:**
```
[Side A]  Vanguard   Bram   h[......] t[X.]    Torvald h[X...] t[XX]    Garrick h[X...] t[XX..]
          Rearguard  Corvin h[...]   t[X...]
[Side B]  Vanguard   Sable  h[XX..]  t[XX.]
          Rearguard  Wren   h[...]   t[X..]     Robin   h[..]  t[XXX]  ·cast, 0 Tempo left

Deferred  Robin → A-Vanguard (slow AoE)
Charge    Garrick → Robin
```

**Actions — the rear answers BEFORE the charger's blow (pre-empt):**
```
  Wren  → Garrick   INSTANT arrow #2 (Might 3)   Garrick EATS → flip   (same card that fired in the Fray —
                                                                       instant fires in BOTH phases)
  Robin → (dodge)   spends its last Tempo: 1×F4 = 4  vs  Garrick's charge bid 1×F4 = 4 → TIE → dodge FAILS
```

**Pre-empt outcome:** Wren's counter-arrow puts Garrick at **2 of 4** Health — *bloodied
but not stopped.* Robin's dodge only tied, so the charge will land. **Had Garrick
entered the Volley one hit weaker (h[XXX.]), Wren's arrow would have dropped him here —
no Breach, and Robin's AoE survives to the Reckoning.** That knife-edge *is* the pre-empt.

**After:**
```
[Side A]  Vanguard   Bram   h[......] t[X.]    Torvald h[X...] t[XX]    Garrick h[XX..] t[XX..]
          Rearguard  Corvin h[...]   t[X...]
[Side B]  Vanguard   Sable  h[XX..]  t[XX.]
          Rearguard  Wren   h[...]   t[XX.]     Robin   h[..]  t[XXX]

Deferred  Robin → A-Vanguard (slow AoE)
Charge    Garrick → Robin  (survived the Volley → strikes in the Breach)
```

---

## Phase 4 — The Breach (survivors land their blows)

**Before:** the Volley "after" — Garrick weathered the pre-empt; Robin's dodge failed and
it is out of Tempo.

**Actions:**
```
  Garrick → Robin   strike (Might 4)   dodge already failed → LANDS → 4 ≥ T2 (twice) → FLIP ×2 → ROBIN DOWN
```

Robin dies **in the Breach — before the Reckoning.** Note Wren (no melee, out of useful
Tempo) cannot shield her further.

**After:**
```
[Side A]  Vanguard   Bram   h[......] t[X.]    Torvald h[X...] t[XX]    Garrick h[XX..] t[XXX.]
          Rearguard  Corvin h[...]   t[X...]
[Side B]  Vanguard   Sable  h[XX..]  t[XX.]
          Rearguard  Wren   h[...]   t[XX.]     (Robin DOWN — cards removed)

Deferred  Robin → A-Vanguard (slow AoE)   ⚠ caster dead
```

---

## Phase 5 — The Reckoning (deferred spells resolve last)

**Before:** the Breach "after" — the Deferred queue still holds Robin's AoE, but its
caster is gone.

**Actions:**
```
  Robin's slow AoE → FIZZLES — the caster died in the Breach (which resolves before the
                     Reckoning, §4.6 order), so the area spell never goes off. A's line is untouched.
```

**After:** the AoE card is discarded unspent.

```
[Side A]  Vanguard   Bram   h[......]    Torvald h[X...]    Garrick h[XX..]
          Rearguard  Corvin h[...]
[Side B]  Vanguard   Sable  h[XX..]
          Rearguard  Wren   h[...]       (Robin down)

Deferred  (resolved → fizzled, empty)
```

---

## Phase 6 — The Lull (refresh)

**Actions:** every spent `tempo` card flips back up (Tempo refills); **Health stays
flipped** (persists); round++.

**After (round-2 opening state — Tempo fresh, Health carried):**
```
[Side A]  Vanguard   Bram   h[......] t[..]    Torvald h[X...] t[..]    Garrick h[XX..] t[....]
          Rearguard  Corvin h[...]   t[....]
[Side B]  Vanguard   Sable  h[XX..]  t[...]
          Rearguard  Wren   h[...]   t[...]

Deferred  (empty)
```

**Round result:** A spent the round **collapsing one side of B's front** — Garrick broke
through the gap Vesper left, **weathered the rear's pre-emptive Volley** (a single arrow
shy of being stopped), and **killed Robin before the Reckoning**, fizzling her AoE. B is
down its **fragile front and its area caster**, and enters round 2 with a lone Vanguard
(Sable, −2) shielding a lone archer (Wren). A is whole but for chip. The glass-cannon's
slow gun never fired.

---

## What this exercised

With the physical layout at every phase:
- **Per-unit lock (the Fray fixes it)** — Garrick came **free** by *killing his own
  front-foe*; Bram/Torvald stayed **locked** to a living Sable. Deaths by melee *or*
  instant ranged both count toward the breach list.
- **Instant fires in both the Fray and the Volley** — Wren's one archer card loosed at a
  front Vanguard in the Fray, then again at the charging breacher in the Volley.
- **The pre-empt (Volley before Breach)** — the rear answered *first*; the counter-arrow
  bloodied Garrick and *almost* stopped the charge. A weaker breacher dies here.
- **Disrupt by kill (Breach before Reckoning)** — Garrick's Breach blow resolved before
  the deferred AoE, so killing the caster fizzled it. The other disrupt flavor — a
  **non-lethal** stagger/silence that cancels the cast without a kill — would slot into
  the Volley or Breach the same way.

Still open (flag to pin):
1. **When a deferred spell is *committed*** — modeled here as Tempo paid **up front at the
   start of the Volley** (so the charge can threaten it). Confirm that's the commit
   moment (vs. committed back in the Fray).
2. **Flank** — a free Vanguard could instead strike a surviving enemy Vanguard; resolves
   in the Volley/Breach like any charge. Not shown here; expected rare.
