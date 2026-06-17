# Deckbound — Zones, Exhaustion & Tempo

A character's cards move through a small set of **zones**, and that movement is a
core system. The metaphor is one line:

> **Form is what you are, your hand is your choices, and Active is your choices
> taking effect.**

These three zones split into two jobs: **Form** is your vitality and capabilities
(its own system — see [form-and-defeat](form-and-defeat.md)), while
**hand (Potential) → Active** is your **tactical** layer — your choices, your
exhaustion, and how predictable you have become. The tactical layer is *never*
lethal; only Form is.

## The three zones

| Zone          | Holds                                                          | Role                                                                                      |
| ------------- | -------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| **Form**      | baseline **capability cards** by aspect (Body, Mind, Spirit)   | **what you are**: capabilities and **health** — see [form-and-defeat](form-and-defeat.md) |
| **Potential** | the cards in your **hand** — **your choices**, not yet made    | your options (and exhaustion / predictability)                                            |
| **Active**    | your **choices taking effect** — played cards that are working | choices in play                                                                           |

Each card **defines its own starting zone**.

## Face up and face down — a card's state within a zone

There is no separate "spent" zone. A card on the table instead has a **facing**, and
the rule is one line: **face up = currently in effect; face down = not.**

- **Face up** — present and **working**: a Form capability you have, or a played card
  doing its job in Active.
- **Face down** — **not in effect right now**: still on the table, but inert. Either
  it has **not taken effect yet** (a hidden commitment, before the reveal) or it is
  **spent / neutralized** (its work is done, or an effect shut it off). Only a reveal
  or a recovery turns it back up.

So the same facing reads consistently across a card's whole life: committed-but-hidden
→ face down (not yet in effect); revealed → face up (in effect); spent or neutralized
→ face down again. And it does double duty across zones:

- in **Form**, a card turned face down is **lost health / a disabled capability**
  (the lethal layer — see [form-and-defeat](form-and-defeat.md));
- in **Active**, a card turned face down is a **spent or neutralized tactic** (never
  lethal).

## Two kinds of played card — Actions and Stances

The cards you commit from your hand come in two kinds (distinct from your **Form**
cards — identity, Vitality, Health, traits — which are your standing sheet, never
"played"):

- **Action cards** — the things you *do*: a physical strike (Bash), an elemental
  delivery (Firestorm — heat damage met by Armor), a **Spirit** effect (Rally, Dread).
  Firestorm isn't special — it's just a **Body** Action whose damage-type happens to be
  elemental; range and area are delivery properties, not a separate category. An Action
  carries **magnitude, type, and effect**; it enters
  **Active** face up and then lives by the rules below — **Lasting** (stays working) or
  **Fleeting** (turned face down), with defensive ones returning to hand. *This is the
  magnitude layer.*
- **Stance cards** — your committed move in the duel rock-paper-scissors this beat
  (Marshal / Unleash / Overwhelm / Parry). A Stance sets your **position** in the clash,
  not a lingering effect. It is **transient**: committed face down, revealed, resolved —
  then **cleared** (returned to hand, or turned face down to recover). **A Stance never
  remains in Active as a standing effect.** Players **choose** their Stance; creatures
  **draw** one from a distribution. *This is the categorical "who gets the upper hand"
  layer.*

The two combine in one beat: the **Stance** decides the categorical outcome (who wins
the exchange), and the **Action** supplies the magnitude (how much, what type). See
[decision-making](decision-making.md#the-core-exchange) — *resolve the stances, then
resolve magnitude.*

## Zones at every scope

Zones aren't only a per-character thing — **the same idea repeats, nested, at every
scope of the game.** A zone is just *a place where cards and effects live*, and there is
one at each level:

- **Character** — each player or creature has their own **Form / Potential / Active**
  (above), plus a **transient round-damage pile** where a round's incoming
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
- each individual holds **Form / Potential / Active** (cards face up or face down).

A card's place is its full address — *combat ▸ heroes ▸ back line ▸ Sefa ▸ Form* — so
**where it sits says what it affects.** "Front is *between*" is pure containment: the
front-line zone sits between the opposition and the back-line zone, which is why a
[Runner](coordination-and-interruption.md#running-the-gauntlet) must cross it. And because
a line is a **set**, there is **no order within it** — the gauntlet is the front-line
zone spending its **combined tempo**, never a sequence of guards.

This *is* the physical table: world and event decks in the center, a zone per region,
the party zone, and each player's own cards in front of them — so **where a card sits
says what it affects.**

## Lasting vs Fleeting — and the two ways out of Active

Every playable card is one or the other, which decides what happens after it
resolves:

- **Lasting** — its effect persists; the card stays **Active**, face up and working,
  until it ends or is removed.
- **Fleeting** — its effect happens once; the card is then **turned face down**
  (spent), and needs recovery before it can be played again.

A card can also leave Active by being **returned to your hand** instead of turned
face down. That difference *is* the central tension:

- **Back to hand (Potential)** — choosable again immediately, no recovery needed.
  This is how **defensive / setup** Stance cards (Block, Evade, Scheme) repeat.
- **Turned face down** — inert until something turns it back up. This is how
  **aggressive / one-shot** cards (a Strike, a Fleeting action) exhaust.

So pressing the attack turns cards face down and **narrows you**, while a patient
defense returns to hand and **repeats** — see
[exhaustion](#exhaustion--predictability-the-central-tension).

## The card lifecycle (tactical layer)

1. **Setup** — each card starts in the zone it names.
2. **Commit** — choose a card (one per aspect your Form allows) and place it **face
   down** in **Active**: committed, but not yet in effect.
3. **Reveal & resolve** — turn the commits **face up** simultaneously; resolve the
   stances, then the magnitude ([decision-making](decision-making.md)).
4. **Settle** — **Lasting** cards stay **Active**, face up; **Fleeting** cards are
   **turned face down** (spent); defensive / setup Stance cards instead **return to hand**.
5. **Recover** — turn a **face-down** Active card back up (or return it to hand).
   Two routes, different costs:
   - a **Mind tactic** with a recovery ability — played *instead of* another tactic
     (a tempo cost), and needing a working Mind plus a card to play; and
   - a baseline **Form** recovery — available even when your hand is empty, but you
     **cannot defend yourself** while using it. This is the deadlock-breaker, and
     the reason recovering solo under attack is dangerous while a group can cover
     you.
6. **Seal** — an effect can force a card **face down** and **tag** it, so ordinary
   recovery cannot turn it back up until a condition is met. Seal is an effect, not a
   zone.

## Exhaustion = predictability (the central tension)

As cards leave your hand and end up **turned face down** in Active, your remaining
options **shrink and narrow**, and an opponent can **predict you more easily** — your
effective mixed strategy collapses toward something predictable. So:

- **Predictability is a managed resource.** Aggression spends it — and unevenly:
  **aggressive, high-damage cards** (a Strike) exhaust **face down**, while
  **defensive and setup cards** (Block, Evade, Scheme) **return to hand**. So pressing
  the attack narrows you, while a patient defense is repeatable.
- **Recovery trades tempo for unpredictability.** Turning face-down cards back up
  restores your options (and your room to bluff) but costs the action you spend on it.

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
- Do face-down cards ever turn back up **automatically** (a rest between conflicts),
  or only via recovery?
- Is **Potential** refilled from the never-shuffled deck between exchanges, or is
  the opening hand all you get for a conflict? (Ties to the never-shuffle principle
  in [decks-and-aspects](decks-and-aspects.md#never-shuffled).)
