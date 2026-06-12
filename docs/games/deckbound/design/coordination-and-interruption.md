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
  (your safety), and, on the front line, you **free-strike (and may interrupt) any
  diver** crossing you toward your back line — see [the gauntlet](#running-the-gauntlet).
  Holding is how a back-liner buys safety at the cost of output, and how a front line
  becomes a **gauntlet**.

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

- **Speed — match theirs.** Your Speed ≥ their Speed: you keep up and impose. (To stop
  a moving **dive**, bodies **pool** their speed — see
  [the gauntlet](#running-the-gauntlet).)
- **Power — at least what they are using.** Your Power ≥ the Power they are
  committing, so they can't simply shrug you off and push through.

## Speed is the currency of engagement

One principle underlies blocking, interrupting, and defending at once: **to impose on
someone your Speed must match theirs (≥), with Power ≥ theirs so you can't be shrugged
off.** From there, **numbers pool and spreading divides**:

- **Pool (coverage).** Several bodies covering a lane add up — fastest **+1 per extra
  Holder** — so slow bodies together wall a fast diver (the
  [gauntlet](#running-the-gauntlet)).
- **Divide (bandwidth).** Engaging several opponents spends Speed: you engage **one**
  for free, then pay each further opponent against what's left —

> at least their Speed, then **subtract their Speed** and judge the next against what
> is left. Power is checked at full against each, and is never spent

— and you may **overextend** past your budget to oppose one more, at the price of
**taking a hit**.

So Speed is **how many fights you can be in at once**. A blur of a duelist parries
two or three foes and stops their dives; a slow bruiser engages one and is
**swarmed** — the rest auto-succeed. It is what makes focus-fire deadly against a
*slow* target, and what lets a fast hero stand against a crowd.

**Defending several** means **one defensive read per attacker** you engage, paying
Speed for each beyond your first; an attacker you cannot afford **auto-succeeds**, and
you choose which to oppose and which to take. (The finer **targeting rules** — who
must be opposed, in what order — still want spelling out.)

Bandwidth covers **offense** too: a fast enough character **attacks several targets**
in one round — each still legal under the line rules (a back-liner's extra targets
are still front-line only; a front-liner splits between the line and dives).
Powerscaling is **uncapped by design**: stack enough Speed (with the Power and
capabilities to match) and one character becomes a **one-man army**. That is the
[asymmetry](philosophy.md) pillar at combat scale — characters aren't balanced;
scenarios are.

## Running the gauntlet

A dive is not stopped by a wall; it **runs a gauntlet**. To reach a back-liner, a
diver must pass the **Holders** covering that lane, and **each Holder it passes lands a
free strike** — an opportunity attack. The diver is committed to the dive (not
[responding](#the-coherence-principle) to the Holder), so each free strike
**auto-lands**: bodies always get their swing, however fast the diver.

**Free strikes** are the damage; **stopping the dive** is where coverage meets speed.
A diver is **blocked** when the wall can **keep up** with it: the wall's speed is its
**fastest Holder, +1 for each extra body** in the lane (more guards cover more
angles). If that **≥ the diver's Speed** — and a Holder's **Power ≥ the diver's** —
the **dive is stopped**:

- A diver **faster than a lone blocker slips past** — but **two slower blockers still
  catch it**, the second covering the angle the first can't.
- A **really fast** diver outruns even that and **slips through**, bloodied.
- A diver that takes **too much** from the free strikes **dies on the run**.

So coverage matters without being an absolute wall: more bodies = more free strikes
**and** more speed to catch the runner. A **gap** (a front-liner Attacking, not
Holding) is an open lane. A Holder may **overextend** to block while already busy, but
**takes a hit** for the divided attention — often worth it.

**Agency, no wasted turns.** Because the lines are open information, a diver chooses
the gauntlet only when it is worth it; against a wall too thick to survive it simply
**engages the front instead**, spending its turn on a Holder rather than throwing it
away.

## Creature targeting (from the deck)

A creature's targeting is part of its [behavior
deck](decision-making.md#environment-creatures--hazards-non-player):

- **Front line** — attack whoever stands in front (the default).
- **Priority target** — dive for a preferred victim (the healer), **running the
  [gauntlet](#running-the-gauntlet)** to reach it. Only free strikes that drop it, or a
  Holder quick and strong enough to interrupt, stop it.

## Why cardless

Lines, targets, and dives are **declarations**, not cards — the positioning layer,
kept separate from the tactical card exchange within each engagement, and free to
take a stance.

## Open questions

- How **Speed / Power** numbers scale across the power curve (shared with the
  [combat](combat.md) sketch).
- How hard a **free strike** hits (a full attack, or reduced), and whether a diver may
  ever defend against one or always eats it.
- Whether to later add **positions** (a row of slots) for flanking and localized gaps
  in the gauntlet.
