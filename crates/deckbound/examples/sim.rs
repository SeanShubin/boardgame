//! `sim` — a headless combat-state simulator (Phase 1 of the combat-engine observability refactor).
//!
//! Loads a serialized combat [`State`] (RON), applies one or more [`Action`]s through the live
//! [`Deckbound`] `contract::Game` impl, and writes the resulting `State` (RON) back out. This is the
//! load → apply → write loop the refactor plan calls for: state in, action(s) in, state out, all over
//! the filesystem or stdin/stdout so the engine is observable from a shell.
//!
//! State is `#[serde(skip)]`-ped on its `scenario`/`campaign` fields (presentation/campaign context),
//! so a round-tripped state is a pure combat state — exactly what this tool drives.
//!
//! ## Usage
//!
//! ```text
//! sim apply  --state <PATH|-> --action <RON-STRING> --out <PATH|->
//! sim run    --state <PATH|-> --actions <PATH|-> --out <PATH|->
//! sim step   --state <PATH|-> --out <PATH|->
//! sim layout --state <PATH|-> --out <PATH|->
//! ```
//!
//! - `--state` / `--out` / `--actions` accept a filesystem path, or `-` for stdin/stdout.
//! - `apply` applies exactly one `Action` parsed from the `--action` RON string.
//! - `run` applies a `Vec<Action>` (RON, read from `--actions`) in order.
//! - `step` advances the in-flight §4.6 resolution machine **one atomic step** (`combat::step`): it
//!   resolves the next sub-phase pair / crosses the next sub-phase boundary. If the loaded state is
//!   not mid-resolution (e.g. at Marshal), it is a no-op that reports so on stderr.
//! - `layout` prints the **derived 2D combat layout** (`State::layout` → `CombatLayout`, side × rank ×
//!   slot) as RON — a read-only view; the state itself is not modified.
//!
//! An illegal action prints the error to stderr and exits non-zero (the `State` is left unmodified by
//! the engine's `apply`, so nothing is written on failure).
//!
//! ### Example
//!
//! ```text
//! # Serialize a state elsewhere (e.g. a test), then advance one declaration:
//! sim apply --state battle.ron --action 'SetVanguard(0)' --out -
//! ```

use std::io::{Read, Write};
use std::process::exit;

use contract::Game;
use deckbound::{Action, Deckbound, State, combat};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(e) = run(&args) {
        eprintln!("sim: {e}");
        exit(1);
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let cmd = args.first().map(String::as_str);
    match cmd {
        Some("apply") => cmd_apply(&args[1..]),
        Some("run") => cmd_run(&args[1..]),
        Some("step") => cmd_step(&args[1..]),
        Some("layout") => cmd_layout(&args[1..]),
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "usage:\n  \
     sim apply --state <PATH|-> --action <RON-STRING> --out <PATH|->\n  \
     sim run   --state <PATH|-> --actions <PATH|-> --out <PATH|->\n  \
     sim step   --state <PATH|-> --out <PATH|->\n  \
     sim layout --state <PATH|-> --out <PATH|->"
        .to_string()
}

/// `apply`: load a State, apply ONE Action (from the `--action` RON string), write the result.
fn cmd_apply(args: &[String]) -> Result<(), String> {
    let state_arg = flag(args, "--state")?;
    let action_str = flag(args, "--action")?;
    let out_arg = flag(args, "--out")?;

    let mut state = load_state(&state_arg)?;
    let action: Action =
        ron::from_str(&action_str).map_err(|e| format!("parsing --action: {e}"))?;

    let game = Deckbound::default();
    game.apply(&mut state, &action)
        .map_err(|e| format!("illegal action {action:?}: {e}"))?;

    write_state(&out_arg, &state)
}

/// `run`: load a State, apply a Vec<Action> (RON) in order, write the final State.
fn cmd_run(args: &[String]) -> Result<(), String> {
    let state_arg = flag(args, "--state")?;
    let actions_arg = flag(args, "--actions")?;
    let out_arg = flag(args, "--out")?;

    let mut state = load_state(&state_arg)?;
    let actions_text = read_source(&actions_arg)?;
    let actions: Vec<Action> =
        ron::from_str(&actions_text).map_err(|e| format!("parsing --actions: {e}"))?;

    let game = Deckbound::default();
    for (i, action) in actions.iter().enumerate() {
        game.apply(&mut state, action)
            .map_err(|e| format!("illegal action #{i} {action:?}: {e}"))?;
    }

    write_state(&out_arg, &state)
}

/// `step`: load a State, advance the in-flight §4.6 resolution machine ONE atomic step
/// ([`combat::step`]), write the result. If the state is not mid-resolution (`resolution` is `None`,
/// e.g. at Marshal), it is a no-op — the state is written back unchanged and a note goes to
/// stderr.
fn cmd_step(args: &[String]) -> Result<(), String> {
    let state_arg = flag(args, "--state")?;
    let out_arg = flag(args, "--out")?;

    let mut state = load_state(&state_arg)?;
    if state.resolution.is_none() {
        eprintln!(
            "sim: state is not mid-resolution (resolution: None) — nothing to step; writing it back unchanged"
        );
    } else {
        let more = combat::step(&mut state);
        if !more {
            eprintln!("sim: resolution complete (last step) — the round should now advance");
        }
    }

    write_state(&out_arg, &state)
}

/// `layout`: load a State and print its **derived 2D combat layout** (`State::layout` → `CombatLayout`,
/// side × rank × slot with group adjacency) as RON. Read-only: the state is not modified; only the
/// derived view is written.
fn cmd_layout(args: &[String]) -> Result<(), String> {
    let state_arg = flag(args, "--state")?;
    let out_arg = flag(args, "--out")?;

    let state = load_state(&state_arg)?;
    let layout = state.layout();
    write_ron(&out_arg, &layout, "layout")
}

// ---- tiny dependency-free arg parsing & I/O (RON throughout, matching the codebase) ----

/// The value following `name` in `args` (e.g. `--state <value>`). Errors if absent.
fn flag(args: &[String], name: &str) -> Result<String, String> {
    let pos = args
        .iter()
        .position(|a| a == name)
        .ok_or_else(|| format!("missing {name}\n{}", usage()))?;
    args.get(pos + 1)
        .cloned()
        .ok_or_else(|| format!("{name} needs a value\n{}", usage()))
}

/// Read from a path, or from stdin when `arg` is `-`.
fn read_source(arg: &str) -> Result<String, String> {
    if arg == "-" {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("reading stdin: {e}"))?;
        Ok(buf)
    } else {
        std::fs::read_to_string(arg).map_err(|e| format!("reading {arg}: {e}"))
    }
}

fn load_state(arg: &str) -> Result<State, String> {
    let text = read_source(arg)?;
    ron::from_str(&text).map_err(|e| format!("parsing state from {arg}: {e}"))
}

/// Write the State (RON) to a path, or to stdout when `arg` is `-`.
fn write_state(arg: &str, state: &State) -> Result<(), String> {
    write_ron(arg, state, "state")
}

/// Serialize any value to RON and write it to a path, or to stdout (with a trailing newline) when
/// `arg` is `-`. `what` names the value for error messages.
fn write_ron<T: serde::Serialize>(arg: &str, value: &T, what: &str) -> Result<(), String> {
    let text = ron::ser::to_string(value).map_err(|e| format!("serializing {what}: {e}"))?;
    if arg == "-" {
        let mut out = std::io::stdout();
        out.write_all(text.as_bytes())
            .and_then(|_| out.write_all(b"\n"))
            .map_err(|e| format!("writing stdout: {e}"))
    } else {
        std::fs::write(arg, text).map_err(|e| format!("writing {arg}: {e}"))
    }
}
