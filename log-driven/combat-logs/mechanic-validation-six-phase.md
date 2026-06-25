# Mechanic validation — pre-empt, groups, flanking (bare six-phase mechanics)

Designer-validation scenarios: each isolates **one mechanic** so you can confirm **what it
IS**, against the §4.6 six-phase model. **Out of scope on purpose:** special-power cards,
armor / damage-types, keywords (persist/cleave), and *tuning* — stats are the minimum
needed to make the mechanic fire, and "should this mechanic exist / is it tuned right" is
a separate step. Physical-card state machine: layout → actions (with targets) → new layout.

**Legend.** `.` fresh · `X` flipped (spent Tempo / lost Health) · `{A=B}` joined group ·
`h[ ]` health pool (flip at Toughness) · `t[ ]` tempo pool (a card is worth Finesse). A
**defender must strictly beat** a bid — a **tie lands the hit**. A landed hit pours **Might**
into the pile. Phases: **Standoff · Fray · Volley · Breach · Reckoning · Lull**.

---

## Scenario 1 — The pre-empt *stops* the charge (+ the cast-vs-defend dilemma)

**Validates:** the Volley resolves **before** the Breach, so the rear's instant fire can
**kill a charger before its blow lands** — the charge is stopped (the mirror of the breach
log, where a healthier charger survived and disrupted the caster). And: a rear caster
**cannot both cast its deferred spell and keep Tempo to defend** — one pool pays for one.

```
SIDE A   Garrick  M3 V2 T2 C3 F3  melee   (breacher)
SIDE B   Vesper   M2 V1 T2 C2 F2  melee   (fragile front)
         Robin    M3 V2 T2 C3 F3  ranged  (rear: instant arrows OR one deferred AoE)
```

**Standoff:**
```
[A]  Vanguard  Garrick h[..] t[...]
[B]  Vanguard  Vesper  h[.]  t[..]
     Rearguard Robin   h[..] t[...]
```

**Fray** (front clash):
```
  Garrick → Vesper   strike (Might 3)   Vesper EATS → 3 ≥ T2 → FLIP → VESPER DOWN (V1)
  Robin   —          holds (deciding: cast, or save Tempo to defend)
```
Breach list: Garrick killed Vesper → **FREE**.
```
[A]  Vanguard  Garrick h[..] t[X..]          (1 Tempo spent, 2 left)
[B]  Vanguard  (Vesper down)
     Rearguard Robin   h[..] t[...]           FREE: Garrick → may charge Robin
```

**Volley — main line (Robin DEFENDS):** Robin does *not* cast. A declares Garrick's charge
on Robin; Garrick means to push through, eating fire to keep Tempo for its Breach strikes.
The rear answers **first**:
```
  Robin → Garrick   instant arrow (Might 3)   Garrick EATS → FLIP   h[X.]
  Robin → Garrick   instant arrow (Might 3)   Garrick EATS → FLIP   h[XX] → GARRICK DOWN (V2)
```
```
[A]  Vanguard  (Garrick down)
[B]  Rearguard Robin h[..] t[XX.]             (2 Tempo spent, 1 left)
```

**Breach:** Garrick is dead at the Volley boundary → **no blow. Charge STOPPED**, Robin
untouched. *Reckoning:* nothing deferred. *Lull:* refresh.

**The fork (same Volley start, Robin CASTS instead):**
```
  Robin casts deferred AoE (2 Tempo) → 1 left
  Pre-empt: Robin → Garrick  1 arrow → 1 FLIP → Garrick h[X.] (survives)
  Breach:   Garrick → Robin  strike ×2 (2 Tempo) → FLIP ×2 → ROBIN DOWN
  Reckoning: Robin's AoE FIZZLES (caster dead)
```
So **defend → Robin lives, no AoE; cast → Robin dies, AoE disrupted.** One Tempo pool, one
choice. *(Whether the AoE is worth casting is a tuning/ought question — out of scope; note
there's no A back-line here for the AoE to even threaten.)*

**To confirm (what IS):** the pre-empt **overrides** "a mortally-wounded body still delivers
its committed blow" (§1.3) **across the Volley→Breach boundary** — a charger killed in the
Volley gets **no** Breach strike. (Within one phase, simultaneous mutual blows still apply;
across the phase boundary, dead is dead.) Is that the rule you intend?

---

## Scenario 2 — Groups: sum-to-block vs weakest-link-to-evade, and spillover

**Validates:** a group **blocks melee by pooling** (sum of members' bids) but **evades ranged
by weakest-link** (every member must beat the bid **alone**); single-target damage **spills**
through the group in declared order. Two **independent** illustrations from the same fresh
group, so each starts with full Tempo.

```
SIDE A   Bram    M3 V2 T2 C2 F2  melee
         Corvin  M3 V2 T2 C2 F4  ranged
SIDE B   {Sable  M2 V2 T2 C2 F3  melee  =  Wren  M2 V2 T2 C2 F2  melee}   (declared order: Sable first)
```
```
[A]  Vanguard  Bram h[..] t[..]    Rearguard Corvin h[..] t[..]
[B]  Vanguard  {Sable h[..] t[..] = Wren h[..] t[..]}
```

**Illustration A — melee, SUM-TO-BLOCK (fresh):**
```
  Bram → {Sable=Wren}   bid 1×F2 = 2
        GROUP BLOCKS by pooling Tempo:  Sable 1×F3=3  +  Wren 1×F2=2  →  sum 5 > 2  → BLOCKED
        costs one Tempo per member →  {Sable t[X.] = Wren t[X.]} ,  no Health lost
```

**Illustration B — ranged, WEAKEST-LINK-EVADE then SPILLOVER (fresh):**
```
  Corvin → {Sable=Wren}   bid 1×F4 = 4
        GROUP tries to EVADE — each must beat 4 ALONE:
            Sable  max 2×F3 = 6 > 4  ✓
            Wren   max 2×F2 = 4 = 4  → TIE → FAILS ✗   ← the weakest link dooms it
        → EVADE FAILS → shot LANDS
  SPILLOVER (declared order, Sable first):  Might 3 → Sable pile 3 ≥ T2 → FLIP → Sable h[X.]
```
```
[B]  Vanguard  {Sable h[X.] = Wren h[..]}      (only Sable took the hit; overflow past a kill would carry to Wren)
```

**To confirm (what IS):**
1. A group is a **strong shield, poor dodger** — pools to block melee, but its **weakest
   member gates a ranged evade** (Wren's F2 fails even maxed). Intended asymmetry?
2. **The Tempo question this surfaces:** on a *doomed* weakest-link evade, do **all** members
   still spend (Sable burns 2 Tempo for nothing), or only the weakest tries? A rule to pin.

---

## Scenario 3 — Flanking: a freed Vanguard hits a surviving front instead of the rear

**Validates:** a **free** Vanguard (its front-foe dead) may, in the Volley, strike a
**surviving enemy Vanguard** (flank) rather than charge a rear — **ganging** it with a
locked teammate.

```
SIDE A   Aldric  M3 V2 T2 C3 F3  melee  (breacher)
         Bram    M3 V2 T2 C2 F2  melee
SIDE B   Vesper  M3 V1 T2 C2 F3  melee  (fragile front)
         Sable   M3 V3 T3 C2 F3  melee  (sturdy front)
```
```
[A]  Vanguard  Aldric h[..] t[...]   Bram h[..] t[..]
[B]  Vanguard  Vesper h[.]  t[..]    Sable h[...] t[..]
```

**Fray:**
```
  Aldric → Vesper   strike (Might 3)   Vesper EATS → 3 ≥ T2 → FLIP → VESPER DOWN (V1)   → Aldric FREE
  Bram   → Sable    strike (Might 3)   Sable EATS → 3 ≥ T3 → FLIP   Sable h[X..]          → Bram LOCKED (Sable alive)
  Sable  → Bram     strike (Might 3)   Bram EATS → 3 ≥ T2 → FLIP    Bram h[X.]
```
```
[A]  Vanguard  Aldric h[..] t[X..]   Bram h[X.] t[X.]
[B]  Vanguard  (Vesper down)   Sable h[X..] t[X.]      FREE: Aldric   LOCKED: Bram → Sable
```

**Volley:** Aldric is free. With no enemy rear to charge, it **flanks** the surviving
Sable — piling onto the foe Bram is locked to:
```
  Aldric → Sable (FLANK)   strike (Might 3)   Sable EATS → 3 ≥ T3 → FLIP   Sable h[XX.]
        (shown as a TRADE — Sable could block / strike back with its last Tempo; here it eats)
```
```
[A]  Vanguard  Aldric h[..] t[XX.]   Bram h[X.] t[X.]
[B]  Vanguard  Sable h[XX.] t[X.]     (ganged across the Fray + the flank: 2 of 3 down)
```

**Confirmed (what IS):** a freed unit is **not forced to the rear** — it may flank a
surviving front. **Resolved:** a flank is a **trade** (adjacent melee — flanker and target
both land, no pre-empt between them), but it resolves **in the Volley**, so a flank that
**kills** its target **intercepts** — if that target was itself charging, its Breach charge
is precluded. Flanking can *gang a survivor* **or** *cut down a breakthrough*. (§4.6 RULE —
flanking intercepts.)

---

## Summary — what each scenario asks you to confirm

| #   | Mechanic     | The thing to confirm IS what you meant                                                                                                                                                      |
| --- | ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | **Pre-empt** | The Volley kills a charger **before** its Breach blow — and overrides §1.3's "dying body still strikes" **across** the phase boundary. The cast-vs-defend dilemma is one shared Tempo pool. |
| 2   | **Groups**   | Sum-to-block (melee) vs **weakest-link**-to-evade (ranged) + spillover. Open: does a doomed evade still cost **every** member Tempo?                                                        |
| 3   | **Flanking** | A freed Vanguard may hit a surviving front instead of the rear. Open: flank = **trade** (Fray-like) or **pre-empt** (charge-like)?                                                          |

**Resolved (2026) by one principle** — now the §4.6 *"why there are phases at all"* PRINCIPLE: separate
phases exist only to impose **death-timing**; *within* a phase, resolution is **order-independent** and a
committed action is **spent before it's resolved**. From that, all three fall out:
1. **Pre-empt** — the Volley and Breach are separate phases, so a charger **dead in the Volley takes no
   Breach action**; §1.3's "dying body still strikes" is a *within-phase* rule, so it does **not** reach
   across the boundary.
2. **Doomed evade** — every member's evade bid is **committed up front** (order-independent), so the Tempo
   is **spent even when the weakest link dooms it** — no take-backs.
3. **Flank** — adjacent melee → a **trade** (both land), but resolved **in the Volley**, so a flank-**kill**
   *intercepts*: it silences a flanked charger's own Breach charge.
