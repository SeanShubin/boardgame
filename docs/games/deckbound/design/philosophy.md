# Deckbound — Design Philosophy

> The north stars. When a design decision is unclear, resolve it in favour of
> these. They are *why* the game is the way it is; the other notes are *how*.
> Keep this short and honest — if a principle stops being true, change it here on
> purpose rather than letting the design drift.

## 1. Represent and reward human intellect

The game exists to make human cleverness matter. Wherever a human mind is
represented — a player, or a computer standing in for one — choices come from
**reasoning**, not from a shuffled deck. Decks are for non-player nature, and even
those are built to be **readable**, so that beating them is the player's
achievement. See [decision-making](decision-making.md).

## 2. Computable tactics, uncomputable strategy

The **tactical** layer — a single hidden, simultaneous exchange — is deliberately
**constrained enough to be computable**: a well-defined rock-paper-scissors-plus-
magnitude game with a solvable optimal (mixed) strategy. That is what lets a
computer stand-in, or a sharp human, play it game-theoretically, and what makes
reads and bluffs meaningful.

The **strategic** layer — which conflicts to enter, when to push your luck, what
to spend and exhaust, which capabilities to chase, when to retreat — is governed
by **risk/reward, push-your-luck, and opportunity cost**, and is deliberately
**not** reducible to a clean computation. Tactics are a near-solvable skill;
strategy is judgment.

**Design consequence.** Keep the tactical exchange **small and well-defined**
(bounded options, crisp resolution) so it stays computable; push richness and
open-endedness into the **strategic / meta** layer (scenarios, acquisition,
exhaustion, routing), where it should *not* be solvable.

- If the moment-to-moment turn is becoming too complex to reason about cleanly —
  that's a smell. Simplify the tactics.
- If the overall campaign is collapsing into one solvable optimization — that's
  also a smell. Add a real risk/reward or opportunity-cost fork.

## 3. Hidden information, simultaneous reveal

Every contest is a hidden, simultaneous commitment resolved at once — a game of
reads and bluffs, not of reacting to a revealed move. See
[decision-making](decision-making.md).

## 4. Asymmetry by design, balance by scenario

Characters are **deliberately unbalanced**; fairness and challenge come from the
environment, the objective, and the team — not from evening out the roster. See
[world-and-progression](world-and-progression.md) and
[decks-and-aspects](decks-and-aspects.md).

## 5. From doom to mastery

Reach is limited early; loss is real; some places mean certain doom until the
character has grown. Progress is **earned** through exploration and combat, and
the world reacts to it. See [world-and-progression](world-and-progression.md);
for the borrowed craft behind this, see
[design-principles](design-principles.md).

## 6. Many systems from few rules

Depth comes from a **few consistent systems interacting**, not from many scripted
features. Prefer rules that generate emergent interactions over one-off content.
See [design-principles](design-principles.md#emergence--systems).

## 7. Playable by hand, cards only

No computer is required to *run* the game; cards and shuffling only; resources are
represented as cards. A computer is an optional convenience, never a dependency.
See [constraints](constraints.md).

## 8. Deliberate decks; predictability is a resource

The player's own decks are **never shuffled** — order is intent, not luck — and a
player's unpredictability is a **managed resource** that erodes as cards exhaust
and is restored only at a tempo cost. See [zones](zones.md) and
[decks-and-aspects](decks-and-aspects.md#never-shuffled).

## 9. Every rule rides on a solid metaphor

The game must be **remembered without a rulebook**. So each mechanic rests on a clear
physical image: you block a path by *keeping pace* with someone; running past a guard
*gets you hit*; more guards *cover more angles*; aggression *spends you*. If a rule
can't be stated as an intuitive picture, it is too abstract to hold at the table —
rework the rule, or the metaphor, until it can. Mechanics serve the metaphor, not the
other way around.

---

**Using this document.** Every other design note should trace back to one or more
of these north stars. If a proposed mechanic doesn't serve any of them — or
actively fights one (e.g. making tactics uncomputable, or balancing characters
against each other) — that is the signal to stop and reconsider, here, on purpose.
