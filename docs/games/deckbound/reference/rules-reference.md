# Deckbound — Rules Reference

> **GENERATED — do not edit.** Regenerate with `cargo run -p deckbound --example handbook`.
> A projection of `crates/deckbound/data/booklet.ron` (the print master) and the Spec.

The in-game encyclopedia, generated from the Spec's `TERM` definitions and the passive powers' card text.

## Roles

- **Assemble** — The one hidden, simultaneous commit: each side assigns every Actor a rank (Vanguard / Skirmisher / Reserve), commits its crossing / catch bids and which Vanguard catches which Skirmisher, and plays its standing cards. Revealed together; everything after resolves in the open, and nobody moves.
- **Vanguard** — The declared melee front line. Holds, and may spend Tempo to catch crossing Skirmishers — as many as it can pay catch-bids for (Speed = breadth, Daring = strength); once the enemy Vanguard it faces is dead it pours through. Shields the Reserve.
- **Skirmisher** — A declared flanker that attempts to cross the enemy line. Held → it trades with its catcher; crossed → it reaches the backfield, where any enemy rank is fair game. The route (besides a broken front) to the enemy Reserve.
- **Reserve** — The declared ranged / support line behind the front. Fires over it and aids allies, can never target the enemy Reserve, and is reached only by a Skirmisher who crossed or a Vanguard pouring through a broken front.
- **The triangle** — Vanguard beats Skirmisher (catches it at the line); Skirmisher beats Reserve (crosses to assassinate); Reserve beats Vanguard (fires from safety, untouchable in melee).
- **Suit** — A role track's **identity** (a substance): Iron · Silver · Brass · Bone · Salt, bound 1:1 to a **Role** (Wall · Infiltrator · Artillery · Controller · Support). The Suit is what a reward *is*; the Role is what it *does*. Name treasure by its Suit — "an Iron reward," never "a Wall reward."

## Combat

- **The Line** — Tier 1: Vanguards strike across, and each crossing Skirmisher's advance Daring is weighed against its catcher's hold. Resolved from a start-of-round snapshot; deaths tally at the boundary.
- **Crossing** — A Skirmisher's attempt to pass the wall: a single simultaneous Daring bid (committed cards × Daring). Strictly more than the catcher's hold slips (taking a parting free hit); equal-or-less is held and trades; an uncaught Skirmisher slips free. Wall powers raise the hold only.
- **The Open** — Tier 2: crossed Skirmishers strike anything behind the line (the Reserve is the prize), a Vanguard whose foe is dead pours through, Reserves fire on the front and pick off exposed Skirmishers, and the struck strike back if they can answer the range.
- **Open brawl** — If neither side fields a front, no line forms and the Reserve's safety lifts: everyone may target anyone with whatever range they carry.
- **Trade** — A same-range engagement: both sides deal their base damage through armor/toughness. In the optional Clash module, the trade is replaced by the four-card mix-up.
- **Auto-hit** — A range mismatch: the attacker lands uncontested (the target can't answer at that range). Armor still blunts it; Focus cannot.
- **Attack type** — Each Actor is Melee, Ranged, Both, or Neither. Crossing contests & Skirmisher strikes are melee; Reserve fire is ranged. Lacking the matching attack means you're auto-hit.

## Resources

- **Speed** — A permanent Form stat: how many **Tempo** cards you start each combat round with (the *count*). It is not a magnitude of movement and never sets turn order.
- **Daring** — A permanent Form stat: the magnitude on each **Tempo** card (the *grade*). Its number matters in exactly one place — a crossing contest, where both sides commit Tempo cards (cards × Daring) and the higher total wins (ties to the catcher). A strike is the same whatever its Daring.
- **Tempo** — The round's pool of action cards: **Speed**-many, each worth **Daring**. Flip one to take any action (strike, contest a crossing, strike back) — standing and soaking are free; spent cards stay spent until the round refreshes.

## Round

- **Phases** — Assemble (hidden: ranks + bids + standing cards) → the Line (Vanguards trade, Skirmishers contest the crossing) → the Open (breakthroughs strike, Reserves fire) → Refresh. Order-independent within each tier, strictly sequenced between.

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
- **Backstab** — Bonus damage when this Skirmisher strikes an enemy Reserve.
- **Longshot** — Lets this Reserve's ranged fire reach the enemy Reserve — the sanctioned sniper exception.
