# Card-system combat log — a 4v4 round (attrition model)

A worked physical-card walkthrough on the **two-position attrition model** (Spec §4,
2026 — supersedes static ranks). Overworld pickup → decks → blind-bid reveal →
flippable pools → a full round (blind bid → Phase 1 → refresh), then a Phase-2
illustration once a front falls. Cards only; every number is a card.

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

## Blind bid → reveal

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
shield). **While each Vanguard lives, neither back can be touched.**

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

## Round 1 — Phase 1 (the front holds)

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
  Garrick(A) → (Vesper=Sable)  bid 1×F4 = 4   EAT — drained by that evade, the pair lets it in → Might 3 ▸
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
  Robin → Garrick(A)        bid 1×F5 = 5   AVOID  Garrick(A) evades 2×F4 = 8 > 5 → no damage (now tapped)
  Wren  → [Bram=Torvald]    bid 1×F3 = 3   EAT    Might 3 ▸ Bram pile 3 < T4 → no flip (the tank shrugs it)
```

Board after Phase 1 (the weakest-link evade gutted B's Tempo; the strike-back cost
Torvald a card; Bram shrugged a sub-Toughness hit):

```
SIDE A   Bram   h[......] t[..]   Torvald h[X...] t[X.]   Garrick h[....] t[XXX] (tapped)   Corvin h[...] t[X...]
SIDE B   Vesper h[XX..] t[XX] (tapped)   Sable h[...] t[XXX] (tapped)   Wren h[.....] t[X.]   Robin h[...] t[X..]
```

**No Vanguard fell → no Phase 2 this round.** Both fronts are chipped (Vesper −2,
Torvald −1), and that weakest-link evade gutted B's Tempo — **both** of the pair are
tapped (Garrick(A) too). Nobody's back opened; but the Health that's gone **doesn't
heal**, and the thin grouped front — a poor dodger — is bleeding fastest.

### Refresh

All `[X]` **Tempo** cards flip back up (refills). **Health stays flipped** (Vesper
keeps `[X][ ][ ][ ]`). Round 2 begins; the battle runs to **5 rounds or a dead side.**

---

## Phase 2 — when a front falls (a later round)

Skip ahead: across rounds 2–3 A keeps grinding the grouped front; with no healing,
**Vesper's pool empties, then Sable's** (spillover) — **B's Vanguard is gone.** The
instant it falls, **B's Rearguard `{Wren, Robin}` is exposed** for the rest of that
round (no Tempo refresh between phases — A attacks the back on whatever it has left):

```
[Side A]
  Vanguard   Bram=Torvald   Garrick
  Rearguard  Corvin

[Side B]
  Vanguard   (down)
  Rearguard  Wren   Robin        ← now exposed!

  Corvin → Robin   bid 1×F5 = 5   Robin out of Tempo this round → cannot evade → EATS
                                  Might 4 ▸ Robin pile 4 ≥ T2 twice → FLIP ×2   Robin h[X][X][ ]
  Corvin → Robin   bid 1×F5 = 5   Robin h[ ] left → FLIP → Robin DOWN
```

The glass-cannon's gamble settles: B out-fired A's front for two rounds, but the thin
shield broke first — and a cannon with no shield and no Tempo is **just a target.** If
A had instead arrived at the open back with an **empty tank** (all its Tempo spent
breaking the line), Robin would have evaded and lived to fire next round. *That* tension
— breaking the front vs. having anything left to cash it in — is the whole model.

**On reaching the back — your note, checked against §4.** The spec is **all-or-nothing,
not per-attacker:** a side's Rearguard is untargetable *while **any** of its Vanguard
lives*, so the back opens only once the **entire** enemy front is gone — and at that point
no living front unit is left to keep an attacker "stuck." Phase 1 is an explicit
**free-for-all** with **no** persistent engagement lock, so the spec has **no** rule that
an attacker who finished its own Phase-1 fight may slip to the back while *other* enemy
front units still stand. Your "B is stuck unless its own engagements are resolved" model
is **stricter / more granular** — it would let a *partial* front-break leak some attackers
through. A real fork (flagged below).

---

## Notes / what's locked vs. open

- **Locked (Spec §4):** two positions; back untargetable while its front lives;
  one simultaneous Tempo bid, defender must **beat not match** (ties land); per-round
  Tempo shared across both phases; Health persists; 5-round cap; groups
  sum-to-block / weakest-link-to-slip, damage spills in declared order, one Tempo per
  member to act.
- **No armor / damage-types** — deferred to gear (§2.2); this is the bare
  `Might → pile → flip per Toughness` core.
- **My liberties (flag any):** the exact bids both sides chose are *a* legal line, not
  a solver-optimal one; standing buffs/braces and role powers (Bulwark, Assassinate,
  Rout-off-the-line) are open dials I left out; "a tie lands the hit" is the single
  contest rule most worth confirming feels right at the table.

## To promote to the spec (surfaced this pass)

Design points raised while building this log — fold into the spec when the gear/combat
layer is next touched:

- **Unified defense verb** — the one contest's defender action wants a single canonical
  term: *avoid* (umbrella) over the context words *block / dodge / slip / evade*. §4
  currently names them per context; "blocking" alone misreads for a ranged dodge.
- **Attack-type cards** — melee / ranged are **cards** an Actor carries, not just a flag.
  They set **reach** (where you may attack from), which *influences* placement without
  dictating it (you may stand a melee body in the back; it's idle until the line breaks).
  A character may carry **both** a melee and a ranged card (multi-reach) — §4.2 already
  allows "both"; the card representation is the new part.
- **Side cards** — a card marks each side's formation; the table is laid out **per side**
  in two labelled rows (Vanguard / Rearguard) — the player's-eye view, not a single
  facing grid.
- **Pool stacking (presentation)** — the **state cards** (the flippable Health/Tempo
  pools) stay **visible** on the table; it's the **Vitality & Cadence stat cards** that
  tuck **under the identity card** (the digital version reveals them on click). After the
  battle, state cards return to the generic pile; the stat cards go back to the deck.
- **Per-unit Phase-2 access (open fork)** — the spec opens the back **all-or-nothing**
  (untargetable while *any* enemy Vanguard lives). Alternative raised here: a *partial*
  front-break lets an attacker reach the back **iff its own Phase-1 engagements are
  resolved** (never engaged, or all its engaged foes dead), else it's stuck. More
  granular; decide whether partial breaks should leak attackers through before §4 locks.
