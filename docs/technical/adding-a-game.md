# Adding a Game

A new game is a new pure crate that implements `engine::Game`. The renderer
(`tabletop`) and the launcher (`boardgame`) do not change, except for the one
line that chooses which game to run.

## Steps

1. **Create the crate.** Add `crates/<your-game>/` with a `Cargo.toml` that
   depends on `engine`:

   ```toml
   [package]
   name = "<your-game>"
   version.workspace = true
   edition.workspace = true

   [dependencies]
   engine = { workspace = true }
   ```

   Add it to `members` in the root `Cargo.toml`, and add a
   `<your-game> = { path = "crates/<your-game>" }` entry under
   `[workspace.dependencies]` if the launcher will reference it.

2. **Define your types.** A `State` (the full game state, `Clone`) and an
   `Action` (one decision a player can make, `Clone`).

3. **Implement `engine::Game`.** Fill in `new_game`, `current_player`,
   `legal_actions`, `action_label`, `apply`, `outcome`, and `view`. Keep it
   pure — seed all randomness from the `seed` argument via `engine::Rng`.

4. **Render the state.** In `view`, turn your `State` into a `TableView`: one
   `ZoneView` per pile on the table, with `CardView`s that are face up or down,
   plus a status line.

5. **Test it.** Game logic is pure, so test it directly — determinism from a
   fixed seed, legal/illegal actions, scoring, and the end condition. See
   [`crates/deckbound/src/game.rs`](../../crates/deckbound/src/game.rs)
   for examples.

6. **Run it.** Point the launcher at your game in
   [`crates/boardgame/src/main.rs`](../../crates/boardgame/src/main.rs):

   ```rust
   .add_plugins(TabletopPlugin::new(YourGame, SEED, PLAYERS))
   ```

7. **Document it.** Create `docs/games/<your-game>/` with player rules under
   `rules/` and design notes under `design/`. See
   [the games folder](../games/README.md).

## Reference implementation

`crates/deckbound` is the worked example — read its `game.rs` end to end when
starting a new game.
