# Deckbound — Physical Representation (the sample combat as cards)

A pressure test of the **cards-only** pillar: for the [sample combat](sample-round.md),
exactly **what is printed on each card, which zone it starts in, and when it moves.** If
this is legible by hand at a table, the constraint holds.

## The audit — every element, classified

The real test of the cards-only pillar: **every piece of game state has a defined
home.** Each row below says **how** it's represented, **when** it's specified, and
**where it lives** — on the table, or a head-number that is *re-derivable from the
table*. A row that can't be filled cleanly is a hole; as of now the only blanks are
**numeric knobs**, not structural ones.

*Specified when:* **authored** (built into the character / scenario) · **form-up** ·
**declare** · **resolve** · **round-end** · **acquisition** (between combats).

### Character — Form & stats

| Element                                                            | How represented                                                           | When                                                      | Lives |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------- | --------------------------------------------------------- | ----- |
| **Body health** (outer pool)                                       | generic **Health cards**, turn face up ↔ face down (in Form)              | authored (count); flips at **resolve**                    | table |
| **Resolve / Mind** (inner thresholds)                              | a standing **capacity value** — *no* Health stack; effects raise/lower it | Fear/Confusion vs it at **resolve**; clears **round-end** | table |
| **Stats** (Speed, Power, Precision, Spr)                           | numbers on the **identity card**                                          | authored; changed by **acquisition**                      | table |
| **Vitality (count + toughness) / armor rules**                     | **rules cards** (the health one is the **Vitality card**)                 | authored                                                  | table |
| **Traits & equipment** (Plate, Shield, weapons, Resolute, Coward…) | **cards** in Form                                                         | authored / acquisition                                    | table |
| **Keystone** (which aspect is lethal)                              | named by a Form card                                                      | authored                                                  | table |
| **Knockout** (keystone at 0)                                       | the flipped cards + a down marker                                         | resolve                                                   | table |

### Positioning & stance

| Element                    | How                                   | When                          | Lives                              |
| -------------------------- | ------------------------------------- | ----------------------------- | ---------------------------------- |
| **Front / back line**      | **table position**                    | form-up (fixed for the round) | table                              |
| **Stance** (Attack / Hold) | a **marker** by the card              | declare                       | table                              |
| **Guard / Runner** (roles) | *derived* from line + stance + target | declare                       | table (read off position + marker) |
| **Target**                 | a **declaration** (pointer)           | declare                       | table                              |

### The stance (Mind)

| Element                                     | How                                                             | When                                        | Lives                       |
| ------------------------------------------- | --------------------------------------------------------------- | ------------------------------------------- | --------------------------- |
| **Stances** (Strike/Block/Evade/Scheme)     | **cards**, hand → Active → face down (defensive return to hand) | committed **declare**, revealed **resolve** | table                       |
| **Hidden commitment**                       | a **face-down** card                                            | declare                                     | table                       |
| **Damage type** (blunt/sharp/heat/fear…)    | printed on the weapon / spell card                              | authored                                    | table                       |
| **Momentum** (banked Power/Speed/Precision) | **cards** moved to Active                                       | resolve                                     | table                       |
| **Stance outcome**                          | *derived* from the two revealed stances                         | resolve                                     | head (= the revealed cards) |

### Tempo & the gauntlet

| Element                          | How                                                      | When                               | Lives                                          |
| -------------------------------- | -------------------------------------------------------- | ---------------------------------- | ---------------------------------------------- |
| **Tempo** (per-round Speed pool) | a **derivable number**                                   | spent at resolve; resets round-end | **head** = Speed − engagements visible in play |
| **First-strike order**           | *derived* — compare leftover tempo                       | resolve                            | head (= two tempo numbers)                     |
| **Pre-emption**                  | *derived* — a lethal first-strike, or a **stagger** card | resolve                            | head (who's felled before they swing)          |
| **Gauntlet drag pool**           | a **derivable number** = sum of Guards' Speeds           | resolve                            | head (= sum of visible Guard Speeds)           |
| **Overextended / Exposed**       | a **marker** (tempo gone negative)                       | resolve; clears round-end          | table                                          |

### Actions, effects, damage

| Element                                                    | How                                                             | When                                   | Lives                                  |
| ---------------------------------------------------------- | --------------------------------------------------------------- | -------------------------------------- | -------------------------------------- |
| **Action cards** (Bash, Firestorm, Rally, Dread, Riposte…) | **cards**, hand → Active → face down (or → hand / → party zone) | committed declare, resolved at resolve | table                                  |
| **Lasting vs Fleeting**                                    | printed on the card                                             | authored                               | table                                  |
| **Collective effects** (Rally)                             | **cards** in the **party zone**                                 | resolve                                | table                                  |
| **Incoming damage** (accumulation)                         | **tokens** in a transient round-damage pile                     | accrues at resolve; clears round-end   | table                                  |
| **Damage magnitude of a hit**                              | *derived* — Power/Precision/type vs armor + toughness           | resolve                                | head → result flips cards on the table |

### Creatures

| Element         | How                                                          | When                                    | Lives                           |
| --------------- | ------------------------------------------------------------ | --------------------------------------- | ------------------------------- |
| **Stat-block**  | **one card**                                                 | authored                                | table                           |
| **Behavior**    | a printed **line** (most), or a shuffled **deck** (bluffers) | authored; deck draw revealed at resolve | table                           |
| **Health**      | a derivable number (`Body N · T`) or cards                   | authored; drops at resolve              | table / head                    |
| **Swarm count** | a **derivable number** (or a pack-token per N)               | drops at resolve                        | head / table (= bodies removed) |

### Zones & the world (between combats)

| Element                                                         | How                              | When                        | Lives |
| --------------------------------------------------------------- | -------------------------------- | --------------------------- | ----- |
| **The zones** (combat ▸ side ▸ front/back ▸ individual ▸ F/P/A) | the **table layout** itself      | form-up                     | table |
| **World / event / scenario decks**                              | **decks of cards**               | authored; tick on **event** | table |
| **Level-cleared marker**                                        | one **marker** per location      | acquisition                 | table |
| **Acquired cards / artifacts**                                  | **cards** added to Form / decks  | acquisition                 | table |
| **Post-combat reset** (win → full)                              | turn all face-down cards back up | combat end                  | table |

### Completeness check

Every **structural** question has an answer: state is either a **card / marker / token on
the table**, or a **single number re-derivable from the table** (tempo, drag,
first-strike, stagger, a swarm count). The remaining blanks are **numeric knobs**, not
representation holes:

- the **gate aggregate** (sum of Guard Speeds, or gentler);
- the **damage formula** (how Power, Precision, and multipliers combine);
- **stance-cycle payoffs** (how much momentum a win banks);
- whether **non-combat actions** (a cast, a Scheme) draw from the tempo pool.

Those are tuning numbers for playtest — the **representation is closed.**

## Components

Three physical things:

1. **Cards** — identity, capabilities (health), stances, actions, traits, rules. Each
   lives in a **zone**.
2. **Tokens** — generic, reusable counters for the two things that *accumulate*:
   **tempo** (Speed) and **damage**. (Forced by the math — see findings.)
3. **The table layout** — front line / back line, and the shared **party zone** (which
   sits inside the current **region**'s zone, inside the **world** zone — zones nest at
   [every scope](zones.md#zones-at-every-scope)). Positions are just *where* you place
   cards; no card needed.

## The representation principles

A handful of rules keep the whole game legible as cards **at any power level**:

1. **Zones at every scope.** Character, party, region, and world each have zones; an
   effect lives in the **smallest zone that contains everything it touches**. *Where a
   card sits says what it affects.* (See [zones](zones.md#zones-at-every-scope).)
2. **Scale the number, not the pile.** Toughness, Power, Speed grow via the value on a
   **rules card** or an
   **[artifact](cards-and-customization.md#artifacts--acquired-modifiers-that-scale-the-numbers)**
   — so card count stays roughly constant from level 1 to god-tier.
3. **Every card-state change is meaningful.** Coarse granularity is the *point*: we
   don't represent 98 vs 99 — a flip always *means* something.
4. **State must be *derivable from the table* — not necessarily *on* it.** Multi-source
   accumulation against a threshold (incoming **damage**) lives as a **pile of cards**, so
   the check stays visual. But a single value you spend *down* — your **tempo** — can be
   a running number **in your head**, because it's **reconstructable any time**: your
   Speed is on your card, and what you spent it on (who you dove past, the attacks you
   made) is visible in play. The table is the source of truth; never hold state you
   couldn't re-derive from it.
5. **Transient state is a card you can remove** — utility / effect decks for buffs and
   temporary effects, drawn when they apply and discarded when they expire.

## Zones

- **Form** — your capabilities, health, traits, stats, rules cards: the "character
  sheet made of cards," face-up. Health turns face up ↔ face down here.
- **Potential** — stances and actions you *could* play this round.
- **Active** — what's in play now (a played stance/action, banked momentum). Spent or
  sealed cards are **turned face down in place**.
- **Party zone** (shared) — collective effects (Rally) live here.
- **Round-damage zone** (per aspect, transient) — incoming damage tokens pile here; the
  rules card reads the pile; partials clear at round's end.
- **Tempo pool** (per combatant, transient) — Speed tokens, spent as you act, refilled
  each round.

## A character as cards — Aldric (Knight), the template

**In Form** (the always-there sheet):

| Card face                                                                                                     | Kind                               |
| ------------------------------------------------------------------------------------------------------------- | ---------------------------------- |
| **Aldric — Knight** · Spd 4 · Pow 4 · Pre 2 · *keystone: Body*                                                | identity + stats                   |
| **Body** — 8 **Health cards** *(generic tokens)*                                                              | health — turned face down when hit |
| **Vitality (Body):** *8 Health cards, toughness 2 — 2 damage turns 1 face down. Partial clears at round end.* | Vitality card                      |
| **Plate:** *armor — physical −3 (blunt −1), heat −0; per source, never cumulative*                            | trait                              |
| **Shield:** *you may play Block and Bash*                                                                     | trait                              |
| **Resolute:** *fearless — fear cannot break you*                                                              | trait (Resolve can't be broken)    |

**In Potential** (playable this round): **Strike · Block · Evade · Scheme** (the four
stances) + **Bash** *(Strike, blunt; on hit, **stagger** — land first and the target loses
its action)*.

**At round start:** 4 **tempo tokens** (= Speed). **Stance:** an Attack/Hold marker.

→ ~6 Form cards + **8 Body Health cards** + 5 Potential cards + tempo. The **8 Health
cards are the bulk** — flag for later.

## The others (distinctive cards only; same skeleton)

- **Vera (Duelist)** — Spd 5 · Pow 3 · Pre 4, Body ×4, Resolve 2. Potential:
  four stances + **Blade** *(Strike, sharp)* + **Riposte** *(combo: Evade → counter →
  reposition; on the Evade, bank +Speed)*.
- **Sefa (Mage)** — Spd 2 · Power 5, Body ×3, **fearful** (Resolve 1). Potential:
  stances + **Firestorm** *(heat, AoE, ranged enemy front)* + **Frostbite** *(cold,
  slows)*.
- **Bram (Warden)** — Spd 3 · Spr 5, Body ×5, Resolve 4. Potential: stances + **Rally**
  *(→ party zone: +4 Resolve to each ally; every Rally boosts every other)* + **Dread**
  *(Spirit attack — Fear vs Resolve)* + **Steel** *(steady your nerve — clear accumulated Fear)*.
- **Ironclad** — Spd 2 · Pow 6, Body ×8 (T3), **plate** *(sharp −4, blunt −3, heat −0)*,
  and a small **behavior deck** *(Strike / Feint — it bluffs)*: the one foe with a hidden
  stance.
- **Stalker** — Spd 6 · Pow 3, Body ×6, **line:** *run the lowest-Body.*
- **Howler** — Spd 4, **Fear 5**, Body ×4, **line:** *howl at the least-resolute;
  fearless → cower.* (Corporeal — its threat is armor-proof Fear, not untouchability.)
- **Husk ×6** — Spd 3 · Pow 1, Body ×1, **swarm line:** *shamble at the front* — one
  card + a count.
- **Shared utility:** one set of **toughness / armor rules cards** referenced
  by all; the **party zone** (empty until a Rally lands).

## Packing a creature onto one card

A hero fans out across many cards because *the player drives him* — identity, health,
the four stances, actions, traits, artifacts. A **creature needs almost none of that**, so
its whole presence fits on **one stat-block card**:

> **Stalker** · Spd 6 · Pow 3 · **Body 6 (T1)** · *run the lowest-Body; alone → flee*

What lets it pack so tight:

- **No stances.** With no theory of mind it has no Strike/Block/Evade/Scheme pool — it
  acts off its printed behavior, not a hand of tactics.
- **Health is a number, not a heap** — `Body 6 (T1)` is a re-derivable count (the
  [toughness rule](form-and-defeat.md#example--a-vitality-card) plus the
  derivable-number rule do the work); no tokens.
- **Traits are compact tags** — *armor −3*, *plate*, and the like.

**Line or deck?** The one real choice is how a creature *decides*:

- **A behavior *line*** (printed on the card) when its behavior is **deterministic or
  conditional on visible state** — *press the front; if wounded, Smash.* No hidden
  choice → nothing to shuffle, nothing to predict. **Most creatures.**
- **A behavior *deck*** (face-down, shuffled) only when it makes a **hidden, simultaneous
  choice you must predict** — it enters the [stance game](decision-making.md) as an opponent
  that might bluff. The deck is its **mixed strategy made physical** (the randomness your
  never-shuffled deck deliberately lacks). Reserved for the sophisticated — a rival
  duelist, a boss.

The sample **warband shows the whole spread** — lines, one deck, and a swarm:

| Creature     | The one card                                                                  | Deck?                                                                             |
| ------------ | ----------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| **Husk ×6**  | Spd 3 · Pow 1 · Body 1 (T1) · *shamble at the front*                          | no — one design + a count (a [swarm](#swarms--a-hundred-as-one-card-and-a-count)) |
| **Stalker**  | Spd 6 · Pow 3 · Body 6 (T1) · *run the lowest-Body*                           | no — a line                                                                       |
| **Howler**   | Spd 4 · Fear 5 · Body 4 (T1) · *howl at the least-resolute; fearless → cower* | no — a line                                                                       |
| **Ironclad** | Spd 2 · Pow 6 · Body 8 (T3) · plate (heat −0) · *Strike / Feint*              | **yes** — it bluffs, so it earns a shuffled deck                                  |

So the warband is **four card designs** (Husks shared as a swarm), each a single card —
**three lines and one deck** (the bluffing Ironclad). That is what keeps the table
playable even when enemies outnumber heroes.

## Swarms — a hundred as one card and a count

A creature with **no independent state** — same stats, same behavior, no individual
wound, buff, or position worth tracking — doesn't need its own card at all. A whole
swarm is **one archetype card + a count**:

> **Goblin** · Spd 4 · Pow 1 · Body 1 · *swarm: press the front* &nbsp;&nbsp; **×100**

Everything scales from the count, read off that one card:

- **Health *is* the count.** The swarm's hit points are how many remain; damage removes
  **bodies**, not pips. Track the count like [tempo](speed-and-tempo.md) — a single
  number you keep in your head and re-derive — or, for a big horde, a **pack token per
  ten**, so every lost pack is a meaningful change (we still don't care about goblin
  #87).
- **Output scales with the count.** A hundred goblins at Pow 1 bring ~Pow 100 to bear —
  the [balance budget](world-and-progression.md#power-scaling-and-the-balance-budget) as
  a single multiplier you tune by setting the number.
- **One behavior drives them all** — the archetype's line; and if the swarm makes a
  hidden choice, it's **one shared draw**, not a hundred.

This is exactly why **AoE matters against a horde and single-target doesn't**: a
Firestorm deletes a whole **pack** at once, while a lone strike kills *one of a hundred*.
Bring fire to a swarm, a blade to a champion.

**Promotion.** A unit stays in the swarm only while it's interchangeable. The moment one
**gains independent state** — a unique wound, a buff, a different order or position — it
**splits off into its own card** and is tracked on its own again. Swarm by default;
promote on divergence.

## A speed tracker, if you want one

Tempo is a re-derivable number, so a tracker is **optional** — but a light one makes the
running count effortless. The cheap version: a **track of spaces 0…Speed** along the
edge of each stat-block, with a single **marker** that advances as you spend — so *how
much Speed you've used* shows at a glance and *what's left* is the rest, exactly like a
score track. It never holds anything you couldn't re-derive from play; it just saves you
the sum. (This is what the [tutorial](../tutorial.html) draws as a **tempo bar**.)

## A worked round, card by card *(earlier draft warband)*

> This table walks an **earlier** version of the fight (Ogre / Wraith / Imps). The
> [current scenario](sample-round.md) uses the same moves and zones — it's queued for a
> refresh alongside the tutorial.


**Setup:** each side lays Form face-up into front/back lines; each combatant places
**tempo = Speed**; behavior decks sit face-down; stances/actions wait in **Potential**.

| #   | What happens                                                                                                                                                                                                                    | Cards / tokens that move                                                                                                                             |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | **Declare.** Aldric & Vera → **Hold**; Sefa → **Attack** (Firestorm); Bram → **Attack** (Rally). Creatures flip their behavior top cards.                                                                                       | stance markers placed; behavior cards revealed (hand→Active)                                                                                         |
| 2   | **Stalker runs, slips Vera** (spends Vera's Speed).                                                                                                                                                                             | Stalker **tempo 6 → 1** (5 to spent)                                                                                                                 |
| 3   | **Stalker caught by Aldric** (tempo 1 < 4). It takes the hit; Aldric plays **Bash** → it lands and **staggers** (run cancelled). Aldric engaging costs 6 tempo, he has 4 → **overextends, Exposed**. Bash 4 → T1 → 4 Body flip. | Bash: hand→Active→**face down**; Stalker **Body ×4: turned face down in Form** (6→2); Aldric tempo→0 + **Exposed** marker; Stalker run **cancelled** |
| 4   | **Wraith haunts Sefa.** Fear 5 vs Rallied Resolve 5 → no break → **recoil**.                                                                                                                                                    | Wraith behavior card Active→**face down**                                                                                                            |
| 5   | **Bram's Rally** lands in the **party zone**, holding Sefa at Resolve 5.                                                                                                                                                        | Rally: hand→Active→**party zone** (Lasting)                                                                                                          |
| 6   | **Sefa's Firestorm.** Imps: Power 5 ≥ Body 1 → **die**. Ogre: heat −0, 5÷T3 = 1 → 1 Body flip.                                                                                                                                  | Firestorm: hand→Active→**face down** (Fleeting); each Imp **Body: turned face down in Form**; Ogre **Body ×1: turned face down in Form** (8→7)       |
| 7   | **Vera's Riposte** vs the Ogre's Press: **Evade** negates, **+Speed** banked, counter 0 (armor).                                                                                                                                | Riposte/Evade: hand→Active→**hand** (returns to hand); **+Speed token → Active**                                                                     |
| 8   | **Round end.** Partial damage clears (none pending); tempo **refills** to Speed; spent *stances* that self-return go back to hand; spent *actions* stay **face down**.                                                          | round-damage zones emptied; tempo pools reset; Aldric's **Exposed** marker cleared                                                                   |

**End state, in cards:** Imps' Body **face down** (dead); **Stalker** 2 Body face up / 4
face down (in Form) (bloodied, stopped); Ogre 7/8; Wraith Presence ×3 (untouched); heroes' Body
untouched; a **+Speed** token in Vera's Active; a **Rally** in the party zone.

## Pressure-test findings

**Clean as cards:**

- **Capabilities, stances, actions, traits, rules** are all just cards moving among the
  zones — tactile and legible. The "character sheet made of cards" works.
- **Stance/action lifecycle** (hand→Active→face down, or →hand) is a clear
  physical motion you can see across the table.
- **Tempo as spent tokens** is *more* intuitive in hand than on paper: you push Speed
  tokens away to slip a guard, and the pile that's left **is** how fast you still are.
  First strike = compare the two piles. Overextend = you ran the pile out.

**Health needs no heap — *toughness* scales it.** A 100-Body creature is **10 cards at
toughness 10**; a 1000-Body one is **10 cards at toughness 100**. The **card count stays
constant; the Vitality-card number grows** (see
[form & defeat](form-and-defeat.md#example--a-vitality-card)). Even god-tier
durability is a small, legible stack, and the coarseness is *deliberate* — sub-toughness
chip is shrugged off, because **we don't represent 98 vs 99: every card-state change is
meaningful.** Aldric's "8 Body" is just a low-toughness case; a tougher foe isn't more
cards, it's a bigger number.

**Utility decks carry bonuses and temporary effects.** Rather than track ad-hoc state,
a **deck of effect cards** (a buff, a hazard, a one-round boon) is **drawn, held in a
zone while it lasts, and discarded when it expires** — transient state you can see and
then remove, never bookkeeping in the head.

**Tempo needs no tokens at all.** It's a single value you spend *down*, and it's
**derivable from the table any time** — start from your Speed (on your card) and subtract
what you visibly did (*dove past two guards, made three attacks*). So you keep the
running total in your head as a convenience and re-derive it from play whenever it's
questioned — no heap, no denominations. (A small **round-end ritual** remains: refill
tempo, clear partial damage, return self-returning stances, drop Exposed markers.)

**Verdict:** representable by hand, and the economy concern largely **dissolves** under
one rule — **scale the *number* on a rules card, not the *count* of cards.** Health is a
constant-size stack; **tempo is a number you can always re-derive from the table**;
utility decks absorb temporary state. The cards-only pillar holds **at any power level.**

## Open

- Whether every tempo spend leaves a **clear enough table trace** to re-derive your
  remaining number in a busy, multi-target round — the derivability the head-count relies
  on.
- How a creature's whole **stats + health + behavior** packs onto the fewest cards —
  creatures outnumber heroes, so *their* economy matters most.
- Whether **toughness** should ever be *non-uniform* across a creature's cards (a tough
  hide that cracks — later Health cards at lower toughness), or always flat.
