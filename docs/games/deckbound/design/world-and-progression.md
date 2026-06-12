# Deckbound — World & Progression

Beyond the player's character, the game itself is run by decks: the world, the
scenarios, and the enemies are all cards.

## The setting

A **generic fantasy setting**, where just about anything can be explained by
magic. This gives wide latitude for new aspects and capabilities without needing
a hard sci-fi-style justification.

## Default scenario: open-world cooperative

Being scenario-based gives enormous flexibility (different environments,
objectives, and teams — the balance levers). The **default** scenario, and the
one the rest of the design assumes unless stated otherwise, is **open-world
cooperative**:

- **Open world** — non-linear exploration. The [world deck](#world-deck) governs
  where you can go and how the map opens up; reach is limited early and widens
  with progression rather than following a fixed track.
- **Cooperative** — one or more player characters work *together* against the
  world and its enemies, not against each other. The adversaries are the
  **environment** and **enemy** decks; other players are allies.

This shapes several things:

- **Hidden information is mostly player-vs-world**, not player-vs-player. The
  simultaneous-reveal exchange usually pits the cooperating players against an
  enemy or environment deck. Game-theoretic computer stand-ins are still
  available — for absent teammates, or for adversarial scenarios — but they are
  not the default opponent.
- **Co-op invites complementary, deliberately unbalanced characters.** Since
  characters need not be individually balanced, a team can cover each other's
  gaps; the challenge is tuned by the scenario, not by evening out the roster.
- **Other scenario shapes are variants**, e.g. competitive (player-vs-player,
  where the game-theoretic opponent comes to the fore), solo (one player plus
  game-theoretic stand-ins or pure environment), or fixed-objective missions
  carved out of the open world.

See [decision-making](decision-making.md) for how the player / computer
stand-in / environment agents differ.

## The decks that run the game

### World deck

The **world is a deck of cards**. It handles **locations** and the **mechanisms
to change locations** — traversal, what is reachable, and how the map opens up.
Locations are grouped into **regions**, the unit of coordinated play (see
[turn structure](turn-structure.md#regions)).

**Representing a map with cards is an open design problem** — it has to stay
practical in physical form as the map grows. Two candidate approaches:

- **Connections on the location card** — each location card lists which other
  locations are reachable from it, and by what travel mechanic. Everything about a
  place lives on its own card.
- **Transition cards** — separate cards that **group locations by the travel
  mechanic connecting them** (e.g. a *road* transition holding the locations a
  road links; a *portal* transition holding another set). Reachability becomes a
  property of which transition cards are in play, factoring shared connectivity
  out of the individual locations and into reusable travel cards.

We will need to puzzle out which of these — or a hybrid — stays manageable
physically. This is explicitly unresolved.

### Scenario decks

The game is **scenario based**. Scenarios are also run by **decks of cards**, and
**scenario decks may or may not be shuffled** — a scenario can be a fixed,
authored sequence or carry deliberate randomness.

Scenarios are the **balance lever** for a roster of deliberately unbalanced
characters. Balance is tuned by varying:

- the **environment**,
- the **objective**, and/or
- the **teams** involved.

### Enemy decks

Enemies have their **own decks**. **Portions of an enemy deck are shuffled** to
represent **simultaneous decisions** — the opponent's hidden, committed choice
that the player must read into (this is the randomness the player's never-shuffled
decks deliberately lack).

### The event deck

The game's **tension engine**. Alongside the world, scenario, and enemy decks, an
**event deck** periodically emits new **threats, mechanics, victory conditions,
and loss conditions**. It is the clock the players race: as it advances, the
situation escalates and changes shape.

- It introduces **threats** to survive and **mechanics** that alter the rules
  mid-game.
- It surfaces **victory conditions** (ways to win) and **loss conditions** (ways
  to lose) on a timer the players do not fully control.

The event deck is what keeps an open world from drifting: it imposes pace and
stakes from the outside. How often it ticks is an open question (see
[turn structure](turn-structure.md#open-questions)).

## Exploration & acquisition

The world has **many options and many places to explore**. Exploration is how a
character grows: new capabilities are acquired as **new cards** for their decks —
and **sometimes in entirely new aspects** (a whole new deck type), not just more
of what they already have.

## The strategic layer

The macro game is a **race to gain power and flexibility fast enough to handle the
[event deck](#the-event-deck)**. The uncomputable, judgment-driven half of the
game (see
[philosophy §2](philosophy.md#2-computable-tactics-uncomputable-strategy)) lives
here:

- **Survive loss conditions** as they surface.
- **Be ready to exploit victory conditions** soon after they appear — having the
  power and flexibility on hand to capitalize before the window closes.
- **Discover victory conditions through exploration**, not only from the event
  deck.

The strategic decisions are push-your-luck and opportunity cost: how far to push
into danger for power, when to consolidate, and which capabilities to chase against
a clock you only partly see.

## Progression, risk, and doom

The intended arc:

- **Limited early reach.** A character can only visit a few places at first.
- **Loss is a real concern.** Conflicts can genuinely be lost; the game is not a
  guaranteed power-fantasy ramp.
- **Doom zones exist.** Some places spell **certain doom** early on.
- **Growth unlocks the world.** Through exploration and combat the character
  acquires more abilities, eventually becoming able to meet challenges that were
  once certain death.

This is a deliberate from-doom-to-mastery curve: the same location that kills you
early should be conquerable late, and the satisfaction is in having built the
decks to do it.

## Open questions

- What **persists** between scenarios — acquired cards, the world state, injuries?
- What happens after **defeat**? The mechanism is settled — Body fails → knockout
  → **retreat** (see [form-and-defeat](form-and-defeat.md)) — but whether to add
  death, attrition, or persistence between scenarios is open.
- How are **locations and connectivity** represented as cards — connections on the
  location, transition cards, or a hybrid — so a growing map stays practical and
  physical? (See [world deck](#world-deck).)
- How is **acquisition** structured — loot tables, authored rewards, purchase,
  crafting from card sets?
- How much of a scenario is **authored vs shuffled**, and who decides per
  scenario?
