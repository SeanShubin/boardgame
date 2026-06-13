# Deckbound — Form, Capabilities & Defeat

Your **Form** is the heart of a character: the cards in the **Form zone** define
what you *are* and *can do*, and they are also your **health**. Where the
[choice cycle](zones.md) (Potential / Active / Dormant) is your *tactical*
resource, **Form is your vitality** — the two are separate systems, and only Form
is lethal.

## Form = capabilities

The Form zone holds **capability cards**, grouped by **aspect**. An aspect is a
*way of acting*; the current starting set (more to come — four is enough for now)
is:

- **Body** — physically strike.
- **Mind** — choose tactics: the hidden-information [read](decision-making.md).
- **Magic** — cast spells.
- **Spirit** — the **will to act**: fear, morale, resolve, disposition (see
  [aspects](decks-and-aspects.md#the-four-aspects)).

A capability card both **grants an action** in its aspect (one play per card — a
*Human Body* "may play one physical attack") **and is a point of health** in that
aspect. How much you can do and how much you can take are the same cards.

## Capabilities are your health

You hold **several capability cards per aspect** — redundancy is hit points.
**Damage knocks capability cards Dormant**; lose an aspect's last card and that
aspect **shuts down** — you can no longer act through it, but you keep fighting "in
some manner" through whatever capabilities remain (a character stripped to Spirit
alone still acts spiritually).

### Body is the keystone

For a typical corporeal creature, **Body is vital**: when your **last Body card**
is lost, **you are knocked out** — Mind, Magic, and Spirit all shut down with it,
having no living body to act through. The keystone is named by a Form card, so it
is **modular**: an incorporeal creature might key on Spirit instead.

### Knockout, recovery, and the wipe

When your **Body** fails (last Body card lost) you are **knocked out** — and you stay
down for the **rest of this combat**, Mind / Magic / Spirit shutting down with it.
There is **no mid-combat revival**: a fallen ally is a real loss for the duration of
the fight, and the party fights on short-handed.

- **Winning resets the party.** The moment **every enemy is defeated** with **at least
  one character still standing**, the combat is won and the **whole party returns to
  full** — every Dormant capability card flips back to Form. Health is a **per-combat**
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

## How damage resolves

Two kinds of card do the work, kept deliberately apart:

- **Capability cards carry no magnitude.** A Body card, a Resolve card, an Armor card
  is simply **up (in Form)** or **down (in Dormant)** — a plain token you flip.
- A separate **rules card** (one per aspect) says **how much each card is worth and how
  damage resolves**: the threshold to flip one down, how **partial** damage is handled,
  and whether multiple sources count **separately or cumulatively**. Resilience is
  *built* from a rules card plus a stack of plain capability cards — modular per
  creature.

Damage is **typed** and **targets an aspect's cards** — a physical blow eats **Body**,
fear eats **Resolve**, and so on.

### Accumulation is always cards in a zone

Nothing is tracked in the head: when damage **accumulates**, it does so as **cards
added to a zone** (the round's incoming hits pile up there as tokens), and the rules
card reads that pile to decide how many capability cards flip down.

- **Body and Resolve** accumulate **within a round, not between rounds**: several small
  hits in one round combine to wound you, but **partial** damage that never crossed a
  threshold is **cleared at round's end** — you shrug off scratches; a focused round
  draws blood.
- **Armor** is **not cumulative at all**: it reduces **each source independently**,
  fresh every hit, and never depletes.

### Example rules card — toughness

A common rules card sets a **quantity**: how much accumulated damage flips one card
down (`cards down ≈ ⌊ damage ÷ quantity ⌋`). Quantity 2 over 3 Body cards:

| Damage (this round) | Body cards down                      |
| ------------------- | ------------------------------------ |
| 1                   | none — below quantity, shrugged off  |
| 2–3                 | 1                                    |
| 4–5                 | 2                                    |
| 6+                  | all 3 → **body fails → knocked out** |

Higher **quantity** = tougher; more **cards** = more hits before the aspect fails.
Other rules cards do other things — "at most one card per source," "ignore hits under
strength 1," "each source resolved separately."

**Quantity is also how health *scales* — without more cards.** A 100-Body creature is
**10 cards at quantity 10**; a 1000-Body one is **10 cards at quantity 100.** The **card
count stays roughly constant; the number on the rules card grows.** So even god-tier
durability is a small, legible stack. The coarseness is *deliberate*: it shrugs off
sub-quantity chip, because **we don't represent the difference between 98 and 99** —
**every card-state change is meaningful.** A blow only matters when it's big enough to
cross the quantity, which is exactly what **Power**, **Precision**, and banked
[momentum](mind-and-reads.md) are *for*: building a hit large enough to *count*.

### Defensive rules cards & damage types

A defensive rules card **counters damage by type**, applied per source before toughness
— the *Armor* card reduces **blunt** by × 2, **sharp** by × 1, and **not piercing**
(the type armor can't stop) — and, being armor, applies **fresh to each source**
(never cumulative).

### Incorporeal — *(deferred special card)*

> **Parked until the core is solid.** Incorporeal is one **special card**, not a core
> rule — captured here but set aside while Body / Mind / Magic / Spirit and the tempo +
> read loop are nailed down. Don't build on it yet.

In brief: an **Incorporeal** creature has **no Body**, so physical aspects can't touch
it — but with no flesh to hide behind it lies wide open to the **inner aspects**
(Spirit). It is one entry in the
[special-card library](#bespoke-traits-are-a-feature) below.

### Bespoke traits are a feature

Beyond the core, Deckbound grows a curated **library of special cards** — strange,
individually-balanced traits, each weighed on **its own** considerations rather than
forced into a uniform rule (a thing that can't be touched physically, a thing that
splits when struck, a thing that *feeds* on a damage type). The intended **shape: a
handful per stat — roughly three special cards for each** of Body, Mind, Magic, Spirit
(and Speed, Power, …), so every stat gets its own pocket of surprises. The variety is
the point — the **core systems stay few and emergent**
([§6](philosophy.md#6-many-systems-from-few-rules)), with this weird layer on top for
**identity and surprise**. **Incorporeal is one such card; the whole library is deferred
until the core is solid.**

## Why this matters for the game

- **Called shots are built in.** Choosing a damage type chooses which capability
  you erode — no separate targeting mechanic needed.
- **Characters degrade, they don't just shrink.** Losing Body ends you; losing
  Mind / Magic / Spirit *transforms* how you fight.
- **The signature move: disable the Mind.** Mind grants the tactical
  [read](decision-making.md#the-three-decision-makers); strip it and the victim can
  no longer bluff — they collapse to environment-creature predictability. In a game
  about human intellect, attacking the mind is the deepest cut.

## Open questions

- Beyond knockout → retreat, what (if anything) is **death**? Deferred.
- Recovery is now **post-combat** (win → full party reset), and **choice recovery**
  is a Mind tactic — but is **mid-combat healing of a Dormant *capability* card** a
  separate thing (a Magic / healing effect) worth having at all, given a win restores
  everyone anyway?
- Do **non-keystone** aspects (Mind / Magic / Spirit) have consequences beyond
  "that aspect shuts off"?
- The full **aspect list** beyond Body / Mind / Magic / Spirit.
- How **strength** and **quantity** numbers scale across the power curve.
