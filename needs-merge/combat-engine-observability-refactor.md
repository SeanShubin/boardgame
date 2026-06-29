# Combat-engine refactor — per-action observable state

**Goal (user, 2026-06-29).** `apply(state, action) -> state` must be **total, fine-grained, and
serializable**: take a state + an action and see the new state **at the per-action level**, not just at
phase boundaries. The model exposes **1D decks** (Health, Tempo, weapon/action cards with facing), a **2D
layout** (side × rank × slot, group adjacency), and **pending-damage counters split into AoE + targeted**.
CLI utilities load state+actions from filesystem/stdin and write the resulting state to filesystem/stdout.

**Scope chosen:** **full physical model** (Health/Tempo as real card decks with facing; cards via the
`zones.rs` Hand/Active/Down machine; explicit 2D layout) + a **per-strike Step machine** (each atomic
resolution transition is one `Step` returning a resting State).

## Why (the two blockers found in the live-engine map)

1. **Resolution is batched.** The whole 5-step schedule runs inside one `combat::resolve_round`
   (`combat.rs:599`) fired by `apply(Deploy)` (`game.rs:859`). `Phase::Engage` is transient. You only ever
   observe DeclareIntentions ↔ next-round. All within-engagement counters (piles, bids) are stack locals.
2. **`State` is not serializable** (`state.rs:121`, only `Clone, Debug`). Save/load is event-replay of a
   `Vec<Action>`, not state serialization.

## Target shape

```
enum Phase { Menu(Menu), DeclareIntentions, Resolve(Resolution), Clash(Clash) }
struct Resolution { step, cycle, cursor: Declaring|Contesting(Decl)|Applying|Boundary, pending: Vec<Decl> }
// Actor: PendingDamage { aoe: u32, targeted: u32 }   (replaces the single health_pile)
// Actor: Health/Tempo as 1D decks of cards with facing; weapon/actions via Zone (Hand/Active/Down)
// State: a Layout { side × rank(V/O/R) × slot } with group adjacency (replaces Intention-as-bare-tag)
```
A new **`Step`** action advances the resolution machine **one atomic transition** (declare next strike →
resolve next contest → apply a flip → finalize deaths → advance engagement → wipe piles). A headless/UI
driver loops `Step` until it rests. Every micro-step is a serializable, observable resting State.

## The gap list (current → needed)

**Counters (move resolver locals into State):**
1. Targeted/spillover pending pool, *persisted* between micro-steps (today `health_pile` is wiped inside
   the resolver → always 0 on return).
2. Separate **AoE** pending pool (today a single pile; the two-pool model lives only in the sim).
3. Resolution **cursor** (step + cycle + sub-phase).
4. **Pending declarations** (committed-unresolved strikes; today a local `Vec<Decl>`).
5. **Pending contest** (a declared strike awaiting the defender's evade/eat; today a local).

**Cards / zones (full physical model):**
6. Health as a 1D deck with facing (face-up intact / face-down flipped), not an integer `remaining`.
7. Tempo as a 1D deck (available/spent), not `i32`.
8. Weapon/action cards through the `zones.rs` Hand/Active/Down machine (it exists, unit-tested, but is
   **unused** by `State`/`Actor`/`combat`).

**Layout:**
9. A 2D combat layout (side × rank × slot) with group adjacency + front-to-back spillover order; today
   position is only the `Intention` tag (the 2D `world.rs` grid is campaign-only).

**Serialization:**
10. `Serialize + Deserialize` on `State` and everything it owns (today only `Action`/`Move` serialize).

**Plumbing / correctness:**
11. Wire **groups** into the live resolver (`Round.foe_group` is dead).
12. Decide **deferred/Reckoning** (populated by `do_play_card`, never fired by `resolve_round`).
13. Remove **dead six-phase resolvers** in `combat.rs` (`fray_clash`, `intercept`, `resolve_volley`,
    `resolve_breach`, `resolve_reckoning`, `compute_locks`).
14. The live resolver predates the §4.5/§4.6 amendments — **port from the `engagement.rs` sim**: cycling to
    exhaustion, the two-pool AoE/spillover accumulator, conditional `R→R`, melee-reflexive strike-back,
    target-judged positive-effect, and the role priority lists. (The sim is the validated reference.)

## Phased plan (each phase its own commit; keep the suite no-worse-than-current)

- **P1 — Serialization + CLI skeleton.** Add `Serialize/Deserialize` to `State` + owned types. New
  `examples/sim.rs` (or a bin): `apply` / `step` / `run` with `--state`/`--out` (file or `-` for std).
  Round-trip test. *Behavior unchanged.* ← delivers the load→apply→write loop immediately.
- **P2 — Physical decks.** Health & Tempo → 1D card decks with facing; weapon/action cards → `Zone`.
  Behavior-preserving (a deck of N face-up cards == the old count). Touches `stats.rs`, `actor.rs`, every
  read site.
- **P3 — DONE** (commit `3803901`): `layout.rs` exposes a derived `CombatLayout` (side × rank × slot +
  `Rank::group_runs` adjacency), `State::layout()`, serde, `sim layout`. Behavior-neutral (derived view, not
  authoritative); downed actors kept with a `down` flag. (Named `CombatLayout` — `world::Layout` owns `Layout`.)
- **P4 — DONE** (commit `8269f11`): `resolve_round = { resolution = Some(start()); while step(state) {} }`.
  `State.resolution: Option<Resolution{step,pair,stage}>` (serde default); `PendingDamage{aoe(0),targeted}`
  replaces `health_pile`; `combat::step` advances one (pair,side) or a boundary; `sim step` added. Guard
  `stepping_reproduces_resolve_round_exactly` (stepped == batched, field-for-field + identical log + RON
  round-trip of micro-states). Suite 90/9 (88 + 2 new, same 9). **Step granularity = per (pair, side) +
  engagement boundary** — the finest *meaningful* unit given §1.9 order-independence within a pair (finer
  than a pair has no defined intermediate state). Original (now-superseded) P4 description:
- **P4 (orig) — Resolution Step machine (BEHAVIOR-PRESERVING).** Two-layer API (user direction 2026-06-29): the
  **high-level** `apply(Deploy)` stays identical (same phase-boundary State); it *delegates* to a
  **low-level** `combat::step(state)` that advances ONE atomic transition for debug/per-strike
  observability. `resolve_round` becomes `while step(state) {}`. Add `Phase::Resolve(Resolution)` (cursor +
  pending strikes) and per-actor `PendingDamage{aoe, targeted}` (targeted = the old `health_pile`; **aoe
  stays 0** — observable structure only). `sim step` subcommand. **Guard: suite stays exactly 88/9 — the
  decomposition must not move any goldens.** The rule-port is explicitly NOT here.
- **P5 — DONE** (commit `3803901`): deleted the dead six-phase cluster (`fray_clash`, `intercept`,
  `compute_locks`, `fray_one`, `ranged_one`, `melee_trade`, `ranged_shot`, `combat::Guard`) + stale doc
  links; rewrote the `combat.rs` module header for the current model. Kept Reckoning/Burn/token machinery
  (P6 wires it) and all live helpers. **Wiring groups (#11) / deferred (#12) is a behavior change → moved
  to P6a**, not done here. Behavior-neutral; suite 92/9.
- **P6 — DONE** (commits `76eab6b` port + `69bcede` per-engagement cycling fix + `40a5a82` doc). Policy
  cleaved into `policy.rs`; mechanics ported from the sim; cycling is per-engagement (§4.6 order-independent).
  `solver_wins` flipped GREEN; no passing test regressed. **Deferred (separate later work):** Reckoning
  firing (`resolve_reckoning` still uncalled) + offensive-ability casting (`card_playable_now` Standing-only).
  **Golden review in progress** (post-P6): regenerate the 2 pure-doc goldens (rules-reference, glossary);
  the 6 combat-dependent (rules-tour transcript, reference_combat_bands, 3× campaign, action_log) need a
  correct-canon-vs-regression judgment before re-baselining — several may be *fixed* by P6, the campaign
  wins may need re-tuning if the party's scripted run now resolves differently. Original split below:
- **P6 (orig) — Align the live engine with canon (DELIBERATE behavior change).** Split along the mechanics /
  interaction seam (user direction 2026-06-29): *mechanics are the game (a rulebook statement); the policy
  is how our code chooses among legal moves (swap human / scripted AI / solver and the mechanics don't
  change).* **Anchor examples: grouping = mechanic; target priority = interaction.** Build it by **cleaving
  the resolver from the policy** — today `resolve_pair` bakes prey-with-fallback targeting *into*
  resolution; separate them.
  - **P6a — mechanics → the resolver (canon, decision-agnostic).** Groups (spillover / melee-all-spend
    crossing / pooled-block / weakest-link-slip), AoE + the two-pool accumulator (populates the `aoe`
    pool), conditional R→R, back-access "broken line" gate, melee-reflexive strike-back (defender must be
    melee-capable), the *capability* to act repeatedly while Tempo allows (cycling-as-rule), Reckoning
    firing, offensive-ability *effects*. Already canon in Spec §4.5/§4.6; port from the validated
    `engagement.rs` sim. The resolver takes *committed decisions* and applies these rules — same whether
    the decider is human, AI, or solver.
  - **P6b — policy → a separate decision module (the swappable balance proxy).** The role priority lists
    (V: O→V→R, etc.), the positive-effect rule (commit a strike only when the *combined* committed Might
    flips — skip futile spends), *when* to stop cycling, *when* to cast an offensive ability. Canon calls
    this "policy, not a hard rule." It *produces* the decisions the resolver consumes; PvE uses the
    predictable stand-in, the solver searches, a human plays.
  - This MOVES the behavioral goldens (reference_combat_bands, campaign wins, solver_wins) — re-tune /
    regenerate as part of it; several are failing *because* the live engine is behind canon, so P6 likely
    turns some green. The P4 Step machine is the scaffold: a step is either a **mechanics transition**
    (deterministic) or a **decision point** (the policy/human chooses) — human/solver/AI all plug into one
    mechanics core.

**Status:** **P1 DONE** (commit `96d0e74`): `State` serializes through RON (serde across the ownership
tree + engine `Rng`/`Outcome`; `scenario`/`campaign` are `#[serde(skip)]`); `examples/sim.rs` gives
`apply`/`run` with `--state`/`--out` file-or-stdio. Suite 87/9 (the 9 are the known migration follow-ups).
**P2 DONE** (commit `37493d7`): `Health` is a `Vec<HealthCard{toughness, down}>` deck with facing; accessors preserve every read; `take_with_toughness` bit-identical for a uniform deck. Tempo left as a fungible count-deck. Suite 88/9.

**P4 is a BEHAVIOR CHANGE, not behavior-preserving** — porting the sim's cycling / two-pool / R→R / strike-back / priorities into the live resolver makes live combat resolve *differently*, so it will move the behavioral goldens (reference_combat_bands, campaign wins, solver_wins) and needs deliberate re-tuning/regeneration, not a "no-new-failures" guard. P3 (2D layout) is behavior-neutral groundwork and **not a prerequisite for P4's mechanics** (vec-order + the existing `Round.*_group` ids suffice for spillover, as the sim does) — it's an observability/UI view that can land before or after P4.

(Owner: this instance — merge into `docs/` / mainline when settled.)
