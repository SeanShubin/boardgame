# Combat — phase-by-phase appendix

> **Auto-generated from `crates/deckbound/src/rules.rs`** (the canonical mechanical text) — do not edit by hand; regenerate with `cargo run -p deckbound --example handbook`. This is the *mechanical* reference: each phase does exactly one thing, over two accumulators (the per-sub-phase damage **pile** and **Tempo**). The thematic overview lives in the rulebook.

## Phases (in round order)

1. **Marshal** — Each unit is secretly assigned an **intention** — Vanguard (hold the front), Outrider (break the line) or Rearguard (deal from the back) — and may be bound into a group. Re-declared every round; declaring is free and may fail (a misplaced unit is idle, not barred).

2. **Reveal** — Intentions and groups are revealed together and positions lock. Nobody moves; everything after resolves in the open.

3. **Ready** — Standing abilities (a Wall's brace, a Support's ally buff) are cast now. They are ally-targeted, auto-land, and last the round.

4. **Engage** — The two lines meet and trade blows: the fixed **sub-phase schedule** resolves in order — Intercept → Volley → Raid → Clash → Breach — each sub-phase a §1.9 boundary. Untyped Might banks into the per-sub-phase pile; clearing a target's Toughness flips a Health card.

5. **Refresh (the Lull)** — Round end: all spent Tempo resets, Health carries over, and the round advances. A battle not decided within five rounds is a draw.

## Sub-phases of the Engage phase (in schedule order)

1. **Intercept** — The front screens the flankers: each Vanguard strikes an enemy Outrider as it crosses, before it can raid. An Outrider cut down here never reaches the back.

2. **Volley** — The back fires on the flankers: each Rearguard shoots an enemy Outrider — before it arrives (the pre-empt). A shot spent here is a shot not fired at the enemy front later.

3. **Raid** — Surviving Outriders strike the enemy Rearguard they crossed for. The breaker that got through the Intercept and Volley lands on the exposed back. This is the Outrider's ONE offensive slot, and its target is a priority, not a fixed rank: the Rearguard, else the Vanguard, else another Outrider. An Outrider that crossed for a back line the enemy never fielded does not stand idle beside the foe in front of it - it falls on the front, here, in its own slot. It has already paid for crossing (it was exposed to the Intercept and the Volley): the declaration fixes what happens TO you, the field fixes what you DO. Only the Outrider re-aims, and only it needs to - a Vanguard and a Rearguard each get a separate slot against each rank, so an empty enemy rank merely costs them an opportunity they never had, with nothing to re-aim to.

4. **Clash** — The lines meet: each Rearguard fires on an enemy Vanguard (the only answer to its Toughness), and each engaging Vanguard strikes an enemy Vanguard. Untyped Might banks into the per-sub-phase pile; clearing the target's Toughness flips a Health card.

5. **Breach** — The deep, trailing blows land last: a Vanguard crosses to an enemy Rearguard whose own front has fallen. Nothing else. Outriders do not appear here - they re-aim in their own slot at Raid, so a Breach fallback would be a second bite, and the Outrider gets exactly one.

## Cross-cutting behaviors

- **Wipe pile** — At each sub-phase boundary the per-sub-phase damage pile is cleared: sub-threshold damage that did not turn a Health card does not carry into the next sub-phase. Only Health persists.

- **Tempo contest** — The one attack-vs-defense mechanic: a single simultaneous Tempo bid (cards x Finesse); the defender must strictly exceed it (a tie lands the hit) to block a melee blow, slip a blocker, or evade ranged fire. Defending is Tempo-negative, so blows eventually land.

- **Strike back** — A melee attacker may be answered: the defender spends a Tempo card to deal its own Might back — but only when that blow can crack the attacker's Toughness, and only if the defender is still alive (a corpse cannot react).

- **Area of effect** — An attack may strike a whole rank at once instead of a single target — width that cannot whiff against a crowd and **bypasses a group's spillover** (hits every member), at the price of not concentrating its force.

- **Grouping** — Same-side units may be bound at form-up into one unit (one position, one shared target, distinct Health): single-target damage **spills** through the front member in declared order (a bodyguard soaks for the squishies), a group sums its members' Tempo to block but needs every member to beat the attacker to slip — a superb wall and a hopeless slipper.

