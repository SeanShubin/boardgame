# Deckbound — Decision-making & Hidden Information

The beating heart of the game: a hidden-information, rock-paper-scissors-style
contest. Every participant — human, computer stand-in for a human, or non-player
environment creature — makes the **same kind of choice** under the **same
rules**. They differ only in *how* the choice is generated, and only the
non-player environment uses a **deck** to do it.

## The core exchange

1. **Commit.** Each side selects an action and places it **face down** — a
   hidden choice (drawn from the cards it is legally allowed to play).
2. **Reveal.** All sides flip **simultaneously**. No one reacts to another's
   choice within the exchange (constraint
   [C4](constraints.md#c4--hidden-simultaneous-choice-must-be-physical)).
3. **Resolve the read.** The **tactical** aspect is the rock-paper-scissors layer:
   who read whom decides who gains the upper hand (see
   [the action cycle](#the-action-cycle) and
   [aspects](decks-and-aspects.md#only-the-tactical-aspect-is-rock-paper-scissors)).
4. **Resolve magnitude.** The other aspects — the Body means, Magic and other
   modifiers, and any attached numbered cards — combine **deterministically and
   order-independently** into *how much*: damage dealt, size of a bonus, whether an
   interrupt lands.

The read is categorical (who gains the upper hand); magnitude is numeric (by how
much). The hidden commitment is what makes it a game of reads and bluffs rather
than pure arithmetic.

## The three decision-makers

A design north star: **Deckbound represents and rewards human intellect**
(see [philosophy](philosophy.md)). Wherever a *human mind* is being represented —
a player, or a computer standing in for one — the choice is made by **actual
reasoning**, not by drawing from a deck. **Decks are reserved for non-player
environment creatures.** All three still perform the same physical act in an
exchange — a hidden choice from legal options — so resolution treats them
identically.

### Human player

Chooses freely and secretly: by judgment, reading opponents, bluffing, and
managing their own predictability — which erodes as cards exhaust (see
[zones](zones.md)). Unpredictability comes from free will.

### Computer stand-in for a human

Represents a human opponent and is **bound by the same rules** (constraint
[C3](constraints.md#c3--every-agent-is-bound-by-the-same-rules)). It plays a
**game-theoretically optimal mixed strategy by computing it directly**, in the
moment — exactly as a thoughtful human could. It does **not** use a deck; a deck
would only be a way to fake what a real mind can simply do. This is possible
because the tactical exchange is deliberately **constrained to be computable**
(see [philosophy §2](philosophy.md#2-computable-tactics-uncomputable-strategy)).

This role is **optional and never required by the system**: a computer fills it
in a digital game, another human fills it at a physical table, and in the default
co-op scenario it is simply absent (the adversaries are environment creatures).
Because the stand-in only ever does what a human in its seat could do, it
satisfies [C1](constraints.md#c1--fully-playable-without-a-computer) — the game
needs no computer to *run*; a computer merely substitutes for a human *player*.

> A precomputed frequency deck *could* approximate a stand-in for a purely
> printed solo game with no second human — but that is a fallback, not the
> canonical design. The canonical stand-in reasons directly.

### Environment creatures & hazards (non-player)

Creatures, traps, and hazards that **no player controls**. These are the agents
that use a **deck**, and they differ from a human's cards in two key ways:

- **They reshuffle after every play** (drawn, in effect, *with replacement*), so
  a creature deck **never depletes or exhausts** the way a human's Potential does. Its
  behavior distribution is **stationary**.
- The deck encodes the creature's **tendencies** — a fixed, *learnable* behavior,
  deliberately **readable rather than arbitrary noise**. The player is meant to be
  **rewarded for reading and exploiting** how a creature tends to act. (This is
  the human-intellect north star applied to solo and co-op play: the cleverness
  is the player's, drawn out by a legible opponent.)

> **The deck as a stationary mixed strategy.** Because a creature deck reshuffles,
> drawing its top card samples a *fixed* distribution every time. The player's
> edge comes from learning that distribution and winning the rock-paper-scissors
> reads — not from counting down a depleting deck.

### How the three compare

| | Source of the hidden choice | Uses a deck? | Depletes / exhausts? |
| --- | --- | --- | --- |
| **Human player** | free, secret reasoning | no — plays from Potential | yes — predictability erodes as cards exhaust ([zones](zones.md)) |
| **Computer stand-in** | game theory, computed live | no | n/a |
| **Environment creature** | a stationary behavior deck | yes | no — reshuffles after every play |

The player's own capability decks are **never shuffled** (deliberate order is part
of the skill); their hidden-ness comes from *which card they choose to commit*,
and their predictability is a managed resource. Environment creatures invert both:
shuffled and reshuffled, never deliberate, never exhausting.

## The action cycle

The rock-paper-scissors cycle is the **Mind** aspect's read game — Strike / Block /
Evade / Scheme, the **momentum** winning reads bank, and the **misread** that
forfeits it. It now has its own home: see
[the Mind: reads & momentum](mind-and-reads.md).

## Open questions

- **Situation-dependent creature behavior.** A creature's right tendencies change
  with its situation (fresh, wounded, cornered, enraged). Does a creature carry
  several behavior decks keyed to state, or one base deck plus modifier cards?
- **Human predictability is a managed resource**, eroding as cards exhaust and
  restored by recovery at a tempo cost — see [zones](zones.md). Open question:
  how fast it erodes and how costly recovery is.
- **Mixing the magnitude layer in.** Exactly how numberless qualities and
  numbered multipliers turn a categorical win into a number (damage, bonus size,
  interrupt threshold).
- **Bluff space.** With hidden commitment, do players have feints / partial
  information / tells, or is it pure simultaneous reveal?
- **Multi-party exchanges.** How resolution generalizes when several allies (who
  share full information) and the environment all commit into one exchange — see
  [turn structure](turn-structure.md). Same-region allies resolve together.
