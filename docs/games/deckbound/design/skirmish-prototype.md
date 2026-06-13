# Deckbound — Skirmish Prototype (6 vs 9)

> **Earlier draft.** This 6v9 predates the gauntlet model and the four fleshed
> aspects; its coverage rule and numbers are superseded. The current combat on the
> live rules is the focused [4v5 sample](sample-round.md) — start there. This is kept
> for its larger roster and the one-threat-per-mechanic table.

The first **concrete, runnable** combat: a 6-hero party (3 front, 3 back) against a
9-creature warband. Its job is to turn the design notes into numbers you can play,
and to demonstrate the thesis — **the creatures are lethal to a careless party, but
coordination beats them comfortably.** All numbers are first-pass, meant for tuning;
several systems are simplified to "just enough" (noted inline).

## Just-enough rules

**Stats.** Each combatant has **Speed (Spd)**, **Power (Pow)**, **Precision (Pre)**,
and casters a **Magic (Mag)** rating. Health is **Body** cards with a **Toughness**
rating — written **Body N × T**.

**Damage → Body.** A landed hit deals **Damage**; subtract the target's **armor**;
it removes **⌊remaining ÷ Toughness⌋** Body cards. **0 Body = knocked out** (an ally
revives you as a passive; a full wipe = defeat). *(Armor here is a flat reduction
with one exception each — the full damage-type matrix is deferred.)*

**A round:**

1. **Lines.** Each side sets front / back (fixed for the round).
2. **Declare** (any order; players coordinate openly, each creature flips a behavior
   card): a **stance** — **Attack** a target, or **Hold**. Back line may attack only
   the opposing **front** line; **front** line may attack the front line or **dive**
   the back line; **ranged** weapons may target the opposing **back** line directly.
3. **Resolve in Speed order**, highest first:
   - **Gauntlet** — a dive runs past the covering **Holders**; each lands a **free
     strike** (auto-hit), and a Holder with **Spd ≥ diver+1 and Pow ≥ the diver's**
     also **cancels** the dive. Survive the free strikes uninterrupted → reach the back
     line, bloodied. A fast Holder gauntlets several, paying Speed per the bandwidth
     rule.
   - **Reads** — in a mutual engagement (or Hold-vs-attacker) both pick **Strike /
     Block / Evade / Scheme**; the cycle (Strike > Scheme, Defense > Strike, Scheme >
     Defense) decides it, the winner banks a bonus (Block → +Pow, Evade → +Spd,
     Scheme → +all), a misread forfeits the bank. Otherwise the attack **auto-succeeds**.
   - Apply damage; banked **Speed** may buy an **extra unopposed action**.
4. Resolve knockouts; next round.

**Creatures are mindless** — stance and read come from a fixed, *readable* behavior
(no bluffing). The deep bluff-RPS is for minded foes and is deferred.

## The party — 3 front, 3 back

Front line (the wall and its hammer):

| Hero           | Spd | Pow | Pre | Body×T | Kit & role                                                                                                      |
| -------------- | --- | --- | --- | ------ | --------------------------------------------------------------------------------------------------------------- |
| **Bulwark**    | 3   | 2   | 1   | 6 × T3 | Plate (−3 phys), Shield (strong **Block**, **Shield-bash** interrupt). **Holds** — coverage + gating. The wall. |
| **Vanguard**   | 3   | 5   | 2   | 4 × T2 | Two-handed **maul** (blunt, +Pow). **Attacks** — the only one who cracks the Juggernaut.                        |
| **Skirmisher** | 6   | 2   | 4   | 3 × T1 | Light blade (pierce). Fastest — gates fast divers *or* dives. Pre 4 cracks the Sentinel. Fragile.               |

Back line (safe only if the wall holds):

| Hero          | Spd | Pow | Pre | Mag | Body×T | Kit & role                                                                                                                                                        |
| ------------- | --- | --- | --- | --- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Channeler** | 2   | 1   | —   | 5   | 2 × T1 | **Firestorm** (AoE: all enemy front line) + **Frost** (slow/seal one). Glass cannon; the swarm answer.                                                            |
| **Tactician** | 4   | 2   | 3   | —   | 3 × T2 | **Recover** (return spent cards to an ally), **Read** (grant an ally Precision / cancel a telegraph), **Steady** (soften misread loss). Keeps the engine running. |
| **Marksman**  | 3   | 3   | 5   | —   | 2 × T1 | **Bow** (ranged, pierce), **Snipe** (hits the enemy **back** line; Pre 5 bypasses armor). Kills the back-line casters; cracks the Sentinel.                       |

**Synergy:** Bulwark (and Skirmisher) **Hold** → divers must run their gauntlet of free
strikes → the three squishies stay protected. Channeler AoEs the front swarm; Marksman snipes the
back-line casters; Vanguard duels the Juggernaut; Skirmisher gates fast divers or
cracks the Sentinel; Tactician recovers spent reads and steadies against Shock. Pull
any one and a job goes uncovered.

## The warband — 9 creatures

| Creature           | Spd | Pow | Pre | Body×T | Threat — *and the only answer*                                                                                                                                                                                |
| ------------------ | --- | --- | --- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Juggernaut** ×1  | 1   | 4   | 1   | 5 × T3 | Armor **−3** (blunt only −1). Brute force bounces — **only Vanguard's blunt + Power** cracks it. Slow (acts last).                                                                                            |
| **Sentinel** ×1    | 2   | 3   | 2   | 4 × T2 | Armor **−4 except a Pre ≥ 4 weak-spot hit**. **Only Precision** (Marksman / Skirmisher) hurts it.                                                                                                             |
| **Swarmling** ×4   | 4   | 2   | 1   | 1 × T1 | Weak alone; **2 hold the front, 2 dive**. As divers they need **coverage + AoE** — Firestorm one-shots the front pack.                                                                                        |
| **Stalker** ×1     | 5   | 3   | 4   | 2 × T1 | Dives the **Channeler**. Too strong to *interrupt* (Pow 3 beats your fast low-Power Holders), so it can't be cancelled — but it **runs the gauntlet**, and a dense wall's free strikes bleed it out (Body 2). |
| **Artillery** ×1   | 2   | 5   | 3   | 2 × T1 | **Ranged**: nukes a back-liner each round (Pow 5 kills a T1 squishy). **Marksman snipes it**, or Skirmisher dives it.                                                                                         |
| **Stormcaller** ×1 | 3   | 2   | 2   | 3 × T1 | **Shock**: seals one hero's Mind each round (Strike-only; disrupts Recover). **Kill it fast** (Snipe) or Steady through it.                                                                                   |

That is exactly one threat per mechanic — damage-type, Precision, coverage, AoE,
ranged reach, interception-vs-coverage, and Mind-resilience. **Neglect any one role
and the matching creature runs free.**

## Setup

- **Creature front:** Juggernaut, Sentinel, 2 Swarmlings. **Creature back:** Artillery,
  Stormcaller. **Diving this round:** Stalker + 2 Swarmlings (3 divers at your back
  line).
- **Player front:** Bulwark, Vanguard, Skirmisher. **Player back:** Channeler,
  Tactician, Marksman.

## Why coordination wins (and carelessness loses)

A few exchanges show the math:

- **Vanguard vs Juggernaut:** Pow 5 blunt − armor 1 = 4 → ⌊4 ÷ 3⌋ = **1 Body/round**;
  the Juggernaut (Body 5) falls in ~5 rounds while its Spd 1 lets the front brace.
  A sharp/pierce hero instead does `Pow − 3 ≈ 0` — bounces. *Bring the right damage.*
- **Marksman vs Sentinel:** Pre 5 ≥ 4 bypasses the −4 armor → full damage; anyone
  else does ~0. *Bring Precision.*
- **Swarmling vs an uncovered Channeler:** Pow 2 → ⌊2 ÷ 1⌋ = **2 Body** → the
  Channeler (Body 2) is **down in one hit**. *Cover the back line.*
- **Stalker vs your interceptors:** Skirmisher reaches it (Spd 6 ≥ 5+1) but Pow 2 < 3
  can't *cancel* it — yet every Holder it passes lands a **free strike**, and a fragile
  diver (Body 2) bleeds out in a dense gauntlet. *Free strikes ≠ interrupts.*

**The coordinated plan:** Bulwark + Skirmisher Hold → 2 Holders cover 2 of the 3
divers (Bulwark bodies the Stalker; Skirmisher intercepts a Swarmling); Channeler
Firestorms the front swarm before they pile on; Marksman snipes the Artillery (its
biggest single threat) then the Stormcaller; Vanguard grinds the Juggernaut; Tactician
recovers the front line's spent reads and steadies whoever gets Shocked. The party
trades efficiently and wins.

**The careless line:** the front line all **Attacks** for damage → coverage 0 → all
3 divers plus the Artillery and Stormcaller fall on the back line → Channeler,
Marksman, and Tactician drop in a round or two → the party loses its AoE, its ranged
answer, and its recovery at once, and the Juggernaut + Sentinel grind down what's
left. Same heroes, same creatures — **a wipe.**

## What this validates, and what to tune

Validated in shape: front/back + Attack/Hold, coverage vs interception, the
damage→Body math, ranged reach, and one creature per mechanic forcing diverse,
synergistic roles.

Open / to tune by playtest:

- The **numbers** — Toughness, armor values, and damage so fights last a satisfying
  few rounds rather than one-shotting (the squishies are currently *very* fragile).
- The **read/momentum** loop is barely exercised here (creatures don't bluff); it
  wants a minded opponent or a PvP slice to test.
- Creature **behavior cards** — the actual readable patterns (when each dives, who it
  targets) need writing out.
- Whether **ranged-reaches-back-line** belongs in the core rules
  ([cards & customization](cards-and-customization.md)) or stays a weapon trait.
