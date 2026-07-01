# Boardgame

A **card-table application** in Rust with a [Bevy](https://bevyengine.org/) front
end, deployed to the web. The interface is a physical card-table metaphor —
everything is a card, every zone a deck; you navigate by single-click and drag.
It is built on a small framework that keeps game *rules* (pure, testable, no
Bevy) separate from *presentation*, so the same renderer can drive any game. The
full combat game **Deckbound** is kept as a reference sample. (No networking —
local play only, for now.)

## Workspace layout

**The product — the card-table app (deployed to the web):**

| Crate                    | Kind | What it is                                                                                                                                 |
| ------------------------ | ---- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `crates/boardgame`       | bin  | **The deployed binary.** Drives the card-table renderer with a starting `Tableau`. No game wired in yet — the seed the UI grows from.      |
| `crates/cardtable`       | lib  | The card-table Bevy renderer: every zone a deck, click-to-focus, drag-to-arrange. A shell over `cardtable-model`, fed by a `TableView`.    |
| `crates/cardtable-model` | lib  | The pure card-table interaction model — decks, cards, selection, reorder, move-between-decks, focus/zoom. No Bevy, so behaviors unit-test. |

**The framework — rules↔presentation seam, shared by every game:**

| Crate             | Kind | What it is                                                                                                             |
| ----------------- | ---- | ---------------------------------------------------------------------------------------------------------------------- |
| `crates/contract` | lib  | The pure interface both sides agree on: the `Game` trait and the `TableView` snapshot of the table. No Bevy, no logic. |
| `crates/engine`   | lib  | The card-game toolkit an implementation uses internally: `Zone`, seeded `Rng`. Pure; no Bevy.                          |

**The reference sample — Deckbound in a renderer:**

| Crate                     | Kind | What it is                                                                                                                |
| ------------------------- | ---- | ------------------------------------------------------------------------------------------------------------------------- |
| `crates/deckbound`        | lib  | The game: *Deckbound*, a cooperative card-combat game with a world-map campaign. Pure logic, fully unit-tested.           |
| `crates/tabletop`         | lib  | The button-based Bevy renderer: draws any `contract::Game` and turns its legal actions into buttons.                      |
| `crates/deckbound-sample` | bin  | The sample launcher. Wires `Deckbound` into a renderer (default `tabletop`, or `cardtable` under `--features cardtable`). |
| `crates/combat-lab`       | lib  | Developer tooling for Deckbound balance analysis (combat resolver, roster detection). Not part of the shipped app.        |

Each new game is a new pure crate that implements `contract::Game`; the renderers
do not change. See
[docs/technical/architecture.md](docs/technical/architecture.md) for the design.

## Quick start

```sh
scripts/run.sh        # or scripts\run.ps1  — run the card-table app (the product)
scripts/sample.sh     # run the Deckbound reference sample
scripts/sandbox.sh    # prototype a UI feature in isolation (renderer core, no game)
scripts/test.sh       # run the test suite
scripts/verify.sh     # the pre-push gauntlet: fmt + clippy + tests + build
```

Or directly: `cargo run -p boardgame`. The [`scripts/`](scripts/) directory has
a script per common command (build, run, sample, sandbox, test, check, fmt, lint,
verify), each with a PowerShell and a bash version.

The app builds to WebAssembly with [Trunk](https://trunkrs.dev/) and deploys to
GitHub Pages; see [`.github/workflows/deploy.yml`](.github/workflows/deploy.yml).

## Documentation

All documentation lives in [`docs/`](docs/), grouped by game:

- **[Games](docs/games/)** — everything specific to one game, with `rules/` and
  `design/` side by side: [Deckbound](docs/games/deckbound/README.md).
- **[Technical](docs/technical/)** — framework docs shared across all games: the
  [architecture](docs/technical/architecture.md) and how to
  [add a game](docs/technical/adding-a-game.md).
