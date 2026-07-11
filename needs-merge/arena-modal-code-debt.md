# Arena / formation code-quality debt (retrospective for P7)

The v2 combat formation UI was unusually bug-prone during playtest (heroes stranded
behind rows, controls vanishing, drops snapping back). Root cause: **a modal UI built
out of non-modal table primitives**, each of which carries an assumption the modal
arena violates, with no single concept keeping the arena-aware systems consistent.

## #1 — Duplicated "is the arena active?" invariant — **DONE (2026-07-10)**

Four systems re-derived "is a fight active", two of them wrongly via `arena == focus_id()`
instead of "the arena exists". Clicking a rank sub-pile drills `focus` into it, so the
focus-keyed checks went false while the arena was still up → the same bug appeared four
times (animate_nodes stranding tiles, affordances dropping the Commit/Cancel controls,
the drag-end resolver snapping heroes back).

Fixed: a single documented authority per crate, gating on existence not focus.
- renderer: `cardtable::active_arena` (was `board_arena`) — doc states the contract.
- game: `deckbound_cardtable::arena::find_arena` — doc states the contract.
Any new arena-aware system must route through these, never `focus_id()`.

## #2 — Dual ownership of position (`Movable` ⊗ flex) — **DONE** (ModalTile marker; animate_nodes excludes Without<ModalTile>)

Formation tiles wear the table's `Movable`, which means "the table's position system
(`animate_nodes`) owns my `left/top`, pulled toward my model `card.pos()`". But the tiles
are **flex**-laid-out, so two systems fought over their position, and the stale map
`card.pos()` leaked through (tiles flew to old grid coords). Patched by special-casing the
arena inside `animate_nodes` (snap to flex base 0,0) — a special-case, not a boundary.
- Fix direction: a modal needs its own "draggable-in-modal" marker the table animation
  ignores by construction, rather than overloading `Movable`.
- Sites: `cardtable::animate_nodes`, `spawn_formation_tile` (adds `Movable`),
  `on_node_drag_end` (reads `Movable`).

## #3 — `on_node_drag_end` is a ~150-line god-function — **DONE** (split into named per-context resolvers)

Five resolution branches (pile-reposition / map-march / projection-equip / formation /
home-reorder) with early returns and order-dependent guards in one function. New drop
contexts get bolted on as "another branch with another `if focus == X`" — exactly how the
stale formation focus-check rode along. Low cohesion, high blast radius.
- Fix direction: split into named per-context resolvers, or have the game/context declare
  its drop strategy so the renderer isn't a switchboard of game-specific branches.

## #4 — String-typed cross-boundary coupling — **MOSTLY RESOLVED by A1**

The renderer used to reach past `cardtable-model` to read combat conventions as bare
strings — `"Arena"`, `"Pool"`, rank labels, `"unit"`/`"foe"`/`"phase"`/`"contact"`. **A1
removed all of these**: the renderer draws a rules-blind `Scene` and never reads combat
state off the board. What remains is the non-combat map-drag coupling (`on_node_drag_end`
still reads `"Locations"` to detect the map zone) — small, and better addressed by the
"context declares its drop strategy" idea in #3 than by shared string constants.

## #5 — Observability was reactive, not designed-in — **PARTIALLY DONE**

Debugging was slow because the failing paths logged nothing (snap-back wrote no trace;
formation tiles weren't in the pickup log — no `CardRef`). We added, per bug: z-order in
the layout log, `Movable`-tile pickup/click logging, and a `drag-end … (no pile change)`
trace. Remaining: treat state-transition logging for high-implicit-state subsystems
(focus, drag offsets, z-order, pile membership) as a default, not an afterthought.

## #6 — Card-position authority (`CardScreenRects`) — **RESOLVED (scoped honestly)**

`CardScreenRects` is the single source for "where is card X on screen" in *logical* px,
rebuilt each frame by `track_card_rects`. The targeting arrows read it. On revisiting the
"remaining" list, most of it was speculative or unavoidably-live, so the honest close is:

- **Conversion exists exactly once — DONE.** The physical->logical box conversion now lives
  solely in `node_box` / `node_rect`; `track_card_rects`, the drop hit-tests
  (`projected_card_under_cursor`), and `logging::log_layout` all route through it. The pin /
  size syncs (`sync_pinned`, `sync_node_sizes`) deliberately keep their own conversions —
  `sync_pinned` maps into the *content* coordinate space, the size syncs need only the size —
  so they are not "re-derivers" to absorb.
- **Track piles / collapsed-card fallback — DEFERRED (YAGNI).** No consumer needs a pile rect
  or a collapsed card's position today (arrows point only at rendered tiles; `log_layout`
  iterates its own entities for name/z). Building `ScreenRects { cards, piles }` + a
  containing-deck fallback would add per-frame state and a board-walk with no caller. Add it
  when a feature actually needs to point at any card/pile on screen.
- **Converged with #2:** the `Movable`-vs-flex fight is resolved by the `ModalTile` marker
  (`animate_nodes` excludes it `Without<ModalTile>`), so there is no arena special-case left
  to reconsider.
