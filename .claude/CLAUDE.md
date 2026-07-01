# Boardgame

A **card-table application** in Rust + Bevy, deployed to the web, built on a
small framework that separates game rules from presentation. The card-table UI
is the product and the main thrust of development, grown one feature at a time;
Deckbound (the full combat game) is kept as a reference sample. See `README.md`
for the full layout and design.

## Architecture in one breath

The product:

- `crates/boardgame` — **the deployed binary**: the card-table app. Drives the
  `cardtable` renderer with a `Tableau`. No game wired in yet — this is the small
  seed the UI grows from. Built to WebAssembly with Trunk (see `deploy.yml`).
- `crates/cardtable` — the card-table Bevy renderer (the product's UI): every
  zone a deck, click-to-focus / drag-to-arrange. A shell over `cardtable-model`.
- `crates/cardtable-model` — the pure card-table interaction model (decks, cards,
  focus/zoom, move/reorder). No Bevy.

The framework underneath:

- `crates/contract` — the pure rules↔presentation interface: the `Game` trait and
  the `TableView` snapshot. No Bevy, no logic.
- `crates/engine` — the pure card-game toolkit: `Zone`, seeded `Rng`. **No Bevy** —
  keep it that way so games stay unit-testable.

The reference sample:

- `crates/deckbound` — one pure crate for the game, implementing `contract::Game`.
  No Bevy; all randomness flows from the seed.
- `crates/tabletop` — the button-based Bevy renderer the sample uses. Generic over
  `contract::Game`; never reference a specific game here.
- `crates/deckbound-sample` — the sample launcher binary: wires `Deckbound` into a
  renderer (default `tabletop`, or `cardtable` under `--features cardtable`).

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
