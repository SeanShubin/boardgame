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

## 14. P2 design — the view emitter (opened)

**Emitter-home (decided):** a **new `contract::Game`** for the card-table world, on the deckbound side
(the deckbound-presentation layer; provisional crate name TBD — names are flexible).

**Non-circular state (the key insight):** the emitter's `State` is **not** a `Tableau`. It is compact
world state — party/kits, which locations are cleared, the day, any active fight. `view()` renders that
state to a nested `TableView`; `from_table_view` inflates the `Tableau` renderer-side. The `Tableau` is
never the source of truth, so there is no Tableau→TableView→Tableau round-trip on the game side. This is
why the reunification is *re-expressing* `fixtures.rs` as `(data + a view fn)`, not moving a Tableau.
`apply()` handles equip / march / fight, delegating combat to deckbound's resolver (the logic
`cardtable-combat` already holds). Content (`catalog`/`fixtures`) is referenced from `cardtable-model`
for now; it physically moves in P3.

**Visual fidelity (decided → full):** keep the product's look (Locations `Grid{columns:3}`, the Inn
`Rows`, the Progress `Grid{columns:5}`). The contract `Layout` enum (Stack/Row/Fan) is a CCG vocabulary
that can't express these, and `from_table_view` ignores layout today. So a small additive growth is
needed: carry a **card-table arrangement** on `ZoneView` and map it via `set_layout`. The Inn's equip
view is authored **inline** by the emitter (no model `projection` needed); character decks become
deckbound-internal state (no model `reflects`).

**P2 sub-roadmap:**
- **P2.0 — Carry arrangement through the seam (additive).** Add a contract-side arrangement type +
  `ZoneView` field; `from_table_view` maps it to the model `Arrangement` via `set_layout`. (Consider a
  `ZoneView` builder here — the struct is accreting optional card-table fields, and per-field literal
  churn across deckbound's ~15 sites is a smell.) Verify: goldens unchanged, new binding test, tabletop
  compiles.
- **P2.1 — Scaffold the emitter `Game`** (State/Action/view/apply) and reproduce the **flat banks**
  (Stats/Abilities/Numbers/Heroes/Kit/Bestiary) — assert `view()`→`from_table_view`→`behavior()` matches
  those slices of the behavioral golden.
- **P2.2 — Locations grid + encounters** (nested, arrangement `Grid`), and the **Inn** (inline equip).
- **P2.3 — Interactive fight as zones** (folds in old P1.3): the arena as a zone, per-blow prompts as
  actionable cards; reproduce the combat behavioral goldens.
- **P2.4 — Point `boardgame` at the emitter** via `Game → TableView → from_table_view`; delete the
  hand-wired `sample_table` + `resolve_*` bypass. Behavioral goldens are the acceptance gate.

## 9. Deferred-questions log (genuine behavior forks only)

### Q1 — What does the seam carry? (P1 gate, OPEN)

**The gap.** The current seam `Game::view() -> contract::TableView` is a **flat, CCG-style**
snapshot: `zones: Vec<ZoneView>`, each a flat card list; no recursion, positions, zoom, sizes,
projections, or arena felt. `from_table_view` (the sole bridge) builds a strictly flat `Tableau`
(root → one pile per zone → cards) and even drops `body`/`corner`/`accent`. The product's `Tableau`
is a real card table (nested zones, drill-in/out, drag-positioning, per-card sizes, the arena as a
distinct felt). **Routing the product through the current seam would flatten it — a behavior
regression, which the invariant forbids.** So "reunify onto the seam" forces a decision about what
the seam carries. Not policy-resolvable: each option reshapes `contract` and `tabletop` differently.

- **A — Grow `TableView` card-table-native** (recursive zones + positions/sizes + arena zone; keep
  `view()->TableView->from_table_view`). One seam for both renderers. Cost: `contract` balloons with
  card-table structure that `tabletop` doesn't need; `from_table_view` grows to inflate nesting.
- **B — Seam carries the card table itself** (deckbound authors a `Tableau` via the generic
  `cardtable-model`; `cardtable` provides only generic services). Best matches the stated boundary
  ("deckbound tells cardtable what it needs; cardtable provides generic services") and preserves
  behavior naturally. Cost: `tabletop`/`deckbound-sample` must consume this too (or `TableView`
  survives as a second, legacy seam → the two-worlds problem returns); `contract`'s purity changes.
- **C — Two honest, layered seams** (`TableView` stays the flat CCG seam for `tabletop`; a distinct
  card-table seam has deckbound build a `Tableau` for the product). Truthful if a CCG table and a
  card table are genuinely different renderers. Cost: not "one boundary."

**RESOLVED → Option A (grow `TableView` card-table-native).** One seam, additive, both renderers.
`tabletop` keeps reading `TableView` and ignores the new fields (additive-optional, per the seam rule).

**Scope note (honest):** fully realizing Option A means `TableView`/`ZoneView`/`CardView` must grow
enough to reconstruct the product `Tableau` — recursive zones, pile placement + layout/arrangement
(and eventually projection/reflects), and richer card faces (detail/panel lines, type, badge, size,
and for kit cards a recipe + quantity). At the limit the grown `TableView` approaches an isomorph of
`Tableau`. That is inherent to "one card-table-native seam"; it is the chosen path, recorded so the
pass doesn't pretend it's a small change. The two big lifts are (i) `deckbound::view()` authoring the
nested world (today it emits flat CCG zones) and (ii) the `catalog`/`fixtures` content moving from
`cardtable-model` into deckbound's view emitter.

## 12. Harness refinement — behavioral tier vs byte tier (adopted)

P0's goldens serialize the **entire internal `Tableau`** (ids, positions). That is the right strictness
for phases that preserve the construction path (P3 purge, P4/P5 moves, P6 rename) — internals must not
move. But P1/P2 **rebuild** the table through a new path (`deckbound::view() -> from_table_view`), which
legitimately changes incidental internals (ids, default positions) while preserving what is **shown and
clickable**. Byte-identity would flag those as failures though behavior is unchanged.

So the witness gets **two tiers**:

- **Byte tier (have):** full `Tableau` RON. Guards P3+ (same construction path; internals frozen).
- **Behavioral tier (to build, P1.0):** a projection of what the renderer would show — the recursive
  zone tree of `(label, layout)` with each card's visible face `(title, type, detail/badge, face-up?)`
  and its `actionable` flag — plus the existing combat **outcome + log + mutation-stream** goldens
  (already behavioral). This is stable across construction-path changes, so it is the acceptance
  criterion for P1/P2. Behavior drift still shows; incidental id/position churn does not.

Acceptance criteria update: **P1/P2 assert the behavioral tier; P3–P6 assert both tiers.**

## 13. P1 sub-roadmap (Option A)

- **P1.0 — Behavioral golden tier.** Add the rendered-projection goldens above (bless from current
  behavior). Same seed/scenarios as P0. *(non-destructive; witness only)*
- **P1.1 — Grow the seam, additively.** Extend `contract::{TableView,ZoneView,CardView}` with the
  card-table-native fields (nesting, placement/layout, rich faces), all defaulted so `tabletop` and
  existing games are untouched. Grow `from_table_view` to inflate them. Verify: workspace builds,
  binding test + both golden tiers unchanged (nothing routes through the new fields yet).
- **P1.2 — Prove the static world round-trips.** Have deckbound author a nested `TableView` that
  `from_table_view` turns into a `Tableau` matching the **behavioral** golden of `sample_table` (not
  byte-identical). Proves the world reconstructs through the seam.
- **P1.3 — Prove an interactive fight round-trips.** Same, for a manual fight in `deckbound-sample`:
  the arena as a zone, per-blow prompts as actionable cards, reproducing the combat behavioral goldens.
  Any residual gap that additive growth can't close returns here as a new §9 question.

**Sub-question RESOLVED (empirically, reading `sample_table.behavior.txt`):** `layout`/arrangement is
**presentation**, and `projection`/`reflects` are **model mechanisms** the reunified emitter reimplements
(the Inn becomes inline cards; character decks become deckbound-internal state). All three are dropped
from the behavioral projection, which is now purely *nesting + order + card content + interactivity* —
construction-path-stable, the point of the tier. The refined `*.behavior.txt` goldens are the **spec the
view emitter must reproduce.**

**P1.2 and P2 merge.** The discovery: reproducing the world through the seam *is* routing the product
through it — the "view emitter" (deckbound authoring the nested `TableView`) is the P2 deliverable. So the
next unit is: build that emitter, guarded by the refined behavioral golden, then point `boardgame` at it.
Flat banks + nesting + rich cards are already proven end-to-end by the P1.1 binding tests; what remains is
authoring the *specific* world content (today in `cardtable-model`'s `catalog`/`fixtures`) as a view.

## 10. Observations (non-blocking; not behavior changes to make in this pass)

- **The product's RON save is non-canonical.** `Tableau` stores `piles`/`cards` in `HashMap`s, so
  serde emits them in per-process-random order — two saves of the same table differ textually.
  Harmless today (autosave dedup is per-process; the fingerprint is per-process), but it means the
  witness must canonicalize (it does). A future *deliberate* change could swap to `BTreeMap`/
  `IndexMap` for stable saves — out of scope here (it would change the on-disk format).

## 11. Progress log (append-only)

- **P2.3.0 — DONE** (`7f2d622`). Combat acceptance criterion set to **outcome-parity + fresh arena**
  (user decision). Added `resolve_fight(kit, location, seed)` to the emitter: builds the same `DuelUnit`s
  the old path built (kit from catalog ROSTER + strike shape; foes from the encounter) and delegates to
  deckbound's resolver, so outcomes match by construction. Parity test pins it (Marksman@Cinderwatch
  seeds 1/7 → Win, Executioner → Loss vs the old `resolve_encounter`). Combat logic is moving from
  `cardtable-combat` to the emitter. Next: **P2.3.1** — the interactive **arena presentation**: model
  `World` state (party/positions/day/active fight) + `Action` (equip/march/fight + per-blow prompts),
  `view()` renders the arena as zones, `apply()` steps it (following the `manual-combat-design` notes);
  bless fresh arena goldens. Then P2.4 route `boardgame`, delete the bypass, retire the old combat goldens.
- **P2.2b — DONE** (`fd03d47`). Added the Rules encyclopedia (6 phases + nested Engage). With every
  top-level zone authored, strengthened the test to **full-world equality**: the emitter's entire
  `view()` → `from_table_view` → `behavior()` equals `sample_table.behavior.txt`. **The complete static
  world is reproduced through the seam** — the reunification's core thesis, proven. Remaining: **P2.3**
  interactive fight (model combat as `World` state + actions; `view()` renders the arena, `apply()`
  steps it, delegating to deckbound's resolver — reproduce the combat behavioral goldens) and **P2.4**
  point `boardgame` at the emitter + delete the bypass. NOTE (fidelity, non-behavioral): the emitter
  doesn't yet set `Free` arrangement on the free-drag banks (behavioral tier ignores arrangement); do a
  byte/visual fidelity pass at P2.4.
- **P2.2 — DONE** (`38bdd0a`). Emitter now authors the nested zones, reproduced verbatim: Locations
  (`Grid{columns:3}` of 9 drill-in places, each with its Location card + encounter [flavor + virtual
  `Foes:` list], Inn authored inline inside Ashfen), Progress (empty day clock), Events (Day Passed ×12).
  Resolved a seam point: `ZoneView` splits cards/zones, so `from_table_view` emits cards before sub-piles
  and can't reproduce arbitrary interleave — the behavioral projection now canonicalizes it (cards first,
  then sub-zones; each order preserved), consistent with the tier already abstracting arrangement/geometry.
  Re-blessed behavioral goldens (pure reordering; byte tier + combat line counts unchanged). 9 of 10
  top-level zones now reproduce verbatim; only **Rules** (nested Engage + phase text — pure content) and
  the interactive fight remain. Next: **P2.2b** Rules → **P2.3** interactive fight as zones →
  **P2.4** point `boardgame` at the emitter + delete the bypass (full-world equality gate).
- **P2.1 — DONE** (`670cc7d`). Scaffolded the view emitter: new crate `deckbound-cardtable` (provisional
  name) holding the card-table world as a `contract::Game` — compact `World` state (not a Tableau),
  `view()` emits a nested `TableView`. Reproduced the six flat banks (Heroes/Kit/Abilities/Stats/Numbers/
  Bestiary) from `catalog` + the hero roster, formatted to the golden's spec. Proof: a characterization
  test drives `view()` through the seam and asserts every emitted zone appears **verbatim** in
  `sample_table.behavior.txt` — passed all six on the first run (incl. derived Kit/Bestiary detail). The
  pattern is established; remaining zones just add a `*_zone()` fn each, guarded by the same test. Next:
  **P2.2** — nested Locations grid + encounters + the Inn (inline equip), Rules (nested Engage), Progress,
  Events. Then P2.3 interactive fight, P2.4 route `boardgame`.
- **P2.0 — DONE** (`5c84ae8`). Smell fixed: added a `ZoneView` builder
  (`new`/`with_layout`/`with_owner`/`with_zones`/`with_arrangement`) and migrated all 8 deckbound sites +
  the binding tests to it — additive seam growth no longer touches call sites. Then carried a card-table
  `Arrangement` (List/Grid/Free/Rows, distinct from the CCG `Layout`) through the seam so the reunified
  product keeps its Locations grid / Inn rows / day calendar; `from_table_view` maps it via `set_layout`.
  New binding test; both golden tiers unchanged; deckbound 109 + model 59 green; clippy clean. Next:
  **P2.1** — scaffold the emitter `Game` (new crate; State/Action/view/apply) and reproduce the flat banks,
  asserting `view()`→`from_table_view`→`behavior()` matches those behavioral-golden slices.
- **P2 — OPENED (design in §14).** Emitter-home decided (new `contract::Game`, compact non-Tableau
  state, reuse deckbound combat); full visual fidelity chosen (carry arrangement). Sub-roadmap P2.0–P2.4
  set. Next executable unit: **P2.0** — carry a card-table arrangement through the seam (additive),
  ideally introducing a `ZoneView` builder to stop per-field literal churn.
- **P1.2 sub-question — RESOLVED; behavioral tier finalized.** Reading `sample_table.behavior.txt`
  showed `layout`/arrangement diverges (presentation) and `projection`/`reflects` are model mechanisms
  the emitter reimplements — all three dropped from the behavioral projection (byte tier still pins them
  for P3+). Refined `*.behavior.txt` re-blessed; now a construction-stable spec for the emitter. Finding:
  **P1.2 ≡ P2** — the view emitter *is* the routing work. Next unit: author the world as `Game::view()`
  guarded by the refined behavioral golden, then point `boardgame` at it.
- **P1.1 — DONE** (`9abe9b4` nesting, `1681efe` richness). The seam is card-table-native, additively:
  `ZoneView.zones` (nested sub-zones) + `from_table_view` recursion; `CardFace::Up.panel` +
  `CardView.quantity` + builders, and `from_table_view` now carries body→detail / panel / quantity
  (type_line already carried). `tabletop`'s face match ends in `..` (robust to future growth). Every
  existing renderer/game compiles unchanged; two new binding tests; both golden tiers unchanged;
  deckbound 109 tests green. Next: **P1.2** — have deckbound author a nested `TableView` that
  `from_table_view` turns into a `Tableau` matching the *behavioral* golden of `sample_table`. This is
  the big lift (the view emitter ≈ the P2 deliverable): reproduce the world through the seam and assert
  behavioral equivalence. Open sub-question from §13 (is `layout`/arrangement behavior or presentation?)
  gets resolved empirically here.
- **P1.0 — DONE.** Behavioral golden tier added (`golden/*.behavior.txt`): a rendered projection
  (recursive zone tree + card face/type/qty/detail/panel + actionable, no geometry), deterministic by
  construction, clippy-clean, byte tier unchanged. Six behavioral goldens parallel the six byte
  goldens; every scenario now asserts both. This is the acceptance criterion for P1.1–P2. Next: **P1.1**
  (grow `contract::{TableView,ZoneView,CardView}` additively + inflate in `from_table_view`).
- **P1 — decision + design done.** §9 Q1 RESOLVED → Option A (grow `TableView` card-table-native).
  Harness two-tier refinement adopted (§12). P1 sub-roadmap set (§13).
- **P0 — DONE, committed `ed0fe25`.** Witness crate `crates/characterization` added to the workspace.
  Six golden-master tests pin all four §2 surfaces and are deterministic across processes + clippy-
  clean: `sample_table`, `interaction_transcript`, `auto_{marksman_seed1,marksman_seed7,
  executioner_seed1}_cinderwatch`, `manual_marksman_cinderwatch_seed7`. Canonicalization sorts every
  map by key (see §10). Bless with `BLESS=1 cargo test -p characterization`. Not yet committed
  (awaiting go-ahead on the per-checkpoint commit cadence).
</content>
