# Boardgame

A framework for turn-based tabletop board games in Rust + Bevy. See `README.md`
for the full layout and design.

## Architecture in one breath

- `crates/engine` — pure framework. The `Game` trait, `Zone`, seeded `Rng`, and
  `TableView`. **No Bevy dependency** — keep it that way so games stay
  unit-testable.
- `crates/<game>` — one pure crate per game (e.g. `deckbound`), implementing
  `engine::Game`. No Bevy; all randomness flows from the seed.
- `crates/tabletop` — the only Bevy crate that renders games. Generic over
  `Game`; never reference a specific game here.
- `crates/boardgame` — the binary that picks a game and runs it.

## Conventions

- Edition 2024. Shared versions live in the root `[workspace.package]` /
  `[workspace.dependencies]`; crates use `version.workspace = true` etc.
- Game logic must be deterministic given a seed. Do not introduce wall-clock
  time or unseeded randomness into the rules.

## Parallel instances: the needs-merge directory

Multiple Claude instances may run in parallel against this repo. To keep them
from stepping on each other when writing documentation, use the `needs-merge/`
directory at the repo root as a staging area.

- When the user says "use the needs-merge directory," write the results of your
  analysis to a **new document** under `needs-merge/` rather than editing the
  mainline docs directly.
- Give the file a descriptive, unique name (e.g. `combat-analysis.md`) so
  concurrent instances do not collide. Do not overwrite a document another
  instance may have written.
- **You merge what you own.** Once the document you authored under `needs-merge/`
  is settled, **that same instance** folds it into the canonical docs (e.g. under
  `docs/`) and removes or marks the staged document. There is no separate merge
  instance. Only touch canonical docs to merge work **you** own — leave another
  instance's staged analysis alone until it merges its own.

## Programming guardrail

Only write code the user explicitly asks for. Refactoring and generating code
the user is actively working on is fine. Do NOT write ad-hoc scripts (Python,
Node, shell, etc.) to accomplish tasks. If a task would be easier with a helper
program, suggest a new Rust crate or `examples/` program and let the user decide.
