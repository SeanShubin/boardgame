# Deckbound — The Mind: Stances & Momentum

> **SUPERSEDED — tactical layer.** The canonical tactical core is now the **Clash**
> ([`spec/README.md` §1](../spec/README.md),
> design rationale in [the-duel.md](the-duel.md)): a beat-by-beat **four-card** duel —
> **Strike · Anticipate · Gather · Evade** — with a single **Force** count (×2 per Force,
> stolen only on Strike-into-Evade), run **ends-on-strike**. Both the
> Strike / Block / Evade / Scheme cycle *and* the Power / Speed / Precision momentum
> banks below are superseded (as is the interim Marshal/Unleash/Overwhelm/Parry Edge
> duel). What carries forward: the **Mind-aspect framing**, **one-way vs two-way
> prediction**, and the **Focus pool** (now purely the *breadth* resource — there is
> no Focus gate *inside* a Clash; see the-duel.md). Read for intent; trust §1.0 for
> mechanics.

The Mind aspect is the **rock-paper-scissors stance** — the hidden-information heart of
the game. The Body supplies Power, Speed, and the physical delivery; the **Mind**
chooses the *intent* (the stance) and turns winning stances into snowballing advantage.
No Mind means you can only **Strike** — swing, predictably; a Mind unlocks the whole
cycle.

## The four stances

The stances are **cards that start in Potential**, and they part ways by temperament.
The defensive and setup stances — **Block, Evade, Scheme** — carry a **self-return**
effect (returns to hand after the exchange), so cautious play never exhausts you.
The aggressive **Strike** instead **exhausts** (turned face down): committing offense
spends you, which is why a striker leans on **recovery** (a Mind tactic, often a
teammate's) to keep swinging. Access to the stances is granted by your **Mind
capability** in [Form](form-and-defeat.md); seal the Mind and Block / Evade / Scheme
go with it, leaving only the bare Body **Strike**.

The **stance** you play is separate from your **engagement**: engagement (Attack or Hold) is
*whom you engage* on the [battlefield](coordination-and-interruption.md); the stance is
*how you play the clash* once engaged. You can Attack a target and still Block its
counter.

- **Strike** — attack now. Direct damage; faster strikes resolve first, and enough
  **Power** drops a foe before they can swing back. This is the **cash-in**.
- **Block** — absorb a strike. Fast, doable while immobile. Win → bank **Power**.
- **Evade** — dodge a strike and reposition. Win → bank **Speed**.
- **Scheme** — set up. Gain position / distance, but very exposed to a Strike. Win
  (uninterrupted) → bank **Power + Speed + Precision** — the big, risky haul.

## The cycle

Rock-paper-scissors with a twist — **Strike → Scheme → Defense → Strike**:

- **Strike beats Scheme** — interrupt the setup.
- **Defense (Block / Evade) beats Strike** — absorb or dodge.
- **Scheme beats Defense** — you set up while they guarded nothing.

Block and Evade are two **Defenses** — same role, different payoff and feel (Block is
fast and works immobile; Evade repositions). Same-vs-same and Block-vs-Evade are
**mirrors**, settled by the [magnitude layer](combat.md) — Speed order and Power.

## Momentum — the thrill

Winning a stance **banks advantage** as Power / Speed / Precision cards in your
**Active** zone. The bank **persists across exchanges and is uncapped** — the more
upside you haven't yet cashed, the more you stand to lose.

**Cashing in** — each banked quality has its own conversion:

| Banked        | Spend it on                                                                                                  |
| ------------- | ------------------------------------------------------------------------------------------------------------ |
| **Power**     | **attach** to a Strike ([modifiers](decks-and-aspects.md#kinds-of-card)) — a blow too strong to Block        |
| **Speed**     | an **extra, unopposed action** — you act again before they can respond; it auto-succeeds, nothing to predict |
| **Precision** | a **weak-spot** hit — bypass armor / bonus damage (the Mind's gift)                                          |

**Losing the bank — the misjudged stance.** Lose a stance (your intent is countered: Scheme
struck, Strike defended, Defense schemed) and you **forfeit your *entire* accumulated
bank** by default. Some **Form** cards — a steady temperament — mitigate this. Ties
and mirrors cost nothing.

So a duel is **build vs cash-in under push-your-luck**: stack advantage with Defense
and Scheme, convert it with a Strike or an extra turn — but every stance risks the whole
pile. The original instincts fall straight out:

- They keep the **pressure** on → **Block** to bank Power and eventually overpower
  them.
- They **turtle** → **Scheme** to stack until your Strike beats their Block.
- They throw **slow, heavy** strikes → **Evade** for Speed and a counter-tempo extra
  action.

## Against instinct vs against a mind

- vs a **creature** — no [theory of mind](decision-making.md#the-line-theory-of-mind):
  prediction is **one-way**. You study its rule-based instinct and out-predict it; it reacts
  to the board but never to you.
- vs a **mind** — a Trickster, another player, a stand-in: prediction is **two-way**, a
  true bluff war. It baits your Scheme with a Strike and adapts to your habits, so
  *when to cash in* is itself something to predict.

The exchange stays **computable** ([philosophy §2](philosophy.md#2-computable-tactics-uncomputable-strategy)):
bounded stances plus a countable bank.

## How many you can predict — bandwidth is the Mind focus pool

Mind is an **inner-aspect capacity** — a standing **threshold**, not a stack that
chips away ([decks-and-aspects](decks-and-aspects.md#kinds-of-card)). Concretely it is
a **focus pool** sized to your Mind stat, refreshed every round, and **each defensive
prediction drains the attacker's Speed** from it. You predict the foes your focus can
cover; the ones it **can't** cover **[free-hit](coordination-and-interruption.md#the-coherence-principle)**
you (their stances auto-succeed). So **Mind is the anti-gank stat**: a deep pool tracks
and counters several foes at once; a thin one is focus-fired. **Prediction-breadth *is*
the Mind stat** — its other duties (Precision, recovery, read-quality) are a separate
stat, a capability card, and player skill, not part of this number. And because a
[duel ends on a single strike](the-duel.md), you pay an attacker's Speed **once per duel**
to cover it; per-duel and per-strike coincide.

The old "**one prediction per attacker, up to Mind**" rule is just the **unweighted
special case** — every foe at Speed 1, so a pool of *n* covers *n* attackers. Weight
each prediction by the attacker's Speed and the pool drains unevenly: a few fast foes
can spend your whole pool, while many slow ones cost little. This **mirrors offense**,
where a tempo pool sized to **Speed** is drained by each strike costing the *target's*
Speed (Speed governs how many you *act on*; Mind, how many you *predict*).

**Confusion shrinks the pool.** [Confusion](decks-and-aspects.md) doesn't chip a stack —
it **lowers the Mind capacity**, so fewer (or none) of an attacker's stances can be
covered and the rest auto-succeed against the confused defender (this is the
"**blind its prediction**" attack vector).

### Mind-capacity is the inner bar; Ward (vs-confusion) is the inner cut

The inner channel for confusion is **cut → bar → break**, with **no pool of its own**:
the **Mind-capacity (the focus pool's size) is the inner BAR** that accumulated confusion
must **exceed** to break the channel. Before confusion reaches that bar it is shaved by
**Ward (vs-confusion)** — the inner **cut**: a **passive, typed, per-source,
never-depleting** value, a **number on a card, not a meter or track**, that reduces the
magnitude of **each** confusion hit **per source** before it tests Mind-capacity — strong
against many small confusion hits, weak against one big one (same logic as Armor /
Toughness). Ward is **not anti-magic**; it guards only confusion here (and fear, in
[Spirit](spirit.md)).

**The two stack.** Ward **cuts the incoming hit**; the bar **is** Mind-capacity. And
confusion still **lowers** Mind-capacity as documented above — so a confusion hit is
first shaved by Ward (per source), then its remainder both adds to the round's pile
tested against the bar **and** drags the capacity itself down, tightening the bar for
what follows. Unspent confusion clears at round's end; a confusion break is a this-round
event.

**Two opposite cognitive directions.** Mind carries prediction in both directions at
once: *predicting* a foe (covering its stances with your focus) and *being predicted*
(your own unpredictability). Collapse a fighter's **own** unpredictability and it can
only throw the bare Body **Strike** — predictable like a Creature. These stay **one Mind
stat — no Read / Poise split.** *Predicting* is the focus pool (your capacity to play the
game); the **positional advantage** that out-predicting earns is already
**[Edge](the-duel.md#edge--per-duel-all-in-linear-public)** — the per-duel upper-hand you
maneuver for (built by Marshal, **stolen by a Parry**) — so the "poise" the split chased
needs no stat of its own. *Being predicted* lives in the [exhaustion](zones.md) and
[theory-of-mind](decision-making.md) systems. Sealing the Mind collapses **both**
directions at once — the deepest cut. (A *Poise* trait that resists being read is reserved
as a future special card.)

## One stance against many

A [multi-target attack](cards-and-customization.md#how-targets-reach-and-the-stance-interact)
commits **one** stance to all its targets and **resolves pairwise** — each engaged defender
predicts back, and whoever predicts it right negates it *for themselves* (the rest eat it).
**Breadth forgoes prediction:** you can't out-guess several foes with a single stance, so
going wide **trades the prediction advantage** that single-target dueling buys.

## Open questions

- Exact **bank → effect** conversions: how much Power makes a Strike unblockable, how
  much Speed buys an extra action, what Precision bypasses.
- Do **special tactic cards** (feints, counters, mind-games) modify the basic cycle,
  and how do they enter/leave play relative to the self-returning core stances?
- Does a won **Defense** ever land a **counter-hit**, or only negate + bank (+ a Speed
  extra action)?
- How **mitigation** Form cards phrase "keep some bank on a misjudged stance."
- Whether **Scheme**'s position/distance advantage ties into the
  [front/back-line](coordination-and-interruption.md) positioning.
- *(Settled: Mind stays one stat — the positional "poise" half is **Edge**, the
  defensive half is **exhaustion**; no Read / Poise split. A Poise trait is a future
  special card.)*
