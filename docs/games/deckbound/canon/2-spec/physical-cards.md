# Spec — Physical Cards (the conserved-deck embodiment)

> **Scope.** This is a Spec document (mechanics — invariants, vocabulary, and a
> procedure), sibling to the combat rules in `README.md`. Where the combat Spec
> says *how state changes*, this says *how state is physically held*. It governs
> the **card-table product** (`crates/cardtable-model`); the combat game
> (deckbound) is content that must obey it. Written RULE / WHY / GUARANTEES, per
> `canon/0-source-of-truth.md`. **DRAFT — awaiting ratification.**

The one line: **the whole game is a single, closed deck of physical cards.** You
can pack it into a box and lay it back out and lose nothing that matters. Every
law below follows from taking that sentence literally.

---

## PC.1 — Cards are the state

**RULE.** Every mechanically-meaningful fact is carried by a **card** — its
**identity** (what is printed on its face), its **facing** (face-up or
face-down), and its **zone** (which pile it is in). Nothing mechanical lives in a
screen position, a focus/selection, a drill-in level, or application memory.

**WHY.** A physical game has no hidden variables — if a fact isn't on the table,
it isn't in the game. This is what makes PC.3 (pack-up) possible at all, and it
is the card-table restatement of the deckbound charter's *state-is-physical /
cards-only, never human memory* stance.

**GUARANTEES.**
- The **mechanical state** of the game is exactly the multiset of cards with
  their (identity, facing, zone, and within-zone order). Two boards with the same
  such multiset are the **same game**.
- The **transitory set** (PC.4) is *not* part of the mechanical state and may be
  discarded and rebuilt at any time.

---

## PC.2 — Conservation

**RULE.** The **total number of physical cards is fixed** for a given game —
**conservation is on the *sum*** (`card_count`), not on the number of distinct
stacks. A run of identical cards in one pile is a single **stack** carrying a
**quantity** (`6 ×12`, PC.5); the *physical* count is the sum of those quantities.
During play a card is only ever **moved** between zones, **flipped** (`set_face`),
or **split / merged** — drawing one off a `×N` stack (`×N → ×(N-1)` plus a `×1`)
and returning it (merging back into the stack). Each of those preserves the sum, so
**no card is created or destroyed on net**. Wholesale minting / discarding happens
**only at setup**, when the closed deck is dealt out. Every quantity a game needs is
therefore a **pre-provisioned, bounded supply of cards** (PC.5), never an unbounded
counter.

**WHY.** You cannot manufacture a card mid-game, and you cannot rewrite what one
says. So a changing *number* is not a mutated card — it is a **different card
drawn from a bank**, or a **count of a growing pile**. Conservation is what lets
PC.3's packed deck be a fixed, checkable size, and it forces the design to state
its own bounds up front (a game that can't say how many cards it needs isn't
printable).

**GUARANTEES.**
- The card count is invariant across every in-play action (an executable check).
- **Numbers are cards, not mutation.** A stat value is a card drawn from a bank of
  value cards (`Might 1 … Might 9`); the day is not a rewritten card but a
  **counted track** (PC.5). `set_face` flips up/down and never rewrites a face's
  title — a face-down card **remembers its front** so the flip is reversible.
- **Duplicates fold — in the model, not just on screen.** A run of identical cards
  in the **same pile** is one stack with a quantity (`card.quantity`); a bank of many
  copies is one card carrying its count. "Same pile" and *contiguous* is the rule, so
  a character deck reading `Might 6 … Vitality 6` keeps its two `6`s separate (a stat
  name sits between them) — positioned stat-pairs are never merged.
- **Split & merge conserve.** Assembling a character *draws one* off each bank stack
  (a split); un-equipping *returns one* (a merge). The physical sum is invariant
  across both — the executable check is `card_count` before == after.

---

## PC.3 — Pack-up: the game is one ordered deck

**RULE.** The entire game **packs into a single one-dimensional ordered deck** and
**unpacks** back to the same mechanical state. The packed deck is a **pre-order
flattening** of the zone tree: each pile contributes its **Zone card** (a
*divider*) followed by its contents; nesting is recovered because a divider's
**kind/type carries its depth** (a `Location` divider opens a top-level zone; a
character's identity divider nests inside the current location; a loose card is a
leaf of whatever is open — depth by type, with an explicit level number as the
general fallback). Unpack reads left-to-right and rebuilds the tree.

**WHY.** This is putting the game back in its box: a sectioned, ordered stack with
a labelled divider before each section. Because every pile already owns a Zone
card, the dividers exist for free; because facing and identity travel with each
card (PC.1, PC.2), a face-down day-copy or a damaged Health card restores to the
right thing. "Between one divider and the next" recovers a single level; a
divider's depth recovers the nesting.

**GUARANTEES.**
- `unpack(pack(state))` equals `state` **modulo the transitory set** (PC.4) — an
  executable round-trip test, stronger than the position-preserving RON save.
- In the packed deck, **global order is mechanical** (it encodes the tree);
  **within a zone, order is preserved** as-is (a draw deck keeps its sequence; a
  fan's order is cosmetic but harmless).
- **Resting-state only** (PC.4): pack-up is defined only at a resting mechanical
  state — between moves, or between combat rounds — never mid-drag or
  mid-resolution.

---

## PC.4 — Mechanical vs transitory

**RULE.** Exactly the following is **mechanical** (survives pack-up) versus
**transitory** (rebuilt on layout, never packed):

| Mechanical (card-encoded, survives pack-up) | Transitory (rebuilt on layout) |
| --- | --- |
| The set of cards and their identities | Screen x/y of every pile and card (the app-persistence layer) |
| Each card's facing (Health damage = face-down; a moved day-copy = face-down) | The focused / grown / fanned-to-front card |
| Each card's zone (a character's location = which location zone holds its location card) | Selection / highlight |
| Within-zone order where order is a rule (a draw deck) | The drill-in navigation stack (which zone you're inside) |
| The day (a counted event/day track, PC.5) | Mid-drag: the card in hand, drop-zone highlights, ghosts |
| | The combat log / play-by-play (regenerated, not state) |
| | Undo history, hover, animation/tween |

**WHY.** The split is forced by PC.1: a fact is mechanical iff it is on a card.
Positions, focus, and the log are *views of* the state, not the state. Naming the
transitory set precisely is the point — it is the exhaustive list of what a
pack-up is *allowed* to forget.

**GUARANTEES.** The two persistence layers are: **app persistence** (the full
`Tableau` → RON, including positions — "resume the window exactly") and
**physical persistence** (PC.3 — "put it in the box and lay it out again"). App
persistence is a superset; physical persistence is the mechanical projection, and
its loss is exactly the transitory column above.

---

## PC.5 — Provisioning (bounds designated up front)

**RULE.** Because the deck is closed (PC.2), the game **designates every supply up
front, bounded and sufficient**:

- **Number cards: 0–9.** A single digit; larger magnitudes are composed or counted,
  never a two-digit card.
- **Stat-value cards: 1–9 per stat** (a bank per stat). Kits reach Might 6; no single
  printed value exceeds 9. (A *swarm* is many one-Health body cards folded to `×N`,
  not a `45` value card.)
- **Party size ≤ 9** (balance work runs at **5**).
- **Identity copies per hero: 4** — three tracker copies (identity / combat-intention
  / location) plus the day-clock copy.
- **Health: a per-character pool sized by Vitality** (flipped down as damage lands,
  up on recovery — never minted).
- **The day is a counted track, not a number.** Each day lays down one card from a
  bounded **event/day reserve**; *the day is the count of that track* (to be replaced
  by real **event** cards — each day an event — so "which day is it?" is answered by
  counting the events). This dodges the 0–9 cap entirely.
- **Reserves / banks** are ordinary zones of unused cards (value banks, identity
  reserve, the event reserve, spare body cards), shown folded with `×N`.

**WHY.** "Sufficient and bounded" is the printability check made physical: a closed
deck must be big enough to play the longest legal game and no bigger. Counting a
track instead of capping a number is what keeps the day unbounded-in-play while the
*cards* stay finite.

**GUARANTEES.** The provisioned totals are enough for the worst-case legal game
(max party, max days, max simultaneous values), and the constant-count check (PC.2)
holds against them.

---

## Vocabulary

- **TERM.** `Zone` — a named pile; its **Zone card** (a divider) labels it and, in a
  packed deck (PC.3), marks where the zone's contents begin.
- **TERM.** `Divider` — a Zone card acting as a section header in the packed deck; its
  kind/type encodes its nesting depth.
- **TERM.** `Reserve` / `Bank` — a zone of unused cards drawn from during play (a value
  bank, the identity reserve, the event reserve); duplicates fold to `×N`.
- **TERM.** `Day track` — the growing pile of day/event cards; the current day is its
  **count** (PC.5), not a number card.
- **TERM.** `Face-down` — a card flipped to hide its front; it still *is* that card (it
  remembers its front, PC.2), so damage and "has-moved" are reversible flips.
- **TERM.** `Multiplier (×N)` — the fold that shows N identical cards as one.
