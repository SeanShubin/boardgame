# Engine migration → the engagement-schedule model

**Status (this run).** Spec §4 (+ §2.2/§4.5/§4.6/§8.5 ripples) **promoted to canon** on branch
`engagement-migration` (commit `33d04f3`). The **engagement model is validated** (`engagement.rs` sim +
`balance.rs`/`engagement.rs` matrix & RPS probes; triangle holds on numbers ≤ 3). The **live engine still
runs the old attrition/six-phase model** — this doc is the plan to make `engagement.rs`'s model THE engine.

## Why it's one big change, not increments

The combat core is tightly coupled: `rules.rs` (the `Rule` registry) feeds `game.rs`'s phase schedule
**and** `ruleset.rs`/`balance.rs`; `state.rs`'s `Phase`/`Round` drive `game.rs` + `combat.rs` + `solver.rs`.
Touch the `Rule` enum or `Phase`/`Round` and the build breaks until **all** of them conform. So this lands
as **one cutover on the branch**: rewrite, get it compiling, then re-green ~50 behavioral tests. The
validated **`engagement.rs` is the reference to port from** (it already conforms to the new §4).

## The reference resolver (`engagement.rs`) — what's proven

- `Intention { Vanguard, Outrider, Rearguard }`; the `SCHEDULE` (Intercept `V→O`, Volley `R→O`, Raid `O→R`,
  Clash `R→V`+`V→V`, Breach `V→R`+`O→V`+`O→O`); one shared per-round Tempo pool; `avoid_cost = Fa/Fd + 1`;
  strike-back gated (alive + flippable); the default policy (prey-with-fallback, every-Tempo-spend-must-matter).
- **Not yet in the sim (add during the port, per §4.5/§4.2/§4.6):** groups (sum-to-block / weakest-link-to-slip /
  spillover / AoE / Hoard), the melee same-range **trade** + the optional **Clash**, `cast`/`resolve`
  windows + Reckoning/disrupt, Controller debuffs. The minimal triad sim exercises only single-target
  melee/ranged on the schedule.

## File-by-file

1. **`state.rs`** — `Phase` enum → the schedule (`DeclareIntentions`, `Reveal`, `Standing`, `Intercept`,
   `Volley`, `Raid`, `Clash`, `Breach`, `Reset`; keep `Clash` module phase). `Round`: replace
   `hero_vanguard: Vec<bool>` / `foe_vanguard` (2 positions) with `intent: Vec<Intention>` per side (3);
   **drop** `hero_guard` (Trade/Block — the contest is unified now) and the **per-unit lock** (`hero_locked`/
   `foe_locked` — the Outrider is declared). Add a schedule cursor if needed.
2. **`rules.rs`** — `Rule` enum → engagement phases + behaviors:
   phases `DeclareIntentions, RevealIntentions, StandingCasts, Intercept, Volley, Raid, Clash, Breach,
   WipePile, Refresh`; behaviors `TempoContest, StrikeBack, Grouping, AreaOfEffect`. Update `ALL_RULES`
   (round order), `info()` descriptions (diegetic), keep `appendix()`. Regenerate `combat-phases.md`
   (`cargo run -p deckbound --example handbook`) and update the `handbook` golden + `rules.rs` tests.
3. **`ruleset.rs`** — generic over `Rule` already; fix the two tests that name retired variants
   (`Interception`, `DeclareGuard`) → new variants. The `u16` bitset still fits (≤16 rules).
4. **`combat.rs`** — port the engagement resolution: the per-engagement declare→resolve→apply over the
   `SCHEDULE`, the unified Tempo contest (port `melee_trade`/`intercept`/`cards_to_evade` → one contest),
   **groups/spillover/AoE** (`groups.rs` already exists — wire it in), strike-back (flippable+alive). Keep
   `stats.rs` (pile→bar→pool) unchanged — the per-engagement pile is the per-phase pile.
5. **`game.rs`** — `legal_actions`/`apply` over the new phases: DeclareIntentions = pick `Intention` per
   unit (+ grouping); per-engagement = pick targets / defend (dodge / eat / strike-back) within reach +
   back-access; drop the Guard declaration. The foe AI (`foe_fray`/`foe_volley`/`foe_pick_target`) → an
   intention-keyed policy (port `engagement.rs`'s `prey`/`choose_target`/`flippable_prey_alive`). Update
   `render`/`status`/`view` labels (positions → 3 intentions). Most menu/Clash/reference code is
   position-agnostic and stays.
6. **`solver.rs`** — searches the `Game`; should work once the `Game` impl is clean. Ensure the state hash
   (`session_key`/the solver's state key) includes `intent` so transpositions are correct. Re-tune the
   analysis-envelope expectations.
7. **`scenarios.rs` / `booklet.ron`** — actors gain a **default intention** (by stats, per
   `engagement.rs::default_intention`): ranged → Rearguard, melee+Finesse≥2 → Outrider, else Vanguard.
   Seed the triad numbers (below).
8. **Tests (~50)** — `game.rs` (17), `combat.rs` (23), `solver.rs` (7) behavioral tests encode the old
   model; rewrite to the schedule. `balance.rs` probes + `engagement.rs` probes already test the new model.
   `reference.rs`/`transcript.rs`/`scenarios.rs` goldens regenerate. Target: whole `deckbound` suite green.

## Seeded numbers (AI-proposed, human-tunes — `booklet.ron`)

The smallest triad that proves the triangle in the validated sim (stats `(Might, Vitality, Toughness,
Cadence, Finesse)`):

| Role / intention | class | M | V | T | C | F |
|---|---|---|---|---|---|---|
| Hold (Vanguard) | Fighter | 1 | 2 | 3 | 1 | 1 |
| Break (Outrider) | Assassin | 2 | 1 | 1 | 2 | 2 |
| Deal (Rearguard) | Mage | 3 | 1 | 2 | 1 | 1 |

Gradient: `M_breaker(2) ≥ T_dealer(2) > M_tank(1)`; only `M_deal(3) ≥ T_hold(3)`. (Mage T was 1 in the
sim's last committed triad; **T2** is the §4 value so the tank's fallback breach bounces — re-validate at
T2 during the port.)

## Recommended sequence

state.rs + rules.rs (the data shapes) → combat.rs (resolution, port from `engagement.rs`) → game.rs
(legal_actions/apply/AI/render) → solver sanity → regenerate goldens → re-green the ~50 behavioral tests.
Do it as a focused effort on this branch; the suite is the acceptance test.
