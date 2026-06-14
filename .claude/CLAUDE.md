# Boardgame

A framework for turn-based tabletop board games in Rust + Bevy. See `README.md`
for the full layout and design.

## Architecture in one breath

- `crates/engine` — pure framework. The `Game` trait, `Zone`, seeded `Rng`, and
  `TableView`. **No Bevy dependency** — keep it that way so games stay
  unit-testable.
- `crates/<game>` — one pure crate per game (e.g. `treasure-dive`), implementing
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
- Do not edit the canonical docs (e.g. under `docs/`) for that analysis — a
  separate Claude instance is responsible for merging `needs-merge/` documents
  into the mainline.

## Programming guardrail

Only write code the user explicitly asks for. Refactoring and generating code
the user is actively working on is fine. Do NOT write ad-hoc scripts (Python,
Node, shell, etc.) to accomplish tasks. If a task would be easier with a helper
program, suggest a new Rust crate or `examples/` program and let the user decide.
