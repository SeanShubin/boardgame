# Combat — phase-by-phase appendix

> **Auto-generated from `crates/deckbound/src/rules.rs`** (the canonical mechanical text) — do not edit by hand; regenerate with `cargo run -p deckbound --example handbook`. This is the *mechanical* reference: each phase does exactly one thing, over two accumulators (the per-phase damage **pile** and **Tempo**). The thematic overview lives in the rulebook.

## Phases (in round order)

1. **Set positions** — Each unit is placed in the Vanguard (front) or the Rearguard (back), re-set each round. Melee units belong in front — the front is also the shield; ranged units belong in back, firing safely over their own line.

2. **Standing casts** — Standing abilities (a Wall's brace, a Support's ally buff) are cast now. They are ally-targeted, auto-land, and last the round.

3. **Declare guard** — Each Vanguard declares how it answers an incoming melee blow this round: Trade (strike back) or Block (spend Tempo to out-bid the attacker and take no blow).

4. **Melee contest** — Each engaging Vanguard strikes an enemy Vanguard, paying one Tempo. The defender answers per its guard: Trade — both blows land (a mortally wounded body still lands its committed blow); Block — the defender out-bids the attacker (cards x Finesse, strictly exceed; a tie lands the hit) to take no blow. Untyped Might banks into the per-phase pile; each time the pile clears the target's Toughness, one Health card turns face down.

5. **Ranged fire** — Each Rearguard carrying a ranged attack fires at the enemy front, paying one Tempo. The target may evade by out-bidding the volley (cards x Finesse, strictly exceed) with its own Tempo; otherwise the shot lands.

6. **Declare charges** — A free Vanguard — one that did not engage in the Fray, or whose front-foe fell — may charge the enemy Rearguard, or flank a surviving enemy Vanguard. A locked Vanguard (its struck foe still stands) stays pinned.

7. **Interception** — A charger crossing toward the enemy Rearguard is struck by each living enemy front Vanguard. The charger slips each via the Tempo contest (spending its own Tempo) or takes the blow; a charger cut down crossing never reaches the back. A wide front drains a crosser slip-by-slip, so only a lone high-Finesse, high-Tempo body gets through.

8. **Pre-empt** — The charged Rearguard answers first — a ranged target counter-fires, a melee target strikes back — before the charge's own blow lands in the Breach.

9. **Breach** — Each charger that survived the Volley lands its blow on the now-exposed enemy Rearguard.

10. **Reckoning** — Deferred effects (wound up earlier this round) resolve last. A caster killed in the Breach has its deferred effect fizzle.

11. **Wipe pile** — At a phase boundary the per-phase damage pile is cleared: sub-threshold damage that did not turn a Health card does not carry into the next phase. Only Health persists.

12. **Refresh (the Lull)** — Round end: all spent Tempo resets, Health carries over, and the round advances. A battle not decided within five rounds is a draw.

## Cross-cutting behaviors

- **Area of effect** — An attack may strike a whole rank at once instead of a single target — width that cannot whiff against a crowd, at the price of not concentrating its force.

- **Grouping** — Same-side units may be bound at form-up into one unit (one position, one shared target, distinct Health): a group sums its members' Tempo to block but needs every member to beat the attacker to slip — a superb wall and a hopeless slipper.

