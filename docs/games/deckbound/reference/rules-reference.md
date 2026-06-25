# Deckbound — Rules Reference

> **GENERATED — do not edit.** Regenerate with `cargo run -p deckbound --example handbook`.
> A projection of `crates/deckbound/data/booklet.ron` (the print master) and the Spec.

The in-game encyclopedia, generated from the Spec's `TERM` definitions and the passive powers' card text.

## Roles

- **Assemble** — The one hidden, simultaneous commit: each side assigns every Actor a rank (Vanguard / Outrider / Rearguard), commits its crossing / catch bids and which Vanguard catches which Outrider, and plays its standing cards. Revealed together; everything after resolves in the open, and nobody moves.
- **Vanguard** — The declared melee front line. Holds, and may spend Tempo to catch crossing Outriders — as many as it can pay catch-bids for (Cadence = breadth, Finesse = strength); once the enemy Vanguard it faces is dead it pours through. Shields the Rearguard.
- **Outrider** — A declared flanker that attempts to cross the enemy line. Held → it trades with its catcher; crossed → it reaches the backfield, where any enemy rank is fair game. The route (besides a broken front) to the enemy Rearguard.
- **Rearguard** — The declared ranged / support line behind the front. Fires over it and aids allies, can never target the enemy Rearguard, and is reached only by an Outrider who crossed or a Vanguard pouring through a broken front.
- **The triangle** — Vanguard beats Outrider (catches it at the line); Outrider beats Rearguard (crosses to assassinate); Rearguard beats Vanguard (fires from safety, untouchable in melee).
- **Suit** — A role track's **identity** (a substance): Iron · Silver · Brass · Bone · Salt, bound 1:1 to a **Role** (Wall · Infiltrator · Artillery · Controller · Support). The Suit is what a reward *is*; the Role is what it *does*. Name treasure by its Suit — "an Iron reward," never "a Wall reward."

## Combat

- **The Line** — Tier 1: Vanguards strike across, and each crossing Outrider's advance Finesse is weighed against its catcher's hold. Resolved from a start-of-round snapshot; deaths tally at the boundary.
- **Crossing** — An Outrider's attempt to pass the wall: a single simultaneous Finesse bid (committed cards × Finesse). Strictly more than the catcher's hold slips (and the bypassed wall may convert any remaining Tempo into one free hit per card, no cap — slipping wins right of way, not immunity); equal-or-less is held and trades; an uncaught Outrider slips free. Wall powers raise the hold only.
- **The Open** — Tier 2: crossed Outriders strike anything behind the line (the Rearguard is the prize), a Vanguard whose foe is dead pours through, Rearguards fire on the front and pick off exposed Outriders, and the struck strike back if they can answer the range.
- **Open brawl** — If neither side fields a front, no line forms and the Rearguard's safety lifts: everyone may target anyone with whatever range they carry.
- **Group** — Several same-side Actors bound at Assemble into one unit: one shared intention and one shared target, but distinct Health pools (members die individually). Single-target blows land whole on a defender-chosen member; area effects hit every member at full value; a grouped Vanguard catches with combined Tempo, a grouped Outrider crosses on its weakest member's Tempo. No size cap, no mixed intentions.
- **Window tag** — A spell's or ranged shot's printed timing: Line (resolves with the Line), Fast (the Open, before the Outrider melee), or Slow (the Open, after it). Casting spends a Tempo card. Persistent **buffs** (Support, ally-targeted) are *not* windowed — they are Assemble standing cards (§4.4), so attacks-before-buffs (§1.9) is never violated; **debuffs** (Controller) are evadable ranged attacks and *are* windowed (§4.2).
- **Trade** — A same-range melee engagement: both sides deal their base through toughness. In the optional Clash module, the trade becomes the four-card mix-up.
- **Evade** — A ranged defense: spend Tempo to dodge a ranged attack (the tempo contest, §3.1) — your evade (cards × Finesse) must strictly beat the attacker's volley, a tie lands the hit. Any target may evade, whatever its own range.
- **Auto-hit** — A ranged or off-range blow the target neither **evades** (Tempo) nor strikes back: it lands uncontested through toughness.
- **Attack type** — Each Actor is Melee, Ranged, Both, or Neither. Crossing contests & Outrider strikes are melee; Rearguard fire is ranged. Lacking the matching attack means you can't strike back — but you may still evade ranged fire with Tempo.

## Resources

- **Cadence** — A permanent Form stat: how many **Tempo** cards you start each combat round with (the *count*). It is not a magnitude of movement and never sets turn order.
- **Finesse** — A permanent Form stat: the magnitude on each **Tempo** card (the *grade*). Its number matters only in a **tempo contest** — a crossing or an evade — where both sides commit Tempo cards (cards × Finesse) and the side avoiding the strike must strictly exceed (a tie lands the strike). A strike's force is the same whatever its Finesse.
- **Tempo** — The round's pool of action cards: **Cadence**-many, each worth **Finesse**. Flip one to take any action (strike, contest a crossing, evade a ranged attack, strike back) — standing and soaking are free; spent cards stay spent until the round refreshes.

## Round

- **Phases** — Assemble (hidden: ranks + groups + bids + standing cards) → the Line (Vanguards trade, Outriders contest the crossing) → the Open, in three ordered sub-windows (Fast ▸ Outrider melee ▸ Slow) → Refresh. Order-independent within each window, strictly sequenced between.

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
