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

The tactical RPS is the **four-card Clash** — Strike / Anticipate / Gather / Evade, the
**Force** a wind-up builds, and ends-on-strike. It has its own home: see
[the duel (the Clash)](the-duel.md). The decks below are how a non-player **instinct** plays
that cycle.

## Creature decision decks — the four-card Clash

A creature's decision deck is a **small pile of the four move-cards** (Strike, Anticipate,
Gather, Evade — see [the-duel](the-duel.md)). Each beat it draws one **with replacement**
(reshuffles, never depletes), and the draw *is* the choice. **The deck's composition is the
creature's mixed strategy and its personality**: a deck of `Strike, Strike, Gather` strikes
two beats in three and holds one. "Few cards" means coarse, **readable** frequencies the
player can learn and exploit.

### Design rule: readable, not optimal

A creature that played the true game-theoretic optimum (a perfect even mix) would be
**unreadable** — and unreadable defeats the whole point, which is for the player to *study its
instinct and out-predict it* (the one-way prediction of the
[theory-of-mind line](#the-line-theory-of-mind)). So creature decks are **deliberately
lopsided** — they lean on a move or two, leaving an exploitable hole. The lean is tuned
**lightly toward** the equilibrium: skewed enough to be read, balanced enough that pure
pattern-reading can't 100% steamroll it. You gain a clear edge, not immunity.

**Difficulty = deck balance (entropy).** That single dial keeps card counts tiny:

| Tier                 | Deck                        | Feel                                      |
| -------------------- | --------------------------- | ----------------------------------------- |
| **Dummy** (tutorial) | 1 move (pure)               | one lesson, fully readable, zero surprise |
| **Standard**         | 2–3 cards, a clear lean     | a real read — threatening but exploitable |
| **Elite / boss**     | 4 cards (≈1 each), balanced | hard to read — the full mind-game         |

A boss is harder via **balance plus bigger stats (Body, Power)**, *not* a bigger deck — counts
stay low at every tier. Every real foe needs **≥1 attack card**, or it can never win; a
pure-defense deck is a non-threatening dummy.

### Reading the four-card game (the theory the decks lean on)

What "out-predicting it" looks like — the player's counter to each lean:

| If the deck leans…      | …the player answers… | because                                         |
| ----------------------- | -------------------- | ----------------------------------------------- |
| **Strike** (hit-now)    | **Evade**            | dodges it *and* steals its Force                |
| **Anticipate** (lead)   | **Gather**           | holds; the lead whiffs (and you build)          |
| **Gather** (hold/build) | **Strike**           | hits the stayer; ends it before the loaded blow |
| **Evade** (move)        | **Anticipate**       | leads the dodge                                 |

Two facts the tuning leans on: **Strike answers both Gather and Anticipate** (the universal
punish for a non-attacker), and **Evade is the only steal vector**. So a Strike-heavy creature
is the most punishing to face (you farm its Force) *and* the easiest lesson — while the light
tuning means even a leaning deck mixes in its off-move often enough to clip a player who only
ever plays the single counter.

### The archetypes (concrete decks)

| Archetype             | Deck                                | Lean                         | Counter / lesson                                                                       |
| --------------------- | ----------------------------------- | ---------------------------- | -------------------------------------------------------------------------------------- |
| **Brute**             | `Gather, Gather, Strike`            | builds, then a loaded Strike | Strike its Gathers to end early, or **Evade the big Strike to steal the loaded Force** |
| **Aggressor**         | `Strike, Strike, Anticipate`        | relentless, mostly hit-now   | Evade (dodge + steal); watch the Anticipate                                            |
| **Hunter**            | `Anticipate, Anticipate, Strike`    | leads, punishes movers       | Gather (the lead whiffs); watch the Strike                                             |
| **Skirmisher**        | `Evade, Strike, Anticipate`         | slippery, picks its moment   | mixed reads; don't over-Strike (it steals)                                             |
| **Duelist** *(elite)* | `Strike, Anticipate, Gather, Evade` | balanced, near-equilibrium   | the full mind-game; win on small edges + stats                                         |

These map onto the role roster in [archetypes](archetypes.md): a **Brute** is a Juggernaut's
or Bruiser's in-duel instinct; a **Duelist** is the Trickster — the one foe that *feels*
minded. The role decides *stats and which mechanic it punishes*; the decision deck decides
*how it fights the beat*.

### Tutorial mapping — one lesson per pure deck

Each tutorial foe is a **1-move dummy** isolating a single read, in difficulty order:

| #   | Foe         | Deck                 | Teaches                                                                                    |
| --- | ----------- | -------------------- | ------------------------------------------------------------------------------------------ |
| 1   | **Post**    | `Gather`             | hit a holder — **Strike beats Gather** (and Anticipate *whiffs*: don't lead a stayer)      |
| 2   | **Leader**  | `Anticipate`         | punish a lead — **Strike beats Anticipate**, or **Gather** to be safe (never Evade a lead) |
| 3   | **Dodger**  | `Evade`              | beat a mover — **Anticipate beats Evade** (don't Strike — it dodges and steals)            |
| 4   | **Brawler** | `Strike`             | survive an aggressor — **Evade beats Strike and steals**; to win you must **trade**        |
| 5   | **Feint**   | `Strike, Anticipate` | a real read — *now or led?* — the first genuine two-way guess                              |
| 6   | **Duelist** | balanced 4           | the full mind-game — the synthesis                                                         |

1–4 isolate the four single counters (4 also introduces the **steal** and the **trade**); 5 is
the first true read; 6 is the synthesis.

### Breadth behavior — principles (pending the round loop)

Everything above is **in-duel**. A creature's **breadth** choices — whom to attack, and
whether to spend Tempo to counterattack a duel started on it — depend on the round loop, which
is **not yet designed**, so these are principles to finalize later:

- **Targeting** keeps the role-based rules ([archetypes](archetypes.md)): the front line,
  lowest-Body, least-Resolute, or a Runner that bolts for the back line.
- **Counterattack** is a *typed tendency*, not a per-beat computation: aggressive archetypes
  (Brute, Aggressor) spend Tempo to counterattack when they have it; defensive or skirmishing
  ones prefer to Focus-defend (survive) or accept a free hit.

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
