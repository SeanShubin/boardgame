# Deckbound — The Duel

> **Status:** **implemented** as the duel sandbox (`crates/deckbound`) — combat is
> now a sequence of these duels (Marshal / Unleash / Overwhelm / Parry, public
> per-duel Edge), with creature read-policies and tutorials that isolate each
> read. The older Strike/Block/Evade/Scheme warband (formation, gauntlet, fear,
> multi-target) is parked while the duel numbers are tuned. Team and god-tier test
> scenarios add matchmaking (you pick who duels whom) and a simplified swarm rule
> (foes beyond your **bandwidth** chip you each beat — the multi-engagement
> approximation). Numbers are first-pass and live in `data/booklet.ron`.

The tactical heart of combat: two fighters **reading each other**. A whole duel
is the mind-game of **a single clash** — one throw, one melee exchange — and it
runs to completion **inside one round** (the round's **Clash** phase). So the
reads below are the moments *within* an attack, not a fight-long timeline.

## What a duel is — anticipation, at any range, within one clash

A **duel** is a mutual-anticipation contest between two actors, resolved in a
single round. It is **not melee-specific** — it is *anticipation*-specific, at any
range.

- A thrown rock against someone dodging *is* a duel: you throw where you think
  they'll go; they dodge, or read it. The whole series of reads is the mind-game
  of that one throw.
- It is about **anticipation, not symmetric offense.** Reach can be lopsided; the
  read stays two-way.
- **No duel → direct resolution.** Mindless fodder, or a target that cannot
  respond to you, just takes the hit by magnitude — the one-way vs two-way read
  distinction from [mind-and-reads](mind-and-reads.md#against-instinct-vs-against-a-mind).

## Edge — per-duel, all-in, linear, public

- **Per-duel.** Every duel starts at **0 Edge**; Edge is built and spent *inside*
  the clash and **does not carry over** to other duels — not even between two
  duels involving the same character. This is the big simplifier: there is no
  fight-long meter to hoard, so the cross-round runaway / hoarding / stall problems
  never arise.
- **All-or-nothing.** Anything that spends Edge spends **all** of it; there is no
  "how much to commit." An Unleash's size is simply your current (public) bank.
- **Linear.** A bank of *n* does *n*. Devastating next to a base poke, but bounded
  — a short duel only builds so much, so there is no one-shot-from-hoarding.
- **Public, and it looms within the clash.** Both fighters see the Edge building;
  someone sitting on 3 Edge is a looming Unleash, and the opponent must respect it
  ("respecting the meter," at the scale of a single exchange).
- **The steal is the comeback.** Parry a real Unleash and you negate it **and take
  its Edge** (or **+1** if it had none — a parry always pays) — the lead flips
  mid-duel. An **Overwhelm** is never stolen.

## The four reads — Marshal · Unleash · Overwhelm · Parry

Each fighter secretly commits one; reveal at once. They are the fighting-game
mix-up — **neutral, strike, throw, block** — a proven, learnable, non-degenerate
four-way:

- **Marshal** *(neutral)* — ready and gather. **Bank Edge.** But you're winding up
  and exposed: an **Unleash** catches you and ends the duel with you struck.
- **Unleash** *(strike)* — pour **all** your Edge into a blow. Catches a Marshaller
  and beats an Overwhelm — but a **Parry** turns it *and steals the bank*.
- **Overwhelm** *(throw)* — drive **all** your Edge *through* a guard. Beats a
  **Parry** — but whiffs against anyone not guarding (a Marshaller, or an
  Unleasher), losing your Edge for nothing.
- **Parry** *(block)* — read the Unleash: negate it **and steal the whole bank**
  (or, if it had none, earn **+1 Edge** — a parry is never dead), the game's
  biggest comeback. But it loses to an **Overwhelm**, and a Marshaller just banks
  while you guard at air.

The offensive triangle is **Unleash ▸ Overwhelm ▸ Parry ▸ Unleash**; Marshal is
the neutral that feeds it.

## Ends-on-strike

**A 0-Edge Unleash is still a strike** (a base hit) — Unleash *is* the attack;
Edge only scales it. So **the duel ends the instant any Unleash or Overwhelm
connects**, mutual included. The only committed attacks that *don't* end it are a
**parried Unleash** (negated + stolen; roles flip) and a **whiffed Overwhelm** (no
guard to break). "Caught while charging" needs no special rule — you simply take
the hit and the duel is over.

| Ends the duel — a strike lands            | Continues — nobody lands                              |
| ----------------------------------------- | ----------------------------------------------------- |
| Unleash vs Marshal — marshaller struck    | Marshal vs Marshal — both +Edge                       |
| Unleash vs Unleash — both struck (mutual) | Marshal vs Parry — marshaller +Edge                   |
| Unleash vs Overwhelm — overwhelmer struck | Marshal vs Overwhelm — overwhelm whiffs               |
| Overwhelm vs Parry — parrier struck       | Unleash vs Parry — parried, bank stolen (or +1), flip |
|                                           | Overwhelm vs Overwhelm — clinch, nothing              |
|                                           | Parry vs Parry — nothing                              |

## The shape of a duel

Because a base Unleash already ends it, the duel has a clean arc:

- **Floor.** If neither escalates, someone pokes for a base hit (or both, mutual)
  and it's over fast. The mind-game is **opt-in**.
- **Escalation = push-your-luck.** Marshal to build a bigger finisher, but every
  beat you don't end it you risk the opponent ending it first, or — Unleashing big
  into a Parry — handing them your Edge. Greed buys a bigger payoff at more
  exposure.
- **Parry / Overwhelm.** Parry stops an Unleash and steals it but ends and builds
  nothing; Overwhelm punches through a Parry to end the duel. A parry-crouch is
  punished (by Overwhelm), and a parried Unleash flips the Edge.

## Why four reads, not three — the parry problem

With only Marshal / Unleash / Parry, a fighter who didn't want to be hit could just
**always Parry**: it negates the Unleash and steals it, with no downside but a
wasted turn. **Overwhelm dissolves that** — the throw beats the block, so an
always-Parry just gets Overwhelmed. No safe square remains: Parry beats Unleash but
loses to Overwhelm; not-parrying beats Overwhelm but loses to Unleash. And the
steal-comeback is untouched, because an Overwhelm is never stolen.

## Termination — self-resolving, with an engine-only backstop

A duel **cannot stall under any reasoning play.** "Both Marshal forever" is not a
stable strategy: the moment you believe your opponent will Marshal, your best move
is to **Unleash** (Unleash beats Marshal — you catch them and end it). The belief
that would sustain the stall is the belief that breaks it. Formally the equilibrium
is mixed, so every beat has a real chance someone commits; the duel terminates with
probability 1 and is short in expectation. **No public rule is needed** — at the
table, two humans simply never both-Marshal indefinitely, and even if they did it
is only slow, never a winning advantage.

The only residual risk is non-rational actors at the **implementation** level — a
buggy AI, or two humans griefing the server by dragging a round out. So the engine
carries a backstop that is **not part of the public rules:**

- After **N consecutive mutual-Marshals** (e.g. 10): if both duelists are
  **human**, surface a warning and **force an Unleash** next exchange; if **any**
  duelist is **AI**, raise an **error** — a stalling AI is a bug, not a play
  pattern.

## Unleash and Overwhelm scale the card's *primary effect*

One global rule, no bespoke per-card logic: **every card has a primary effect (its
headline), and spending your Edge scales that effect.** Three clean roles:

- **Card = *what*** — the maneuver and its primary effect.
- **Read (Marshal / Unleash / Overwhelm / Parry) = the *anticipation*** — hidden,
  decoupled from the card, so the card never telegraphs the read.
- **Edge = *how much*** — spent all at once.

Damage is the common primary effect (a strike maneuver → devastation), but not the
only one — a **Sunder** shears off armor, a **Disarm** rips cards from hand, a
**Shove** breaks them out of the line. No card has to "know about" Edge.

## Range, split attention, and many at once

A duel is pairwise, so engaging a crowd means **several simultaneous pairwise
duels** (or one sweeping breadth-attack), governed by the coordination layer:

- **Symmetric reach** → a full duel; both run the four-way.
- **Lopsided reach** → the out-ranged fighter is in the duel *defensively* (read
  and Parry/dodge) but **cannot Unleash or Overwhelm back**, and their **attention
  is split** — each turn, read the blow *or* act in their own range, not both.
- **Bandwidth = Mind.** You can fully duel as many foes as your Mind affords;
  beyond that, the extra attackers **free-hit** you. A god clears the crowd it can
  attend to and is countered by being **swarmed past its bandwidth** (the gank) —
  asymmetry by design, balance by scenario. Because Edge **resets per duel**,
  breadth never compounds into one mega-bank: a god is a stack of independent short
  duels, powerful in each but hard-capped on how many it can read.

## A worked exchange — the rock *(one duel, all-in, linear)*

Two brawlers, **10 HP** each; Marshal = **+1 Edge**; a spent bank of *n* does *n*.
This is one duel, resolved inside one round's Clash:

1. **Both Marshal** → Edge **1–1**, banks in the open.
2. **Both Marshal** → **2–2**; both escalating, two looming bombs.
3. **A Unleashes (2)** — but **B Parries!** Negated, and B **steals A's 2** → B on
   **4**, A on **0**. Roles flip mid-duel. *(The steal — the comeback.)*
4. **B sits on 4** (a near-finisher). A, empty, **Parries** to avoid it — so **B
   Overwhelms (4)**, punching through the guard: **A struck for 4 (→ 6 HP)**. A
   strike landed — **the duel ends.**

Four beats, one round: build race, the steal flipping the lead, and Overwhelm
cracking a parry to finish. Next round is a fresh duel at 0 Edge.

## Lineage

The closest solved version is the fighting-game cluster of **meter + supers +
yomi**: build in neutral (**Marshal**), pop a super (**Unleash**), a hard-read
whiff-punish (**Parry**), and a throw that beats the block (**Overwhelm**) — the
**strike / throw / block** mix-up with Marshal as neutral. A bank that changes the
opponent's behaviour is "respecting the meter," and "scale a normal with spent
meter" is the **EX move**. The breakable charge and precise perfect-parry come from
brawlers; *Doom Eternal* contributes "play aggressively to fuel yourself."

## What this supersedes

- The four reads **Strike / Block / Evade / Scheme** as the RPS cycle → replaced by
  **Marshal / Unleash / Overwhelm / Parry**. The old "reads" become **maneuver
  cards** (what you Unleash with).
- **Momentum banking Power / Speed / Precision** → collapsed into one generic,
  public, **per-duel Edge**.

## Open questions — the tuning lives here

- **Edge → effect rate.** 1 Edge = how much of a primary effect (1 damage? one
  target? one armor pip?). Each effect needs its natural per-Edge unit.
- **Perfect-parry counter.** The steal is settled; does a Parry *also* deal
  counter-damage, and how much?
- **Clinch (Overwhelm vs Overwhelm).** Provisionally a wash — nothing happens.
  Confirm in play.
- **Natural duel length.** Tune how risky a caught Marshal is so duels run a few
  beats — a "single throw" — rather than a long dance. (Pacing, not termination;
  termination is settled above.)
- **Duel detection.** The exact "can respond to me" rule that switches the
  read-game on, and how split-attention is modelled for lopsided reach.
