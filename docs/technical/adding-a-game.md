# Adding a Game

A new game is a new pure crate that implements `contract::Game`. The renderers
(`tabletop`, `cardtable`) do not change; you give the game its own launcher
binary, mirroring `deckbound-sample`.

## Steps

1. **Create the crate.** Add `crates/<your-game>/` with a `Cargo.toml` that
   depends on `contract` (the interface) and, if you want the card-game toolkit,
   `engine`:

   ```toml
   [package]
   name = "<your-game>"
   version.workspace = true
   edition.workspace = true

   [dependencies]
   contract = { workspace = true }
   engine = { workspace = true }   # optional: Zone, seeded Rng
   ```

   Add it to `members` in the root `Cargo.toml`, and add a
   `<your-game> = { path = "crates/<your-game>" }` entry under
   `[workspace.dependencies]` if a launcher will reference it.

2. **Define your types.** A `State` (the full game state, `Clone`) and an
   `Action` (one decision a player can make, `Clone`).

3. **Implement `contract::Game`.** Fill in `new_game`, `current_player`,
   `legal_actions`, `action_label`, `apply`, `outcome`, and `view`. Keep it
   pure — seed all randomness from the `seed` argument via `engine::Rng`.

4. **Render the state.** In `view`, turn your `State` into a `TableView`: one
   `ZoneView` per pile on the table, with `CardView`s that are face up or down,
   plus a status line.

5. **Test it.** Game logic is pure, so test it directly — determinism from a
   fixed seed, legal/illegal actions, scoring, and the end condition. See
   [`crates/deckbound/src/game.rs`](../../crates/deckbound/src/game.rs)
   for examples.

6. **Run it.** Give your game a launcher binary — copy `crates/deckbound-sample`
   (its `Cargo.toml` and `src/main.rs`), rename it, and point it at your game:

   ```rust
   .add_plugins(TabletopPlugin::new(YourGame, SEED, PLAYERS))
   ```

   Build with `--features cardtable` to drive the card-table renderer instead.

7. **Document it.** Create `docs/games/<your-game>/` with player rules under
   `rules/` and design notes under `design/`. See
   [the games folder](../games/README.md).

## Reference implementation

`crates/deckbound` is the worked example — read its `game.rs` end to end when
starting a new game.
