# Deckbound — Coordination & Interruption

A layer above the card duel: not *how* an exchange resolves, but **who fights
whom**, who is exposed, and who gets interrupted. It is **cardless** — positioning
and target choice, not card plays.

## Attack or Hold

Each character takes one of two **stances** — a free declaration, no card:

- **Attack** — commit to a target: the opposing front line, or a **dive** to their
  back line. You deal your attack, but you are **exposed everywhere else** — any
  *other* attacker auto-succeeds against you. You get the [RPS](decision-making.md)
  read only against a target who is attacking you back (a mutual engagement).
- **Hold** — forgo attacking. Instead you **RPS-respond to whoever attacks you**
  (your safety), and, on the front line, you **intercept and nuke any diver** that
  crosses you toward your back line. Holding is how a back-liner buys safety at the
  cost of output, and how a front line becomes a **wall**.

The whole positioning game turns on this: a front line that **Holds** gates the back
line; a front line that **Attacks** is aggressive but leaves the gate open. Push or
protect.

## The coherence principle

> The **RPS read happens only when you are engaging your attacker** — by **Holding**
> (you respond to all comers), or by **mutually attacking** the same character.
> Against anyone you are *not* engaging, their attack **auto-succeeds**.

- **Hold** → you read and defend **every** attacker.
- **Mutual attack** → a full [rock-paper-scissors](decision-making.md) duel.
- **An attacker you're not engaging** (you committed your Attack elsewhere) →
  **auto-succeeds** against you.

Consequences fall out for free: **attacking exposes you**, you **can't strike in two
directions and stay safe**, and **focus-fire is deadly against the slow** — you read
only as many attackers as your **Speed** affords (one by default; see [engagement
bandwidth](#speed-is-engagement-bandwidth)), while a Holder pours that whole budget
into defense and gating but lands no offense of its own.

## Front line and back line

Each side arranges into a **front line** and a **back line**:

- **Back line** may attack **only the opposing front line** — it cannot reach the
  enemy back line. Back-liners strike from safety, shielded by their own front
  line, and the two back lines never trade directly.
- **Front line** may attack the opposing front line, **or dive** to the opposing
  back line — running **past** the enemy front line to get there.

The front line is therefore the **gate**: to reach the enemy's back-line mage, a
front-liner must get past their front line.

Lines are **fluid between rounds** — re-formed each round so a formation can react to
a fallen ally — but **fixed within a round**.

## Resolution

The targeting limits above are **standing rules**, not steps. The sequence is three
phases:

1. **Form up** — each side arranges into front and back lines, **revealed**.
   Formations are open information, so what follows is informed.
2. **Declare targets & stances** — with the formations visible, each character picks
   a **stance** (Attack / Hold) and **target(s)**, obeying the targeting rules above.
   Crucially, **the choice to dive is informed** — you see the enemy's front line and
   where their back line sits before committing.
3. **Resolve in Speed order** — the fastest act first, and an acting character
   **interrupts** (cancels) any slower action it is positioned to stop: a gate (or
   anyone) catching a diver crossing it, a quick strike spoiling a slow Scheme — per
   the [interrupt rule](#interruption-the-rule). A defender reads with RPS where
   engaged, else takes the hit.

"Gating happens first" is **not** a special step — a gate fast enough to catch a
diver simply **acts before** the diver gets through; Speed order does the work.

## Interruption (the rule)

A **successful interrupt cancels the target's action**, and may carry **its own
effects** depending on its nature (a shield bash staggers; a grapple locks; …).

To intercept an enemy you need **both**:

- **Speed — one more than theirs.** Your Speed ≥ their Speed + 1: you reach them in
  time.
- **Power — at least what they are using.** Your Power ≥ the Power they are
  committing, so they can't simply shrug you off and push through.

## Speed is engagement bandwidth

The Speed budget that gates several divers also lets a fast character **engage
several opponents at once**. By default you engage **one** — your mutual target, or,
Holding, one attacker — and any further attacker **auto-succeeds**. Each *additional*
opponent you engage (to read and defend, or to intercept a diver) is bought with
Speed by the same rule:

> **one more Speed than theirs**, then **subtract their Speed** and judge the next
> against what is left. Power is checked at full against each, and is never spent.

So Speed is **how many fights you can be in at once**. A blur of a duelist parries
two or three foes and stops their dives; a slow bruiser engages one and is
**swarmed** — the rest auto-succeed. It is what makes focus-fire deadly against a
*slow* target, and what lets a fast hero stand against a crowd.

Bandwidth covers **offense** too: a fast enough character **attacks several targets**
in one round — each still legal under the line rules (a back-liner's extra targets
are still front-line only; a front-liner splits between the line and dives).
Powerscaling is **uncapped by design**: stack enough Speed (with the Power and
capabilities to match) and one character becomes a **one-man army**. That is the
[asymmetry](philosophy.md) pillar at combat scale — characters aren't balanced;
scenarios are.

## Coverage — stopping a diver no one can catch

Interception is measured in **Speed** (reaction time). Coverage is the *other* axis,
measured in **bodies** (obstruction): you can't pool Speed, but you can pool coverage.
On each front, count your **Holders** against the enemy **divers** aimed at your back
line:

- Each **Holder ties up one diver** simply by standing in the way — **no Speed
  check**. That is coverage.
- **Holders ≥ divers → the back line is sealed.** Even a lone, un-interceptable,
  hyper-fast diver is blocked: it can't be everywhere, and your bodies can.
- **Divers > Holders → the surplus get a clean run**, and only *those* are contested
  the usual way — a Holder fast enough
  ([Speed bandwidth](#speed-is-engagement-bandwidth)) may still **intercept** one.
  Divers neither bodied nor intercepted get through.

So **sufficient coverage = at least as many Holders as divers**; Speed is only what
lets an *outnumbered* wall catch the overflow. (Only **Hold**-stance front-liners
count — a front-liner who Attacks is pressing the enemy, not holding the wall.)

**Example.** Three enemies dive your back line; you hold two front-liners. Coverage
bodies two of them with no Speed check; one breaks through. If one of your Holders is
fast enough (Speed ≥ that diver + 1, Power to match) it intercepts the runner and the
line holds; otherwise one diver reaches your back line.

This is why a wall of Holders is **impenetrable but toothless** (all bodies, no
offense), and why cracking a back line means **out-numbering the wall with divers**
faster than it can catch the overflow.

## Creature targeting (from the deck)

A creature's targeting is part of its [behavior
deck](decision-making.md#environment-creatures--hazards-non-player):

- **Front line** — attack whoever stands in front (the default).
- **Priority target** — dive for a preferred victim (the healer), **accepting the
  front line's interrupt** to reach it. Only a front line quick and strong enough
  stops it.

## Why cardless

Lines, targets, and dives are **declarations**, not cards — the positioning layer,
kept separate from the tactical card exchange within each engagement, and free to
take a stance.

## Open questions

- How **Speed / Power** numbers scale across the power curve (shared with the
  [combat](combat.md) sketch).
- Whether to later promote coverage from a **count** (Holders vs divers) to
  **positions** (a row of slots) for flanking and localized gaps.
