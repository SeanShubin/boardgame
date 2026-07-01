# Technical Documentation

How the framework is built and how to extend it. This describes the code; it
says nothing about how to play or how a game is balanced — see
the game's own `rules/` and `design/` notes under [`docs/games/`](../games/) for
those.

## Contents

- [Architecture](architecture.md) — the workspace crates (the card-table product
  and the reference sample), the `Game` trait that every game implements, and the
  `TableView` seam that lets either renderer draw any game.
- [Adding a game](adding-a-game.md) — a step-by-step recipe for a new game crate.

## Suggested topics for later

As the framework grows, this is where the following would live:

- The seeded RNG and reproducibility/replay model.
- The presentation layer in depth (zones, layouts, input).
- Testing strategy for game logic.
- AI / bot players built on `legal_actions`.
