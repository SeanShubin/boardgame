# Deckbound — Speed & Tempo

> **PARTIALLY SUPERSEDED — see Spec §3.** Speed→Tempo and Mind→Focus survive as the two
> breadth budgets, but their *roles* are rewritten
> ([`spec/README.md` §3](../spec/README.md)): **Tempo** = the duels you *start* (results
> stick), **Focus** = the duels *started on you* (a Focus-defense is **reset** — survival
> only, no damage to the attacker), a **free hit** if uncovered, and a **Tempo
> counterattack** to fight back. **Pay-after is kept; the Exposed / Focus→0 penalty is
> removed.** Read for the stat-vs-resource framing; trust §3 for how the budgets are spent.

Speed is **not turn order** — and **Speed** and **tempo** are not the same thing.
**Speed** is a **stat** (a fixed rating); **tempo** is the **resource** you get from it:
each round you have a pool of tempo equal to your Speed, and you spend it to act — to
evade, to engage, to strike again. Speed also gives the faster fighter real advantages
*inside* a clash. This note is the whole system in one place;
[coordination](coordination-and-interruption.md) and [combat](combat.md) lean on it.

The whole split fits in one line:

> **Speed swings, Mind reads, toughness endures.**

Speed is **offense** (how many foes you strike, which Runners you catch, and
first-strike pre-emption); **Mind** is **active defense** (how many incoming blows you
negate by [predicting](mind-and-stances.md) the foe's stance); and **toughness** is
**passive defense** (how many blows you simply absorb without predicting). Offense and
defense **mirror** each other — a tempo pool to strike, a focus pool to predict — which
the [symmetric drain](#symmetric-drain--tempo-and-focus) below makes exact.

## Speed (the rating) and tempo (the resource)

Keep the two straight — they relate like **maximum** to **current**:

- **Speed** is the **stat**: a fixed number on your card that **never depletes**. It does
  two things — it sets the **size of your tempo pool**, and it's the **price others pay**
  to deal with you on **both** axes: striking *you* costs *your* Speed out of *their*
  tempo, and predicting *you* costs *your* Speed out of *their* focus (see
  [symmetric drain](#symmetric-drain--tempo-and-focus)).
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

Nothing resolves "in Speed order." Stances are chosen **simultaneously and blind** (the
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

Landing first is **resolution** order, **not information** — nobody reveals their stance
early. **Speed sequences the blows, not the secrets.**

## Speed vs Power — timing vs force

The two split cleanly, **no overlap**:

- **Speed = who lands first.** More leftover tempo strikes first; equal tempo trades.
- **Power = how hard it lands** — magnitude: whether it cracks armor and toughness, and
  whether it **drops** the target. Power has **no separate "interrupt" job**.

So you **stop** a foe's blow only two ways: **drop them first** (a lethal first-strike —
no swinging once felled), or **out-predict them** (the [cycle](mind-and-stances.md): a Defense
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

## Symmetric drain — tempo and focus

Defense is the **mirror** of offense, pool for pool:

- **Offense** spends a **tempo** pool sized to your **Speed**; each strike costs the
  **target's Speed**.
- **Defense** spends a **focus** pool sized to your **Mind**; each prediction (a
  defensive [stance](mind-and-stances.md)) costs the **attacker's Speed**.

Both pools **refresh each round**, and you spend them **per duel, not per blow** — a duel
is a single, **ends-on-strike** exchange ([the duel](the-duel.md)), so paying a foe's
Speed **once** (out of tempo to engage it, out of focus to cover it) buys that one
strike's worth of threat or defense; per-duel and per-blow coincide. Engage a duel your
focus can't **cover** and it goes **one-way**: you strike, but the foe **free-hits** back.
Both pools also let you **overextend for one more** — an extra strike, or one more
attacker covered — but going **negative in any single duel exposes you table-wide**: you
drop to the bottom of the [first-strike order](#speed-in-a-clash--first-strike) in
**every** duel this round, so greed against a crowd is lethal. The two are the same shape
pointed opposite ways.

This unifies the whole stat into one sentence:

> **Speed is how hard an actor is to deal with** — the price *others* pay, out of their
> **tempo** to strike it and out of their **focus** to predict it.

Slow foes are **cheap on both axes**; fast foes **dear on both**. The defensive side is
an **inverse telegraph**: a slow or overextended attacker is heavily telegraphed, so it's
**cheap to predict**; a fast, fresh one **costs more focus** to read. The old rule — *one
prediction slot per attacker, up to Mind* — is just the **unweighted special case** where
every foe has Speed 1; weighting each prediction by the attacker's Speed generalizes it.

One **hard decision** keeps the stats balanced: attacker Speed drains the defender's
**focus** (Mind) pool, **not** their tempo pool. Routing prediction through Mind prevents
a single fast fighter from owning **both** offense and defense — Speed can't monopolize
the table.

## Three survival tools vs a crowd

Facing a swarm, you have **three non-overlapping** answers to each incoming blow:

- **Predict it** — spend **focus** ([Mind](mind-and-stances.md)) to negate it.
- **Kill it first** — spend **tempo** (**Speed**) to [pre-empt](#speed-in-a-clash--first-strike)
  it: drop the attacker before it swings.
- **Eat it** — let **toughness** absorb it with no prediction at all.

So your **own Speed** helps you survive a crowd by **pre-emption** — killing into the
crowd first, thinning the blows that ever land — **not** by widening defensive bandwidth
(that job is Mind's). Speed kills the threat; Mind reads it; toughness outlasts it.

## The three stats, divided cleanly

| Stat                            | Job in a clash                                                                                                                                                                                                                          |
| ------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **[Mind](mind-and-stances.md)** | the **stance** (which move, depth, bluff), the **focus pool** — how many incoming blows you negate by predicting (active-defense breadth), Precision, recovery                                                                          |
| **Speed**                       | the **tempo pool** — *who lands first*, *how many you strike* (offensive breadth), catching Runners (drag), and killing first (pre-emption)                                                                                             |
| **toughness**                   | **endurance** — how many blows you **absorb** without predicting (passive defense), the per-card capacity governed by the [Vitality card](form-and-defeat.md#how-damage-resolves--the-vitality-card-and-health-cards) over Health cards |
| **Power**                       | **force** — magnitude: cracking armor / toughness and **dropping** the target (no separate interrupt)                                                                                                                                   |

No overlap: **Speed swings (and pre-empts), Mind reads, toughness endures, and Power
gives the blows killing weight.**

## Open questions

- The exact **drain** (a guard's full Speed, or a function of it) and how **leftover
  tempo** converts to first-strike margin at the level of numbers.
- Whether **ties** (equal tempo) are strictly "both land," or take a finer tiebreak.
- How Speed's **volume** is bounded against Power so neither stat eclipses the other up
  the power curve.
- **A cast's tempo cost (RESOLVED).** A cast is a **physical action**, so it draws
  **tempo** from the same pool as any action — no separate magic resource. The open
  number is only its **base cost** relative to other actions.
- What **base costs** other non-combat actions (a Scheme) draw from the same pool, if any.
- What **base costs** non-combat actions draw from focus, if any (a defensive Scheme).
