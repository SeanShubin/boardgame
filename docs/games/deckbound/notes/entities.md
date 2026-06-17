# Deckbound — Entities: Actor, Character, Creature

What kinds of *things* fight, and the one word that covers them both. This is the
**noun taxonomy**; for *how* each kind makes its hidden choice, see
[decision-making](decision-making.md).

## The umbrella: Actor

An **Actor** is anything that takes a turn in a conflict — it has a
[Form](form-and-defeat.md) (capabilities + health by aspect), a **keystone**, and it
commits hidden choices into an [exchange](decision-making.md#the-core-exchange) under
the same rules as everyone else. Resolution treats all Actors **identically**;
knockout, damage, predictions, and tempo don't care what kind an Actor is.

Actors come in two kinds, split by **one decisive question — does it predict you back?**

> Every Actor is a performer in the fight. A **Character improvises**; a **Creature
> follows a script.**

## Character — the improviser (full agency)

A **Character** decides by **actual reasoning**. It has a **theory of mind**: it
models *you* as a strategist, predicts your tendencies, and bluffs back. Its cards are a
**toolkit** — *what it can do* — and the agency is laid on top: which option to
commit, when to bluff, when to cash in. A Character **never draws from a deck**;
its capability decks are [never shuffled](decks-and-aspects.md#never-shuffled), and its
hidden-ness comes from *which card it chooses to commit*, not from a random draw.

A Character is **controlled by either**:

- a **human player**, or
- a **computer stand-in** that imitates human agency — it computes a
  game-theoretically sound mixed strategy live and adapts to *this* opponent, exactly
  as a thoughtful human could (see
  [decision-making](decision-making.md#computer-stand-in-for-a-human)).

Both are the same entity; only the driver differs. Prediction against a Character is
**two-way** — you predict each other.

## Creature — the script (rule-driven)

A **Creature** decides by **rules card + behavior deck**. The deck is not a menu it
chooses from — it **is** the decision: the cards are the Creature's **instinct made
physical**, drawn to produce its action (drawing *is* choosing). Its behavior can be
sophisticated, conditional on what it observes (*"if outnumbered, flee; else press the
front"*), and it **can bluff and play the duel via a distribution** over its stances — a
mixed strategy baked into the deck. The deck **reshuffles after every play**, so a
Creature **never exhausts**: its instinct has no fatigue and no memory.

What a Creature **cannot** do is condition on *you*. It reacts to observable state
(numbers, position, wounds), never to a model of your mind — it won't learn your
habits, bait your tells, or build a counter-strategy. Prediction against a Creature is
**one-way** — you study its rules; it cannot predict you back. This category also covers
**non-fighter scripted threats** (traps, hazards) that decide the same way.

## At a glance

|                          | **Character**                                   | **Creature**                            |
| ------------------------ | ----------------------------------------------- | --------------------------------------- |
| **Decides by**           | reasoning — a deliberate, chosen strategy       | rules card + behavior deck (instinct)   |
| **Theory of mind**       | yes — predicts you back (two-way)               | no — reads the board, not you (one-way) |
| **Randomness in choice** | none — chooses; never shuffles                  | the deck *is* its mixed strategy        |
| **Controlled by**        | a human, or an AI imitating human agency        | the rules, every time                   |
| **Exhausts?**            | yes — predictability erodes ([zones](zones.md)) | no — reshuffles each play               |

Both are **Actors**, and both can be powerful, durable, and unpredictable about *what*
they do. The line is never strength or surprise — it is **theory of mind**: a Character
can get inside your head; a Creature never can. See
[decision-making](decision-making.md#the-line-theory-of-mind) for the full treatment.
