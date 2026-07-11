# Architecture

The framework separates a game's **rules** (pure, testable, no Bevy) from its
**presentation** (a generic Bevy renderer). Two things are built on that split:

- **The product** — the **card-table app** (`boardgame`), the deployed binary. It
  runs the *Deckbound* card-table game on a **persistent physical board**, drawn by
  the generic `cardtable` renderer.
- **The reference sample** — `deckbound-sample`, which wires the full *Deckbound*
  game into a button renderer through the older `contract::Game` / `TableView`
  snapshot seam. Kept as a reference; it does not drive the product.

The product and the sample are two different renderers over the same rules — but
they reach the rules through **two different seams** (below), because a card table
and a button table want different things.

## Cards are the source of truth (three layers)

The product is built on one principle: **the physical cards are the entire game
state.** Everything partitions into three layers.

1. **Physical model** — a conserved set of cards (with label / divider cards
   marking zones), nested into piles. This *is* the state: the save-game, the undo
   history, the thing you serialize. If it cannot be expressed as conserved cards,
   it is not state — it is scaffolding. The rules operate **directly** on it. The
   type is `cardtable_model::Tableau` (the physical board).
2. **UI model** — everything about *regarding* the cards: focus / what takes the
   felt, selection, arrangement (grid / rows / free), and a staging buffer of
   pending intentions. Per-observer and disposable; the physical model knows
   nothing of it. Held in `cardtable_model::ui::UiModel`.
3. **Rendering / IO** — pixels, input, animation, over the UI model (`cardtable`).

The rules seam is a predicate + reducer over the physical model:

- `legal_intentions(board) -> [intention]`
- `apply(board, intentions[]) -> board` — **batch, order-free**, conservation-preserving

`apply` takes the whole intention set at once, so the order the player expresses
things cannot matter (this is what lets combat hits queue and land together at a
phase boundary). Nothing abstract survives a call: the resolver may build throwaway
scratch inside `apply`, but the cards are the only stored truth.

## The two seams

### `BoardGame` — the product's seam (cards-as-truth)

The product's rules↔renderer boundary is `cardtable_model::BoardGame`: a trait the
game implements over the physical board.

```rust
pub trait BoardGame {
    type Intention;
    fn opening(&self) -> Tableau;                                          // the starting board
    fn apply(&self, board: &mut Tableau, intentions: &[Self::Intention]);  // batch, order-free
    fn drop_intention(&self, board: &Tableau, dragged: CardId, onto: DropTarget) -> Option<Self::Intention>;
    fn affordances(&self, board: &Tableau, focus: PileId) -> Vec<(String, Self::Intention)>;
    // (tap_intention for in-arena taps; DropTarget = Card | Pile.)
}
```

`deckbound-cardtable::CardTableGame` implements it. The generic `cardtable` renderer
is generic over `G: BoardGame`: it turns a drag into `drop_intention` and a control
tap into an affordance intention, then calls `apply` on the persistent board — and
never mentions Deckbound. Because the board persists (it is not rebuilt per frame),
focus / drag position / navigation survive every action.

Non-combat intentions (`Equip` / `Unequip` / `March` / `AdvanceDay`) are
conservation-clean card moves. **Combat** is the *v2 arena*: a fight lives on the
board as rank piles (Vanguard / Outrider / Rearguard) plus a phase deck, and each
combat decision is a staged intention resolved as an order-free batch. See the
`combat` (headless brain), `battle` / `solver` (analysis tooling), and `arena`
(board ↔ combat) modules in `deckbound-cardtable`.

### `contract::Game` — the reference-sample seam (snapshot)

The older seam: a game implements `contract::Game` over its own `State` / `Action`
and renders a flat `TableView` snapshot; `tabletop` draws it and sends legal actions
back as buttons. `cardtable_model::from_table_view` can inflate a `TableView` into a
`Tableau` for the `cardtable`-under-`--features cardtable` sample path. This seam is
**sample-only** now — the product does not use it.

## Workspace crates

| Crate | Kind | What it is |
| --- | --- | --- |
| `crates/contract` | lib | The **sample seam**: the `Game` trait + the `TableView` snapshot family. No Bevy. Used by `deckbound-sample` / `tabletop`, not the product. |
| `crates/engine` | lib | Shared **card-game toolkit** (`Zone`, seeded `Rng`). Pure; no `contract` dep. |
| `crates/cardtable-model` | lib | The pure **physical board** (`Tableau`) + `ui::UiModel` + conservation primitives (move / split / merge / flip / focus / layout), and the **`BoardGame` seam trait**. Also holds `from_table_view` (the sample-seam binding). No Bevy, no game. |
| `crates/cardtable` | lib | **The product's renderer** — the Bevy card-table renderer, generic over `BoardGame`: every zone a deck, click-to-focus, drag-to-arrange, and the interactive arena. No game words. |
| `crates/deckbound` | lib | The **reference-sample game** (`contract::Game`) *and* the shared content used by the product: `catalog` (kits / creatures / encounters / rumors), `combat` (the `SCHEDULE`), `actor` (the V/O/R ranks). Pure, no Bevy. |
| `crates/deckbound-cardtable` | lib | **The product's game** — implements `BoardGame` (`CardTableGame`): equip / march / day as conservation-clean transitions, and the v2 combat arena. Owns `sample_table` (the opening board). Uses `deckbound::{catalog, combat, actor}`. |
| `crates/tabletop` | lib | The button renderer for the sample (draws any `contract::Game`). |
| `crates/boardgame` | bin | **The deployed product**: wires `CardTableGame` into the `cardtable` renderer + persistence. Built to WebAssembly with Trunk. |
| `crates/deckbound-sample` | bin | The reference-sample launcher: wires `deckbound` into `tabletop` (or `cardtable`) through `contract::Game`. |
| `tools/combat-lab`, `tools/gatcg` | — | Not-the-game tooling (a gear-system experiment; a Grand Archive analysis stub). Nothing depends on them. |

The **bins** are the composition roots. The product path is `boardgame → CardTableGame
(BoardGame) → cardtable`; the sample path is `deckbound-sample → deckbound
(contract::Game) → tabletop`.

## Determinism

Game logic is deterministic given a seed: all randomness flows from `engine::Rng`
(seeded SplitMix64). No wall-clock time, no unseeded randomness in the rules — that
is what makes the seed-based tests and the exact combat solver reproducible.

## Reorg still open (from the reunification)

The cards-as-truth reunification is complete and shipping. A cosmetic / structural
tail remains, tracked in `needs-merge/cardtable-deckbound-reunification-plan.md`:

- **P6 — honest renames**: `Tableau → Board`, `deckbound-cardtable → deckbound-board`,
  a `physical` module beside `ui`, and finishing `actor::Intention → Rank`.
- **P4 — extract `deckbound-balance`**: move the *legacy sample's* balance / solver
  tooling out of `deckbound` (the product's v2 balance tooling already lives in
  `deckbound-cardtable`).
- **A1 — arena rendering out of `cardtable`**: the generic renderer still holds the
  arena UI and combat-affordability logic; the ideal is "the arena is just another
  zone."
- **P3a.3 / .4 (dropped)**: moving per-card/pile UI state (`pos` / `size` / `layout`
  / `collapsed`) into id-keyed side-tables was deferred as high-ripple. Its
  motivation — "focus survives rebuilds" and "many UI models per one truth" — was
  resolved by the persistent board (single local observer), so it is **not planned**
  for this single-player product; revisit only if shared / multi-observer boards
  become a goal.
</content>
