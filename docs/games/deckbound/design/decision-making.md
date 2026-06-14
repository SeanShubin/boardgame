# Deckbound — Decision-making & Hidden Information

The beating heart of the game: a hidden-information, rock-paper-scissors-style
contest. Every participant — human, computer stand-in for a human, or non-player
environment creature — makes the **same kind of choice** under the **same rules**,
and resolution treats them identically. They differ in one decisive way: whether
they have a **theory of mind** — the ability to model *you* as a strategist and
predict your plan. Humans and their stand-ins do; creatures don't — which is why
only creatures decide by **deck**.

## The core exchange

1. **Commit.** Each side selects an action and places it **face down** — a
   hidden choice (drawn from the cards it is legally allowed to play).
2. **Reveal.** All sides flip **simultaneously**. No one reacts to another's
   choice within the exchange (constraint
   [C4](constraints.md#c4--hidden-simultaneous-choice-must-be-physical)).
3. **Resolve the stances.** The **tactical** aspect is the rock-paper-scissors layer:
   who predicted whom decides who gains the upper hand (see
   [the action cycle](#the-action-cycle) and
   [aspects](decks-and-aspects.md#only-the-tactical-aspect-is-rock-paper-scissors)).
4. **Resolve magnitude.** The other aspects — the Body means (a strike or a cast
   alike) and other modifiers, and any attached numbered cards — combine **deterministically and
   order-independently** into *how much*: damage dealt, size of a bonus, whether a blow
   **drops** its target.

The stance outcome is categorical (who gains the upper hand); magnitude is numeric (by how
much). The hidden commitment is what makes it a game of stances and bluffs rather
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

A human's cards are a **toolkit** — *what the character can do*; the **agency** is
the player's, laid on top: which option to commit, when to bluff, how to predict the
opponent. The human supplies the theory of mind. Unpredictability comes from free
will, and predictability is a managed resource as cards exhaust (see [zones](zones.md)).

### Computer stand-in for a human

Represents a human opponent and is **bound by the same rules** (constraint
[C3](constraints.md#c3--every-agent-is-bound-by-the-same-rules)). It plays a
**game-theoretically optimal mixed strategy by computing it directly**, in the
moment — exactly as a thoughtful human could. It does **not** use a deck; a deck
would only be a way to fake what a real mind can simply do. Above all it has a
**theory of mind** — it models *this* opponent and adapts, predicting your tendencies
and bluffing back; that adaptiveness is exactly why it must **compute live** rather
than draw a fixed deck. (This stays feasible because the tactical exchange is
deliberately **constrained to be computable** —
see [philosophy §2](philosophy.md#2-computable-tactics-uncomputable-strategy).)

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

Creatures, traps, and hazards that **no player controls**. These decide by **deck** —
and the deck is not a menu they choose from, it **is the decision**: the cards are the
creature's **instinct made physical**, drawn to produce its action. There is no
separate chooser; drawing *is* choosing.

That instinct can be **sophisticated and even unpredictable** — reposition, lunge,
claw, retreat, regroup, ambush — and it can be **conditional on what the creature
observes**. A behavior card might read:

> *If outnumbered, run the enemy back line; if alone, flee; otherwise press the
> front line.*

So creatures are **not** simple or perfectly predictable. **Game theory can be baked
into the deck** — sound play in the abstract — and the conditions let it react to the
board. What the deck **cannot** do is condition on *you*: it responds to observable
state (numbers, position, wounds), never to a model of the opponent's mind. It will
not learn your habits, bait your tells, or build a counter-strategy to your plan.

A creature deck also **reshuffles after every play** (drawn with replacement), so it
**never depletes or exhausts** the way a human's Potential does — its instinct has no
fatigue and no memory.

> **The deck is instinct, not a mind.** A creature can surprise you with *what* it
> does, but never out-predict you about *who you are*. Your edge is to study its rules
> and devise a strategy it has no way to counter — exactly the human intellect the
> game rewards.

### The line: theory of mind

The decisive difference among the three is **theory of mind** — modelling the
opponent as a strategist and predicting their plan. Humans and their stand-ins have
it; creatures do not. This is why their **cards serve different purposes**:

- A **human's** cards are *options under agency* — the character's toolkit, with the
  player reasoning over them, predicting, and bluffing on top.
- A **creature's** cards *are the agency* — the rule-based instinct that decides for
  it, with no mind behind the wheel.

Same physical medium, opposite role: for the human the cards are *what they can do*;
for the creature they are *how it decides*. And only a mind can get inside another
mind — so prediction is **two-way against a human or stand-in** (you predict each other)
but **one-way against a creature** (you study its rules; it cannot predict you back).

### How the three compare

|                          | Source of the hidden choice            | Models *you*?                            | Uses a deck?              | Exhausts?                                       |
| ------------------------ | -------------------------------------- | ---------------------------------------- | ------------------------- | ----------------------------------------------- |
| **Human player**         | free, secret reasoning                 | yes — theory of mind                     | no — plays from Potential | yes — predictability erodes ([zones](zones.md)) |
| **Computer stand-in**    | game theory, computed live             | yes — adapts to this opponent            | no                        | n/a                                             |
| **Environment creature** | a conditional behavior deck (instinct) | **no** — reacts to the board, not to you | yes                       | no — reshuffles each play                       |

The player's own capability decks are **never shuffled** (deliberate order is part of
the skill); their hidden-ness comes from *which card they choose to commit*. Creatures
invert this: their cards are **drawn, not chosen**, and they decide by **rule, not by
predicting you**.

## The action cycle

The rock-paper-scissors cycle is the **Mind** aspect's stance game — Strike / Block /
Evade / Scheme, the **momentum** winning stances bank, and the **misjudged stance** that
forfeits it. It now has its own home: see
[the Mind: stances & momentum](mind-and-stances.md).

## Open questions

- **Conditional behavior, in practice.** Creature actions branch on observable state
  via rule-based cards (settled above). Open: how complex those conditions can get
  while staying quick to adjudicate by hand.
- **Human predictability is a managed resource**, eroding as cards exhaust and
  restored by recovery at a tempo cost — see [zones](zones.md). Open question:
  how fast it erodes and how costly recovery is.
- **Mixing the magnitude layer in.** Exactly how numberless qualities and
  numbered multipliers turn a categorical win into a number (damage, bonus size,
  the lethal threshold).
- **Bluff space.** With hidden commitment, do players have feints / partial
  information / tells, or is it pure simultaneous reveal?
- **Multi-party exchanges.** How resolution generalizes when several allies (who
  share full information) and the environment all commit into one exchange — see
  [turn structure](turn-structure.md). Same-region allies resolve together.
