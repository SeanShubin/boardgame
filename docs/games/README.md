# Games

Each game has its own folder here — the common root for everything specific to
that game. Inside each:

- `rules/` — player-facing rules (how to play).
- `design/` — design notes (why the game is built the way it is).

Framework documentation shared across all games lives in
[`../technical/`](../technical/) instead.

## The games

- **[Deckbound](deckbound/README.md)** — a cooperative, hidden-information card-combat
  game with a world-map campaign.

## Adding a game's docs

1. Create `docs/games/<your-game>/`, named after the game's crate.
2. Add `rules/README.md` — how to play, free of implementation detail.
3. Add `design/` — a `README.md` overview plus one file per system for a large
   game, or just a single `design/README.md` for a small one.
4. Link the game from the list above (and from [`docs/README.md`](../README.md)).
