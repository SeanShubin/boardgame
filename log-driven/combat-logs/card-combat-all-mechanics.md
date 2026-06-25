# Card-system combat log — the grand tour (every bare mechanic, action by action)

A single battle that exercises **every core combat mechanic** on the §4.6 six-phase model.
Unlike the other logs, this one resolves **one action at a time** and prints the **full
board after each action** — because although a phase is **order-independent** (§1.9: the
end-state is the same whatever order a human picks), a human *does* move one card at a
time, and the intermediate states are where mistakes hide. **Accumulator pools are shown
explicitly.**

**Out of scope (the "ought it exist / gear" layer, by design):** armor / damage-types,
role-power cards, and the `persist` / `cleave` keywords — though the pile legend notes how
those two would bend the accumulator.

**Legend.**
- `h[..]` — Health pool (Vitality cards); a card flips when the pile meets Toughness.
- **`P n/T`** — the **accumulator pile**: `n` damage banked toward Toughness `T`. A landed
  hit adds **Might** to `n`. When `n ≥ T`, **flip one Health card** and the **overflow is
  wasted** (`P` resets to 0). The pile **carries across actions within a round** and is
  **wiped at the Lull** (the per-round reset). *(With `cleave` the overflow would cascade to
  the next card instead of being wasted; with `persist` the pile would survive the Lull.)*
- `t[X.]` — Tempo pool (Cadence cards, each worth Finesse). A bid is `cards × Finesse`; the
  **defender must strictly beat it** — **a tie lands the hit**.
- `{A = B}` joined group · `Mob⟨n⟩` a **Hoard** of `n` one-Health bodies · `✗` down.
- Three defender responses (§3.4): **AVOID** (block/dodge/slip/evade) · **STRIKE-BACK**
  (counter — a mutual trade) · **EAT** (take it, spend nothing).

---

## Roster (stats shown once)

```
SIDE A — Wardens                                         reach
  Brand  M3 V4 T3 C4 F3   melee + ranged  (MULTI-REACH; the breacher carries both cards)
  Corin  M3 V3 T2 C3 F4   ranged          (instant)
  Dru    M2 V4 T3 C4 F3   melee           (low Might → builds piles; ends up the breacher)
  Mob    Hoard 3 (a,b,c)  each M1 V1 T1 C1 F2   melee   (three one-Health bodies)

SIDE B — Holdfast
  Vesk   M2 V1 T3 C2 F2   melee           (solo Vanguard, fragile)
  Gale   M3 V3 T2 C2 F3   melee  } joined group {Gale = Hob}, Vanguard
  Hob    M2 V2 T2 C2 F2   melee  }   (declared spill order: Gale first)
  Orin   M2 V2 T2 C4 F3   ranged          (Rearguard caster: instant fire OR one deferred AoE)
```

Formation — A Vanguard `{Brand, Dru, Mob}`, Rearguard `{Corin}`; B Vanguard `{Vesk,
Gale=Hob}`, Rearguard `{Orin}`.

---

## The Standoff (reveal; a Standing brace lands)

Decks reveal; positions lock. **Standing effect:** Gale pre-commits one Tempo in the bid as
a **brace** (a held block, worth its Finesse) — it auto-lands and will feed the group's first
sum-block. Opening board (piles all empty, Gale braced):

```
A:  Brand h[....] P0/3 t[....]   Corin h[...] P0/2 t[...]   Dru h[....] P0/3 t[....]   Mob⟨3⟩ a·b·c
B:  Vesk h[.] P0/3 t[..]    {Gale h[...] P0/2 t[X.]·braced = Hob h[..] P0/2 t[..]}    Orin h[..] P0/2 t[....]
```

---

## The Fray (front clash — resolved one action at a time)

All Fray actions are committed at once and are order-independent; here a human walks them in
a chosen order. Deaths during the Fray **fix the breach list** at the end.

**F1 · Dru → Vesk** (melee, bid 1×F3=3, Might 2). Vesk **EATS**. Pile climbs but does **not**
flip:
```
A:  Brand h[....] P0/3 t[....]   Corin h[...] P0/2 t[...]   Dru h[....] P0/3 t[X...]   Mob⟨3⟩
B:  Vesk h[.] P2/3 t[..]    {Gale h[...] P0/2 t[X.]·braced = Hob h[..] P0/2 t[..]}    Orin h[..] P0/2 t[....]
```
↳ *EAT; the accumulator at `P2/3` — banked, not yet a wound.*

**F2 · Dru → Vesk** (melee, bid 3, Might 2). Vesk **EATS**. Pile `2 + 2 = 4 ≥ T3` → **FLIP**;
Vesk is V1 → **DOWN**; overflow `1` **wasted**:
```
A:  Brand h[....] P0/3 t[....]   Corin h[...] P0/2 t[...]   Dru h[....] P0/3 t[XX..]·FREE   Mob⟨3⟩
B:  Vesk ✗    {Gale h[...] P0/2 t[X.]·braced = Hob h[..] P0/2 t[..]}    Orin h[..] P0/2 t[....]
```
↳ *Pile crosses Toughness → one flip + **wasted overflow**. Dru struck Vesk, now dead → **Dru is FREE** (per-unit lock).*

**F3 · Brand → {Gale=Hob}** (melee, bid 1×F3=3, Might 3). Group **AVOIDS by SUM-BLOCK** —
pooling Tempo: Gale's **brace** (F3) + Hob one card (F2) = `5 > 3` → **blocked**, no damage,
one Tempo per member:
```
A:  Brand h[....] P0/3 t[X...]·LOCKED   Corin h[...] P0/2 t[...]   Dru …·FREE   Mob⟨3⟩
B:  {Gale h[...] P0/2 t[X.] = Hob h[..] P0/2 t[X.]}    Orin h[..] P0/2 t[....]
```
↳ *Sum-to-block (a strong wall); the Standing brace fed it. Brand struck a living Vanguard → **Brand LOCKED**.*

**F4 · Corin → {Gale=Hob}** (ranged, bid 1×F4=4, Might 3). To dodge ranged a group needs
**weakest-link evade** — *every* member must beat 4 **alone**; Hob's max is `1×F2=2`, so the
group **can't** evade and **EATS**. Damage **spills to Gale** (declared first): `P0+3=3 ≥ T2`
→ **FLIP** (Gale V3→2), waste 1:
```
A:  Brand …·LOCKED   Corin h[...] P0/2 t[X..]·LOCKED   Dru …·FREE   Mob⟨3⟩
B:  {Gale h[..] P0/2 t[X.] = Hob h[..] P0/2 t[X.]}    Orin h[..] P0/2 t[....]
```
↳ *Weakest-link: the **weakest member gates the whole group's evade** — a great shield, a poor dodger. Spillover + flip + waste.*

**F5 · Mob (Hoard) gangs Hob** — three one-Health bodies, one Tempo each:

**F5a · Rat-a → Hob** (Might 1, bid 1×F2=2). Hob **STRIKE-BACKS** (counter, 1 Tempo) — a
mutual trade: Hob takes Rat-a's M1 (`P0+1=1/2`, no flip) **and** deals M2 → Rat-a (T1) flips
→ **DOWN**:
```
A:  Brand …·LOCKED   Corin …·LOCKED   Dru …·FREE   Mob⟨2⟩ b·c  (a ✗)
B:  {Gale h[..] P0/2 t[X.] = Hob h[..] P1/2 t[XX]}    Orin h[..] P0/2 t[....]
```
↳ *STRIKE-BACK (both land). Rat-a dies **yet still lands its own blow** — §1.3, the dying body delivers its committed strike **within** the phase.*

**F5b · Rat-b → Hob** (Might 1, bid 2). Hob is out of Tempo → **EATS**. Pile `1+1=2 ≥ T2` →
**FLIP** (Hob V2→1):
```
A:  Brand …·LOCKED   Corin …·LOCKED   Dru …·FREE   Mob⟨2⟩ (b spent) c
B:  {Gale h[..] P0/2 t[X.] = Hob h[.] P0/2 t[XX]}    Orin h[..] P0/2 t[....]
```
↳ *Two pin-pricks accumulate into one wound — the pile carrying across actions is the whole point.*

**F5c · Rat-c → Hob** (Might 1, bid 2). Hob **EATS** → `P0+1=1/2`, no flip:
```
A:  Brand …·LOCKED   Corin …·LOCKED   Dru …·FREE   Mob⟨2⟩ (b,c spent)·LOCKED
B:  {Gale h[..] P0/2 t[X.] = Hob h[.] P1/2 t[XX]}    Orin h[..] P0/2 t[....]
```
↳ *Mob struck the living group → **Mob LOCKED**. A new pile begins under Toughness.*

**Breach list fixes:** **FREE — Dru** (its only target, Vesk, is dead). **LOCKED — Brand,
Corin, Mob** (all struck the still-living group). A **partial** break: one body slips free
while the rest stay pinned.

---

## The Volley (free Vanguards charge; the rear answers first — pre-empt)

**B commits its deferred spell:** Orin casts a **deferred AoE** at A's Vanguard line, paying
2 Tempo up front — it sits in the queue and resolves only at the Reckoning. **A declares:**
**Dru (free) charges Orin**; the locked melee can't reach the back, but **Brand fires its
ranged card** (multi-reach) and Corin fires again (instant, second phase) at the group.

```
Deferred:  Orin → A-Vanguard (AoE)        Charge:  Dru → Orin
```

**V1 · Brand → {Gale=Hob}** (now **RANGED** — same body that melee'd in F3; **multi-reach**,
bid 1×F3=3, Might 3). Group **EATS** → spill to Gale: `P0+3=3 ≥ T2` → **FLIP** (Gale V2→1):
```
A:  Brand h[....] P0/3 t[XX..]   Corin …   Dru …·FREE   Mob⟨2⟩
B:  {Gale h[.] P0/2 t[X.] = Hob h[.] P1/2 t[XX]}    Orin h[..] P0/2 t[XX..]·(cast)
```
↳ ***MULTI-REACH:** one body, melee in the Fray, ranged in the Volley.*

**V2 · Corin → {Gale=Hob}** (ranged again — **instant fires in both the Fray and the
Volley**, bid 1×F4=4, Might 3). Group EATS → spill to Gale: `P0+3=3 ≥ T2` → **FLIP** → Gale
**DOWN**:
```
A:  Brand …   Corin h[...] P0/2 t[XX.]   Dru …·FREE   Mob⟨2⟩
B:  {Gale ✗ … Hob h[.] P1/2 t[XX]}    Orin h[..] P0/2 t[XX..]·(cast)
```
↳ *Instant-in-both-phases. The group is down to Hob, but Hob lives → A's locked units stay locked.*

**V3 · Dru charges Orin — the rear answers FIRST (pre-empt).** Orin still has 2 Tempo after
casting, so it instant-fires the charger: **Orin → Dru** (Might 2, bid 1×F3=3). Dru **EATS**
→ `P0+2=2/3` (no flip — bloodied, not stopped):
```
A:  Brand …   Corin …   Dru h[....] P2/3 t[XX..]·charging   Mob⟨2⟩
B:  {Hob h[.] P1/2 t[XX]}    Orin h[..] P0/2 t[XXX.]·(1 Tempo left)
```
↳ ***PRE-EMPT:** the Volley resolves before the Breach, so the rear shoots first. Orin both **cast and defended** — only its high Cadence allowed both.*

---

## The Breach (the charge lands — survivors strike)

**B1 · Dru → Orin** (the charge blow, Might 2, bid 1×F3=3). Orin **DODGES** with its last
card: `1×F3=3` vs `3` → **TIE → the hit lands**. `P0+2=2 ≥ T2` → **FLIP** (Orin V2→1):
```
A:  Dru h[....] P2/3 t[XXX.]   …
B:  {Hob h[.] P1/2 t[XX]}    Orin h[.] P0/2 t[XXXX]
```
↳ ***TIE LANDS:** the defender must *strictly* beat the bid; an exact match is not enough.*

**B2 · Dru → Orin** (Might 2, bid 3). Orin is dry → **EATS** → `P0+2=2 ≥ T2` → **FLIP** →
Orin **DOWN**:
```
A:  Dru h[....] P2/3 t[XXXX]   …
B:  {Hob h[.] P1/2 t[XX]}    Orin ✗
```
↳ ***DISRUPT by kill:** the caster dies in the Breach, **before** the Reckoning.*

---

## The Reckoning (deferred spells resolve last)

**Orin's deferred AoE → FIZZLES.** Its caster died in the Breach, which resolves *before* the
Reckoning (§4.6 order), so the area spell never goes off — A's Vanguard is untouched. The
deferred card is discarded unspent.

---

## The Lull (refresh — Health persists, piles wipe)

Every spent Tempo card flips back up. **Health stays flipped.** Every **un-flipped pile is
wiped** (the per-round reset): Dru's `P2/3` and Hob's `P1/2` both clear. Round-2 opening
board:

```
A:  Brand h[....] P0/3 t[....]   Corin h[...] P0/2 t[...]   Dru h[....] P0/3 t[....]   Mob⟨2⟩ t·fresh
B:  {Hob h[.] P0/2 t[..]}    (Vesk, Gale, Orin ✗)
```
↳ *Tempo full again; **wounds carried** (Hob still `h[.]`); piles back to zero. The battle runs up to **5 rounds or a dead side** — B is a lone wounded body behind no shield, so it is effectively lost.*

---

## Inset — flanking that *intercepts* (a 2-front mutual breakthrough)

The main battle never produced two simultaneous charges, so here is the smallest board that
shows **flanking interception** (§4.6): each side breaks one of the other's front, and a freed
unit **cuts down the enemy charger before it lands** instead of charging itself.

```
A:  {Ander M3 V2 T2 C3 F3 melee, Esk M2 V1 T2 C2 F2 melee} Vanguard   ·   Bryn (rear)
B:  {Cull M3 V1 T2 C2 F2 melee, Fross M3 V1 T2 C2 F2 melee} Vanguard   ·   Dane (rear)
```

**Fray** — mutual front losses:
```
  Ander → Cull   M3 ≥ T2 → FLIP → Cull (V1) ✗      → Ander FREE
  Fross → Esk    M3 ≥ T2 → FLIP → Esk  (V1) ✗      → Fross FREE
```

**Volley** — Fross (B, free) **charges** A's rear Bryn; Ander (A, free) does **not** charge —
it **flanks Fross**:
```
  Charge:  Fross → Bryn   (declared; its blow would land in the Breach)
  Ander → Fross (FLANK)   M3 ≥ T2 → FLIP → Fross (V1) ✗   ← killed IN THE VOLLEY
```
↳ *A flank is a **trade** (adjacent melee, both would land), but it resolves **in the Volley — before the Breach.** Fross is dead at the Volley boundary, so its charge on Bryn is **precluded** (§4.6: a death in one phase silences anything it had pending in a later one).*

**Breach** — Fross is gone → **no blow on Bryn. The interception saved the back.**

---

## Coverage — every mechanic, and where it shows

| Mechanic | Where |
|----------|-------|
| Health / Tempo pools, bid = cards×Finesse | throughout |
| **Accumulator pile** (`P n/T`), sub-threshold banking | F1, F4, F5c, V3 |
| Flip at Toughness + **wasted overflow** | F2, F4, F5b |
| Pile **carries within a round, wipes at the Lull** | F5b→Lull |
| **Strictly-beat / TIE LANDS** | B1 (dodge ties → lands) |
| Three responses: **EAT / AVOID(block) / STRIKE-BACK** | F1 / F3 / F5a |
| Positions & reach (Vanguard/Rearguard, melee/ranged) | formation |
| **Six phases** Standoff→Fray→Volley→Breach→Reckoning→Lull | section order |
| **Standing** effect (brace, auto-lands) | Standoff → used F3 |
| **Per-unit lock**, partial break | F2/F5c breach list |
| **Pre-empt** (Volley before Breach) | V3 |
| **Instant in both** Fray & Volley | Corin F4 + V2 |
| **Multi-reach** (one body, melee then ranged) | Brand F3 + V1 |
| **Deferred** spell + **Reckoning** + **disrupt by kill** | Volley/B2/Reckoning |
| **Groups**: sum-block / weakest-link evade / spillover / one-Tempo-per-member | F3 / F4 / F4,V1 / F3 |
| **Hoard** (n one-Health bodies attacking) | F5a–c |
| **§1.3** dying body still lands its blow (within a phase) | F5a |
| Order-independence (resolved one at a time) | whole Fray |
| **Health persists; Tempo refreshes; 5-round cap** | Lull |
| **Flanking intercepts** (kill in Volley precludes a charge) | Inset |
| Force-not-fiat (every position dies to enough Might/Tempo) | Vesk, Gale, Orin all fall |
