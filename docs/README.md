# Documentation

The documentation hub. Everything specific to a single game lives under that
game's folder; documentation that applies to all games is kept separate.

## Games

Each game has a common root under [`games/`](games/) holding its player **rules**
and **design** notes.

- [Treasure Dive](games/treasure-dive/README.md) — a small push-your-luck card
  game *(implemented)*.
- [Deckbound](games/deckbound/README.md) — a simulation-style fantasy card game
  *(early design — not yet playable)*.

## Technical (all games)

How the framework itself is built and extended — shared across every game.

- [Architecture](technical/architecture.md) — the crates, the `Game` trait, and
  the `TableView` rendering seam.
- [Adding a game](technical/adding-a-game.md) — step-by-step.

## Layout

- `docs/games/<game>/` — the common root for one game: `rules/` (how to play) and
  `design/` (why it is built that way).
- `docs/technical/` — framework documentation that applies to all games.
