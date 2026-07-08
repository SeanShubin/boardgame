# Cardtable ↔ Deckbound Reunification & Reorg — Automated-Pass Plan

**Status:** design, not yet executing beyond Phase 0.
**Owner:** this instance (folds into `docs/` when settled, per the needs-merge convention).
**Nature:** a long, mostly-unattended, **behavior-preserving** refactor. Same behavior in →
same behavior out. That invariant is what lets the pass run with few questions.

Working crate names below are **placeholders** — the boundary matters, not the names. Any
rename is cosmetic and behavior-preserving, so it is decided by policy (§8), not by asking.

---

## 1. Goal — the boundary

One clean abstraction boundary where **the game declares what it needs and the renderer provides
all game-agnostic services**. That boundary already exists half-built: it is `contract`.

- **Game → renderer (declares needs):** `deckbound` implements `contract::Game` and emits a
  `TableView` — a declarative snapshot of zones, cards, and legal actions. No renderer words.
- **Renderer → services (game-agnostic):** `cardtable` consumes the snapshot via
  `cardtable_model::from_table_view` and owns everything mechanical — draw piles, focus/zoom,
  drag-to-arrange, drill in/out, route clicks back as action requests, persistence. No game words.
- **The seam is data, not calls:** the view flows down, action requests flow up. Neither side
  references the other's internals.

The design is sound and already present. **The shipping product bypasses it** — `boardgame`
hand-builds a `Tableau` from deckbound's `catalog`/`sample_table` and reaches combat through a
side door, leaving `Game`/`TableView`/`apply` idle. This plan routes the product **through** the
seam and deletes the bypass, so the boundary that ships is the boundary we want.

## 2. Invariant — behavior preservation (the acceptance criterion)

Every step must be provably behavior-identical. "Behavior" is pinned at the **pure-state**
surfaces, because rendering is a pure function of them (identical `Tableau`/`TableView` ⇒
identical pixels, so we never need a GPU in the loop):

1. **`sample_table()`** — the opening world, serialized to RON.
2. **Interaction transcripts** — a scripted sequence of model operations (select / grow / move /
   drill in / drill out) applied to the sample table, snapshotting the `Tableau` after each op.
3. **Auto-combat** — `resolve_encounter` outcome + resulting `Tableau`, over a fixed seed set.
4. **Manual combat** — `begin_manual_combat` then stepping every decision with a fixed policy;
   snapshot per-step outcomes + final `Tableau`, over a fixed seed set.

A step is **accepted** iff, after it: `cargo build` + `cargo clippy -- -D warnings` + `cargo test`
are green **and** all golden masters from §3 are byte-identical to the pre-pass baseline.

Determinism makes this stable: everything flows from a seed (no wall-clock, no unseeded RNG).

## 3. The characterization harness (Phase 0 — build FIRST)

Golden-master (a.k.a. characterization) tests capture *current* behavior **before** any refactor,
then guard it after every step. This harness is the thing that lets the pass run unattended.

- Lives as **Rust `#[test]` code**, not ad-hoc scripts (repo guardrail: no Python/shell helpers).
- Snapshots are RON files checked into the repo under a `tests/golden/` tree; the test
  re-serializes and compares. `cardtable-model` already dev-deps `ron` for exactly this.
- Targets: the four surfaces in §2. Combat surfaces are pure functions over the model
  (`cardtable-combat` is "a plain function over the model — unit-tests in isolation"), so they
  snapshot without Bevy.
- **Baseline captured at the pass's first commit and never regenerated** except when a step is a
  deliberate, logged behavior change (there should be none in this plan; if one appears it is a
  §9 deferred question, not a silent re-baseline).

Phase 0 exit: harness exists, baseline committed, all golden tests green on untouched code.

## 4. Target architecture (working names)

```
generic substrate — driven ONLY by contract; no game word compiles here
  contract      the seam: Game + TableView + actions      ← THE boundary (grow additively only)
  engine        Zone / Rng toolkit
  card-model    Tableau/Pile/Card + focus/zoom + from_table_view   (purge catalog/fixtures/day)
  card-render   Bevy renderer: draw, drill, drag, route clicks     (purge arena/combat)

the game — everything deckbound, behind the seam
  deckbound         rules; implements contract::Game, emits TableView (absorbs cardtable-combat)
  deckbound-balance solver/duel/balance + analysis examples + data  (extracted from deckbound)

composition root
  boardgame     wires the game's Game impl into the renderer + persistence/window

not-the-game tooling — demoted out of peer position (non-destructive: moved, not deleted)
  tools/combat-lab   separate gear-system experiment
  tools/gatcg        Grand Archive downloader (nothing depends on it)
```

`tabletop` and `deckbound-sample` survive as "the other renderer over the same seam" — no longer a
parallel world, just a different renderer choice behind the same `Game`.

## 5. Generic-service inventory (what stays vs. what moves)

The test for each deckbound-named thing now in the generic crates: *find the game-agnostic service
hiding inside it.* The service stays in the renderer; the meaning moves behind the seam.

| In `card-render` / `card-model` today | Generic service it becomes (stays) | Meaning (moves to deckbound, via TableView/apply) |
|---|---|---|
| `ArenaCombat`/`ArenaState`/`drive_arena` | render an interactive sub-zone from a view; send clicks as actions | "this zone is an arena"; evolve it via `apply` |
| `CombatRequest`/`ManualCombatRequest` | already generic: `ActionRequests` | "this action starts a fight" |
| phase/tempo/strike/evade labels | draw a card: title + stat lines from the view | the words/numbers are deckbound's view content |
| `catalog` (stat/strike/creature cards) | — (no generic service) | pure deckbound content |
| `sample_table()` fixtures | — (a renderer ships no world) | deckbound's opening `TableView` |
| `character_recipe`/`current_day`/`advance_day` | — (game-state queries) | deckbound state; renderer only shows cards |

Payoff: once the arena is "just another zone the view declares and the player acts on,"
`card-render` needs **zero** combat knowledge — the arena becomes emergent from the `Game` trait.

## 6. The load-bearing work — seam expressiveness

The product bypassed `contract` almost certainly because `TableView`/`apply` could not yet express
the interactive arena (per-blow evade/strike-back prompts, animate-the-diff). So the crux is
**growing the seam until a full deckbound turn — including an interactive fight — round-trips
through `Game → TableView → actions`.** Combat becomes a sequence of decision-states: each prompt
is a game state awaiting a player action, surfaced as prompt-cards with legal actions; animation is
a `card-render` reaction to view deltas. Deckbound's resolver already models this
(`PendingDecision`/`StepOutcome`), so it is feasible.

Constraint (existing seam rule): **grow `contract` additively only** — never break `tabletop`.

## 7. Execution plan — phased, each phase a verified checkpoint

Each phase ends at a compiling, test-green, golden-identical commit (staged by explicit path, never
`git add -A`). The pass can stop/resume at any checkpoint.

- **P0 — Characterization harness.** Build §3. Baseline committed, all golden green. *(non-destructive)*
- **P1 — Prove the seam carries a fight.** In `deckbound-sample` (seam already wired), grow
  `contract`/`deckbound` so a full turn incl. an interactive fight round-trips through
  `Game → TableView → from_table_view → Tableau`, reproducing the P0 combat golden masters. This is
  §6 and the riskiest phase; do it first so any real gap surfaces early.
- **P2 — Route `boardgame` through the seam.** Replace the hand-wired `Table = sample_table()` +
  `resolve_combat`/`resolve_manual_combat` bypass with `deckbound: Game → TableView`. Golden masters
  must hold: the seam-built `Tableau` equals the fixtures-built one, op for op.
- **P3 — Purge the generic crates.** Remove `catalog`/`fixtures`/day-queries from `card-model` and
  arena/combat from `card-render`. They must stop compiling any game word; the deckbound content
  moves into `deckbound` (absorbing `cardtable-combat`). Golden masters unchanged.
- **P4 — Extract `deckbound-balance`.** Move `balance.rs`/`solver.rs`/`duel.rs` + analysis examples
  + `data/balance/` out of `deckbound`. Pure move; deckbound tests + balance examples still run.
- **P5 — Demote tooling.** Move `combat-lab`, `gatcg` under `tools/`. Non-destructive; workspace
  still builds.
- **P6 — Rename to honest names** (if adopting §4 names). Cosmetic, behavior-preserving.
- **P7 — Quality review.** Only now run the ECS quality pass, against honest boundaries.

## 8. Decision policies (resolve without asking)

When a choice arises mid-pass, apply in order:

1. **Preserve behavior.** If an option changes a golden master, it's wrong (or a §9 question).
2. **Generic crates stay game-word-free.** Prefer the option that removes a game word from
   `card-render`/`card-model`.
3. **Grow `contract` additively only.** Never break `tabletop`/`deckbound-sample`.
4. **Minimize public-API churn**; when churn is needed, keep the old surface until its callers move.
5. **Non-destructive for tooling** (move, don't delete `combat-lab`/`gatcg`).
6. **Names:** pick sensible ones and record them here; do not ask. Generic crates keep honest
   names; game-side new crates take a `deckbound-` prefix.
7. **Anything not settled by 1–6 that would change behavior → §9 (defer), don't guess.**

## 9. Deferred-questions log (genuine behavior forks only)

*(empty — append here if the seam genuinely cannot express something the bypass did; each entry:
what, the exact gap, the options, why it needs a human.)*

## 10. Observations (non-blocking; not behavior changes to make in this pass)

- **The product's RON save is non-canonical.** `Tableau` stores `piles`/`cards` in `HashMap`s, so
  serde emits them in per-process-random order — two saves of the same table differ textually.
  Harmless today (autosave dedup is per-process; the fingerprint is per-process), but it means the
  witness must canonicalize (it does). A future *deliberate* change could swap to `BTreeMap`/
  `IndexMap` for stable saves — out of scope here (it would change the on-disk format).

## 11. Progress log (append-only)

- **P0 — DONE (pending commit).** Witness crate `crates/characterization` added to the workspace.
  Six golden-master tests pin all four §2 surfaces and are deterministic across processes + clippy-
  clean: `sample_table`, `interaction_transcript`, `auto_{marksman_seed1,marksman_seed7,
  executioner_seed1}_cinderwatch`, `manual_marksman_cinderwatch_seed7`. Canonicalization sorts every
  map by key (see §10). Bless with `BLESS=1 cargo test -p characterization`. Not yet committed
  (awaiting go-ahead on the per-checkpoint commit cadence).
</content>
