# Deckbound — Speed & Tempo

Speed is **not turn order** — and **Speed** and **tempo** are not the same thing.
**Speed** is a **stat** (a fixed rating); **tempo** is the **resource** you get from it:
each round you have a pool of tempo equal to your Speed, and you spend it to act — to
evade, to engage, to strike again. Speed also gives the faster fighter real advantages
*inside* a clash. This note is the whole system in one place;
[coordination](coordination-and-interruption.md) and [combat](combat.md) lean on it.

## Speed (the rating) and tempo (the resource)

Keep the two straight — they relate like **maximum** to **current**:

- **Speed** is the **stat**: a fixed number on your card that **never depletes**. It does
  two things — it sets the **size of your tempo pool**, and it's the **price others pay**
  to deal with you (evading or engaging *you* costs *your* Speed out of *their* tempo).
- **Tempo** is the **resource**: a pool that **starts equal to your Speed each round and
  depletes as you act.** What's left is your standing for the rest of the round, and it
  **resets every round.**

The spending rule is uniform:

> **You may act while your tempo is ≥ 0. After each action, subtract its cost. The
> action that takes you *negative* is your last — you have overextended.**

- You always get a **base action.** You start the round with tempo, so you can always
  do *something* — even if its cost drops you into the red.
- **Volume self-caps.** Keep acting while you have tempo; crossing into the negative
  stops you. Your tempo *is* the limit — no separate "how many actions" rule.

Pay **after**, never before: you don't have to "afford" an action up front. That is why
a slow fighter can still act, and why the **negative line — not zero — is the wall.**

At the table this needs **no tokens**: tempo is a single running number you keep in your
head and can **re-derive any time** from your Speed minus the actions visible in play
(the foes you've engaged, the strikes you've made). State you can always reconstruct from the
table is fair game — see [representation](physical-representation.md).

## What an action costs

Acting on someone — **engaging or striking** them — costs **their Speed** from your
tempo; faster foes take more to deal with. You may go **negative** (overextend) to land
one more blow, at the price of being [exposed](#overextending). That's the whole cost
rule: **one currency, spent on engagements.**

The **gauntlet** is that same rule pointed *outward*: a front line spends its **combined
tempo as drag** on the [Runners](coordination-and-interruption.md#running-the-gauntlet)
crossing it. A Runner doesn't pay to run — the **Guards pay to slow or stop it** (drag ≥
its Speed stops it; less only trims its lead). A Runner's tempo stays its own, for its
attack; the wall just **subtracts** from it.

## Overextending

The action that drives your pool **negative is an overextension**: it resolves, but
you've spent past your means and you're **exposed** for the rest of the round — you sit
at the **bottom of the tempo order**, so you **get hit first and can't respond**
([first strike](#speed-in-a-clash--first-strike)). Overextending to land a killing blow,
or to stop one more foe, is a real option at a real price. (This is the old
"overextend to oppose one more, and take a hit" made into one clean line.)

## Speed in a clash — first strike

Nothing resolves "in Speed order." Reads are chosen **simultaneously and blind** (the
[hidden-information](decision-making.md) core is intact); then, when two blows clash,
**whoever has more tempo right then lands first:**

- **Faster lands first.** If that blow **drops** the target — or carries a
  [stagger](keywords.md) — the slower's never happens; otherwise both blows land.
- **Equal tempo → both land.** Simultaneous trades are real, and **mutual kills are
  possible** — every point of Speed matters.
- **The leftover is the telegraph.** A wall's **drag** trims a
  [Runner](coordination-and-interruption.md#running-the-gauntlet)'s lead, so it can reach
  the back line with little tempo left; the fresh defender then out-tempos it and lands
  first. Arrive overextended and you eat the first blow with no reply.

Landing first is **resolution** order, **not information** — nobody reveals their read
early. **Speed sequences the blows, not the secrets.**

## Speed vs Power — timing vs force

The two split cleanly, **no overlap**:

- **Speed = who lands first.** More leftover tempo strikes first; equal tempo trades.
- **Power = how hard it lands** — magnitude: whether it cracks armor and toughness, and
  whether it **drops** the target. Power has **no separate "interrupt" job**.

So you **stop** a foe's blow only two ways: **drop them first** (a lethal first-strike —
no swinging once felled), or **out-read them** (the [cycle](mind-and-reads.md): a Defense
negates a Strike, a Strike spoils a Scheme). A deliberate non-lethal **stagger** is a
**[keyword](keywords.md)** on cards that earn it (a shield **Bash**), not a universal
rule.

So Speed is never an auto-win: a glass-cannon speedster lands first but, without the
**Power** to drop a tough foe, just trades blows and may **still die** to the heavier
hitter. **Speed buys timing; Power buys the killing weight.**

## Volume — extra hits

Surplus tempo buys **extra strikes** — on the same target or others. Each costs the
target's Speed; "match again" and you get another blow, then another, until the pool
runs dry. Enough Speed (with the Power and capabilities to match) makes a character a
**whirlwind striking several foes in a round** — the uncapped
[one-man army](coordination-and-interruption.md#speed-is-the-currency-of-engagement) of
the asymmetry pillar.

## The three stats, divided cleanly

| Stat                          | Job in a clash                                                                                                             |
| ----------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| **[Mind](mind-and-reads.md)** | the **read** (which move, depth, bluff), **how many foes you can read at once** (defensive bandwidth), Precision, recovery |
| **Speed**                     | **tempo** — *who lands first*, *how many you can strike* (offensive volume), and catching Runners (drag)                   |
| **Power**                     | **force** — magnitude: cracking armor / toughness and **dropping** the target (no separate interrupt)                      |

No overlap: **Mind picks the move and reads the crowd; Speed times and multiplies your
blows; Power gives them killing weight.**

## Open questions

- The exact **drain** (a guard's full Speed, or a function of it) and how **leftover
  tempo** converts to first-strike margin at the level of numbers.
- Whether **ties** (equal tempo) are strictly "both land," or take a finer tiebreak.
- How Speed's **volume** is bounded against Power so neither stat eclipses the other up
  the power curve.
- What **base costs** non-combat actions (a spell, a Scheme) draw from the same pool, if
  any.
