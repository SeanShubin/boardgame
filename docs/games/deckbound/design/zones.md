# Deckbound — Zones, Exhaustion & Tempo

A character's cards move through a set of **zones**, and that movement is a core
system. The zones split into two jobs: **Form** is your vitality and capabilities
(its own system — see [form-and-defeat](form-and-defeat.md)), while
**Potential → Active → Dormant** is your **tactical** layer — your choices, your
exhaustion, and how predictable you have become. The tactical layer is *never*
lethal; only Form is.

## The four zones

| Zone | Holds | Role |
| --- | --- | --- |
| **Form** | baseline **capability cards** by aspect (Body, Mind, Magic, Spirit) | what you *are*: capabilities and **health** — see [form-and-defeat](form-and-defeat.md) |
| **Potential** | the cards you can still play — your options | your **choices** (and exhaustion / predictability) |
| **Active** | this exchange's commits + **Lasting** effects still working | choices in play |
| **Dormant** | **Fleeting** cards after they resolve, plus sealed or discarded cards | used or locked away; recover only via a mechanic |

**Active ↔ Dormant** is the heart of the tactical layer: a played card is Active;
once its work is done (or it is sealed) it goes Dormant, unavailable until
recovered. Each card **defines its own starting zone**.

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

- **Predictability is a managed resource.** Aggression spends it.
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
