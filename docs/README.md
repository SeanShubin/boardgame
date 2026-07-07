# Documentation

The documentation hub. Everything specific to a single game lives under that
game's folder; documentation that applies to all games is kept separate.

## Games

Each game has a common root under [`games/`](games/) holding its player **rules**
and **design** notes.

- [Deckbound](games/deckbound/README.md) — a cooperative card-combat game with a
  world-map campaign.

## Technical (all games)

How the framework itself is built and extended — shared across every game.

- [Architecture](technical/architecture.md) — the crates, the `Game` trait, and
  the `TableView` rendering seam.
- [Adding a game](technical/adding-a-game.md) — step-by-step.

## Reference (external material)

Rulesets and material from other games, mirrored for study — not games built here.

- [Grand Archive](reference/grand-archive/README.md) — a verbatim local mirror of
  the Grand Archive TCG Comprehensive Rules (v1.1.1).

## Layout

- `docs/games/<game>/` — the common root for one game: `rules/` (how to play) and
  `design/` (why it is built that way).
- `docs/technical/` — framework documentation that applies to all games.
- `docs/reference/` — external rulesets mirrored for reference.
