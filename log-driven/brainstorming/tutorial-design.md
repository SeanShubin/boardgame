# Deckbound — Tutorial design (DEFERRED): forcing-function lessons

> **Status: DEFERRED — capture only, do not build yet.** The designer is still vetting
> mechanics via *designer-validation* scenarios (see the `card-combat-round-*.md` logs).
> This document records the agreed shape of the eventual tutorial series so it isn't lost.
> Ratified 2026-06-25.

## The goal

A series of tutorials that demonstrate **every mechanic** with a scenario that is
**impossible to win without understanding that mechanic.**

## The decision — a layered hybrid (reject both extremes)

The "impossible to win without X" constraint is a **credit-assignment** requirement, and
it decides the structure:

- **NOT one scenario for everything.** A grand scenario fails credit assignment: a win
  doesn't tell the player *which* insight mattered; a loss doesn't tell them *which*
  mechanic they missed. To make a scenario unwinnable-without-`X` you must *remove the
  other ways to win* — which strips it back toward `X`. Aggregation fights the forcing
  constraint.
- **NOT arbitrary "few mechanics each."** The right discriminator isn't "few" — it's
  "one **new**."
- **BACKBONE: one tutorial per mechanic** — each a puzzle **unwinnable without that
  mechanic**, ordered so each introduces **exactly one *new* mechanic** on top of
  already-taught prerequisites. A later puzzle may *use* earlier mechanics freely; it just
  may not *introduce* more than one new forced node.
- **Dependency graph / tech tree.** Higher-order mechanics have prerequisites (pre-empt
  needs breach + instant-in-both + Tempo scarcity; weakest-link-evade needs groups), so
  the series is a **topological sort** of the mechanic dependency graph.
- **CAPSTONES.** A handful of integration scenarios per chapter combine that chapter's
  mechanics — **reinforcement / assessment, not teaching primitives.** This is the only
  place "few scenarios, many mechanics" earns its keep.

## The link to balance — the same artifact

Each forcing puzzle **is** the **executable necessity test**
([computability-and-balance.md §6.1](../../docs/games/deckbound/computability-and-balance.md)):
naive line provably loses, keyed line wins. So the tutorial suite and the
necessity-audit suite are **one artifact, read two ways** (teach vs. test). Building the
tutorials later also yields the necessity **regression suite** *and* a **design audit** — a
mechanic for which no forcing puzzle can be constructed is **fiat or redundant** (the
removal test, made runnable).

## Distinct from designer-verification logs

The dense `card-combat-round-*.md` logs are **designer-verification** artifacts (many
mechanics at once, built so the *designer* can spot lost nuance). They are **not** player
tutorials and shouldn't be forced into that mold — they can **seed capstones**, but the
tutorial spine wants fresh, minimal, single-purpose puzzles.

## Next step when resumed

Draft the **mechanic dependency graph** (the topological order), and for each node a
one-line **"naive line that loses / keyed line that wins"** sketch. That single pass
yields, at once: the tutorial order, the forcing puzzles, and the **audit checklist** of
mechanics that cannot yet be forced.
