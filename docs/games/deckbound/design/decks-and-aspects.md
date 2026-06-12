# Deckbound — Decks & Aspects

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

## Aspects: effect, means, and the read

Aspects **split apart the dimensions of a single action** — above all, *the
effect you want to achieve* from *the means you use to achieve it*, plus the read
of your opponent. A character acting plays **one card from each aspect they
have**, and the cards combine into a single result.

The current aspects — a starting set, with more to come — are **Body**, **Mind**,
**Magic**, and **Spirit**. Take "I want to deal damage":

- **Body** — the **means**: the physical action, e.g. *punch*.
- **Magic** / **Spirit** — other means and modifiers, e.g. a *fire* spell, or
  reaching something *incorporeal*.
- **Mind** — the **read**: anticipating the opponent and shaping the action to
  suit — e.g. *dodge* their punch so yours lands harder, or *punch faster*. Mind is
  the **tactical** aspect.

So the aspects answer different questions about the same act: *what am I doing*
(Body / Magic / Spirit) and *how does it account for the opponent* (Mind). The
capability to use an aspect at all comes from your **Form** cards — see
[form-and-defeat](form-and-defeat.md).

### Order never matters between aspects

Aspects **combine commutatively** — a chord's result does not depend on the order
the cards are played or read, and combination is always by **well-defined rules**,
never ad-hoc. (This is the deliberate opposite of
[attached modifiers on a single card](#kinds-of-card), where order *does* matter.)

### Only the tactical aspect is rock-paper-scissors

The **tactical aspect — Mind — is the only one with rock-paper-scissors
behavior**: it is where the hidden-information mind-game lives, reading and
countering the opponent. The other aspects (Body, Magic, Spirit) **compose
deterministically** — they add magnitude and modifiers but do not themselves play
the guessing game. (This is also why disabling a creature's Mind makes it
predictable; see [form-and-defeat](form-and-defeat.md).)

This is what keeps the [tactical exchange
computable](philosophy.md#2-computable-tactics-uncomputable-strategy): the RPS that
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

A character's cards live in four **zones** — **Form** (your capability cards and
health), **Potential** (the cards you can play), **Active** (cards in play,
including Lasting effects), and **Dormant** (used or sealed cards). Form is your
vitality ([form-and-defeat](form-and-defeat.md)); Potential → Active → Dormant is
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
  modifiers, and the tactical read produce one result? (Settled in principle:
  aspects combine commutatively, and only the tactical aspect is rock-paper-
  scissors.)
- How much of the **never-shuffled order** does the player see and control? Can
  order be rearranged, or only built?
- What is the granularity of **acquiring a set of cards** — fixed bundles,
  individual cards, upgrades?
- How is an **entirely new aspect/deck** introduced to an existing character?
