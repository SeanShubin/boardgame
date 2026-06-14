# Deckbound — Form, Capabilities & Defeat

Your **Form** is the heart of a character: the cards in the **Form zone** define
what you *are* and *can do*, and they are also your **health**. Where the
[choice cycle](zones.md) (Potential / Active) is your *tactical*
resource, **Form is your vitality** — the two are separate systems, and only Form
is lethal.

## Form = capabilities

The Form zone holds **capability cards**, grouped by **aspect**. An aspect is a
*way of acting*; the current starting set (more to come — three is enough for now)
is:

- **Body** — physically strike. (Casting a spell is one such Body delivery — a typed
  physical effect with reach, not a separate aspect; see [the complete defense
  model](#the-complete-defense-model).)
- **Mind** — choose tactics: the hidden-information [stance](decision-making.md).
- **Spirit** — the **will to act**: fear, morale, resolve, disposition (see
  [aspects](decks-and-aspects.md#the-aspects)).

A capability card **grants an action** in its aspect (one play per card — a
*Human Body* "may play one physical attack"). For the **outer** aspect (Body)
those same cards are **also points of health**; the **inner** aspects (Mind, Spirit) work
differently — they have **no Health-card stack at all** (see [outer vs inner](#outer-aspects-are-pools-inner-aspects-are-thresholds)
below).

## Outer aspects are pools; inner aspects are thresholds

The aspects do **not** all take damage the same way. The canonical split:
**inner aspects are thresholds, outer aspects are pools.**

- **The outer aspect (Body) is a POOL.** You hold **several Health cards per aspect**
  — redundancy is hit points — governed by a **Vitality card** (below). Typed **damage
  turns Health cards face down in place**; lose an aspect's last (face-up) card and that
  aspect **shuts down**. You keep fighting "in some manner" through whatever capabilities
  remain (a character stripped to Spirit alone still acts spiritually).
- **Inner aspects (Mind, Spirit) are CAPACITIES / THRESHOLDS.** They are a **standing
  value**, modified by effects, that an attack tries to **overcome — not deplete** — and
  there is **no Health-card stack** behind them. Nothing is "turned face down" as you take
  inner pressure; the value simply has to be *exceeded*.
  - **Spirit's** capacity is **Resolve**: a single standing value (e.g. "Resolve 5") set
    by Form and raised/lowered by effects. **Fear** accumulates within a round and must
    **exceed** current Resolve to break you (freeze / flee / scared-to-death); accumulated
    fear clears at round end. **Resolve itself never depletes** — it was a threshold all
    along. See [the Spirit aspect](spirit.md).
  - **Mind's** capacity is a **focus pool** (prediction bandwidth): its **size = Mind**,
    spent predicting attackers (each prediction costs the attacker's Speed), refreshing per
    round. **Confusion lowers the capacity** (shrinks the pool) rather than chipping a stack.

### Body is the keystone

For a typical corporeal creature, **Body is vital**: when your **last Body card**
is lost, **you are knocked out** — Mind and Spirit both shut down with it,
having no living body to act through. The keystone is named by a Form card, so it
is **modular**: an incorporeal creature might key on Spirit instead.

But the keystone is **not the only way out of the fight.** Body's loss is *terminal*
(it shuts the rest down), yet a foe whose **will** or **mind** is broken is **functionally
removed** too — see the four channels below. Body remains the keystone; the other channels
are distinct, equally valid removals.

## The four channels — four ways to take an Actor out

An Actor is removed from the fight if **any one channel fully breaks** — not only Body.
Each broken channel is its own way to win; **called shots (damage typing) become "called
channels."**

1. **Make it predictable** — collapse its Mind's unpredictability so it can only throw the
   bare **Body Strike**, predictable like a Creature. (Lives in Mind: *being predicted*.)
2. **Break its will** — drive **Fear > Resolve** so it **freezes, flees, or is scared to
   death**. (Lives in Spirit.)
3. **Blind its prediction** — **Confusion** shrinks its **focus pool** until your stances
   **auto-succeed** against it. (Lives in Mind: *predicting*.)
4. **Wreck its body** — physical damage to the **Body** pool; the **last Body Health card**
   is a **knockout**. (Body is the keystone — its loss is terminal and shuts the rest down.)

Channels **1 and 3 both live in Mind** — being-predicted vs predicting are two faces of the
same aspect, kept as **one stat**: predicting is the Mind focus pool, the positional
upper-hand it earns is the duel's **Edge**, and being-predicted is the
[exhaustion](zones.md) system — so sealing the Mind breaks both at once (a *Poise* trait
is reserved as a future special card).

This is the deeper answer to "what do non-keystone aspects *do*": losing Mind or
Spirit is not merely "that aspect shuts off" — a **will-broken or mind-broken Actor is out
of the fight**, just by a different door than the body.

### Knockout, recovery, and the wipe

When your **Body** fails (last Body card lost) you are **knocked out** — and you stay
down for the **rest of this combat**, Mind / Spirit shutting down with it.
There is **no mid-combat revival**: a fallen ally is a real loss for the duration of
the fight, and the party fights on short-handed.

- **Winning resets the party.** The moment **every enemy is defeated** with **at least
  one character still standing**, the combat is won and the **whole party returns to
  full** — every face-down capability card is turned back up (in Form). Health is a **per-combat**
  resource: spent during a fight, restored by winning it, not carried between fights as
  attrition.
- **A full wipe ends the run.** If your **last standing character falls** before the
  enemies are beaten, no one is left to win — that is the loss, the way a run actually
  ends.

So a fight is binary at the edges: survive with *anyone* up and you recover everything;
lose the last body and it's over. This is what makes "at least one character standing"
the real [balance margin](world-and-progression.md#the-shape-of-progression--the-rule-of-three).
(What a wipe *costs* beyond ending the run — permadeath, lost cards, world state — is
still deferred.)

## How damage resolves — the Vitality card and Health cards

This section governs the **outer aspect** (Body) — the **pool**. The **inner**
aspects (Mind, Spirit) have **no Health cards**; they resolve as thresholds (see
[outer vs inner](#outer-aspects-are-pools-inner-aspects-are-thresholds) and, for Spirit, [the Spirit
aspect](spirit.md)). The full picture — outer and inner together, with the **Ward** cut —
lives in [the complete defense model](#the-complete-defense-model) below.

For an outer pool, two kinds of card do the work, kept deliberately apart:

- **Health cards** carry no magnitude. Each is a **generic, interchangeable** token —
  these are your Body Form cards seen from the health side — and a card is simply **face
  up (in Form)** or **face down**. Your *current* health is just **how many are still
  face up**. (Armor is a rules card, not a Health card — see below.)
- A single **Vitality card** (one per aspect) holds the **rules of health**: how many
  Health cards you **start with**, and the **toughness** — how much damage each Health
  card can take before it turns face down. It also fixes how **partial** damage is
  handled and whether multiple sources count **separately or cumulatively**. Resilience
  is *built* from a Vitality card plus a stack of generic Health cards — modular per
  Actor.

Damage is **typed** and **targets an aspect** — a physical blow eats **Body's** Health
cards, while **fear** is measured against the **Resolve threshold** (no cards to eat), and
so on.

### Accumulation is always cards in a zone

Nothing is tracked in the head: when damage **accumulates**, it does so as **cards
added to a zone** (the round's incoming hits pile up there as tokens), and the rules
card reads that pile to decide how many capability cards turn face down.

- **Body damage** (and likewise **Fear** against the Resolve threshold) accumulates
  **within a round, not between rounds**: several small hits in one round combine against
  you, but **partial** damage that never crossed a threshold is **cleared at round's end**
  — you shrug off scratches; a focused round draws blood. (For Body the threshold is a
  card's toughness; for the Resolve threshold the accumulating quantity is Fear, and
  Resolve itself never depletes.)
- **Armor** is **not cumulative at all**: it reduces **each source independently**,
  fresh every hit, and never depletes.

### Example — a Vitality card

A Vitality card states a **count** and a **toughness**, and damage resolves as
`Health cards down = ⌊ damage ÷ toughness ⌋`. Take **4 Health cards at toughness 3**:

| Damage (this round) | Health cards face down               |
| ------------------- | ------------------------------------ |
| 1–2                 | none — below toughness, shrugged off |
| 3–5                 | 1                                    |
| 6–8                 | 2                                    |
| 9–11                | 3                                    |
| 12+                 | all 4 → **body fails → knocked out** |

Higher **toughness** = each card soaks more; more **cards** = more hits before the
aspect fails. Other rules a Vitality card (or a companion rules card) can set: "at most
one card per source," "ignore hits under strength 1," "each source resolved separately."

**Toughness is also how health *scales* — without more cards.** A 100-Body creature is
**10 Health cards at toughness 10**; a 1000-Body one is **10 cards at toughness 100.**
The **card count stays roughly constant; the toughness on the Vitality card grows.** So
even god-tier durability is a small, legible stack. The coarseness is *deliberate*: it
shrugs off sub-toughness chip, because **we don't represent the difference between 98
and 99** — **every card-state change is meaningful.** A blow only matters when it's big
enough to cross the toughness, which is exactly what **Power**, **Precision**, and banked
[momentum](mind-and-stances.md) are *for*: building a hit large enough to *count*.

### Defensive rules cards & damage types

A defensive rules card **counters damage by type**, applied per source before toughness
— the *Armor* card reduces **blunt** by × 2, **sharp** by × 1, and **not piercing**
(the type armor can't stop) — and, being armor, applies **fresh to each source**
(never cumulative). Elemental delivery (former *Magic* — *Firestorm*, *Spell*, and the
rest) is **outer/physical**: a fireball is a **heat** damage-type, stopped by **Armor
vs-heat**, not by Ward. Casters differ only by **which physical/elemental cards they
hold**.

## The complete defense model

The two subsections above scoped damage to the **outer pool**. Here is the whole model —
**outer and inner together** — and it is the canonical home for it.

**Every attack is either OUTER or INNER.**

- **OUTER** = physical, *including every former-Magic element* (fire, frost, …).
- **INNER** = aimed at the psyche: **fear → Spirit**, **confusion → Mind**.

Each channel resolves through a **per-source cut**, then a **bar**; only the **outer**
channel has a health **pool** behind the bar:

| Channel                      | Cut (per source, typed, never depletes)     | Bar                            | Pool                                    |
| ---------------------------- | ------------------------------------------- | ------------------------------ | --------------------------------------- |
| **OUTER** (Body)             | **Armor** (typed: blunt / sharp / heat / …) | **Toughness**                  | **Body Health cards** (last → knockout) |
| **INNER — fear** (Spirit)    | **Ward** vs-fear                            | **Resolve**                    | **none** — breaks on one crossing       |
| **INNER — confusion** (Mind) | **Ward** vs-confusion                       | **Mind-capacity** (focus pool) | **none** — breaks on one crossing       |

### Ward — the inner cut

**Ward** is the inner counterpart to Armor: a **passive, typed, per-source,
never-depleting cut**, subtracted **before the inner bar**. It is **a number on a card**
(like Armor's reduction), **not a meter** — **one typed stat** (vs-fear, vs-confusion).
Ward is **not anti-magic**: a fireball is outer and meets **Armor vs-heat**; Ward guards
**only fear (Spirit) and confusion (Mind)**.

### Cross-immunity

The two sides do not bleed into each other:

- **Outer attacks** take the **Armor** cut, **ignore Ward**, and eat the **Body pool**.
- **Inner attacks** take the **Ward** cut, **ignore Armor**, and test the **bar** (no pool).

### The resolution pipeline (one source)

1. **Build the hit** — **Power**, **Precision**, banked **Edge**, and modifiers combine
   into the raw magnitude.
2. **Subtract the cut** — **Armor** (outer) or **Ward** (inner), **per source, by type**;
   the cut **never depletes**.
3. **Accumulate into the round's pile** — what survives the cut is added as **tokens in a
   zone** (accumulation is always cards in a zone).
4. **Compare the pile to the bar**:
   - **OUTER:** Health cards down = `⌊ pile ÷ Toughness ⌋` (see [the Vitality-card
     example](#example--a-vitality-card)); the **last Body card → knockout**.
   - **INNER:** if the pile **exceeds the bar** the channel **breaks now** — Spirit:
     **freeze / flee / scared-to-death**; Mind: **predictable / blind**.
5. **Clear unspent pile at round end.** Bars **never deplete**; only the **Body pool** is a
   maintained meter — **Armor, Ward, Toughness, Resolve, Mind-capacity are all passive
   stats**.

### One health track

There is **exactly one maintained meter**: the **Body pool** of Health cards. Everything
else defending you — **Armor, Ward, Toughness, Resolve, Mind-capacity** — is a **passive
stat** that is read, not spent.

### When breaks clear

An inner break **clears at round end** with the unspent pile — **except
scared-to-death**, which **bleeds permanently into the Body pool** (it is the one inner
result that touches the one health track).

A **cast** is an action like any other: it **draws tempo** the same way a physical
strike does (elemental delivery is just Body delivery).

### Incorporeal — *(deferred special card)*

> **Parked until the core is solid.** Incorporeal is one **special card**, not a core
> rule — captured here but set aside while Body / Mind / Spirit and the tempo +
> stance loop are nailed down. Don't build on it yet.

In brief: an **Incorporeal** creature has **no Body**, so physical aspects can't touch
it — but with no flesh to hide behind it lies wide open to the **inner aspects**
(Spirit). It is one entry in the
[special-card library](#bespoke-traits-are-a-feature) below.

### Bespoke traits are a feature

Beyond the core, Deckbound grows a curated **library of special cards** — strange,
individually-balanced traits, each weighed on **its own** considerations rather than
forced into a uniform rule (a thing that can't be touched physically, a thing that
splits when struck, a thing that *feeds* on a damage type). The intended **shape: a
handful per stat — roughly three special cards for each** of Body, Mind, Spirit
(and Speed, Power, …), so every stat gets its own pocket of surprises. The variety is
the point — the **core systems stay few and emergent**
([§6](philosophy.md#6-many-systems-from-few-rules)), with this weird layer on top for
**identity and surprise**. **Incorporeal is one such card; the whole library is deferred
until the core is solid.**

## Why this matters for the game

- **Called shots are built in.** Choosing a damage type chooses which capability
  you erode — no separate targeting mechanic needed.
- **Characters degrade, they don't just shrink.** Losing Body ends you; losing
  Mind / Spirit *transforms* how you fight.
- **The signature move: disable the Mind.** Mind grants the tactical
  [stance](decision-making.md#the-three-decision-makers); strip it and the victim can
  no longer bluff — they collapse to environment-creature predictability. In a game
  about human intellect, attacking the mind is the deepest cut.

## Open questions

- Beyond knockout → retreat, what (if anything) is **death**? Deferred.
- Recovery is now **post-combat** (win → full party reset), and **choice recovery**
  is a Mind tactic — but is **mid-combat healing of a face-down *capability* card** a
  separate thing (a healing effect) worth having at all, given a win restores
  everyone anyway?
- Do the inner aspects keep any **small health-stack**, or are they **pure capacity**?
  And does **Mind need two numbers** — a capacity value plus something else?
- *(Settled: **Mind stays one aspect** — predicting is the focus pool, the positional
  half of "poise" is the duel's **Edge**, and being-predicted is **exhaustion**; no
  Read / Poise split. A Poise trait is a future special card.)*
- *(Settled: **Magic is a physical delivery** (typed element + reach), not an aspect —
  folded into Body.)*
- The full **aspect list** beyond Body / Mind / Spirit.
- How **strength** and **toughness** numbers scale across the power curve.
