# Deckbound — Combat Resolution

> **SUPERSEDED — clash mechanics.** The tactical core is now the **Clash**
> ([`spec/README.md` §1](../canon/2-spec/README.md), rationale in [the-duel.md](the-duel.md)): a
> four-card duel — **Strike · Anticipate · Gather · Evade** — with a single **Force** count
> (×2 per Force, stolen only on Strike-into-Evade), run **ends-on-strike**. The Strike/Block/Evade/Scheme cycle, Power/Precision
> momentum, and the "Speed = first-strike" pre-emption model below are stale. What
> carries forward: **Power as pure magnitude**, **typed damage through armor +
> toughness** (now `power × 2^Force`, spec §2), and the principle that you stop a
> blow by out-*predicting* it — now via the Clash's complete-defense invariant, not a
> first-strike race. Read for intent; trust the spec for mechanics.

How a committed exchange resolves into **magnitude**. Once the stances
([the Mind](mind-and-stances.md)) and the cards
([Body stats & deliveries](cards-and-customization.md)) are chosen, this is the math
that turns them into an outcome. Numbers here are deferred until there is something to
playtest.

## Order and pre-emption

- **Speed decides who lands first** — not a global turn order but a **per-clash** edge:
  the fighter with more [tempo](speed-and-tempo.md) left **strikes first**, and **equal
  tempo means both land**. Stances are still chosen **blind and simultaneous**.
- **Power is pure magnitude** — it sets force and cracks armor; it has **no separate
  interrupt job**. You **stop** the other's blow only by **dropping them first** (a lethal
  first-strike — no swinging once felled) or **out-predicting them** (a Defense beats their
  Strike, a Strike spoils their Scheme). A non-lethal **stagger** is a
  [keyword](keywords.md) on specific cards, not a universal rule.
- The full **tempo pool** — spend to evade/engage/strike, overextend, volume — is in
  [speed & tempo](speed-and-tempo.md);
  **[pre-emption](coordination-and-interruption.md#pre-emption--stopping-a-foes-blow)** and
  front-line interception in [coordination & interruption](coordination-and-interruption.md).

## From a stance to damage

A landed Strike's damage is built from the Body axes plus the
[Mind's banked momentum](mind-and-stances.md#momentum--the-thrill): **Power** (and any
attached multipliers) sets force; **Precision** exploits weak spots and bypasses
armor. That damage is **typed**, then erodes the target's **Body** capability cards
through their armor and toughness — see
[form & defeat](form-and-defeat.md#how-damage-resolves--the-vitality-card-and-health-cards).

Mirror and trade matchups in the [stance cycle](mind-and-stances.md#the-cycle) (Strike vs
Strike, Block vs Evade, …) are settled here, by **tempo** (first strike) and **Power**.

## Open questions

- The **damage formula** — how Power, Precision, and attached multipliers combine
  into a number.
- How **multiplier** cards attach — one per quality, or a limit?
- How **positional / distance** advantage (from Evade / Scheme) is represented.
- How **heavy weapons** gate on Power — a threshold, a cost?
