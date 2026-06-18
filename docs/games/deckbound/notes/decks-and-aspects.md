# Deckbound — Decks & Aspects

> **DEFERRED — the aspect/chord combo system is parked → `future-possibilities.md` (entry 4).** The
> current core is a **single deck** (Form + attachments + Action cards over the §5 zones), not a
> chord of aspect-decks. This note is **frozen history**. *(The three defense channels Body / Mind /
> Spirit, §2, are unaffected — different "aspect.")*

The core system. Everything a character is and does is expressed as cards in
decks.

## A character is a set of decks

Each deck represents some facet of a character's capabilities **at the current
point in the game** — decks change as the character grows. A character may hold
**many** decks, of different **deck types**, and those types have **predefined
ways of interacting** with one another.

Characters are **not meant to be balanced** against each other; uneven
capabilities are a feature. Fairness comes from scenario design (see
[world-and-progression](world-and-progression.md)).

## The aspects

An **aspect** is a dimension of a single action: a character acting plays **one card
per aspect they have**, and the cards combine into one result. (The capability to use
an aspect at all comes from your **Form** cards — see
[form-and-defeat](form-and-defeat.md).) They split cleanly into **outer** (it
manifests physical, typed effects — armor applies) and **inner** (they reach the
*capacity to act*, not the body — armor is no defense).

The distinction is deeper than "armor doesn't help against the inner ones." It is
structural: **inner aspects (Mind, Spirit) are thresholds; the outer aspect (Body)
is a pool.** The outer aspect is a *pool* of Health cards (governed by a Vitality card)
that typed damage **erodes** card by card until it runs out — cut by **Armor** and
gated by **Toughness** (per-card capacity). An inner aspect is a **standing capacity /
threshold** — a value an attack must **overcome**, not deplete — with **no Health-card
stack** behind it. Inner attacks are first cut by **Ward** (a passive, typed
vs-fear / vs-confusion number on a card, applied before the inner bar), then must clear
a **bar**: **Resolve** for Spirit, **Mind-capacity** for Mind — mirroring
**Armor → Toughness** on the outer side:

- **Mind** is a **prediction-bandwidth capacity** — a focus pool sized by Mind, spent
  predicting attackers; **Confusion lowers the capacity** rather than chipping a stack.
- **Spirit** is the **Resolve threshold** — a single standing value that accumulated
  **Fear** must *exceed* to break you; Resolve itself never depletes.

See [form & defeat](form-and-defeat.md#how-damage-resolves--the-vitality-card-and-health-cards)
for the full pools-vs-thresholds mechanics.

**Outer — physical (Body only):**

- **Body** — your own **physical means**: Power and Speed; the punch, the parry, the
  run. Force on flesh. It is the **one outer pool** and the **one outer health pool**.
  A Body card carries a **damage type** (heat, cold, lightning, force, …) and
  **delivery** properties (range, area, status); a conjured effect is just a Body
  delivery — **typed and stopped by Armor vs that type** (e.g. vs-heat) like any blow,
  never a bypass. *Muscle and bone — and a conjurer's fire still burns like fire.*

**Inner — the capacity to act:**

- **Mind** — the **stance**: cognition, prediction, **Precision** (knowing where to
  strike), the [rock-paper-scissors](mind-and-stances.md) of stances. Its bar is a
  **focus pool / Mind-capacity** (prediction bandwidth) sized by Mind; hit it with
  **Confusion** and you shrink that capacity until the foe can no longer out-predict
  you. Incoming confusion is first cut by **Ward (vs-confusion)** — a passive, typed,
  per-source, never-depleting number on a card, applied **before** the Mind-capacity
  bar (the inner analogue of Armor → Toughness). *Wits.*
- **Spirit** — the **will to act**: courage against fear, resolve against faltering,
  morale, disposition. Its bar is the **Resolve threshold** — a standing value that
  accumulated **Fear** must *exceed* to break you. Incoming fear is first cut by
  **Ward (vs-fear)** — a passive, typed, per-source, never-depleting number on a card,
  applied **before** the Resolve bar (the inner analogue of Armor → Toughness). Ward
  guards only **fear (Spirit)** and **confusion (Mind)**; it is **not** anti-magic — a
  conjured fireball is stopped by **Armor vs-heat**, not Ward. Spirit lands no physical
  blow and works **only if you let it** — it acts *through your own response*. A
  **fearless** character
  ignores a ghost
  outright; a **fearful** one can be **scared to death** by their own body's panic.
  (A ghost is essentially a *fear elemental*: no physical effect, only the
  psychological — and the only thing that touches the incorporeal, which has will and
  presence but no body.) *A sound body is useless if the spirit won't swing it.*

So a full action answers two questions at once: *what physical thing happens* (Body)
and *how the actor thinks and wills it* (Mind, Spirit). "I want to deal
damage": a **Body** punch, maybe with a fire **damage type** on it, shaped by a **Mind**
stance against the dodge, driven by the **Spirit** to commit. The **outer** aspect is stopped
by **Armor and Toughness**; **inner** aspects are turned aside by **Ward** and then by
**composure and resolve (the bar)** — meet them with enough and they wash over you;
fall short and you undo yourself. The set will grow, but these three are the spine.

### Order never matters between aspects

Aspects **combine commutatively** — a chord's result does not depend on the order
the cards are played or read, and combination is always by **well-defined rules**,
never ad-hoc. (This is the deliberate opposite of
[attached modifiers on a single card](#kinds-of-card), where order *does* matter.)

### Only the tactical aspect is rock-paper-scissors

The **tactical aspect — Mind — is the only one with rock-paper-scissors
behavior**: it is where the hidden-information mind-game lives, predicting and
countering the opponent. The other aspects (Body, Spirit) **compose
deterministically** — they add magnitude and modifiers but do not themselves play
the guessing game. (This is also why disabling a creature's Mind makes it
predictable; see [form-and-defeat](form-and-defeat.md).)

This is what keeps the [tactical exchange
computable](../canon/1-charter.md#2-computable-tactics-uncomputable-strategy): the RPS that
must stay solvable is a single aspect, layered over an otherwise deterministic
composition. The Body side — its stats and deliveries — lives in
[cards & customization](cards-and-customization.md).

## Kinds of card

- **Numberless cards** represent a **quality or effect** directly — e.g. *speed*,
  *power*, *precision*, or *1 damage*.
- **Modifier cards** **attach to another card** and change its value — e.g. a
  *+1* or a *×2*.

**Attachment order matters.** Modifiers apply in the order they are attached, so a
base *1 damage* with *+1* then *×2* is `(1 + 1) × 2 = 4`, while *×2* then *+1* is
`(1 × 2) + 1 = 3`. This is the one place order is significant — deliberately the
opposite of aspect combination, where order never matters. A stack of a base card
plus its attached modifiers expresses both *what* is brought to bear and *how
much*.

### Passive attribute cards

Not every card is an action. A character also has cards representing **passive
attributes** — standing traits that modify how things resolve. Example: an
**armor** card that changes how physical damage is applied to the character.

## Never shuffled

A character's decks are **never shuffled**. Deck order is deliberate — it
carries intent and information rather than luck. (The randomness in the game
lives elsewhere: in enemy decks and some scenario decks — see
[world-and-progression](world-and-progression.md).)

This principle implies a very different feel from a typical card game: drawing is
predictable, so sequencing, planning, and *building* the deck in the right order
become the skill. **Each card defines its own starting zone** (see
[zones](zones.md)), so a character's opening configuration is built into the cards
themselves rather than dealt out. Exactly how much foresight the player has beyond
that, and how order is manipulated, is an open question.

## Card zones

A character's cards live in three **zones** — **Form** (your capability cards and
health), **Potential** (the cards you can play), and **Active** (cards in play,
including Lasting effects). A card on the table has a **facing**: face up = in effect,
face down = not in effect (a spent or sealed card). Form is your
vitality ([form-and-defeat](form-and-defeat.md)); Potential → Active is
the tactical layer, where playing cards exhausts options and makes you more
predictable, and recovery costs tempo ([zones](zones.md)).

**How many cards can be played is dependent on game mechanics** — the cap is set
by the rules of the situation, not fixed globally.

## Equipping capabilities

Sometimes a character can **equip a capability**, which is represented by
**acquiring a set of cards**. Equipping is therefore a deck-building act: gaining
a capability literally adds cards (and possibly a whole new aspect/deck) to the
character.

## Open questions

- What are the **exact combination rules** for a chord — how the physical means,
  modifiers, and the tactical stance produce one result? (Settled in principle:
  aspects combine commutatively, and only the tactical aspect is rock-paper-
  scissors.)
- How much of the **never-shuffled order** does the player see and control? Can
  order be rearranged, or only built?
- What is the granularity of **acquiring a set of cards** — fixed bundles,
  individual cards, upgrades?
- How is an **entirely new aspect/deck** introduced to an existing character?
