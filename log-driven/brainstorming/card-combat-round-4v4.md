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
resolved together (order-independent, §1.9). Notation: `bid vs block → result`.

**A → B's front** (target the `[Vesper=Sable]` group; single-target damage **spills**
to Vesper first):

```
  Corvin → (Vesper=Sable)   bid 1×F5 = 5   (Vesper=Sable) SUM-blocks: Vesper 1×3 + Sable 1×4 = 7 > 5  → BLOCKED
  Garrick(A) → (Vesper=Sable)  bid 1×F4 = 4   the pair is low on Tempo, EATS → Might 3 ▸ Vesper pile 3 ≥ T3 → FLIP
  Torvald → (Vesper=Sable)     bid 1×F2 = 2   Sable blocks 1×F4 = 4 > 2  → BLOCKED  (Might 5 stopped by one cheap card —
                                  low Finesse means a big blow is easy to turn until the defender runs dry)
```

**B → A's front** (cannons fire; A blocks):

```
  Robin → Garrick(A)        bid 1×F5 = 5   Garrick(A) blocks 2×F4 = 8 > 5  → BLOCKED (Garrick(A) now tapped)
  Wren  → [Bram=Torvald]    bid 1×F3 = 3   they SUM-block: Bram 1×3 + Torvald 1×2 = 5 > 3  → BLOCKED
```

Board after Phase 1 (only Vesper took damage; everyone else blocked, bleeding
Tempo to do it):

```
SIDE A   Bram   h[......] t[X.]   Torvald h[....] t[X.]   Garrick h[....] t[XX.] (tapped)   Corvin h[...] t[X...]
SIDE B   Vesper h[X...] t[X.]     Sable  h[...] t[X..]    Wren h[.....] t[X.]    Robin h[...] t[X..]
```

**No Vanguard fell → no Phase 2 this round.** B's front is intact but down a Health
card and low on Tempo; A spent most of its Tempo *defending* and lands only a chip.
This is the attrition: a thin, grouped front holds *one* round by sum-blocking, but
it's bleeding — and **Health doesn't heal.**

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
