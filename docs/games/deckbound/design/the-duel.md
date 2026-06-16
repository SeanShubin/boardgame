# Deckbound — The Duel (the Clash)

> **Status:** the duel is **the Clash**, specced canonically in
> [`spec/README.md` §1.0](../spec/README.md). That section is the **source of truth for
> mechanics**; this note is its **design background** — the WHY behind the shape. Both the
> stance/Edge duel and the interim six-move *charge* duel this note once described are
> **superseded** (see [What this supersedes](#what-this-supersedes)); their intent carried
> forward, only the mechanics changed.

The tactical heart of combat: two fighters **predicting each other**. A duel is a **sequence
of beats**; each beat both fighters secretly choose one card, reveal at once, and the duel
**ends the instant one or both are struck**. The cards below are the moments *within* an
exchange, repeated beat by beat.

## What a duel is — hidden, simultaneous prediction

- A duel is a **mutual-prediction contest**, at any range — a melee swing, a thrown rock, a
  spell. It is *prediction*-specific, not melee-specific; reach can be lopsided, the
  prediction stays two-way.
- Choices are **hidden and simultaneous** — no one reveals first. Any "see their card, then
  choose" effect is a special ability layered on later, never the core.
- **"Perfect read" is a lens, not an action.** With no reveal mechanic, the invariants below
  are stated under *"suppose I happened to guess right"* — the analytical limit used to check
  the shape, not a move you can buy.
- A **creature does not read you back** — its instinct *is* its card
  ([decision-making](decision-making.md)); the mind-game lives on the side that reads.

## The four cards

One complete kit, always available (no hand to deplete):

| Card           | Meaning                               | Beats        | Stopped by |
| -------------- | ------------------------------------- | ------------ | ---------- |
| **Strike**     | hit *where they are now*              | Gather       | Evade      |
| **Anticipate** | hit *where they'll be* (lead them)    | Evade        | Gather     |
| **Gather**     | *hold your ground* + build Force (+1) | (Anticipate) | Strike     |
| **Evade**      | *move*                                | (Strike)     | Anticipate |

The two attacks read **position**: Strike commits to where they *are*, Anticipate leads to
where they'll *go*. The two non-attacks are **stay (Gather)** and **move (Evade)** — you stop
an attack by *matching its read* (hold beats a lead; move beats a now-strike) and eat it on a
mismatch. That one idea regenerates the whole table.

## The counter-cycle

**Anticipate ▸ Evade ▸ Strike ▸ Gather ▸ Anticipate** — each beats the next. Plus:

- **Strike > Anticipate** when both attack (the immediate blow beats the led one).
- **Strike vs Strike → trade** (both hit) — the hinge of invariant 3.
- **Anticipate vs Anticipate → whiff** (two leads at targets who didn't move).

Full table (result shown for the row player):

| you ↓ \ them → | Gather   | Evade             | Strike            | Anticipate |
| -------------- | -------- | ----------------- | ----------------- | ---------- |
| **Strike**     | you hit  | your Force → them | trade (both hit)  | you hit    |
| **Anticipate** | —        | you hit           | you're hit        | —          |
| **Gather**     | +1 Force | +1 Force          | you're hit        | +1 Force   |
| **Evade**      | —        | —                 | their Force → you | you're hit |

The *enders* — a strike connects, so the duel is over — are **you hit / you're hit / trade**.
Every other cell is the **non-connecting dance**, where Force builds and the duel continues.

## Force — escalation made visible

- A single **public count per side** — no hidden meter, no face-down state. Each Force
  **doubles** the connecting hit: damage = `base × 2^Force`.
- **Gather** adds +1. The **only** way Force changes hands is **Strike into Evade**: you
  commit a Strike, they slip it, and your Force **goes to them** — your own momentum turned
  against you. The re-derivable principle: *only an active dodge of a committed Strike
  reverses; the passive build (Gather) never steals.*
- **No cap** (unlimited) — building is bounded in practice by ends-on-strike (the duel ends
  when a blow lands), not by a ceiling.
- **Per-duel** — Force resets each duel; only **Body** persists between duels.

## Ends-on-strike — Body across duels

A duel ends the instant a strike connects (any *you hit / you're hit / trade* cell); the
connecting blow lands for `base × 2^Force`. Force is **built during the non-connecting dance**
(Gathers, whiffs, dodges) and **spent on the one blow that lands**. **Body persists across
duels**, so a fight to the death is several short duels of chip and spike — not one long
beat-count.

Termination is guaranteed in practice: under blind, simultaneous guessing someone eventually
misreads and a strike connects. An engine-only backstop ([spec §1.6](../spec/README.md))
breaks off the purely theoretical perfect-mutual-defense case; it is invisible in normal play.

## The three invariants — the heart of it

Under perfect guessing (*"suppose I guessed right every beat"*):

1. **Avoid.** You can pass a duel **un-hit** — every attack has a card that negates it
   (Strike ↦ Evade, Anticipate ↦ Gather).
2. **Land.** You can force a connecting hit — every move has an answering attack
   (Gather ↦ Strike, Evade ↦ Anticipate, Strike ↦ Strike-trade).
3. **Not both, free.** Landing on a committed **Strike** means **trading** a hit. You cannot
   have invulnerability *and* a free kill.

The deeper truth these encode — and the reason the breadth layer below matters — is
**survival is free; victory costs exposure**. Pure defense can keep you un-hit forever but
never *wins* (it deals no damage); to win you must attack, and attacking into resistance risks
the trade. This is **computable yomi**: defense and offense are both *complete*, so the duel is
a clean read rather than a guessing game, yet the trade cell forbids any dominant option.

## Tempo & Focus — the breadth layer

Tempo and Focus never gate *which cards you hold* (the kit is always complete). They gate
*which duels you are a full participant in* ([spec §3](../spec/README.md)):

- **Tempo = the duels you start.** Spend it to **initiate** (cost = the foe's Speed); inside,
  **results stick** — you can damage or kill.
- **Focus = the duels started on you.** Spend it to **defend**; you play the full duel, **but
  the attacker is reset afterward** — you can avoid, survive, and disengage, but **cannot
  damage the attacker**. Defense is **survival, never victory**.
- **No Focus → free hit** (you eat the blow, no duel).
- **Counterattack:** when attacked you may instead spend **Tempo** → a **mutual** clash where
  results stick both ways and the trade is live. So *every kill — initiated or countered —
  costs Tempo*: a single capped offense pool.

This is what makes "survival vs victory" real at scale, and it keeps a god **balanceable**:
kills tie to one pool (Tempo), being swarmed cannot *feed* a counter-reaper, and numbers stay
a genuine threat (overflow free-hits). **Pay-after** lets even a fighter too slow to afford a
foe take one action (its last); there is **no Exposed penalty** — the offense/defense balance
lives entirely in the Speed-vs-Mind split.

A Focus-defense's "can't damage the attacker" needs no new mechanic: *run the duel, then reset
the attacker*. A defender's own connecting strike simply **ends the exchange safely** (a clean
disengage) instead of wounding. So on defense your offense becomes a **deny-and-escape** tool —
Strike a Gathering attacker to break off before their loaded blow lands — and a *trade* is a
straight loss (you're hit, theirs is rolled back), so you never trade on defense.

## Gandalf vs Balrog — asymmetry by design

Because defense is complete, a **weak fighter can steal a duel on perfect reads** — avoid
everything, wait for an opening, chip back. That is the underdog's chance (Charter north star
#4). But the instant a read is wrong the doubled blow lands, and the downside is far worse for
the weaker side; and to actually *win* the underdog must reach for offense and risk the trade,
where the stronger fighter's bigger Force and Body win the exchange. So the upset is real but a
**bad bet**: survival is achievable, victory is where the underdog dies.

## Lineage

The fighting-game cluster of **mix-up + meter + yomi**: a position read (now vs led) over a
stay/move defense, a wind-up resource you pour into one blow (**Force**, the ×2), and a hard
read that turns the wind-up against you (the Strike-into-Evade steal). Ends-on-strike is "the
first clean touch resolves the exchange," with Body carrying the war across many exchanges.

## What this supersedes

- **Stance/Edge duel** (Marshal · Unleash · Overwhelm · Parry over a tracked **Edge** meter)
  → first replaced by a six-move *charge* Clash, now by the **four-card Force Clash**.
- **Six-move charge Clash** (Strike · Throw · Parry · Evade · Charge · Recover, with
  face-up/face-down **Charges** and **Body-attrition**) → replaced by **four cards**
  (Strike · Anticipate · Gather · Evade), a **single unbounded Force count** (steal only on
  Strike-into-Evade), and a return to **ends-on-strike**. The mapping: Hold + Charge + Recover
  → one **Gather**; the old around-the-guard **Throw** → **Strike** (hit-now); the old
  **Strike** (the led blow) → **Anticipate** (hit-future).
- **Tempo/Focus** rewritten from in-duel/coverage to pure breadth admission with
  **reset-defense + Tempo-counterattack**; the **Exposed** overextension penalty is removed.

## Open questions — tuning, not shape

- **The ×2 curve.** Is doubling-per-Force the right base, and how high does unbounded Force
  realistically climb under ends-on-strike? (`booklet.ron`.)
- **Counter-damage on defense.** Settled: a Focus-defense deals none (survival only); a
  **Tempo counterattack** is the way to deal damage when attacked.
- **The round loop.** The concrete sequencing — initiate-phase, foe-attacks, multi-foe
  defense, the counterattack choice — is **not yet designed**, and it gates the code.
