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
that the player must predict (this is the randomness the player's never-shuffled
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
[philosophy §2](../canon/1-charter.md#2-computable-tactics-uncomputable-strategy)) lives
here:

- **Survive loss conditions** as they surface.
- **Be ready to exploit victory conditions** soon after they appear — having the
  power and flexibility on hand to capitalize before the window closes.
- **Discover victory conditions through exploration**, not only from the event
  deck.

The strategic decisions are push-your-luck and opportunity cost: how far to push
into danger for power, when to consolidate, and which capabilities to chase against
a clock you only partly see.

## Power, scaling, and the balance budget

Numbers scale **linearly and without a cap**: higher always means more, and a
**substantial** number gap is a **god-like** disparity — the bigger side simply
dominates. That single rule yields the cleanest knob in the design — a **balance
budget**:

> **The world hands out a roughly constant amount of power, divided among the party,
> and challenges are tuned to the party's *total*.** 600 power in one character, or
> 600 split across six, is the **same difficulty** — and a **completely different
> game**.

- **Solo concentrates.** One character must raise **every** dimension toward god-level
  — including **Speed**, so a lone hero can engage a whole swarm at once
  ([engagement bandwidth](coordination-and-interruption.md#speed-is-the-currency-of-engagement))
  rather than be mobbed. The solo experience is **raw dominance** across the board.
- **Co-op distributes.** Six specialists each hold a slice and must **combine** —
  many bodies' coverage instead of one god's Speed, focus-fire to aggregate damage.
  The co-op experience is **coordination**: the same 600, but pooled in play rather
  than owned by one.

The two answers to one swarm are the two faces of the budget: **one god-Speed body, or
many ordinary bodies**. Balance falls out of the total; the *distribution* is what
makes solo and co-op feel like different games. (This is the
[asymmetry pillar](../canon/1-charter.md) made quantitative.)

### God vs party — depth for breadth at equal budget

Spend the same fixed budget *B* as **one god** (all *B*) or a **party of N** (*B/N*
each) and you buy **roughly equal raw throughput in an opposite shape**. Stats **add
linearly under concentration** — the god's Speed is the sum of the party's Speeds, its
Mind the sum of their Minds, and so on — so neither shape is inherently stronger; they
fail and win against *different* things.

- **Party = wide & simultaneous.** Division of labor across bodies **in one round** —
  one Holds the wall, one nukes from the back, one Runs the flank. Redundant against
  debuffs and seals (lose one capability, the others still act), but it **cannot
  concentrate force** and is **fragile to losing a specialist**.
- **God = deep & sequential.** It cannot split, so it **mode-switches across rounds**
  (an Attack round is the blade clearing the crowd it can predict; a Hold round is the
  wall absorbing and dragging) and **concentrates force to cross thresholds no single
  party member could**. But it is **one body, one place, one Mind to seal**.
- **Signature counters.** The god's is the **gank** — swarm past its Mind so the
  overflow free-hits, or seal its one Mind. The party's is a **threshold or an AoE** — a
  Juggernaut no member can crack, or one blast catching the clustered party.

Capability for capability at equal budget:

| Capability                           | Winner              | Why                                                                                   |
| ------------------------------------ | ------------------- | ------------------------------------------------------------------------------------- |
| **Strike many**                      | ≈ even              | totals match; the god is more flexible about who                                      |
| **Negate many** (predict blows)      | ≈ even total        | same total focus; the **per-body cap** differs                                        |
| **Absorb** (toughness)               | differs *by design* | same total HP; god's one pool is **chip-immune**, party's N pools risk **focus-fire** |
| **Be in many places**                | **party**           | one god has one stance per round                                                      |
| **Concentrate to cross a threshold** | **god**             | linear sum clears a bar no single member reaches                                      |
| **Survive a seal**                   | **party**           | the god is a single point of failure                                                  |
| **Coordination tax**                 | **god**             | no synergy to set up; nothing to mis-time                                             |

#### Linearity invariants

For the equivalence above to hold, these must stay true:

1. **Concentration is linear** — no big-number bonus and no concentration penalty;
   summing stats into one body neither over- nor under-pays.
2. **Bandwidth caps are real and per-body** — the Mind **focus pool** is per-Actor, and
   a fast attacker drains a defender's focus faster (each prediction costs the attacker's
   Speed). This is what keeps "negate many" even in *total* but capped per body.
3. **Thresholds must exist** — Juggernaut armor, drag ≥ Runner Speed, the Power needed to
   drop a target. Thresholds are the **god's unique payoff**: the one place linear
   concentration buys something a divided budget cannot.
4. **HP concentration is flat.** A god's one pool and a party's N pools carry the **same
   total** effective HP; the *shape* difference is intended texture — a god is
   **chip-immune** (each hit meets its toughness once, so swarms of weak hits do little;
   the **gank** counters it by free-hitting *past* the duel, not through toughness), while
   a party risks **focus-fire dropping a whole capability** (countered by protecting
   keystone members). Toughness stays **flat under concentration**; a **concentration
   tax** is held in reserve as a tuning knob, applied only if playtest shows the god too
   chip-immune.

## The shape of progression — the rule of three

At any point the [world deck](#world-deck) offers, on average, **three places to go**,
each **balanced to your current power** and each holding **different cards** to acquire.
"Balanced" is precise:

- **You will wipe if you don't play strategically** — a balanced fight is a *real*
  fight.
- **But barring exceptionally bad luck you prevail** — typically with **at least one
  character still standing when the last enemy falls**, which is exactly the condition
  that **resets the whole party to full**
  ([recovery](form-and-defeat.md#knockout-recovery-and-the-wipe)). Survival has a
  *margin*, not a guarantee.

Beyond the three, there is always the option to **press into harder ground** — success
no longer certain, but the **reward proportional**: the push-your-luck accelerator for
players willing to gamble for faster power.

Strategic depth differs by mode:

- **Solo** — at least **three viable routes** to power (different orders, places, and
  builds all reach strength), so optimizing your path is a skill, not a solved line.
- **Co-op** — a standing choice between **splitting up** (cover more
  [regions](turn-structure.md#regions) at once, in parallel) and **staying together**
  (concentrate to take on situations harder than any one of you could). Both viable;
  the call is the strategy.

## Inside a location — levels and the cleared marker

A location need not be one fight. A natural shape: it holds **several levels** (say
five), each harder than the last, and a single **"highest cleared" marker** records how
far you have gotten. This is the press-on accelerator made concrete — and **card-cheap**:

- **Grind or skip.** Attempt the level **balanced to your power** for a safe gain, or
  **skip ahead** to a harder one for more — success isn't certain, but the **reward is
  cumulative**: clearing level *N* hands you the rewards of levels 1..*N* you hadn't yet
  taken (the location card lists which **reward cards** each rung grants; on a clear you
  take the stack up to your highest). **Fail** the gamble and you gain nothing — but the
  marker holds at your last cleared rung, so you lose the *attempt*, not your *progress*.
  A whole **location is one of the three places**; the *level* is its internal risk dial.
- **The marker is the state.** One marker per location ("cleared to *N*") is the *only*
  persistent bookkeeping — no per-level tracking, no double-dipping (clear level 5 later
  and you collect only 5). It also powers the **doom-to-mastery** return: a location that
  was certain death is one you come back to and push *deeper* once strong.
- **Levels are the power tags.** A location's ladder *is* the
  [power-tagging](#the-shape-of-progression--the-rule-of-three) the world deck needs — the
  rung matching your power is the "balanced" node; higher rungs are the press-on. Three
  locations thus present nine graded options through **three cards plus three markers**.

Card economy: the location card carries the **level ladder and its reward schedule**; the
fights themselves are **scaled draws** from the shared enemy / scenario decks, not bespoke
per-level cards; a marker tracks progress. State stays tiny.

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
- **HP concentration tax (tuning knob, not an open design question).** The *rule* is
  settled — toughness stays **flat**, so one pool vs N pools have the **same total** HP and
  the shape-asymmetry (god chip-immune; party focus-fire-vulnerable) is **intended**, each
  with its own counter (see [god vs party](#god-vs-party--depth-for-breadth-at-equal-budget)).
  The only open *number* is whether playtest forces a mild concentration tax to rein in a
  too-chip-immune god.
