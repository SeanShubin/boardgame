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

## #2 — Dual ownership of position (`Movable` ⊗ flex) — **PENDING**

Formation tiles wear the table's `Movable`, which means "the table's position system
(`animate_nodes`) owns my `left/top`, pulled toward my model `card.pos()`". But the tiles
are **flex**-laid-out, so two systems fought over their position, and the stale map
`card.pos()` leaked through (tiles flew to old grid coords). Patched by special-casing the
arena inside `animate_nodes` (snap to flex base 0,0) — a special-case, not a boundary.
- Fix direction: a modal needs its own "draggable-in-modal" marker the table animation
  ignores by construction, rather than overloading `Movable`.
- Sites: `cardtable::animate_nodes`, `spawn_formation_tile` (adds `Movable`),
  `on_node_drag_end` (reads `Movable`).

## #3 — `on_node_drag_end` is a ~150-line god-function — **PENDING**

Five resolution branches (pile-reposition / map-march / projection-equip / formation /
home-reorder) with early returns and order-dependent guards in one function. New drop
contexts get bolted on as "another branch with another `if focus == X`" — exactly how the
stale formation focus-check rode along. Low cohesion, high blast radius.
- Fix direction: split into named per-context resolvers, or have the game/context declare
  its drop strategy so the renderer isn't a switchboard of game-specific branches.

## #4 — String-typed cross-boundary coupling — **PENDING (overlaps P3c)**

The renderer reaches past `cardtable-model` to read game conventions as bare strings —
`"Arena"`, `"Pool"`, rank labels, `"unit"`/`"foe"`/`"phase"`/`"contact"`. No shared
constants, no type-checking; a rename on either side breaks the other silently. Fold into
P3c (purge game words from generic crates) / the seam.

## #5 — Observability was reactive, not designed-in — **PARTIALLY DONE**

Debugging was slow because the failing paths logged nothing (snap-back wrote no trace;
formation tiles weren't in the pickup log — no `CardRef`). We added, per bug: z-order in
the layout log, `Movable`-tile pickup/click logging, and a `drag-end … (no pile change)`
trace. Remaining: treat state-transition logging for high-implicit-state subsystems
(focus, drag offsets, z-order, pile membership) as a default, not an afterthought.

## #6 — Card-position authority (`CardScreenRects`) — started, not finished

`CardScreenRects` (commit 3d4c3c6) is now the single source for "where is card X on
screen" in *logical* px, rebuilt each frame by `track_card_rects` (the one place the
physical->logical / `inverse_scale_factor` conversion lives). The targeting arrows read it;
this fixed a HiDPI offset bug that came from re-deriving the conversion ad hoc. Root cause
is the same as #2: position was *computed*, never *owned*.

Remaining (execute when we next touch UI positioning):
- **Absorb the other re-derivers.** `logging::log_layout`, `scroll_hovered_panel`, and any
  drop hit-testing still do their own `translation * inverse_scale_factor`. Route them
  through the authority so the conversion exists exactly once.
- **Track piles too**, not just cards (`CardRef` / `ArenaUnitCard`). `log_layout` needs
  `PileDropZone` rects; generalize to `ScreenRects { cards, piles }` (or key by a node id).
- **Answer "where is a *collapsed* card?"** A card folded into a deck has no tile of its
  own; its honest physical position is its deck's rect. The authority should fall back to
  the containing deck so callers can point at any card, on screen or not (the physical-
  metaphor promise, doc SS0 / SS0.6).
- Converges with #2: once position is owned, `animate_nodes`' arena special-case and the
  `Movable`-vs-flex fight can be reconsidered against a single position model.
