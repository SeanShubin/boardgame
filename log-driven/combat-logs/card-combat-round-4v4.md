# Card-system combat log — a 4v4 round (the front holds)

A worked physical-card walkthrough on the **six-phase round** (Spec §4 / §4.6, 2026 —
supersedes static ranks). This is the **front-holds / attrition** case: overworld pickup →
decks → blind-bid reveal → flippable pools → a full round whose front does **not** break
(**Standoff → Fray → empty Volley / Breach → Lull**), then an illustration of a later round
where a front *does* fall and the **per-unit lock** opens the back. Its focus is **groups**
(sum-block, weakest-link evade, spillover) and the **three defender responses**; the breach
mechanics get their own log (`card-combat-round-breach.md`). Cards only; every number is a card.

Legend: `[ ]` fresh · `[X]` flipped (spent Tempo / lost Health) · `=` joined group ·
*pile* = damage accumulating toward the Toughness bar.

Rules in one breath: **Health = Vitality × Toughness** (persists all 5 rounds),
**Tempo = Cadence × Finesse** (refills each round, **shared across the round's two
phases**). **Standing and soaking are free; only acting spends Tempo.** Every attack
is one Tempo bid (`cards × Finesse`); the defender must **strictly beat** it to **avoid**
the hit — *block, dodge, slip, or evade* by context (a **tie lands the hit**). A landed
hit deals **Might** (Finesse-blind) into the pile.

---

## The battle

**4v4.** `Bram · Torvald · Garrick · Corvin` **vs** `Vesper · Sable · Wren · Robin`.

Each identity has **3 copies** — but they are the *same* character in **different
contexts**, never two of one character in a single battle. The three are functional:
one marks the character's **location** on the overworld map (laid on its location
card), one is committed in the **formation blind bid**, and one tracks **which state
belongs to whom** during combat.

**Overworld.** I drop my four identity cards onto the enemy's map space → combat
triggers; the log starts.

---

## The decks — stats + attack type

`Might / Toughness / Finesse` stay as numbers; **Vitality** and **Cadence** are set
aside and become **counts of flippable cards** at setup. Each Actor also carries an
**attack-type card** — **melee** or **ranged** — that sets its **reach**: melee strikes
from the front, ranged from the back. Reach *influences* where it's wise to stand but
does not dictate it (a melee body parked in the back is just idle until the line breaks).
*(For clarity this example gives each character a single type; some characters carry
both a melee and a ranged card.)*

```
SIDE A                                          SIDE B
  Bram     M2 V6 T4 C2 F3   melee               Vesper   M3 V4 T3 C2 F3   melee
  Torvald  M5 V4 T3 C2 F2   melee               Sable    M4 V3 T3 C3 F4   melee
  Garrick  M3 V4 T3 C3 F4   melee               Wren     M3 V5 T3 C2 F3   ranged
  Corvin   M4 V3 T2 C4 F5   ranged              Robin    M4 V3 T2 C3 F5   ranged
```

---

## Blind bid → reveal (the Standoff)

Each round opens with a **hidden, simultaneous** commit: group your Actors, assign
each group **Vanguard** (front) or **Rearguard** (back), play standing buffs. Melee
self-sorts to the front, ranged to the back. Reveal together; nobody moves.

```
SIDE A bids                  SIDE B bids
  [Vanguard]                   [Vanguard]
  Bram [Join] Torvald          Vesper [Join] Sable
  Garrick                      [Rearguard]
  [Rearguard]                  Wren
  Corvin                       Robin
```

Reveal — each side lays its formation behind a **side card** in two labelled rows
(Vanguard front, Rearguard back) — the 2-D table as each player sees it:

```
[Side A]
  Vanguard   Bram=Torvald   Garrick
  Rearguard  Corvin

[Side B]
  Vanguard   Vesper=Sable
  Rearguard  Wren   Robin
```

The two Vanguards face across the line; each Rearguard sits behind its own front
(front exposed, back shielded).

Side A fields a 3-body front + 1 cannon. Side B fields a 2-body **grouped** front
shielding **two** cannons (the glass-cannon lean — more back-line fire, a thinner
shield). **A back stays shielded until its own front falls and the per-unit lock releases the
killers (§4.6); with a single front group per side, that means the whole group.**

---

## Setup — pull Vitality & Cadence into flippable pools

```
SIDE A
  Bram      M2  T4  health [ ][ ][ ][ ][ ][ ]   F3  tempo [ ][ ]
  Torvald   M5  T3  health [ ][ ][ ][ ]         F2  tempo [ ][ ]
  Garrick   M3  T3  health [ ][ ][ ][ ]         F4  tempo [ ][ ][ ]
  Corvin    M4  T2  health [ ][ ][ ]            F5  tempo [ ][ ][ ][ ]
SIDE B
  Vesper    M3  T3  health [ ][ ][ ][ ]         F3  tempo [ ][ ]
  Sable     M4  T3  health [ ][ ][ ]            F4  tempo [ ][ ][ ]
  Wren      M3  T3  health [ ][ ][ ][ ][ ]      F3  tempo [ ][ ]
  Robin     M4  T2  health [ ][ ][ ]            F5  tempo [ ][ ][ ]
```

The generic **state cards** — the `[ ]`/`[X]` flippables that record Health and Tempo —
stay **visible and distinct** on the table all fight. What tucks **under the identity
card** is the pair of **Vitality and Cadence** stat cards (their values already spent
into the flippable counts); the digital version reveals them on a click. When the battle
ends, the state cards return to their **generic pile**, and the stat cards (Might,
Toughness, Finesse, Vitality, Cadence) go back into the **character deck**.

---

## Round 1 — the Fray (the front holds)

Both backs are protected, so **every attack lands on an enemy Vanguard.** A's three
front bodies and Corvin pound B's `[Vesper=Sable]` group; B's two cannons (Wren,
Robin) fire over their line at A's front. All bids are committed simultaneously;
resolved together (order-independent, §1.9).

**Each defender picks one of three responses** (Spec §3.4), labelled in the log:
- **AVOID** — spend Tempo to **beat** the bid; the blow whiffs, nobody's hurt (block / dodge / slip / evade).
- **STRIKE-BACK** — spend Tempo to **counter**: a mutual trade — the blow still lands *and* you deal your Might back.
- **EAT** — spend nothing; take the Might, deal nothing back (conserve Tempo, avoid being locked from striking reserve).

**A → B's front** (target the `[Vesper=Sable]` group; single-target damage **spills**
to Vesper first):

```
  Corvin → (Vesper=Sable)      bid 1×F5 = 5   AVOID (evade) — WEAKEST-LINK: each must beat 5 alone →
                                              Vesper 2×F3 = 6 ✓ and Sable 2×F4 = 8 ✓ → evaded, but it costs the
                                              pair 4 cards (a soloist spends 2): possible, just dear — and it taps them
  Garrick → (Vesper=Sable)  bid 1×F4 = 4   EAT — drained by that evade, the pair lets it in → Might 3 ▸
                                              Vesper pile 3 ≥ T3 → FLIP
  Torvald → (Vesper=Sable)     bid 1×F2 = 2   STRIKE-BACK — Sable spends its last to counter → Torvald's Might 5
                                              ▸ Vesper → FLIP, and Sable's Might 4 ▸ Torvald pile 4 ≥ T3 → FLIP
```
*A group **blocks** a melee blow by **pooling** Tempo (sum — a strong wall), but to **evade**
ranged fire or **slip** toward the back it's **weakest-link**: every member must beat the bid
**alone**. So a blob is a great shield and a poor dodger — much harder to sneak a group past
guards than a lone body, but never impossible (force-not-fiat, §4.5).*

**B → A's front** (cannons fire; melee can't strike back at range — A only eats or evades):

```
  Robin → Garrick        bid 1×F5 = 5   AVOID  Garrick evades 2×F4 = 8 > 5 → no damage (now tapped)
  Wren  → [Bram=Torvald]    bid 1×F3 = 3   EAT    Might 3 ▸ Bram pile 3 < T4 → no flip (the tank shrugs it)
```

Board after the Fray (the weakest-link evade gutted B's Tempo; the strike-back cost
Torvald a card; Bram shrugged a sub-Toughness hit):

```
SIDE A   Bram   h[......] t[..]   Torvald h[X...] t[X.]   Garrick h[....] t[XXX] (tapped)   Corvin h[...] t[X...]
SIDE B   Vesper h[XX..] t[XX] (tapped)   Sable h[...] t[XXX] (tapped)   Wren h[.....] t[X.]   Robin h[...] t[X..]
```

**No front fell → no one is freed, so the Volley and Breach are empty; nothing was deferred, so
the Reckoning is empty too.** Both fronts are chipped (Vesper −2,
Torvald −1), and that weakest-link evade gutted B's Tempo — **both** of the pair are
tapped (Garrick too). Nobody's back opened; but the Health that's gone **doesn't
heal**, and the thin grouped front — a poor dodger — is bleeding fastest.


### The Lull (refresh)

All `[X]` **Tempo** cards flip back up (refills). **Health stays flipped** (Vesper
keeps `[X][ ][ ][ ]`). Round 2 begins; the battle runs to **5 rounds or a dead side.**

---

## A later round — the front falls, the Volley opens the back

A front *holds* in the **Fray**; the back opens only once a front *falls* and the **per-unit
lock** releases its killers (§4.6). Skip ahead to a later round where the grind finally breaks
B's line: across rounds 2–3, with no healing, **Vesper's pool empties, then Sable's**
(spillover) — **B's Vanguard group is gone.** Every A unit that was attacking it is now **free**
(the enemy Vanguard it struck is dead), so in the **Volley** the back is fair game — B's
Rearguard `{Wren, Robin}` is reachable, on whatever Tempo the round has left (one pool, no
refresh between phases):

```
[Side A]
  Vanguard   Bram=Torvald   Garrick
  Rearguard  Corvin

[Side B]
  Vanguard   (down — the lock releases A's front)
  Rearguard  Wren   Robin        ← now reachable

  Volley — A reaches the back; the rear would answer FIRST (pre-empt), but Robin is dry:
  Corvin → Robin   instant fire, bid 1×F5 = 5   Robin out of Tempo → cannot pre-empt / dodge → EATS
                                                Might 4 ▸ Robin pile 4 ≥ T2 (twice) → FLIP ×2   Robin h[X][X][ ]
  Corvin → Robin   instant fire, bid 1×F5 = 5   Robin h[ ] left → FLIP → ROBIN DOWN
```

The glass-cannon's gamble settles: B out-fired A's front for two rounds, but the thin shield
broke first — and a cannon with **no shield and no Tempo to pre-empt** is just a target. The
**pre-empt** is exactly what Robin lacked: with Tempo it could have answered the incoming fire
(or a charger) *first*; dry, it can only eat. Had A instead reached the open back with an
**empty tank** (all its Tempo spent breaking the line), the back would have answered and lived.
*That* tension — breaking the front vs. having anything left to cash it in — is the whole model.

**Per-unit lock (now canon, §4.6).** Here B's front is a **single group**, so killing it frees
*everyone* at once — the shape the old "all-or-nothing" draft described. The current rule is
**per-unit:** an attacker is free the instant **the enemy Vanguard *it* struck is dead**, even
while *other* enemy Vanguards stand — so a **multi-body** front leaks its dead foes' killers
through on a *partial* break while the rest stay locked. (That earlier "open fork" is decided;
the breach log works the multi-body case.)

---

## Notes / what's locked vs. open

- **Locked (Spec §4 / §4.6):** the **six phases** (Standoff → Fray → Volley → Breach →
  Reckoning → Lull); **per-unit lock** (a killer is freed when the Vanguard it struck dies);
  one simultaneous Tempo bid, defender must **beat not match** (ties land); per-round Tempo
  shared across **all** phases; Health persists; 5-round cap; groups sum-to-block /
  weakest-link-to-evade, damage spills in declared order, one Tempo per member to act.
- **No armor / damage-types** — deferred to gear (§2.2); this is the bare
  `Might → pile → flip per Toughness` core.
- **My liberties (flag any):** the exact bids both sides chose are *a* legal line, not
  a solver-optimal one; standing buffs/braces and role powers (Bulwark, Assassinate,
  Rout-off-the-line) are open dials I left out; "a tie lands the hit" is the single
  contest rule most worth confirming feels right at the table.

## Promoted to the spec (these have since landed)

Design points surfaced while building this log — now folded into canon:

- **Unified defense verb** — *avoid* as the umbrella over the context words *block / dodge /
  slip / evade*. ✓
- **Attack-type cards** — melee / ranged as **cards** an Actor carries that set **reach**;
  a body may carry **both** (multi-reach, §4.2). ✓
- **Side cards / per-side layout** — each formation laid out per side in two labelled rows
  (Vanguard / Rearguard), the player's-eye view rather than one facing grid. ✓
- **Pool stacking** — the flippable Health/Tempo **state cards** stay **visible**; the
  **Vitality & Cadence stat cards** tuck **under the identity card** (revealed on click). ✓
- **Per-unit back access** — the old "all-or-nothing vs partial-break" fork is **decided:
  per-unit lock** (§4.6) — a killer is freed the instant the Vanguard it struck dies, so a
  partial front-break leaks the freed killers through while the rest stay locked. ✓

Still bare here, by design (out of scope for this log): armor / damage-types (gear, §2.2),
Standing buffs / braces, role powers, and the breach/Volley mechanics (their own log).
