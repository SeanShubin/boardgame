# Deckbound — Coordination & Interruption

A layer above the card duel: not *how* an exchange resolves, but **who fights
whom**, who is exposed, and who gets interrupted. It is **cardless** — positioning
and target choice, not card plays.

## Attack or Hold

Each character takes one of two **stances** — a free declaration, no card:

- **Attack** — commit to a target: the opposing front line, or a **run** to their
  back line. You deal your attack, but you are **exposed everywhere else** — any
  *other* attacker auto-succeeds against you. You get the [RPS](decision-making.md)
  read only against a target who is attacking you back (a mutual engagement).
- **Hold** — forgo attacking. Instead you **RPS-respond to whoever attacks you**
  (your safety), and you add your tempo to the front line's **combined pool**, which it
  spends to **engage Runners** crossing toward the back line — see
  [the gauntlet](#running-the-gauntlet). Holding is how a back-liner buys safety at the
  cost of output, and how a front line becomes a **gauntlet**.

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
bandwidth](#speed-is-engagement-bandwidth)), while a Guard pours that whole budget
into defense and gating but lands no offense of its own.

### Breadth, reads, and the gank

Offense and defense obey **different limits**, which is what makes the asymmetry fair:

- **Offense breadth hits everyone in reach** — a multi-target attack lands on all its
  targets regardless of your read-budget (it's [breadth, not bandwidth](cards-and-customization.md#how-targets-reach-and-the-read-interact)).
- **Reads are bandwidth-limited.** You **defend** only the attackers you're engaging — as
  many as your **Speed** affords. An attacker you can't afford to read **free-strikes**
  you (an auto-success).
- **A target defends *your* attack only if it is reading *you*** — and the two checks are
  independent. So **focusing a foe who's occupied elsewhere is a one-way gank:** you read
  (and counter) their blows *and* land yours free, because their read is spent on someone
  else. The asymmetry is always **paid for by Speed** — the occupied side could buy the
  bandwidth to read you back (see [engagement bandwidth](#speed-is-the-currency-of-engagement)).

Two consequences worth naming:

- **The cleave is a Speed-poor tool.** Breadth forgoes anticipation *because you can't
  afford to duel each foe*; enough Speed buys separate, anticipated engagements on all of
  them and the tradeoff vanishes.
- **Being ganked is a Speed deficit.** One-way dominance is always a gap in the victim's
  bandwidth — closeable by buying the Speed to read back. No one is ganked who could
  afford it.

## Front line and back line

Each side arranges into a **front line** and a **back line**, laid out as a line of
ranks: `[your back][your front] ‖ [their front][their back]`. What you can hit is set by
**reach** (below); the front line **gates bodies** — to *melee* the enemy back line you
must **run the gauntlet** past their front.

- **Melee** reaches only the **adjacent** rank — the two **front lines** clash; a
  back-liner melees nothing.
- **Ranged & inner attacks** (arrows, spells, fear) **shoot over the wall** to any rank
  within reach — they don't run the gauntlet.
- **Back lines never trade directly** unless someone carries reach all the way (3); the
  enemy mage is the hardest target on the table.

Lines are **fluid between rounds** — re-formed each round so a formation can react to a
fallen ally — but **fixed within a round**.

### Reach — the jump line

Targeting is **distance in jumps** along that line —
`front↔front = 1, front↔back = 2, back↔back = 3` — and every attack carries a **reach
`[min, max]`**, hitting an enemy at distance *d* when **min ≤ d ≤ max**:

| Attack                    | Reach             | Hits                                                                           |
| ------------------------- | ----------------- | ------------------------------------------------------------------------------ |
| **Melee**                 | `[1,1]`           | front↔front only; close via the [gauntlet](#running-the-gauntlet) to go deeper |
| **Bow** (min-range)       | `[2,2]`           | your front → their **back**, your back → their **front**; no point-blank       |
| **Thrown / reach weapon** | `[1,2]`           | adjacent, *or* one rank past                                                   |
| **Artillery**             | `[2,3]` / `[3,3]` | reaches their **back** from your **back** — the powerscaled sniper             |

So **`min` is the "can't hit adjacent" knob, `max` is "how far"** — reach is a
**per-weapon band**, not one global rule. And reach **shoots over the wall**: the
[gauntlet](#running-the-gauntlet) drags **bodies** (melee and Runners crossing
physically), but a ranged or inner attack within reach lands without running it. So a
back-line mage is safe from enemy **melee** behind the wall, yet **exposed to enemy
ranged within reach** — you protect it by killing or interrupting the shooter, not by
the wall.

## Resolution

The targeting limits above are **standing rules**, not steps. The sequence is three
phases:

1. **Form up** — each side arranges into front and back lines, **revealed**.
   Formations are open information, so what follows is informed.
2. **Declare targets & stances** — with the formations visible, each character picks
   a **stance** (Attack / Hold) and **target(s)**, obeying the targeting rules above.
   Crucially, **the choice to run is informed** — you see the enemy's front line and
   where their back line sits before committing.
3. **Reveal & resolve** — reads are revealed **simultaneously**; each clash then
   settles by **[tempo](speed-and-tempo.md)** — whoever has more Speed left **lands
   first**, and **interrupts** (cancels) the other's action if its **Power** suffices:
   a guard catching a Runner, a quick strike spoiling a slow Scheme — per the
   [interrupt rule](#interruption-the-rule). A defender reads with RPS where engaged,
   else takes the hit.

Gating is **not** a special step — the front line simply **spends its combined tempo to
engage** Runners as part of resolution (see [the gauntlet](#running-the-gauntlet)).

## Interruption (the rule)

A **successful interrupt cancels the target's action**, and may carry **its own
effects** depending on its nature (a shield bash staggers; a grapple locks; …).

To intercept an enemy you need **both**:

- **Speed — match theirs.** Your Speed ≥ their Speed: you keep up and impose. (To stop
  a **run**, the front line spends its **combined tempo** to engage the Runner — see
  [the gauntlet](#running-the-gauntlet).)
- **Power — at least what they are using.** Your Power ≥ the Power they are
  committing, so they can't simply shrug you off and push through.

## Speed is the currency of engagement

One principle underlies blocking, interrupting, and defending at once: **to impose on
someone your Speed must match theirs (≥), with Power ≥ theirs so you can't be shrugged
off.** From there, **numbers pool and spreading divides**:

- **Pool (coverage).** Several Guards pool their tempo into one **combined** pool the
  line spends to **engage Runners** — the [gauntlet](#running-the-gauntlet) is just
  bandwidth pointed outward, with no order to it.
- **Divide (bandwidth).** Engaging several opponents spends Speed: each foe costs
  **their Speed** from your [tempo](speed-and-tempo.md) pool, paid after, and the
  engagement that takes you **negative overextends** you — it lands, but you're left
  **exposed**. (Power is checked at full against each, and is never spent.)

So Speed is **how many fights you can be in at once**. A blur of a duelist parries
two or three foes and stops their runs; a slow bruiser engages one and is
**swarmed** — the rest auto-succeed. It is what makes focus-fire deadly against a
*slow* target, and what lets a fast hero stand against a crowd.

**Defending several** means **one defensive read per attacker** you engage, paying
Speed for each beyond your first; an attacker you cannot afford **auto-succeeds**, and
you choose which to oppose and which to take. (The finer **targeting rules** — who
must be opposed, in what order — still want spelling out.)

Bandwidth covers **offense** too: a fast enough character **attacks several targets**
in one round — each still legal under the line rules (a back-liner's extra targets
are still front-line only; a front-liner splits between the line and runs).
Powerscaling is **uncapped by design**: stack enough Speed (with the Power and
capabilities to match) and one character becomes a **one-man army**. That is the
[asymmetry](philosophy.md) pillar at combat scale — characters aren't balanced;
scenarios are.

## Running the gauntlet

A run isn't *sequenced* past guards one at a time — that would need a within-line order
we deliberately don't have (a [front line is a **set**](zones.md#zones-at-every-scope),
not a row). Instead, **the gauntlet is just [bandwidth](#speed-is-the-currency-of-engagement)
pointed outward:** the front line spends its **combined tempo** to **engage the Runners
crossing it**, and whatever it can't afford **passes through.**

- **The line's drag pool = its Guards' combined tempo** — the **sum** of their Speeds,
  one derivable number, spent as **drag** subtracted from Runners.
- The defenders **allocate drag** across the incoming Runners. For a Runner of Speed *s*
  hit with drag *d*:
  - **d ≥ s → STOPPED** — caught, and **struck / interrupted** if an engaging Guard's
    **Power ≥** it ([the interrupt rule](#interruption-the-rule)).
  - **d < s → SLOWED, and through:** it reaches its target with **leftover tempo =
    s − d** (telegraphed), but **untouched** — too fast to grab, you only trim its lead.
  - **d = 0 → through at full Speed.**
- When the pool is spent, the rest pass.

The boundary is exact: **drag must *meet or beat* Speed to stop.** Ten guards at 6
(pool 60) **stop** a Speed-60 Runner (60 ≥ 60); a **Speed-61** Runner **slips through — but
arrives at tempo 1**, a hair from helpless. One point of Speed is the line between caught
and through-by-a-thread.

It scales by itself:

- **Few or slow Runners** → the pool out-drags them all → **stopped.**
- **Runners far faster than the line** → the defenders **stop the priority few** (full
  drag each) and **slow or let the rest through.**
- **A lone god** — Speed 100 vs a **60** pool → slowed to **40 and through, untouched.**
- **A swarm** → the pool stops a few; the **overflow floods past** (bring AoE).

The coordination is the **allocation** — which threats the line spends itself on, and
which Guard's **Power** takes each. **No order, no positions** — just a combined tempo
spent down. And **Frostbite helps the wall:** slow a Runner and it's *cheaper to stop*,
so the same pool covers more. A **gap** (a front-liner who **Attacks** instead of
Holding) just **lowers the pool**. **Agency:** the run is informed — a Runner runs only
when the line can't afford to stop it; otherwise it **engages the front instead.**

## Creature targeting (from the deck)

A creature's targeting is part of its [behavior
deck](decision-making.md#environment-creatures--hazards-non-player):

- **Front line** — attack whoever stands in front (the default).
- **Priority target** — run for a preferred victim (the healer), **running the
  [gauntlet](#running-the-gauntlet)** to reach it. Only a guard **quick enough to catch
  it and strong enough to interrupt** stops it; lesser guards merely bleed its Speed.

## Why cardless

Lines, targets, and runs are **declarations**, not cards — the positioning layer,
kept separate from the tactical card exchange within each engagement, and free to
take a stance.

## Open questions

- How **Speed / Power** numbers scale across the power curve (shared with the
  [combat](combat.md) sketch).
- The **gate aggregate** — is the line's hindering pool the **sum** of Guard Speeds
  (first-pass), or a gentler function?
- We are **deliberately staying abstract** — a front line is an **unordered set**, no
  rows or positions. Revisit only if play surfaces a concrete reason to add them.
