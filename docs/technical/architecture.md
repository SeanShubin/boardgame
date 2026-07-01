# Architecture

The framework separates a game's **rules** (pure, testable, no Bevy) from its
**presentation** (a generic Bevy renderer). Rules are plugged into a renderer;
the renderer never knows which game it is showing.

Two things are built on that seam:

- **The product** — the **card-table app** (`boardgame`), the deployed binary. It
  drives the `cardtable` renderer directly with a `Tableau`. No game is wired in
  yet; the UI grows one feature at a time.
- **The reference sample** — `deckbound-sample`, which wires the full *Deckbound*
  game into a renderer end to end. Kept for reference and compatibility.

## Workspace crates

| Crate                     | Kind | What it is                                                                                                                                                                                                                             |
| ------------------------- | ---- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/contract`         | lib  | **The contract** — the pure rules↔presentation interface: the [`Game`](#the-game-trait) trait and the `TableView` snapshot family. No logic, no Bevy. The one thing both sides must agree on.                                          |
| `crates/engine`           | lib  | The shared **card-game toolkit**: reusable building blocks (`Zone`, seeded `Rng`) an implementation uses internally. Pure; does **not** depend on `contract`.                                                                          |
| `crates/deckbound`        | lib  | The game: *Deckbound*. Pure logic, fully unit-tested. Implements `contract::Game`; uses the `engine` toolkit.                                                                                                                          |
| `crates/cardtable-model`  | lib  | The pure **card-table interaction model** — decks, cards, selection, reorder, move-between-decks, focus/zoom. No Bevy, no game, so behaviors unit-test in isolation. Touches `contract` only to ingest a `TableView`.                  |
| `crates/cardtable`        | lib  | **The product's renderer** — the Bevy renderer drawing the **card-table metaphor**: every zone a deck, collapse-the-unattended, click-to-focus / zoom-out, drag-to-arrange. A thin shell over `cardtable-model`, fed by a `TableView`. |
| `crates/tabletop`         | lib  | The button-based Bevy renderer used by the reference sample: draws any `contract::Game` and turns its legal actions into clickable buttons. Depends on Bevy and on a game's *shape* (not its rules).                                   |
| `crates/boardgame`        | bin  | **The deployed product** — the card-table app. Drives the `cardtable` renderer with a starting `Tableau`; no game wired in yet. Built to WebAssembly with Trunk.                                                                       |
| `crates/deckbound-sample` | bin  | The reference sample launcher. Wires `Deckbound` into a renderer (default `tabletop`, or `cardtable` under `--features cardtable`) and runs it.                                                                                        |

Each new game is a new pure crate that implements `contract::Game`; the renderers
do not change. The dependency arrows form clean composition roots: each **bin**
(`boardgame`, `deckbound-sample`) wires implementations together, while the
implementations (`deckbound`, `tabletop`, `cardtable`, `cardtable-model`) know
only `contract`, never each other. Two renderers against one `TableView` is the
seam paying off — a new presentation never touches the rules, and the card-table
product and the button-based sample share the same model of the table.

## The two seams

Two boundaries keep rules and presentation independent.

### The `Game` trait

A game implements [`contract::Game`](../../crates/contract/src/game.rs) over its own
`State` and `Action` types. It is the single source of truth for the rules, and
it is **pure**: given a state and an action it produces the next state, with all
randomness flowing from the seed passed to `new_game`. This keeps a game fully
reproducible and unit-testable, and lets the same implementation drive a
renderer, a bot, or a test harness.

Key methods: `new_game`, `current_player`, `legal_actions`, `action_label`,
`apply`, `outcome`, and `view`.

### The `TableView` snapshot

A game renders its private state into a
[`TableView`](../../crates/contract/src/view.rs): a plain description of the zones
on the table (each a list of face-up or face-down `CardView`s) plus a status
line. The presentation layer draws a `TableView` without knowing any game's
rules, and a game produces one without knowing how it will be drawn. This is the
seam that lets one renderer display every game.

## How a turn flows (the reference sample)

This is the game-driven path, exercised end to end by `deckbound-sample`:

```text
        contract::Game  (the interface)
          ^        \
          |         \  view() -> TableView
     deckbound       \
          |           v
        tabletop  (Bevy: draws TableView, sends legal_actions back as buttons)
          |
    deckbound-sample  (binary)
```

1. `tabletop` asks the game for a `TableView` and draws it.
2. It asks for `legal_actions` and renders one button per action, labelled with
   `action_label`.
3. A click calls `apply` with the chosen action, advancing the state.
4. The table redraws from the new state.

See [`crates/tabletop/src/lib.rs`](../../crates/tabletop/src/lib.rs) for the
plugin, resources, and systems.

## The product path (the card-table app)

The deployed `boardgame` bin does **not** wire a game in yet. It drives the
`cardtable` renderer directly with a hand-built `Tableau` (see
[`crates/cardtable-model/src/fixtures.rs`](../../crates/cardtable-model/src/fixtures.rs)),
reporting clicks on actionable controls back to the app. The same renderer can be
driven from a `contract::Game` through `cardtable`'s `game` feature — the path the
sample uses under `--features cardtable` — so when the card-table UI is ready to
host real rules, the seam is already in place.

See [`crates/cardtable/src/lib.rs`](../../crates/cardtable/src/lib.rs) for the
renderer core and the game adapter.

## Determinism

Game logic must be deterministic given a seed. All randomness comes from
`engine::Rng` (a seeded SplitMix64 generator), seeded once in `new_game`. Do not
introduce wall-clock time or unseeded randomness into a game's rules — it would
break reproducibility and the seed-based tests.

## Building blocks

In `contract` (the interface):

- [`PlayerId`](../../crates/contract/src/player.rs) — a seat index.
- [`view`](../../crates/contract/src/view.rs) — `TableView`, `ZoneView`,
  `CardView`, `CardFace`, `Layout`.

In `engine` (the toolkit an implementation uses):

- [`Zone<C>`](../../crates/engine/src/zone.rs) — an ordered pile of cards (deck,
  hand, discard, play area). The "top" is the end of the vector, so draw/place
  are O(1).
- [`Rng`](../../crates/engine/src/rng.rs) — seeded, dependency-free PRNG with a
  Fisher-Yates `shuffle`.
