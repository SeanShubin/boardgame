# Deckbound — Combos & Meaningful Interactions

> **DEFERRED — the combo/chord system is parked → `future-possibilities.md` (entry 4).** A
> single-deck core comes first. **Frozen history.** *(The §5 zone-move interactions — tags, charges
> — are a separate, current mechanic, not the deferred aspect-chord.)*

The design target: **the meaningful choices are combinations.** A single atomic
action is rarely the interesting unit; the richness is in **chaining effects into one
interaction** — predict a strike, dodge it, counter, and recover, all in the same
breath. Combos are what the game is *for*.

## Three sources of combination

1. **Aspect chords.** One card per aspect combines into a single action (the core —
   see [decks & aspects](decks-and-aspects.md)). Physical, tactical, and magical
   layers compose: a thrown strike, a fire manifestation, and a prediction of the dodge are
   *one* act.
2. **Multi-effect cards.** A single card can carry several effects that all fire in
   one interaction — e.g. a *Riposte* that predicts a strike, evades it,
   counterattacks, **and** turns a face-down card back up.
3. **Stance-outcome chains.** Winning a [stance](mind-and-stances.md) doesn't just bank a
   number; it opens a follow-up. A successful **Evade** repositions *and* sets up a
   counter; a **Block** banks Power *and* leaves the attacker committed.

Crucially, aspects combine in **well-defined, order-independent** ways
([decks & aspects](decks-and-aspects.md#order-never-matters-between-aspects)) — a combo
is **computed, not adjudicated ad hoc** — which keeps even rich interactions
[computable](../canon/1-charter.md#2-computable-tactics-uncomputable-strategy).

## Worked examples

- **Strike + Dodge → counterattack with positional advantage.** Commit a Strike intent
  *and* an Evade stance: you slip the incoming blow (Evade), and the strike you prepared
  lands as a **counter**, carrying the **positional advantage** the Evade grants. Two
  simple pieces, a richer outcome than either alone.
- **Predict + Dodge + Counter + Recover** — one *Riposte* interaction: predict the
  strike, evade it (bank Speed, reposition), counterattack on the banked tempo, and
  turn a face-down card back up (or return it to hand) — resolved together.

## Why combos, not atoms

Meaningful interaction is the goal (see [design principles](design-principles.md):
*dilemmas over optimal plays*, *interconnected systems reward experimentation*). When
effects combine only in well-defined ways, players discover potent chains the designer
never spelled out — exactly the emergence the game is built to reward. A card or a
chord should usually **do several things**, and the skill is assembling the right
interaction for the moment.

## Open questions

- **Where combos live** — fixed bundles (a *Riposte* card), assembled live from
  separate aspect cards, or both?
- **Conditional chains** — "*if* the Evade lands, *then* counter" — how triggers are
  phrased and the order they resolve in.
- **Limits** — what caps a combo (Speed, action economy, aspect slots) so it stays
  bounded and computable?
- How banked **momentum** and **positional advantage** carry into the *next* combo.
