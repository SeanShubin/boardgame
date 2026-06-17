# Deckbound — Coordination & Interruption

> **PARTIALLY SUPERSEDED — see Spec §3.** The breadth model (who fights whom, who's exposed,
> who's interrupted) is now [`spec/README.md` §3](../canon/2-spec/README.md): **Tempo** starts duels,
> **Focus** defends them (reset — survival only), uncovered foes **free-hit**, and a **Tempo
> counterattack** is the only way to damage an aggressor. The positioning/targeting intent
> here carries forward; the interrupt/Exposed specifics do not. The concrete round loop is
> still to be designed.

A layer above the card duel: not *how* an exchange resolves, but **who fights
whom**, who is exposed, and who gets interrupted. It is **cardless** — positioning
and target choice, not card plays.

## Attack or Hold

Each character takes one of two **stances** — a free declaration, no card:

- **Attack** — commit to a target: the opposing front line, or a **run** to their
  back line. You deal your attack, but you are **exposed everywhere else** — any
  *other* attacker auto-succeeds against you. You get the [RPS](decision-making.md)
  stance only against a target who is attacking you back (a mutual engagement).
- **Hold** — forgo attacking. Instead you make yourself **available to answer any
  attacker** (your safety) — but availability is not free coverage: you **negate** a blow
  only while your **focus pool** still covers it, each prediction costing the
  **attacker's Speed** out of focus (the
  [symmetric drain](speed-and-tempo.md#symmetric-drain--tempo-and-focus)). What your focus
  can't cover overflows and is **absorbed by toughness**, not magically predicted. You
  also add your tempo to the front line's **combined pool**, which it spends to **engage
  Runners** crossing toward the back line — see [the gauntlet](#running-the-gauntlet).
  Holding is how a back-liner buys safety at the cost of output, and how a front line
  becomes a **gauntlet**.

The whole positioning game turns on this: a front line that **Holds** gates the back
line; a front line that **Attacks** is aggressive but leaves the gate open. Push or
protect.

## The coherence principle

> The **RPS stance happens only when you are engaging your attacker** — by **Holding**
> (you are available to all comers), or by **mutually attacking** the same character.
> Against anyone you are *not* engaging, their attack **auto-succeeds**.

- **Hold** → you are **available to every** attacker, but **negate only while your focus
  pool covers them** (each prediction costs the attacker's Speed); the overflow is
  **absorbed by toughness**, not predicted.
- **Mutual attack** → a full [rock-paper-scissors](decision-making.md) duel.
- **An attacker you're not engaging** (you committed your Attack elsewhere) →
  **auto-succeeds** against you.

Consequences fall out for free: **attacking exposes you**, you **can't strike in two
directions and stay safe**, and **focus-fire punishes a thin Mind** — your **focus pool**
covers only so many attackers' Speed (one Speed-1 foe by default), while a Guard turns its
whole turn to defense and gating but lands no offense of its own.

### Breadth, prediction, and the gank

Offense and defense obey **different limits**, which is what makes the asymmetry fair:

- **Offense breadth hits everyone in reach** — a multi-target attack lands on all its
  targets regardless of your prediction budget (it's [breadth, not bandwidth](cards-and-customization.md#how-targets-reach-and-the-stance-interact)).
- **Prediction is focus-pool-limited.** You **negate** an attacker's blow only while your
  **focus pool** (sized to your **Mind**) covers it, each prediction costing the
  **attacker's Speed** out of focus — so **fast attackers are harder to wall** than slow
  ones. An attacker your focus can't afford to predict **free-strikes** you (an
  auto-success), and **toughness** absorbs what lands.
- **A target defends *your* attack only if it is predicting *you*** — and the two checks are
  independent. So **focusing a foe who's occupied elsewhere is a one-way gank:** you predict
  (and counter) their blows *and* land yours free, because their prediction is spent on someone
  else. The asymmetry is always **paid for by Mind** — the occupied side could buy the
  Mind to predict you back (see [engagement bandwidth](#speed-is-the-currency-of-engagement)).

Two consequences worth naming:

- **The cleave trades finesse for width.** One blind stance can't out-guess each foe;
  predicting and countering them separately takes **Mind** (to track the crowd) *and*
  **Speed** (to act on each), so a hero rich in both **duels** the crowd instead of
  **cleaving** it.
- **Being ganked is a Mind deficit.** One-way dominance is always a gap in the victim's
  **prediction** — closeable by buying the **Mind** to watch the extra attacker.

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
3. **Reveal & resolve** — stances flip **simultaneously**; the clash then settles by
   [tempo](speed-and-tempo.md) (who lands first) and the stance cycle. The full round
   (Form Up → Declare → Reveal → Clash → Recover) and how one blow **pre-empts** another
   is the **[resolution procedure](resolution.md)**. A defender predicts where engaged, else
   takes the hit.

Gating is **not** a special step — the front line simply **spends its combined tempo to
engage** Runners as part of resolution (see [the gauntlet](#running-the-gauntlet)).

## Pre-emption — stopping a foe's blow

You **cancel** a committed blow only by:

- **Dropping them first** — a [faster](speed-and-tempo.md) blow that fells the target
  before it can swing (inherent — no acting once felled); or
- **Out-predicting them** — the [stance cycle](mind-and-stances.md): a Defense negates a Strike,
  a Strike spoils a Scheme.

There is **no universal "Power interrupts" rule** — **Power is magnitude** (it cracks
armor and drops foes; *dropping* is what pre-empts). A deliberate non-lethal **stagger** —
"land first and the target loses its action" — is a **[keyword](keywords.md)** on cards
that earn it (a shield **Bash**), never something every blow does.

A **Runner** is stopped by the wall's **drag** (Speed), not by an interrupt: cover its
Speed and it's halted at the front — then a Guard's **Power** simply damages it.

## Speed is the currency of engagement

The split has a name — **"Speed swings, Mind reads, toughness endures"** (see
[speed & tempo](speed-and-tempo.md)): Speed is how many foes you **land** on, Mind is how
many you **predict**. Engagement has **two** mirrored "handle many" limits, one **pool**
per stat:

- **Speed = how many blows you can land.** Striking several foes (or several times)
  spends [tempo](speed-and-tempo.md): each engagement costs the target's Speed, paid
  after; **overextend** (go negative) and the extra blow still lands but leaves you
  **exposed**. Speed also catches Runners (the wall's [drag](#running-the-gauntlet)) and
  decides who lands first.
- **Mind = how many blows you can negate.** Prediction spends a **focus pool** sized to
  your **Mind**, the [mirror](speed-and-tempo.md#symmetric-drain--tempo-and-focus) of the
  tempo pool. Each defensive stance costs the **attacker's Speed** out of focus — so a
  **fast** attacker is **harder to wall** than a slow one, and a slow or overextended one
  is cheap to predict (an **inverse telegraph**). When your focus is spent, the foes you
  can no longer cover **free-hit** you; the **toughness** of your Health absorbs whatever
  lands. The old "one slot per attacker, up to Mind" is the **unweighted special case**
  (every foe Speed 1).

So a **blur of a duelist** strikes several and stops their runs (Speed), while a **sharp
mind** predicts several at once and can't be ganked (Mind) — offense-at-scale and
defense-at-scale on different stats. A thin Mind gets **focus-fired**; a thin Speed
**can't press a crowd**. Powerscaling stays **uncapped**: stack both (with the **Power**
to make the blows bite) and one character becomes a **one-man army** — the
[asymmetry](../canon/1-charter.md) pillar, now spread across stats so it isn't a Speed monopoly.

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
  - **d ≥ s → STOPPED** — halted at the wall; an engaging Guard then **strikes** it
    (its **Power** is the damage; a **[stagger](keywords.md)** card also cancels its action).
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
  [gauntlet](#running-the-gauntlet)** to reach it. Only the wall's **drag** (enough
  Speed) stops it; a Guard's **Power** then damages it, and a thinner wall merely bleeds
  its Speed.

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
- How **prediction** numbers scale across the power curve (with [speed & tempo](speed-and-tempo.md)).
