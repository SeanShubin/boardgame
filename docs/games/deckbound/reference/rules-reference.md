# Deckbound — Rules Reference

> **GENERATED — do not edit.** Regenerate with `cargo run -p deckbound --example handbook`.
> A projection of `crates/deckbound/data/booklet.ron` (the print master) and the Spec.

The in-game encyclopedia, generated from the Spec's `TERM` definitions and the passive powers' card text.

## Roles

- **Marshal** — The round's opening step: a hidden, simultaneous commit where each side groups its Actors, assigns each group an intention — Vanguard (front), Outrider (flank), or Rearguard (back) — and plays its standing buffs / braces. Intentions are re-declared every round. Revealed together (the Reveal step); everything after resolves in the open, nobody moves.
- **Vanguard** — The declared front: hold the line. The position that can be hit and the shield — while a side's Vanguard lives, its Rearguard is reachable only by an Outrider's raid. Melee Actors fight from here; it screens enemy Outriders, then fights the front, then cleans up.
- **Outrider** — The declared flank: break the line. Forgoes the shield and the safe back to raid the enemy Rearguard directly — but is exposed to the enemy front (Intercept) and back (Volley) *before* it strikes. A lone, high-Tempo melee body; a group cannot raid (slips weakest-link).
- **Rearguard** — The declared back: deal from safety. Untargetable while its own Vanguard lives and no Outrider has reached it; from the back it fires on the enemy front (ranged), buffs allies, and degrades foes. The only answer to an enemy Vanguard's Toughness.
- **Suit** — A role track's **identity** (a substance): Iron · Silver · Brass · Bone · Salt, bound 1:1 to a **Role** (Wall · Infiltrator · Artillery · Controller · Support). The Suit is what a reward *is*; the Role is what it *does*. Name treasure by its Suit — "an Iron reward," never "a Wall reward."

## Combat

- **Engagement schedule** — The fixed order strikes resolve in each round: Intercept (Vanguard→Outrider), Volley (Rearguard→Outrider), Raid (Outrider→Rearguard), Clash (Rearguard→Vanguard, Vanguard→Vanguard), Breach (Vanguard→Rearguard, Outrider→Vanguard, Outrider→Outrider, and Rearguard→Rearguard once the enemy Vanguard has fallen). Each engagement cycles until no one will spend Tempo, then resolves. The order is the whole interception / pre-empt / Reckoning system; consult it only when timing is ambiguous.
- **Tempo contest** — The one attack-vs-defense mechanic: a single simultaneous Tempo bid (cards × Finesse); the defender must strictly **beat** it (a tie lands the hit) to block a melee blow, slip past a blocker, or evade ranged fire. Defending is Tempo-negative, so blows eventually land. No iterated raise-war.
- **Reach** — Where you can attack from: melee strikes from the Vanguard or raids as an Outrider, ranged deals from the Rearguard. Positions self-sort by attack type; a misplaced unit is idle, not barred.
- **Group** — Same-side Actors bound at form-up into one unit: one position, distinct Health; members target individually but reposition collectively. Single-target damage spills in declared order (overflowing on a death); AoE hits every member. A melee attack spends one Tempo per member (the whole group crosses); a ranged attack spends only the shooters'. Blocking/catching pools member Tempo; slipping needs every member to beat the attacker. No size cap, no mixed positions.
- **Hoard X** — A creature whose X health cards each act as a separate entity — mechanically a built-in group of X one-health bodies (a swarm): sums to block, cannot slip, melts to AoE, and loses an attack per body killed.
- **Spillover** — Accumulated single-target damage on a group applied point-by-point in declared order, overflowing to the next member only when the current **dies**. AoE instead hits every member at full value; the two-pool resolution (AoE counted in-pile while spillover cascades, flip at the boundary) is §4.6.
- **Trade** — A same-range melee engagement: both sides deal their base through toughness. In the optional Clash module, the trade becomes the four-card mix-up.
- **Evade** — A ranged defense: spend Tempo to dodge a ranged attack (the tempo contest, §3.1) — your evade (cards × Finesse) must strictly beat the attacker's volley, a tie lands the hit. Any target may evade, whatever its own range.
- **Auto-hit** — A ranged or off-range blow the target neither **evades** (Tempo) nor strikes back: it lands uncontested through toughness.
- **Attack type** — Each Actor is Melee, Ranged, Both, or Neither, **read from its strike card** (the weapon's `reach`; no strike card = Neither, §4.3). Melee strikes from the Vanguard; ranged fire from the Rearguard. Lacking the matching attack means you can't strike back — but you may still evade ranged fire with Tempo.

## Resources

- **Cadence** — A permanent Form stat: how many **Tempo** cards you start each combat round with (the *count*). It is not a magnitude of movement and never sets turn order.
- **Finesse** — A permanent Form stat: the magnitude on each **Tempo** card (the *grade*). Its number matters only in a **Tempo contest** — block / slip / evade — where both sides commit Tempo cards (cards × Finesse) and the side avoiding the strike must strictly exceed (a tie lands the strike). A strike's force is the same whatever its Finesse.
- **Tempo** — The round's pool of action cards: **Cadence**-many, each worth **Finesse**. Flip one to take any action (strike, block / slip / evade, strike back) — standing and soaking are free; spent cards stay spent until the round refresh (shared across the round's two phases; a **Recover** verb can return one mid-round, §5).

## Clash module

- **The Clash** — An optional 1v1 mix-up that replaces a same-range trade. Each beat both pick a card and reveal at once: Strike, Anticipate, Gather, Evade.
- **Cards** — Strike beats Gather; Anticipate beats Evade; Gather beats Anticipate; Evade beats Strike. Strike also beats Anticipate; Strike-vs-Strike trades.
- **Force** — Gather builds +1 Force; each Force doubles your connecting hit. Evading a Strike steals the striker's Force (always at least 1).

## Powers

- **Phalanx** — Wall chargers who stop together combine their effort to intercept runners — several defenders hold the front as one.
- **Bulwark** — Holds the line for the whole front — every allied Vanguard's hold rises, not only this Wall's.
- **Taunt** — Draws fire: chargers are pulled to intercept this Wall first, sparing the rest of the line.
- **Blitz** — The first slip each round is free — it costs no Tempo.
- **Shadowstep** — Win a tie when slipping past an interceptor — your equal Tempo gets you through anyway.
- **Backstab** — Bonus damage when this Outrider strikes an enemy Rearguard.
- **Longshot** — Lets this Rearguard's ranged fire reach the enemy Rearguard — the sanctioned sniper exception.
