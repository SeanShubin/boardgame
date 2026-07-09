# Cardtable ↔ Deckbound Reunification & Reorg — Automated-Pass Plan

**Status:** P0–P2 built (product runs behind the seam). **RE-AIMED 2026-07-09 — see §0:** physical
cards are the source of truth; resuming under the three-layer model (physical / UI / render).
**Owner:** this instance (the sole worker — no separate merge instance; folds into `docs/` when settled).
**Nature:** a long, mostly-unattended, **behavior-preserving** refactor. Same behavior in →
same behavior out. That invariant is what lets the pass run with few questions.

Working crate names below are **placeholders** — the boundary matters, not the names. Any
rename is cosmetic and behavior-preserving, so it is decided by policy (§8), not by asking.

---

## 0. RE-AIMING — cards are the source of truth (2026-07-09)

A design conversation reframed the whole pass. **This section supersedes the parts of §9/§14/§15 it
names in §0.4.** The reunification (P0–P2) proved deckbound *can* run behind a seam — but it built that
seam the wrong way round (abstract state deriving the cards). This is the correction we resume under.

### 0.1 Three layers, one truth

1. **Physical model** — a one-dimensional ordering of **conserved** cards, with label/divider cards
   marking zones. This is the **entire** state: the save-game, the undo history, the thing you serialize
   or send over a wire. *If it cannot be expressed as conserved cards, it is not state — it is
   scaffolding.* The game rules operate **directly** on this.
2. **UI model** — everything about *regarding* the cards: current zone, navigation stack, focus / what
   takes the felt, **viewing surfaces** (a projection "show Heroes and Kit together" is the purest case),
   arrangement (grid/rows/free), selection, and a **staging buffer** of pending intentions. The physical
   model knows *nothing* about attention; the UI model owns all of it. One physical truth can back many
   UI models (per player, per session) — so the physical model serializes; "where you were looking" is a
   separate, disposable session concern.

   **"Everything is a card" is a rendering vocabulary, not a claim that everything is physical.**
   Card-shaped things partition across the layers: *physical cards* (conserved) = game / rank / phase /
   label cards; *UI-model cards* (per-observer, not conserved) = Back/navigation, selection, and the
   staging buffer's **pending-intention previews**; *render-only cards* = log entries and the action
   affordances standing in for legal intentions. This is why UI-only "cards" (a Back card, a pending-hit
   preview) exist without violating conservation — same look, different layer.
3. **Rendering / IO** — pixels, input, animation, over the UI model.

The two seams: the physical model answers *"what cards exist and what moves are legal"*; the UI model
answers *"what am I looking at, and how."* Neither ever answers the other's question.

### 0.2 The rules seam (replaces the abstract-`World` emitter of §14)

The game is a predicate + reducer over the physical card model:

- `legal_intentions(all_cards) -> [intention]`
- `apply(all_cards, intentions[]) -> all_cards`  — **batch, order-free**, conservation-preserving

`apply` takes the *whole* intention set at once, so the order the player expresses things cannot matter —
there is no partial state between them. That order-independence is a **designed invariant** (it is why
hits queue and land together at phase end). Nothing abstract survives a call: the resolver may build
abstract `Actor`s as **throwaway scratch inside** `apply`, but they are never stored — the cards are the
only truth. (This dissolves the old (a) rewrite-vs-(b) wrap fork: we take the cards→cards *shape* and
keep the proven resolver as internal scratch.)

### 0.3 Combat vocabulary — ranks vs intentions

- **Ranks** — Vanguard / Outrider / Rearguard. **Physical cards** (rename from the V/O/R "intentions").
  They persist in `all_cards` and, **paired with the current phase card**, gate which intentions are
  legal: legality is a `(rank × phase)` lookup, both operands physical cards.
- **Intentions** — the **transient** set the UI assembles and hands to `apply`. Never cards; they exist
  only to be applied, then they *are* the new card state.
- **Transient events** — the raw UI editing (add a hit, retarget, drop, queue another) that churns a
  staging buffer into the final intention set. `apply` never sees events, only the set — the source of
  the order-independence. (Events are how you edit; intentions are what you submit.)
- **Phase is physical** — a rotating **phase deck** (front card rotates to the back), or a one-card
  current-phase deck plus a not-in-phase deck. The phase card is a **first-class input to legality**, not
  a counter; rotating it unlocks a different slice of what the same ranks may do.
- **Advancing the phase is an intention** (RESOLVED). When the user signals *"done with the phase,"* the
  UI appends a `rotate-phase` intention to the staging set and commits the whole batch to `apply` once —
  so rotation is neither a hardcoded tail of `apply` nor its own transition; it goes through the same one
  door as every state change. Consequences: **commit ≡ end-phase** (never apply mid-phase), and an empty
  phase still advances via a one-element `[rotate-phase]` batch. Because `apply` computes from the
  *pre-batch* state, hits resolve under the phase being left while the rotation is just another output
  card-move — the simultaneity is safe. *Minor rules note:* whether "end phase" is always available or a
  phase can gate its own exit is up to `legal_intentions` (it chooses whether to offer `rotate-phase`).
- **Small-phase invariant** — phases are deliberately sized so a human can hold every legal intention in
  their head at once; that is what makes each order-free batch humane and bounds `intentions[]` by
  construction.

### 0.4 What this supersedes

- **§14 (emitter `State` = compact non-Tableau world state).** *Inverted.* The abstract `World` made the
  cards a per-frame derivation of abstract truth; the physical card model must **be** the truth and the
  game must operate on it. `deckbound-cardtable`'s `World` is the thing to replace.
- **§9 Q1 Option A — *for the product only*.** Growing `TableView` card-table-native was right for the
  **sample** (a button renderer genuinely needs a serialized snapshot). For the **product** it planted a
  lossy snapshot boundary where the 1↔2 seam belongs and collapses physical+UI into one regenerated blob.
  **The product retires the `TableView` snapshot; `TableView` survives only as the sample's seam.**
- **The `focus` hint + `Tableau` holding `focus`/`arrangement`.** These are UI-model (attention) concerns.
  A game emitting `focus: Some(0)` is a layer violation; *"the arena takes over the felt"* must become a
  **UI-model navigation policy** reacting to the physical model, never a rules push. `focus`/`arrangement`
  move out of the physical `Tableau` into the UI-model layer.
- **§15 pairing.** Still valid as a gesture, now reframed: a pairing (and its target) is one kind of
  **transient event** feeding an intention set — not a stored card.

### 0.5 Revised roadmap

The generic-vs-game split (P3–P7) still stands, but the target it splits *toward* is now the three-layer
model, and one new structural phase precedes the cleanup:

- **P3a — Split `cardtable-model` into a physical-model layer and a UI-model layer.** Physical: conserved
  1-D card ordering + labels + move/conserve ops, knowing nothing of attention. UI-model:
  navigation/focus/viewing-surfaces/arrangement/staging, persistent across transitions. Move
  `focus`/`arrangement` out of the physical `Tableau`.
- **P3b — Put the game on the physical model.** Replace `deckbound-cardtable`'s abstract `World` with
  `legal_intentions`/`apply` over the physical card model; equip becomes conservation-clean deck assembly
  again; combat becomes `(rank × phase)`-gated order-free batches with a physical phase deck; rename
  V/O/R → **ranks**. Retire the product's `TableView` path (keep it for the sample).
- **P3c — Purge game words from the generic crates** (the original P3): the leftover arena/equip/march in
  the renderer, `catalog`/`fixtures`/day-queries in the model, absorb `cardtable-combat`.
- **P4–P7** — as before (extract `deckbound-balance`; evict Grand Archive + demote `combat-lab`; rename;
  ECS quality review), now against honest three-layer boundaries.

Behavior preservation (§2) still governs: the golden masters guard outcome / log / rendered-projection
across the re-aiming.

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
- **P5 — Demote tooling + evict Grand Archive.** `combat-lab` → `tools/` (stays in-repo). **`gatcg`
  content moves OUT to `../grand-archive`** (a sibling repo): the actual TCG rules, the card-database
  downloaders, the image/rules mirrors. **Keep in-repo only the comparative-analysis notes** — future
  design inspiration + game-theoretic comparison of deckbound vs Grand Archive vs Magic: the Gathering.
  So this repo retains only the *thinking about* Grand Archive, not the Grand Archive *content/tooling*.
  (User directive; the external move is destructive-for-this-repo, so confirm the `../grand-archive`
  landing before deleting here.)
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

## 15. Interaction model (decided)

The `contract` seam expressed only click-actions, but the product's equip is a **drag** (hero onto kit).
Decision (user): **grow the seam for drag-drop**, but state it **neutrally** as a *pairing* so the renderer
performs it as a drag-drop **or** a click-source-then-target, per an **interaction-mode toggle** — one or
the other, never both at once (user prefers toggling over supporting both, which reads as clunky). So the
mode toggle is a **renderer-side** switch with zero game/seam change. Drag-drop is the default mode; the
click-only mode is a later `cardtable` toggle. Implemented in `contract` as `Pairing { onto, action }` +
`CardView.pair_key`/`pairings` (P2.3.1a). Drag-to-*arrange* piles stays a pure renderer service (not a
game interaction). Note: only relevant to *game-meaningful* drags (equip); the arena's per-blow prompts are
plain click-actions.

## 16. P3a design — the physical/UI split (field-by-field) + naming

`model.rs` (3057 lines) fuses **three** concerns. P3a separates the first two; the third is P3c/P3b:

1. **Physical model** — the conserved card tree: card content, pile labels (dividers), nesting, order,
   ids, and the move/conserve ops.
2. **UI model** — attention + presentation: focus, selection, collapsed, layout/arrangement, geometry
   (pos/size/footprint), zoom (card `Size`), projection (viewing surface), renderer-fed surface/pinned.
3. **Deckbound game-semantics** (to leave in P3c/P3b, not P3a): `equip_character`, `unequip_character`,
   `instantiate_encounter_foes`, `return_foes_to_bestiary`, `character_recipe`, `move_character`,
   `mark_moved`, `day_is_over`, `advance_day`, `current_day`, `reflects` (the character-deck marker),
   `recipe`. These are the "deckbound queries" §5 already flags — they move behind the seam.

### 16.1 Field classification

**`Tableau`** — `piles`, `cards`, `root`, `next_pile`, `next_card` → **physical**; `focus`, `selection` →
**UI**; `surface`, `pinned` (both `#[serde(skip)]`, renderer-fed) → **UI (transient)**.

**`Pile`** — `id`, `label`, `parent`, `children` → **physical**; `collapsed`, `pos`, `size`, `layout`,
`projection` → **UI**; `reflects` → **deckbound-semantic** (P3b: becomes a real assembled deck).

**`Card`** — `id`, `face`, `detail`, `panel`, `kind`, `card_type`, `home`, `quantity` → **physical**;
`size` (zoom), `pos`, `footprint` → **UI**; `actionable`, `pair_key`, `pairings` → **seam/interaction**
(populated from the `contract` view; under re-aiming these are how a card affords an intention — keep with
the card as rendered content for now); `recipe` → **deckbound-semantic** (P3b).

### 16.2 Method classification (the renderer-ripple map)

- **Physical (no ripple to move):** `add_pile`, `add_card`, `root_id`, `pile`, `card`, `card_count`,
  `stack_count`, `reorder`, `move_card`, `remove_card`, `move_pile`, `remove_pile`, `set_card_detail`,
  `set_card_panel`, `set_card_kind`, `set_card_type`, `set_face`, `flip_down`, `flip_up`, `card_index`,
  `zone_card`, `content_cards`, `physical_card_count`, `name_runs`, `name_runs`, `set_card_pair_key`,
  `set_card_pairings`, `set_card_quantity`.
- **UI (these ripple to `cardtable`):** `focus`, `focus_id`, `zoom_out`, `selection`, `is_selected`,
  `select`, `deselect`, `clear_selection`, `set_layout`, `set_collapsed`, `set_pile_pos`, `set_pile_size`,
  `set_card_pos`, `set_card_footprint`, `place_pile`, `cycle_card_size`, `set_projection`,
  `projection_groups`, `row_groups`, `separate`, `arrange_row`, `structured_positions`,
  `movable_children`, `set_surface`, `set_pinned`, `surface`.
- **Deckbound-semantic (P3c/P3b):** the §16-intro list.

### 16.3 Naming (given the layer clarification — the user asked to revisit)

Targets recorded here; disruptive cross-crate renames still land in **P6**, now informed by the layers.

- **The two layers, as modules first:** `cardtable_model::physical` (the conserved card board) and
  `cardtable_model::ui` (attention/presentation). Promote to separate crates only if the boundary proves
  clean. Provisional crate-level names for P6: **`card-board`** (physical) + **`card-ui-model`** (UI).
- **Types:** keep `Tableau` as the *transitional composed* type (a `physical::Board` + a `ui::UiModel`)
  so the renderer's `Tableau` calls keep compiling via delegation; the physical core type is **`Board`**.
  `Pile`→ stays `Pile` (physical: label + children); `Card` stays `Card`. `Layout`/`Arrangement`/`Size`
  are UI-model presentation names — fine. `focus`/`selection` are UI names — fine.
- **Ranks vs intentions** (deckbound-side, P3b) already recorded in §0.3 — rename V/O/R `intention`→`rank`.
- `from_table_view`/`binding` — becomes sample-only when the product retires `TableView` (§0.4); name is
  still accurate, leave it.

### 16.4 Safe step sequence (each a compiling, golden-green, committed checkpoint)

The behavioral goldens ignore focus/selection/layout/geometry (they render the whole tree, faces + actionable
+ pairings), so the internal restructure below cannot change them. Nothing here changes the **public API**, so
`cardtable` is untouched until the deliberate high-ripple step, which is teed up for careful (attended) work.

- **P3a.1 — extract pure geometry** (`box_center`/`clamp_box`/`overlap_area`/`place_clear_of`/
  `separate_boxes` + `Pos`) into `model/geometry.rs`. Free functions, no struct privacy — trivially safe.
- **P3a.2 — introduce `ui::UiModel` sub-struct inside `Tableau`**, moving the attention fields into it
  (`selection`, then `focus`, then `surface`/`pinned`), with `Tableau`'s public methods delegating. Serde
  layout changes, which is harmless (nothing persists a `Tableau` since P2.4 dropped save). No renderer change.
- **P3a.3 — per-pile/card UI state → id-keyed side-tables (HIGH-RIPPLE; do attended).** REVISED: the
  per-`Pile`/`Card` UI fields (`collapsed`, `pos`, `size`, `layout`, `projection`; card `size`(zoom)/`pos`/
  `footprint`) must **not** be grouped into an in-struct `CardUi`/`PileUi`. The architecture forbids it:
  *"one physical truth backs many UI models"* means per-card/pile UI state cannot be embedded in the single
  physical `Card`/`Pile` (that permits only one observer and dies on rebuild). So it moves into id-keyed
  side-tables in the UI layer — `UiModel { pile_ui: HashMap<PileId, PileUi>, card_ui: HashMap<CardId, CardUi>,
  focus, selection, surface, pinned }`. Consequence: `pile.pos()` / `card.size()` stop being `Pile`/`Card`
  methods (the physical structs no longer hold the state) and become `Tableau`/UI-layer lookups — which
  **ripples into the 3400-line renderer**. There is therefore *no* low-ripple in-struct intermediate that is
  architecturally correct; the honest per-card/pile step is inherently high-ripple. **P3a.2 was the natural
  end of safe autonomous work.**
- **P3a.4 — promote the boundary into the API (HIGH-RIPPLE; do attended, folds P3a.3 in).** Point `cardtable`
  at the physical layer + `UiModel` directly and dissolve the `Tableau` delegator. Same edit as P3a.3's
  side-table ripple, so do them together: this is where the UI model gains its *persistence across rebuilds*
  (the world-focus-reset fix) and so overlaps P3b. Not an unattended blast.

## 10. Observations (non-blocking; not behavior changes to make in this pass)

- **The product's RON save is non-canonical.** `Tableau` stores `piles`/`cards` in `HashMap`s, so
  serde emits them in per-process-random order — two saves of the same table differ textually.
  Harmless today (autosave dedup is per-process; the fingerprint is per-process), but it means the
  witness must canonicalize (it does). A future *deliberate* change could swap to `BTreeMap`/
  `IndexMap` for stable saves — out of scope here (it would change the on-disk format).

## 11. Progress log (append-only)

- **P3a.2 — DONE** (`873f3dd`). Separated the **attention state** out of the physical `Tableau`: `focus`,
  `selection`, and the renderer-fed transient `surface`/`pinned` moved into a distinct `ui::UiModel`
  sub-struct that `Tableau` holds; the public methods delegate through `self.ui`. The physical tree
  (`piles`/`cards`/`root`/ids) now knows nothing of attention — the first *semantic* cut of the split.
  21 field-access sites (all `self.X`, zero method-call collisions) redirected via `replace_all`; public
  API unchanged so `cardtable` + `boardgame` build unchanged; 60 model tests + 6 behavioral goldens green.
- **P3a.1 — DONE** (`d453db6`). Extracted the pure box-geometry helpers to `model/geometry.rs` (free
  functions, no struct privacy, no API change). Trivially-safe first cut.
- **P3a design + re-aiming captured — DONE** (`065b6d3`). Plan §0 (RE-AIMING: cards-are-truth three-layer
  model) + §16 (field/method classification + naming + safe step sequence).
- **P3a REMAINING (resume here, attended) — see §16.4 (revised):** the per-`Pile`/`Card` UI state
  (`collapsed`, `pos`, `size`, `layout`, `projection`; card `size`(zoom)/`pos`/`footprint`) moves to
  **id-keyed side-tables** in `UiModel` (`pile_ui`/`card_ui` maps), *not* in-struct `CardUi`/`PileUi` —
  because "many UI models per one physical truth" forbids embedding UI state in the single physical struct.
  That makes `pile.pos()`/`card.size()` become `Tableau`/UI-layer lookups instead of `Pile`/`Card` methods,
  which **ripples into the 3400-line renderer** — so P3a.3 (side-tables) and P3a.4 (renderer talks to
  physical + `UiModel` directly, gaining focus-persistence-across-rebuilds) are one attended step, and
  overlap P3b. `reflects` (deckbound-semantic) stays for P3b/P3c. **P3a.2 was the natural end of safe
  autonomous work.**
  - **Naming (user asked 2026-07-09):** targets recorded in §16.3 (`physical::Board` + `ui::UiModel`
    modules now; provisional crates `card-board` + `card-ui-model` at P6; V/O/R→**ranks** at P3b). The
    `ui` module now exists; a `physical` module is the natural next container for the P3a.3 fields.

- **P2.3.1c (renderer) — DONE** (`fd9632e`) + **P2.4 (routing) — DONE** (`8021079`). **P2 COMPLETE.** The
  renderer performs game-declared **pairing** drops (`pairing_action` reads `card.pairings()`/`pair_key()`
  from the view-authored model; both drop paths report the action to `ActionRequests`; additive, current
  product unaffected). Then `boardgame` was routed through `cardtable`'s `GamePlugin` bound to
  `deckbound_cardtable::CardTableWorld`: the Table is built from the game's `view` each frame, and clicks +
  pairing drops flow into `Game::apply`. The hand-wired bypass is **deleted** (sample_table, FactoryBase,
  resolve_combat/manual, the `cardtable-combat` dep). Verified: `cargo run -p boardgame` **boots cleanly**
  (view→Table→renderer wiring sound; window opens on the GPU, no panic). Dropped persistence (state is now
  the source of truth; action-stream save is a follow-on; `persistence.rs` removed).
  **The deployed product runs deckbound's entire card-table world — banks, map, interactive per-blow arena —
  through the `contract::Game` seam. No bypass.** 13 goldens green; gate crates clippy-clean.
  Remaining is the **reorg cleanup** (P3–P7): purge the leftover game words from the generic crates
  (`cardtable`'s hardcoded equip/march, `cardtable-model`'s catalog/fixtures; absorb `cardtable-combat`),
  extract `deckbound-balance`, evict Grand Archive + demote `combat-lab`, rename, then the quality review.

- **P2.3.1 arena per-blow choices — DONE** (`9585986`). The manual-combat design is realized: the player
  makes every combat decision through the seam. The arena renders the current hero decision as answerable
  `choice` cards — Strike-enemy / Hold (Target), Evade / Endure, Strike-back / Decline — each a clickable
  action; `apply(answer)` sets the pending decision and drives on (foe greedy), folding on finish. Test
  plays a whole fight via explicit choices. **The ENTIRE game now runs through `contract::Game`** — world +
  equip/march/fight + interactive combat, all golden-tested (13 goldens). **P2.3.1 (emitter) is complete.**
  Remaining for P2: **c** the `cardtable` renderer (perform the pairing gesture drag-drop + the click/drag
  mode toggle — Bevy, verified by driving the app, not goldens), then **P2.4** point `boardgame` at the
  emitter + delete the `sample_table`/`cardtable-combat` bypass (behavioral goldens the gate).
- **P2.3.1 fight + arena — DONE** (`545d230` auto-resolve Fight, `6affd46` interactive arena foundation).
  Full play loop through the seam: recruit → march → **fight**. `Action::Fight` auto-resolves (outcome-
  parity); `Action::Arena` opens the interactive arena — an active battle `State` in `World`, the view
  TAKES OVER the felt as an `[Arena]` (combatants w/ Health + log), `Action::StepArena` drives deckbound's
  resumable resolver and folds back (Victory/Defeat log + cleared encounter). Fight- + arena-flow tests.
  12 goldens green. **Remaining in P2.3.1: per-blow player choices** (Target/Evade/StrikeBack as answerable
  prompt cards, replacing the arena foundation's greedy StepArena) — the manual-combat design's core. Then
  **c** the `cardtable` renderer (drag-drop gesture + mode toggle, driven via run/verify), then **P2.4**.
- **P2.3.1d (march) — DONE** (`f00f70e`). March is a pairing too: Location cards are targets
  (`pair_key=100+idx`), a stationed character pairs onto another location's card to march. `Character`
  gains `location`; `Action::March` re-stations; `place_zone` renders characters at their place with march
  pairings; the inn is now purely the recruit affordance. Emitter_world re-blessed (Location cards gain
  pair_key). March-flow test via `legal_actions`. 10 goldens green. **Remaining interactive piece: the
  FIGHT/arena** — `Action::Fight`, an active-fight `World` state, arena `view()`, and per-blow stepping via
  deckbound's resumable resolver (`resolve_fight` already gives outcome-parity), with fresh arena goldens.
  Plus **c** the renderer drag-drop + mode toggle (Bevy). Then P2.4.
- **P2.3.1d (equip) — DONE** (`2e5fded`). First interaction through the reunified seam: `World` gains a
  recruited party; the Inn is now the functional recruit view (each un-recruited hero card **pairs onto**
  a kit; `Action::Equip` recruits). Behavioral projection renders pairings. The emitter's Inn intentionally
  diverges from `sample_table`'s projection stub, so the gate moved to a fresh `emitter_world` golden;
  `sample_table.behavior.txt` stays as the old-path witness. Tests: `emitter_world_view` + equip-flow. 9
  goldens green. Remaining P2.3.1: **march** (character → location), **fight** (start + arena `view()` +
  per-blow stepping via `resolve_fight`/deckbound's resumable resolver, fresh arena goldens), and **c**
  the renderer drag-drop + mode toggle in `cardtable` (Bevy, drive with run/verify). Then P2.4.
- **P2.3.1b — DONE** (`df774dd`). Pairing carries seam → model: `Card` gains `pair_key`/`pairings`
  (native tuples, no contract dep) + accessors/setters; `from_table_view` carries them; `skip_serializing_if`
  keeps the byte format + golden unchanged. So the renderer (reads the Tableau) can perform pairings.
  Remaining P2.3.1: **c** renderer drag-drop handling + mode toggle in `cardtable` (Bevy; drive with
  run/verify, no golden); **d** emitter `World` state + equip/march/fight actions (emit pairings for equip)
  + arena `view()`; bless fresh arena goldens.
- **P2.3.1a — DONE** (`34d58d2`). Interaction model decided (§15): grow the seam for a **neutral pairing**
  (drag-drop or click-then-click per a renderer-side mode toggle). Added `contract::Pairing` +
  `CardView.pair_key`/`pairings` + builders; additive, unit-tested, goldens unchanged. Remaining P2.3.1:
  **b** carry pairing through `from_table_view` → model `Card`; **c** renderer drag-drop handling + the
  mode toggle in `cardtable`; **d** emitter `World` state + equip/march/fight actions + arena `view()`;
  bless fresh arena goldens. Then P2.4 routing.
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
