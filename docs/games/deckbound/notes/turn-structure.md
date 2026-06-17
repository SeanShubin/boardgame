# Deckbound — Turn Structure & Regions

How play flows across one or more players without exploding in length or leaving
anyone idle. This is the play loop that sits above individual
[exchanges](decision-making.md).

## Simultaneous, order-light turns

Turn order should **not matter much**. The game leans into **simultaneous turns**
so that length does not balloon with player count and nobody sits waiting. The
default cooperative experience is tuned for **30 minutes to 2 hours** regardless
of how many players are at the table.

This is a constraint on every other system: a mechanic that forces a strict global
turn order, or that makes one player's turn block everyone else's, is fighting the
design. Prefer resolutions players can perform at the same time.

## Declaring targets and guards

Within a conflict, participants **arrange into front and back lines and declare
their targets** in **any order** before resolving — coordination without a turn
order. The lines, interception, and interruption form their own system: see
[coordination & interruption](coordination-and-interruption.md).

## Regions

The world is divided into **regions** — groupings of locations from the
[world deck](world-and-progression.md#world-deck). A region is the unit of
**coordination**:

- Players **in the same region coordinate their turns** — they face the same
  situation and threats and resolve together.
- Players **in different regions act independently** — there is little reason to
  synchronize, so they don't, which stops a large group from bottlenecking on one
  shared clock.

Coordination is therefore **local, not global**, which is what lets the game scale
to more players without scaling its running time.

## Solo first, multiplayer from the start

The **solo** experience is the practical development focus, but **multiplayer is
designed in from the beginning, not bolted on later**. The cooperative structure —
shared goals, regions, the [event deck](world-and-progression.md#the-event-deck) —
is part of the core, and the solo game is simply its one-player case, not a
separate mode.

## Characters and players

- **One character cannot be run by two players** — a character is a single locus
  of decisions.
- **One player may run several characters**, and doing so is **mechanically
  indistinguishable** from one-player-per-character. Characters are the unit the
  rules speak about; who holds them does not change resolution.

## Cooperative information is open

In co-op, **all information is shared** — there are no hidden hands between allies.
The strategy *is* the discussion ("if you handle that threat, I'll focus on this
one"). Hidden information exists between the **players and the world** (simultaneous
reveal against environment creatures), never between teammates.

## Open questions

- What exactly do same-region players resolve **simultaneously** — do they reveal
  against the environment together, and how are several allies' commitments
  combined into one exchange?
- What moves a character **between regions**, and how often?
- Is there any **soft tie-breaker** within a region (e.g. by Speed), or is it
  fully simultaneous?
- How does the **event deck** tick relative to turns — per round, per region, per
  some action count? (See
  [world-and-progression](world-and-progression.md#the-event-deck).)
