# Tutorial sequence — one mechanic per log, in dependency order

The ordered spine for the tutorial series (`../../brainstorming/tutorial-design.md` is the philosophy;
this is the topological sort it asked for). Each log teaches **exactly one new mechanic** and may use
**only** mechanics taught in **earlier** logs — and only the ones **necessary** to teach the current one.
Same worked-card, action-by-action style as `../designer/card-combat-all-mechanics.md`, which is the
**capstone** (#20) where everything reassembles. Read `../combat-reference.md` for notation.

**How to read the list.** *Teaches* = the single new mechanic. *Needs* = the earlier logs (by #)
it builds on. *Scenario* = the smallest board that makes the mechanic fire. The five stats enter
as parameters of the first mechanic that uses them (M/V/T at #1; C at #2; F at #3).

**The six phases assemble across the series** — they are *not* introduced all at once: the **Lull**
at #5, the **Fray** at #7, the **Volley/Breach** at #13, the **Reckoning** at #15, the **Standoff**
at #19; the capstone (#20) runs all six in order. The governing **PRINCIPLE** (within-phase
order-independence + between-phase death-timing, §4.6) is shown progressively — §1.3 at #7,
pre-empt at #13, disrupt at #16, flank-intercept at #17 — and stated outright at the capstone.

---

## Chapter 1 — the single clash (1v1; no positions, no phases)

| #     | Teaches                                                                                                                                        | Needs | Scenario                                                                                                     |
| ----- | ---------------------------------------------------------------------------------------------------------------------------------------------- | ----- | ------------------------------------------------------------------------------------------------------------ |
| **1** | **The wound** — Might → pile → flip a Health card at Toughness; overflow wasted; death *(introduces M, V, T, the accumulator pile)*            | —     | One attacker Punches a passive dummy until it falls; watch the pile build to Toughness, flip a card, repeat. |
| **2** | **Tempo** — Cadence cards = actions per round; you act only as often as your Tempo allows                                                      | 1     | A short-Cadence attacker runs dry mid-round; the dummy survives because the attacker is out of actions.      |
| **3** | **The contest + EAT / AVOID** — Finesse, the bid `cards×F`, **strictly-beat (tie lands)**; defender spends Tempo to AVOID, or EATs to conserve | 1, 2  | Defender out-bids one hit (negated), **ties** another (it lands), EATs a third to save Tempo.                |
| **4** | **STRIKE-BACK** — the mutual counter (both land; the counter needs your own attack)                                                            | 3     | Defender counters instead of avoiding; both take a wound.                                                    |
| **5** | **Across rounds** — Health **persists**, Tempo **refreshes**, the pile **resets at round end**, 5-round cap                                    | 1, 2  | A duel over several rounds: wounds carry, Tempo refills, the pile clears each round; first to fall loses.    |

## Chapter 2 — the battlefield

| #     | Teaches                                                                                                                                | Needs   | Scenario                                                                                                                        |
| ----- | -------------------------------------------------------------------------------------------------------------------------------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------- |
| **6** | **Positions & reach** — Vanguard/Rearguard; melee vs ranged reach; a back is shielded by its front (untouchable while the front lives) | 1, 2, 3 | A front + a back archer per side; archers fire over the line at enemy fronts; neither back is reachable while its front stands. |

## Chapter 3 — the front clash & groups

| #      | Teaches                                                                                                                                                                                 | Needs   | Scenario                                                                                                                                 |
| ------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| **7**  | **The Fray** — the round has ordered **phases**; the Fray is the front clash, resolved **simultaneously / order-independently**; a dying body still lands its committed blow (**§1.3**) | 3, 4, 6 | Two fronts clash at once; a unit is reduced to zero but its committed strike still lands.                                                |
| **8**  | **Groups: sum-to-block & spillover** — Join `{A=B}`; melee AVOID by **pooling** Tempo (sum of bids); single-target damage **spills** in declared order                                  | 3, 6    | A joined front sum-blocks a melee hit (one Tempo per member); a hit spills onto the first-declared member.                               |
| **9**  | **Weakest-link evade** — group ranged AVOID: **every member must beat the bid alone**; the weakest gates it                                                                             | 6, 8    | The same group can't dodge a ranged shot because its lowest-Finesse member can't beat the bid even maxed — a strong wall, a poor dodger. |
| **10** | **Hoard** — a built-in group of *n* one-Health bodies                                                                                                                                   | 8       | A `Hoard⟨3⟩` attacks and defends as a group, losing one body per landed hit.                                                             |
| **11** | **Area (AoE)** — hits **every** member of a group at once (not spillover)                                                                                                               | 8       | An area attack lands on all of a group simultaneously; contrast the declared-order spillover of #8.                                      |

## Chapter 4 — the back line

| #      | Teaches                                                                                                                                                                                                                                                                                         | Needs    | Scenario                                                                                                                                                     |
| ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **12** | **The breach list (per-unit lock)** — the Fray fixes it: a Vanguard is **free** if every foe it struck is dead (only attacking locks), else **locked**; the back opens only to a free unit                                                                                                      | 6, 7     | A kills one enemy front → that killer comes **free** while another (its foe alive) stays **locked**; the enemy back opens only to the free one.              |
| **13** | **The charge & the pre-empt** — a free Vanguard **charges** the rear in the **Volley**; the rear answers **first** (pre-empt) and can drop it before its blow lands in the **Breach** *(introduces the Volley & Breach phases, and resolve-timing: `on-cast` vs the charge's derived `breach`)* | 3, 6, 12 | The free charger meets a rear that counter-fires/dodges first; show it **stopped** in the Volley, and a tougher charger **surviving** to land in the Breach. |
| **14** | **The per-phase pile** — each phase owns its accumulator; sub-threshold damage **wipes at the phase boundary**; only Health crosses                                                                                                                                                             | 1, 13    | A unit chipped (no flip) in the Fray enters the Volley with a clean pile; only its flipped Health carried.                                                   |
| **15** | **Held attacks & the Reckoning** — the **cast / resolve** model; a **held** (`resolve: reckoning`) attack pays up front and lands **last**, in the Reckoning                                                                                                                                    | 13       | A rear caster winds up a held attack in the Volley; it resolves in the Reckoning after everything else.                                                      |
| **16** | **Disrupt** — kill the held caster **before** the Reckoning (in the Breach) → its attack **fizzles**                                                                                                                                                                                            | 13, 15   | A charger reaches the caster and kills it in the Breach; come the Reckoning, no caster → the held attack never goes off.                                     |
| **17** | **Flank & intercept** — a free Vanguard may **flank** a surviving enemy front (a **trade**, on-cast); resolving in the Volley, a flank-**kill** **intercepts** a charger                                                                                                                        | 12, 13   | Mutual breakthrough: a freed unit flanks an enemy charger and kills it before its Breach blow — the charge is precluded.                                     |
| **18** | **One ability, both Strike phases / multi-reach** — an `on-cast` ability fires in the **Fray and the Volley**; a body carrying melee **and** ranged uses each in a different phase                                                                                                              | 6, 13    | An archer looses in the Fray, then again at a charger in the Volley; a multi-reach unit melees in the Fray and fires in the Volley.                          |

## Chapter 5 — buffs & the Form

| #      | Teaches                                                                                                                                                                                                                                              | Needs | Scenario                                                                                               |
| ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- | ------------------------------------------------------------------------------------------------------ |
| **19** | **Standing buffs (the Standoff)** — a pre-battle buff that **auto-lands**; a **one-shot** limits via flip-face-down-for-combat; **temporary Tempo** expires at the Lull *(and the Form-ability framing: passive enablers, Tempo-gated, never Spend)* | 2, 7  | A unit opens with Rallying Cry; the party's temp Tempo funds an extra action that round, then expires. |

## Capstone

| #      | Teaches                                                                                             | Needs | Scenario                                            |
| ------ | --------------------------------------------------------------------------------------------------- | ----- | --------------------------------------------------- |
| **20** | **The full round** — all six phases and every mechanic in one battle; the PRINCIPLE stated outright | all   | = `../designer/card-combat-all-mechanics.md` (already written). |

---

## Notes

- **Validity:** every *Needs* entry is an earlier number, so the order is a legal topological sort
  of the dependency graph. No log forward-references a mechanic.
- **Groups (8–11) are independent of the back line (12–18)** — both chapters depend only on
  positions+contest (#3, #6), so their relative order is free; this sequence does the front-clash
  family first.
- **Forcing-function upgrade (per `../../brainstorming/tutorial-design.md`):** each scenario above is a *demonstration*;
  to make it a true tutorial, tighten it until the **naive line provably loses and the keyed line
  wins** (the §6.1 necessity test). That turns the series into an executable regression suite too.
- **Next step:** write the logs in order, starting at #1, reusing the `../combat-reference.md` notation.
