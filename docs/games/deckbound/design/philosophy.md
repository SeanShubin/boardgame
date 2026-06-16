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
predictions and bluffs meaningful.

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
predictions and bluffs, not of reacting to a revealed move. See
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

## 10. Conceptual integrity — every rule re-derivable from its intent

The game must be **reconstructable, not memorised.** A *simple* rule is still bad if it
is **arbitrary** — its shape has no reason, so it can only be held by rote. A *complex*
rule is fine if it is **motivated** — its form follows from its intent, so anyone who
holds the intent can **re-derive** the parts they forget. Prefer a motivated rule over a
merely short one; arbitrary simplicity is the trap.

The whole design should spring from a **few intents** — these north stars, and each
mechanic's stated reason — so a reader (player, designer, or AI) who grasps the intents
can rebuild the mechanics rather than recall them. That property is **conceptual
integrity**: it is what lets a large game still be held in one head.

This is the general principle of which **#9 (every rule rides on a metaphor) is the
fiction engine.** A metaphor motivates a rule by tying it to a physical image; a rule
can also be motivated by its **consequence** (Edge is per-duel because a fight-long
meter would snowball — re-derivable, but no picture). Either way the test is one
question: *could someone who forgot this rule rebuild it from why it exists?*

- If a rule can only be stated as a bare fact — no metaphor, no consequence, no intent —
  it is **arbitrary**. Rework it until it carries its own reason, or cut it.
- "Simplifying" a rule by **severing it from its reason** is a regression, even if the
  text got shorter.

---

**Using this document.** Every other design note should trace back to one or more
of these north stars. If a proposed mechanic doesn't serve any of them — or
actively fights one (e.g. making tactics uncomputable, or balancing characters
against each other) — that is the signal to stop and reconsider, here, on purpose.
