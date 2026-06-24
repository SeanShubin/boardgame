# Deckbound — Stats Overview

> **SUPERSEDED (2026) — the stat collapse (Spec §2 / §3).** The stat set is now five —
> **Might · Vitality · Toughness · Speed · Daring** — and combat has **one** channel (Might → the
> health pool). The inner **Fear/Spirit** channel (Dread / Resolve / Ward) is **gone**; **Armor /
> Precision / damage-types** are deferred to the gear system; **Inspiration** is dropped. This page's
> "three aspects" and "every channel has a cut, Body keeps a pool" are **stale** — trust the Spec.

The one page where a reader meets **every stat at once**. Each system doc expands its
own; this is the map. The design is built so **theme tells you the mechanic** — your body
is the only thing that *bleeds*; everything else is either a fixed *quality* or a *budget
you spend and refill each round*.

## The one rule that organizes everything

> **You maintain exactly one meter — your Body health. Everything else is either a number
> you read off a card, or a fast count you keep in your head and refill each round.**

Every quantity is exactly one of three kinds:

| Kind                | What it is                                                | Maintained?             | Members                                                                         |
| ------------------- | --------------------------------------------------------- | ----------------------- | ------------------------------------------------------------------------------- |
| **Health track**    | face-down **cards**, per-combat, restored on a win        | **yes — the only one**  | the **Body pool**                                                               |
| **Passive stat**    | a **number on a card**, modified by effects, never spent  | no — read off the board | Armor, Ward, Toughness, Resolve, Mind-capacity, Power, Precision, Speed, Spirit |
| **Ephemeral spend** | a Speed-like count, spent per action, refreshes per round | no — re-derived in head | **Tempo**, **Focus**                                                            |

## 1. The three aspects — the ways you can be taken out

Magic is folded into Body, so there are **three** ([decks & aspects](decks-and-aspects.md#the-aspects)).
*You only die one way — your body fails — but you can be beaten three ways.*

- **Body** — flesh and force. A **pool** you deplete (a fireball is just a heat-typed Body hit).
- **Mind** — wits. A **threshold** you break, not a pool you drain.
- **Spirit** — will. Likewise a **threshold**.

Body is the **keystone**: lose its last card and Mind & Spirit shut down with it. But a
**mind-broken or will-broken** Actor is out of the fight too — same defeat, different door
([form & defeat](form-and-defeat.md#the-four-channels--four-ways-to-take-an-actor-out)).

## 2. Defense — every channel has a *cut*, then a *bar* (and Body alone keeps a *pool*)

The full model: [the complete defense model](form-and-defeat.md#the-complete-defense-model).
*Theme:* Armor/Ward **shave each blow**; the bar is **how big a blow it takes to land**;
only the body keeps a wound-count.

| Channel                       | Cut (per source, typed)              | Bar (a hit must exceed it) | Pool behind it                          |
| ----------------------------- | ------------------------------------ | -------------------------- | --------------------------------------- |
| **Body** (physical/elemental) | **Armor** (blunt / sharp / heat / …) | **Toughness**              | **Body Health cards** (last → knockout) |
| **Spirit** (fear)             | **Ward** vs-fear                     | **Resolve**                | none — you **break**                    |
| **Mind** (confusion)          | **Ward** vs-confusion                | **Mind-capacity**          | none — you **break**                    |

*Intent:* a per-source **cut** is strong against *many small* hits; a high **bar** shrugs
*any* sub-bar hit — so the two are not redundant, and you want both. Cross-immunity keeps
it clean: physical hits ignore Ward, inner hits ignore Armor.

## 3. Offense & timing — how hard, how precise, how fast

*The line the manual leads with:* **"Speed swings, Mind reads, toughness endures"**
([speed & tempo](speed-and-tempo.md#the-three-stats-divided-cleanly)).

- **Power** — raw force; cracks Armor + Toughness and **drops** a foe. Also the magnitude
  of conjured elements (`Mag` is gone — a cast uses Power).
- **Spirit (Spr)** — the *force of fear*: powers Dread / Terror and Rally ([spirit](spirit.md)).
- **Precision** — weak-spot bypass (ignore armor / bonus damage); **the Mind's gift** to a blow.
- **Speed** — timing and volume: who lands first, how many you strike, dragging Runners.

## 4. The two budgets you spend each round (head-tracked)

Offense and defense **mirror** each other ([symmetric drain](speed-and-tempo.md#symmetric-drain--tempo-and-focus));
both refresh each round, both are re-derived from the table, and overspending **either**
leaves you **Exposed** (table-wide, bottom of the first-strike order).

- **Tempo** (= your **Speed**) — your *action* budget; each strike/engage costs the **target's** Speed.
- **Focus** (= your **Mind-capacity**) — your *defense* budget; each foe you predict costs the **attacker's** Speed
  ([bandwidth is the focus pool](mind-and-stances.md#how-many-you-can-predict--bandwidth-is-the-mind-focus-pool)).

*Double-duty worth noticing:* **Mind-capacity is the same number twice** — the *bar*
confusion must cross, and the *size of the Focus pool* you spend to predict. Confusion
shrinks it, so you both break sooner and defend fewer.

*(Not a character stat: **Edge** is a public, per-duel buildup spent inside a single duel —
see [the duel](the-duel.md). It resets every duel and isn't maintained between them.)*

## Master reference

| Stat              | Kind            | Role in one line                                                                 | Detailed in                                                                                      |
| ----------------- | --------------- | -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| **Body health**   | health track    | the only maintained meter; Health cards flip face down as you're hurt            | [form & defeat](form-and-defeat.md#how-damage-resolves--the-vitality-card-and-health-cards)      |
| **Toughness**     | passive (bar)   | damage per Health card — `cards down = ⌊damage ÷ toughness⌋`                     | [form & defeat](form-and-defeat.md#example--a-vitality-card)                                     |
| **Armor**         | passive (cut)   | typed per-source reduction of physical/elemental hits; never depletes            | [form & defeat](form-and-defeat.md#the-complete-defense-model)                                   |
| **Resolve**       | passive (bar)   | the Spirit threshold; fear must **exceed** it to break your will                 | [spirit](spirit.md)                                                                              |
| **Mind-capacity** | passive (bar)   | the Mind threshold *and* the size of your Focus pool                             | [mind & stances](mind-and-stances.md#how-many-you-can-predict--bandwidth-is-the-mind-focus-pool) |
| **Ward**          | passive (cut)   | typed per-source reduction of fear/confusion; **not** anti-magic                 | [form & defeat](form-and-defeat.md#ward--the-inner-cut)                                          |
| **Power**         | passive (force) | magnitude — cracks Armor/Toughness and drops a foe (incl. conjured)              | [combat](combat.md)                                                                              |
| **Spirit (Spr)**  | passive (force) | the force of fear — Dread, Terror, Rally                                         | [spirit](spirit.md)                                                                              |
| **Precision**     | passive (force) | weak-spot bypass / ignore armor — the Mind's gift to a blow                      | [cards & customization](cards-and-customization.md)                                              |
| **Speed**         | passive → spend | sets the **Tempo** pool; first-strike, volume, drag                              | [speed & tempo](speed-and-tempo.md)                                                              |
| **Tempo**         | ephemeral spend | action budget (= Speed); each engage/strike costs the **target's** Speed         | [speed & tempo](speed-and-tempo.md#symmetric-drain--tempo-and-focus)                             |
| **Focus**         | ephemeral spend | defense budget (= Mind-capacity); each prediction costs the **attacker's** Speed | [speed & tempo](speed-and-tempo.md#symmetric-drain--tempo-and-focus)                             |

## The shape, and the one asymmetry

Each aspect is *almost* perfectly parallel — **offense · cut · bar · pool**:

| Aspect     | Offense                                      | Cut   | Bar           | Pool   |
| ---------- | -------------------------------------------- | ----- | ------------- | ------ |
| **Body**   | **Power**                                    | Armor | Toughness     | Health |
| **Spirit** | **Spr** (fear)                               | Ward  | Resolve       | —      |
| **Mind**   | *(the read itself; Precision is its export)* | Ward  | Mind-capacity | —      |

Mind is the odd one out: it has no "magnitude" stat like Power/Spr — its offense **is**
out-predicting (winning duels → [Edge](the-duel.md)), and **Precision** is how it sharpens
a *Body* blow. Two stats sit **outside** the aspect grid entirely — **Speed** (universal
timing) and **Precision** (Mind-gated, but spent on Body hits).
