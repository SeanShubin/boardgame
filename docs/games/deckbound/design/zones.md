# Deckbound — Zones, Exhaustion & Tempo

A character's cards move through a set of **zones**, and that movement is a core
system. The zones split into two jobs: **Form** is your vitality and capabilities
(its own system — see [form-and-defeat](form-and-defeat.md)), while
**Potential → Active → Dormant** is your **tactical** layer — your choices, your
exhaustion, and how predictable you have become. The tactical layer is *never*
lethal; only Form is.

## The four zones

| Zone          | Holds                                                                 | Role                                                                                    |
| ------------- | --------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| **Form**      | baseline **capability cards** by aspect (Body, Mind, Magic, Spirit)   | what you *are*: capabilities and **health** — see [form-and-defeat](form-and-defeat.md) |
| **Potential** | the cards you can still play — your options                           | your **choices** (and exhaustion / predictability)                                      |
| **Active**    | this exchange's commits + **Lasting** effects still working           | choices in play                                                                         |
| **Dormant**   | **Fleeting** cards after they resolve, plus sealed or discarded cards | used or locked away; recover only via a mechanic                                        |

**Active ↔ Dormant** is the heart of the tactical layer: a played card is Active;
once its work is done (or it is sealed) it goes Dormant, unavailable until
recovered. Each card **defines its own starting zone**.

## Zones at every scope

Zones aren't only a per-character thing — **the same idea repeats, nested, at every
scope of the game.** A zone is just *a place where cards and effects live*, and there is
one at each level:

- **Character** — each player or creature has their own **Form / Potential / Active /
  Dormant** (above), plus a **transient round-damage pile** where a round's incoming
  hits accumulate as cards and clear at round's end (see
  [form-and-defeat](form-and-defeat.md#accumulation-is-always-cards-in-a-zone)).
- **Party** — the whole party shares a **party zone** for **collective** effects. A
  **Rally** lives here, and **every Rally boosts every other**: morale built *between*
  people, not held alone.
- **Region** — each [region](turn-structure.md#regions) has a zone holding the **local
  situation**: the creatures present, regional hazards and events, the terrain and the
  local objective. The party occupies a region, and that region's zone *is* the table
  for the encounter. Players in the **same region** share it — which is why a region is
  the unit of coordinated play.
- **World** — a global zone for the
  **[world and event decks](world-and-progression.md)**, world-level conditions, and the
  clock the players race.

The organizing rule is one line: **an effect lives in the smallest zone that contains
everything it touches.** A buff on one fighter sits in their Active; a party-wide morale
effect in the party zone; a hazard blanketing a region in the region zone; a rule that
reshapes the whole game in the world zone. **Nothing accumulates in the head — only ever
as cards in a zone.**

### In a fight, the zones nest all the way down

A combat makes the nesting concrete:

- the **combat zone** holds a **side zone** per side — *your* side zone **is** the party
  zone (where Rally lives);
- each side zone splits into a **front-line zone** and a **back-line zone**, each an
  **unordered set** of **individual zones**;
- each individual holds **Form / Potential / Active / Dormant**.

A card's place is its full address — *combat ▸ heroes ▸ back line ▸ Sefa ▸ Form* — so
**where it sits says what it affects.** "Front is *between*" is pure containment: the
front-line zone sits between the opposition and the back-line zone, which is why a
[Runner](coordination-and-interruption.md#running-the-gauntlet) must cross it. And because
a line is a **set**, there is **no order within it** — the gauntlet is the front-line
zone spending its **combined tempo**, never a sequence of guards.

This *is* the physical table: world and event decks in the center, a zone per region,
the party zone, and each player's own cards in front of them — so **where a card sits
says what it affects.**

## Lasting vs Fleeting

Every playable card is one or the other, which decides where it goes after it
resolves:

- **Lasting** — its effect persists; the card stays **Active**, working, until it
  ends or is removed.
- **Fleeting** — its effect happens once; the card then drops to **Dormant**.

## The card lifecycle (tactical layer)

1. **Setup** — each card starts in the zone it names.
2. **Commit** — choose a card (one per aspect your Form allows) and place it face
   down in **Active**.
3. **Reveal & resolve** — flip simultaneously; resolve the read, then the magnitude
   ([decision-making](decision-making.md)).
4. **Settle** — **Lasting** cards stay **Active**; **Fleeting** cards go **Dormant**.
5. **Recover** — move cards from **Dormant** back to **Potential**. Two routes,
   different costs:
   - a **Mind tactic** with a recovery ability — played *instead of* another tactic
     (a tempo cost), and needing a working Mind plus a card to play; and
   - a baseline **Form** recovery — available even when Potential is empty, but you
     **cannot defend yourself** while using it. This is the deadlock-breaker, and
     the reason recovering solo under attack is dangerous while a group can cover
     you.
6. **Seal** — an effect can force a card to **Dormant**, optionally **tagged** so
   ordinary recovery cannot rouse it until a condition is met. Seal is an effect,
   not a zone.

## Exhaustion = predictability (the central tension)

As cards leave **Potential** for **Active** and **Dormant**, your remaining options
**shrink and narrow**, and an opponent can **predict you more easily** — your
effective mixed strategy collapses toward something readable. So:

- **Predictability is a managed resource.** Aggression spends it — and unevenly:
  **aggressive, high-damage cards** (a Strike) exhaust to **Dormant**, while
  **defensive and setup cards** (Block, Evade, Scheme) **self-return** to Potential.
  So pressing the attack narrows you, while a patient defense is repeatable.
- **Recovery trades tempo for unpredictability.** Rousing Dormant cards restores
  your options (and your room to bluff) but costs the action you spend on it.

This is the player-side source of hidden-information difficulty: not counting a
depleting deck, but **husbanding your own options**. It contrasts with environment
creatures, whose decks **reshuffle and never exhaust**
([decision-making](decision-making.md#environment-creatures--hazards-non-player)).
The spend→recover cycle is also a deliberate pain/relief loop (see
[design-principles](design-principles.md#risk-loss--relief)).

> **Tactical, not lethal.** Running out of Potential cards makes you *predictable*,
> not *dead*. Survival lives entirely in **Form** — see
> [form-and-defeat](form-and-defeat.md).

## To act, you need both

A play takes **two things**: a working **capability** in your Form (the slot for
that aspect) *and* a **choice** to play from Potential. Disabling a capability
removes the slot entirely; exhausting your choices only narrows what you can do.

## Open questions

- How much does each play move — one card, or the whole committed chord?
- Do Dormant cards ever return **automatically** (a rest between conflicts), or
  only via recovery?
- Is **Potential** refilled from the never-shuffled deck between exchanges, or is
  the opening hand all you get for a conflict? (Ties to the never-shuffle principle
  in [decks-and-aspects](decks-and-aspects.md#never-shuffled).)
