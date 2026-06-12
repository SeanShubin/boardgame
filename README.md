# Boardgame

A framework for building turn-based tabletop board games in Rust, with a
[Bevy](https://bevyengine.org/) front end. Game *rules* live in small, pure,
dependency-light crates that know nothing about rendering; a single generic
presentation layer draws and drives any of them. (No networking — local play
only, for now.)

## Workspace layout

| Crate                  | Kind | What it is |
| ---------------------- | ---- | ---------- |
| `crates/engine`        | lib  | The framework: the `Game` trait, card-game building blocks (`Zone`, seeded `Rng`), and `TableView` — a renderer-agnostic snapshot of the table. No Bevy dependency. |
| `crates/treasure-dive` | lib  | The first game: *Treasure Dive*, an original push-your-luck card game. Pure logic, fully unit-tested. |
| `crates/tabletop`      | lib  | A Bevy plugin that renders any `engine::Game` and turns its legal actions into buttons. |
| `crates/boardgame`     | bin  | The launcher. Wires one game into the renderer and runs it. |

Each new game is a new pure crate that implements `engine::Game`; the renderer
and launcher do not change. See
[docs/technical/architecture.md](docs/technical/architecture.md) for the design.

## Quick start

```sh
scripts/run.sh        # or scripts\run.ps1  — run the game
scripts/test.sh       # run the test suite
scripts/verify.sh     # the pre-push gauntlet: fmt + clippy + tests + build
```

Or directly: `cargo run -p boardgame`. The [`scripts/`](scripts/) directory has
a script per common command (build, run, test, check, fmt, lint, verify), each
with a PowerShell and a bash version.

## Documentation

All documentation lives in [`docs/`](docs/), grouped by game:

- **[Games](docs/games/)** — everything specific to one game, with `rules/` and
  `design/` side by side: [Treasure Dive](docs/games/treasure-dive/README.md) and
  [Deckbound](docs/games/deckbound/README.md) *(early design)*.
- **[Technical](docs/technical/)** — framework docs shared across all games: the
  [architecture](docs/technical/architecture.md) and how to
  [add a game](docs/technical/adding-a-game.md).
