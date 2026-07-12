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

3. **Raid** — Surviving Outriders strike the enemy Rearguard they crossed for. The breaker that got through the Intercept and Volley lands on the exposed back. This is the Outrider's EARLY slot, and the whole of what the role buys: every other role reaches the enemy back last, at the Breach. If the enemy fielded no Rearguard the pairing is simply void - as it is for any role facing an empty rank - and the Outrider strikes with its remaining two slots, at the Breach, like everyone else. A misdeclared intent is punished by timing, not silence.

4. **Clash** — The lines meet: each Rearguard fires on an enemy Vanguard (the only answer to its Toughness), and each engaging Vanguard strikes an enemy Vanguard. Untyped Might banks into the per-sub-phase pile; clearing the target's Toughness flips a Health card.

5. **Breach** — The deep, trailing blows land last: a Vanguard or a Rearguard crosses to an enemy Rearguard whose own front has fallen, and the Outriders - who reached the back early - arrive here for everything else, striking the enemy front and each other.

## Cross-cutting behaviors

- **Wipe pile** — At each sub-phase boundary the per-sub-phase damage pile is cleared: sub-threshold damage that did not turn a Health card does not carry into the next sub-phase. Only Health persists.

- **Tempo contest** — The one attack-vs-defense mechanic: a single simultaneous Tempo bid (cards x Finesse); the defender must strictly exceed it (a tie lands the hit) to block a melee blow, slip a blocker, or evade ranged fire. Defending is Tempo-negative, so blows eventually land.

- **Strike back** — A melee attacker may be answered: the defender spends a Tempo card to deal its own Might back — but only when that blow can crack the attacker's Toughness, and only if the defender is still alive (a corpse cannot react).

- **Area of effect** — An attack may strike a whole rank at once instead of a single target — width that cannot whiff against a crowd and **bypasses a group's spillover** (hits every member), at the price of not concentrating its force.

- **Grouping** — Same-side units may be bound at form-up into one unit (one position, one shared target, distinct Health): single-target damage **spills** through the front member in declared order (a bodyguard soaks for the squishies), a group sums its members' Tempo to block but needs every member to beat the attacker to slip — a superb wall and a hopeless slipper.

