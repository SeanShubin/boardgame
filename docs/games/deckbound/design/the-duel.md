# Deckbound — The Duel (the Clash)

> **Status:** the duel is now **the Clash**, specced canonically in
> [`spec/README.md` §1.0](../spec/README.md#10-the-clash--beats-six-moves-charges)
> — that section is the **source of truth for mechanics**; this note is its
> **design background** (the WHY behind the shape). The earlier stance/Edge duel
> (Marshal · Unleash · Overwhelm · Parry over a tracked Edge meter) it once
> described is **superseded** — see [What this supersedes](#what-this-supersedes).
> Its intent carried forward intact; only the mechanics changed.

The tactical heart of combat: two fighters **predicting each other**. A duel is the
fighting-game **strike / throw / block** mix-up, played as cards — and Deckbound's
version is the **Clash**: a duel is a **sequence of beats**, each beat both fighters
secretly pick one move and reveal at once, and the Clash runs **until a Body reaches
0** (Body-attrition — not "ends on the first strike"). The moves below are the
moments *within* an exchange, repeated beat by beat, not a fight-long timeline.

## What a duel is — prediction, at any range

A **duel** is a mutual-prediction contest between two actors. It is **not
melee-specific** — it is *prediction*-specific, at any range.

- A thrown rock against someone dodging *is* a duel: you throw where you think
  they'll go; they read it. The whole beat-by-beat mind-game is that exchange.
- It is about **prediction, not symmetric offense.** Reach can be lopsided; the
  prediction stays two-way.
- **No duel → direct resolution.** Mindless fodder, or a target that cannot
  respond to you, just takes the hit by magnitude — the one-way vs two-way
  prediction distinction from
  [mind-and-stances](mind-and-stances.md#against-instinct-vs-against-a-mind). A
  creature does not read you back: its instinct *is* its move, so the duel lives on
  the side that reads.

## The six moves — two kinds

Each beat, each fighter secretly commits **one of six moves** and reveals at once.
They split by temperament into **standing** moves and **setups**.

**Standing** *(always available, never deplete)* — the four that resolve a beat:

- **Strike** *(offense)* — a direct blow.
- **Throw** *(offense)* — a blow aimed *through* a guard.
- **Parry** *(defense)* — read a Strike and negate it.
- **Evade** *(defense)* — read a Throw and slip it.

**Setups** *(the escalation resource — durable face-up cards)* — the two that wind
up rather than resolve:

- **Charge** — place one active **Charge** in the open. Each active Charge
  **doubles** your attack damage (**×2 per Charge**, so *n* Charges = ×2ⁿ). Charge
  capacity is a per-actor stat ([booklet](../../../../crates/deckbound/data/booklet.ron)),
  so Charge is offered only below capacity.
- **Recover** — flip your own **face-down** Charges back up (offered only when you
  have one). Charges are *disabled* by a successful defense, not destroyed; Recover
  re-arms them.

The crucial property: **the standing moves never deplete.** A perfect reader can
*always* answer the move in front of them, every beat, for the whole duel. That is
what makes the invariants below hold across the entire Clash, not just one exchange.

## The counter-cycle

Re-derivable as one small table:

- **Cycle.** **Strike ▸ Evade ▸ Throw ▸ Parry ▸ Strike** — each attack beats one
  defense and loses to the other:
  - **Strike beats Evade** (you can't dodge a direct blow) and **loses to Parry**.
  - **Throw beats Parry** (the throw goes through a guard) and **loses to Evade**.
- **Trade.** **Strike vs Strike → both hit.** And **Strike clips Throw** (Strike >
  Throw): the striker lands, the thrower does not.
- **Attacks beat setups.** A connecting Strike or Throw **hits and interrupts** a
  Charging or Recovering foe — the setup does not resolve that beat. Winding up in
  front of an attacker is punished by the attack itself; no bespoke "interrupt" rule
  is needed.
- **Setups resolve if unopposed.** Against anything that does *not* connect (a
  defense, or the opponent's own setup), Charge places its Charge and Recover
  re-arms its flipped ones.
- **The defense flips charges — the comeback.** A *successful* defense (a Parry that
  catches a Strike, an Evade that slips a Throw) **flips the attacker's active
  Charges face-down** — disabled, not destroyed. A read wind-up, met by the right
  guard, is knocked down rather than spent into the void; Recover brings it back.
- **Damage** = `power × 2^(active Charges)`, routed through the armor/toughness
  pipeline ([defense model](form-and-defeat.md), spec §2). Body 0 = down.

## The three invariants — the heart of it

Read under **last-word reads** — the opponent commits face-up, then you choose
(the perfect-read limit). Three things must all hold; the design is the unique shape
that buys all three at once.

1. **Avoid.** Spending the *defensive* read, you can pass through the **whole** duel
   **un-hit** if you choose — every attack has a standing defense that negates it
   (Strike ↦ Parry, Throw ↦ Evade) and the defenses never deplete.
2. **Land.** Spending the *offensive* read, you can land a hit by the end if you
   choose — for every move the opponent can make, some standing attack lands (Throw
   beats Parry; Strike beats Evade, trades into Strike, clips Throw; either attack
   hits a setup).
3. **Not both, free.** You **cannot guarantee both at once.** Against a committed
   **Strike**, the *only* landing answer is Strike — and Strike-vs-Strike **trades**.
   Landing on a committed attacker means **taking a hit** too.

This is **computable yomi**: defense is *complete* and offense is *complete*, so the
game is a clean read rather than a guessing game — yet the trade cell forbids a free
win, so there is **no dominant option**. (The old "always-Parry is safe" hole is
closed the same way it always was: Throw beats Parry, so a parry-crouch just gets
thrown — and now the trade also forbids landing for free against a Strike.)

## Charges — escalation made visible

The old duel banked a hidden-ish **Edge** number; the Clash replaces it with
**Charges** — durable, **face-up**, per-duel cards. The reasons it changed:

- **Visible and durable.** A Charge is a card on the table, not a meter in the head.
  Both fighters see the wind-up looming and must respect it — yomi over a public
  quantity, not a hidden one.
- **Multiplicative, so a completed wind-up is genuinely lethal.** ×2 per Charge
  means a foe who lands a two-Charge blow hits for **four times** their base. The
  payoff is what makes charging worth the exposure.
- **The defended Charge flips, not vanishes — the comeback.** Catch a charged
  attacker with the right defense and you knock their Charges *down*; they have to
  spend beats to Recover them. The lead swings without the wind-up being destroyed
  outright (the same comeback the Parry-steal used to provide).
- **Per-duel.** Charges reset each duel and never carry between duels — even two
  duels involving the same Actor. No fight-long hoard, no runaway snowball; facing a
  crowd is a stack of independent short Clashes, never one accumulating super-bank.

## Body-attrition — the duel runs to 0

A Clash does **not** end on the first strike. It runs **beat by beat until a
fighter's Body reaches 0**. This is the change the charge → big-hit arc requires: if
a single hit ended the duel, winding up a doubling blow would be pointless — you'd
just poke. Because a hit no longer ends it, **charging is meaningful**, the mix-up
plays out over several beats, and the comeback (flipped Charges) has time to matter.

**Termination is still guaranteed.** Every Clash ends at Body 0; and an engine-only
backstop ([spec §1.6](../spec/README.md)) breaks off a Clash that makes no progress
for *N* beats — the corner case where armor/toughness fully absorbs every connecting
hit so neither side can wound the other. The backstop is invisible in normal play
and not part of the public rules.

## The shape of a duel

- **Floor.** Two fighters who never charge trade base blows until one falls — short,
  no mind-game. Escalation is **opt-in**.
- **Escalation = push-your-luck.** Charge to wind up a doubling blow, but every beat
  you spend charging is a beat you might be **read and interrupted** (an attack hits
  a setup) or, once charged, **defended and knocked down** (a successful guard flips
  your Charges). Greed buys a bigger payoff at more exposure.
- **The read decides it.** Against a committed move there is always a right answer;
  the duel is won by *reading which move is coming* and paying the right resource —
  defense to avoid, offense to land, knowing you cannot have both free.

## Gandalf vs Balrog — asymmetry by design

The invariants are what make a **weak fighter able to steal a duel on perfect
reads** — defense is complete, so a flawless reader can avoid everything and chip
back. That is the Gandalf-vs-Balrog fantasy (Charter north star #4): the underdog
*can* win the exchange.

But the **instant a read is wrong, the doubled blow lands** — and the downside is far
worse for the weaker side (less Body, less Power, less to absorb the ×2). So over
many duels the upset is a **bad bet**: the asymmetry lets the underdog win *a* duel
with perfect play, while the math still favors the stronger fighter across the war.
Folding the **trade** into the cycle (Strike-vs-Strike = both hit) is exactly what
forbids buying the win for free — you cannot land on a committed striker without
trading — so neither invariant can be had cheaply.

## Facing a crowd — K Clashes, two caps

A Clash is pairwise, so engaging several foes is **K simultaneous pairwise Clashes**
(or one breadth-attack, which reads no one and stays unopposed). Two separate
per-Actor pools gate K (spec §1.7):

- **Speed / Tempo** caps how many you can sustain **offensively** — engaging each
  costs the target's Speed from your Tempo.
- **Mind / Focus** caps how many you can **cover** **defensively** — each costs the
  attacker's Speed from your Focus.

When Speed affords **K** but Focus covers only **J < K**, the **K − J** extra foes
are **one-way**: you strike them, but can't cover them, so they **free-hit** you.

**A reconciliation note:** in the old duel, Focus gated your *stance menu inside* a
clash — without a read you could only swing. In the Clash **all six moves are
standing**, so there is **no Focus gate inside a duel**. Focus is now purely the
**breadth** resource — round-end coverage of foes you did not engage. What carries
over unchanged: engaging costs Tempo, an engaged foe doesn't also free-hit, breadth
and self/ally actions are unopposed, and overextending in any one pool marks you
**Exposed** table-wide (spec §3.3).

## Lineage

The closest solved version is the fighting-game cluster of **mix-up + meter + yomi**:
the **strike / throw / block** triangle (Strike / Throw / Parry-Evade), a wind-up
resource you can pop for a big payoff (**Charge**, the ×2), and a hard read that
knocks the wind-up down (a successful defense flipping Charges). Charges-as-cards are
the brawler's **breakable charge** made durable and public; Body-attrition is the
"duel runs until someone drops," not "first touch wins"; *Doom Eternal* contributes
"play aggressively to fuel yourself."

## What this supersedes

- The stance/Edge duel **Marshal · Unleash · Overwhelm · Parry** over a tracked
  **Edge** meter → replaced by the **six-move Clash** (Strike · Throw · Parry ·
  Evade · Charge · Recover) with **Charges** as the escalation resource. The
  mapping: the offensive triangle (no dominant option, the throw beating the block)
  → the §1.0 cycle; the Parry-steal comeback → the **defended Charge flips
  face-down**; per-duel public Edge → per-duel public **Charges**; Edge's linear
  `+1/Edge` damage → Charges' multiplicative `×2/Charge`; **ends-on-strike** →
  **Body-attrition**.
- Still earlier: Strike / Block / Evade / Scheme as the RPS cycle, and the
  Power/Speed/Precision momentum banks — both already retired by the stance/Edge
  duel and now by the Clash.

## Open questions — tuning, not shape

- **Charge capacity and the ×2 curve** — how many Charges a fighter can hold, and
  whether ×2 is the right base; numbers live in `booklet.ron`.
- **Counter-damage on a defense.** Does a successful Parry/Evade also *deal* damage
  (beyond flipping Charges), or only negate + disable? (Was open under the old duel
  too.)
- **Throw vs Throw, Charge vs Charge.** Provisional reads — two throws clinch
  (nothing connects), two charges both resolve. Confirm in play.
- **Natural duel length.** Tune Body, Power, and Charge payoff so a Clash runs a
  handful of beats — a real exchange, not a long dance. (Pacing; termination is
  settled above.)
</content>
</invoke>
