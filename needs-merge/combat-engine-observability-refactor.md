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
- **P3 — 2D layout.** `Layout { side × rank × slot }` + group adjacency; derive/replace the Intention tag.
- **P4 — Resolution Step machine (BEHAVIOR-PRESERVING).** Two-layer API (user direction 2026-06-29): the
  **high-level** `apply(Deploy)` stays identical (same phase-boundary State); it *delegates* to a
  **low-level** `combat::step(state)` that advances ONE atomic transition for debug/per-strike
  observability. `resolve_round` becomes `while step(state) {}`. Add `Phase::Resolve(Resolution)` (cursor +
  pending strikes) and per-actor `PendingDamage{aoe, targeted}` (targeted = the old `health_pile`; **aoe
  stays 0** — observable structure only). `sim step` subcommand. **Guard: suite stays exactly 88/9 — the
  decomposition must not move any goldens.** The rule-port is explicitly NOT here.
- **P5 — Cleanup.** Wire groups (#11) / deferred (#12); delete dead six-phase code (#13).
- **P6 — Port canon rules into the live engine (DELIBERATE behavior change, separate).** Cycling /
  two-pool AoE+spillover (populates the `aoe` pool) / conditional R→R / melee-reflexive strike-back / role
  priorities, from the validated `engagement.rs` sim (#14). This MOVES the behavioral goldens
  (reference_combat_bands, campaign wins, solver_wins) — re-tune/regenerate as part of it. Was conflated
  into P4; split out per user direction (the Step decomposition is behavior-preserving; aligning live
  combat with canon is a distinct, opt-in change).

**Status:** **P1 DONE** (commit `96d0e74`): `State` serializes through RON (serde across the ownership
tree + engine `Rng`/`Outcome`; `scenario`/`campaign` are `#[serde(skip)]`); `examples/sim.rs` gives
`apply`/`run` with `--state`/`--out` file-or-stdio. Suite 87/9 (the 9 are the known migration follow-ups).
**P2 DONE** (commit `37493d7`): `Health` is a `Vec<HealthCard{toughness, down}>` deck with facing; accessors preserve every read; `take_with_toughness` bit-identical for a uniform deck. Tempo left as a fungible count-deck. Suite 88/9.

**P4 is a BEHAVIOR CHANGE, not behavior-preserving** — porting the sim's cycling / two-pool / R→R / strike-back / priorities into the live resolver makes live combat resolve *differently*, so it will move the behavioral goldens (reference_combat_bands, campaign wins, solver_wins) and needs deliberate re-tuning/regeneration, not a "no-new-failures" guard. P3 (2D layout) is behavior-neutral groundwork and **not a prerequisite for P4's mechanics** (vec-order + the existing `Round.*_group` ids suffice for spillover, as the sim does) — it's an observability/UI view that can land before or after P4.

(Owner: this instance — merge into `docs/` / mainline when settled.)
