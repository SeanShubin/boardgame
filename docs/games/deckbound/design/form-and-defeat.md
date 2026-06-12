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
- **Spirit** — affect the incorporeal (e.g. damage a ghost).

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

### Knockout, revival, and death

When your **Body** fails (last Body card lost) you are **knocked out**. From there:

- **Revival is a passive action.** An ally — or, solo, a dedicated revive card —
  with the ability brings a knocked-out character back **without spending its
  turn's choices** on it.
- **Death happens only if your Body is at zero *and no one remains to revive
  you***. A lone fall is recoverable; a full wipe (nobody left to revive) is how a
  run actually ends.

(What death *costs* beyond ending the run — permadeath, attrition, lost cards — is
still deferred.)

## How damage resolves

Damage is **typed** and **targets an aspect's cards** — a physical blow eats
**Body** cards, a mental assault eats **Mind**, and so on. Resolution is governed
by **Form cards**, which makes it modular per creature.

### Toughness (quantity)

A Form card sets the **quantity** — how much damage each capability card of an
aspect absorbs. Cards lost to one attack ≈ ⌊ damage ÷ quantity ⌋. Worked example,
**Body quantity 2 with 3 Body cards**:

| Damage | Body cards lost |
| --- | --- |
| 1 | none — below quantity, shrugged off |
| 2–3 | 1 |
| 4–5 | 2 |
| 6+ | all 3 → **body fails → knocked out** |

Higher **quantity** = tougher (each card soaks more, small hits ignored); more
**cards** = more hits before the aspect fails.

### Defensive Form cards & damage types

Form cards can also **counter damage by type**, applied before toughness. The
*Armor* card, for instance:

- reduces **blunt** damage by amount × 2,
- reduces **sharp** damage by amount × 1,
- does **not** reduce **piercing** — piercing is the type armor can't stop.

Other Form cards carry resolution rules of their own ("lose at most 2 Body per
attack," "ignore attacks under strength 1"). Toughness, type defenses, and which
aspect a card protects are all just Form cards — so a creature's resilience is
**built, not fixed**.

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
- Revival is passive and **choice recovery** is a Mind tactic — but is **healing a
  Dormant *capability* card** (Form) a separate thing (a Magic / healing effect),
  and how hard?
- Do **non-keystone** aspects (Mind / Magic / Spirit) have consequences beyond
  "that aspect shuts off"?
- The full **aspect list** beyond Body / Mind / Magic / Spirit.
- How **strength** and **quantity** numbers scale across the power curve.
