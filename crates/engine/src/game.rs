//! The contract every game implements.

use std::fmt;

use crate::player::PlayerId;
use crate::view::TableView;

/// How a finished game turned out.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Outcome {
    /// A single player won outright.
    Win(PlayerId),
    /// Several players tied for the win.
    Tie(Vec<PlayerId>),
}

/// One entry in a game's in-app rules reference (the encyclopedia): a `term` and its rules
/// `text`, grouped under a `category`. A presentation layer can surface these in an overlay so
/// the rules are discoverable in context, without knowing any specific game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefEntry {
    pub category: String,
    pub term: String,
    pub text: String,
}

impl RefEntry {
    pub fn new(
        category: impl Into<String>,
        term: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        Self {
            category: category.into(),
            term: term.into(),
            text: text.into(),
        }
    }
}

/// An attempt to apply an illegal or impossible action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameError(pub String);

impl GameError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for GameError {}

/// A complete, self-contained, turn-based game.
///
/// Implementations are the single source of truth for a game's rules. They are
/// pure: given a state and an action they produce the next state, with all
/// randomness flowing from the seed passed to [`Game::new_game`]. This keeps a
/// game fully reproducible and unit-testable, and lets the same implementation
/// drive a renderer, a bot, or a test harness.
pub trait Game: Send + Sync + 'static {
    /// The full state of a game in progress.
    type State: Clone + Send + Sync + 'static;
    /// A single decision a player can make. Serializable because the stream of actions (plus the
    /// seed) is the canonical, replayable record of a match — what a presentation layer persists for
    /// save/load, undo, and deterministic bug-repro. State is never serialized; it is reconstructed
    /// by replaying actions from [`new_game`](Game::new_game).
    type Action: Clone + Send + Sync + 'static + serde::Serialize + serde::de::DeserializeOwned;

    /// Sets up a fresh game for `players` seats, seeding all randomness from
    /// `seed`.
    fn new_game(&self, seed: u64, players: usize) -> Self::State;

    /// Whose decision the game is waiting on, or `None` once it is over.
    fn current_player(&self, state: &Self::State) -> Option<PlayerId>;

    /// Every action the current player may legally take right now. Empty once
    /// the game is over.
    fn legal_actions(&self, state: &Self::State) -> Vec<Self::Action>;

    /// A short, human-readable label for an action, e.g. "Draw a card".
    fn action_label(&self, state: &Self::State, action: &Self::Action) -> String;

    /// Applies `action` to `state`, advancing the game. Returns an error
    /// without modifying `state` if the action is not legal.
    fn apply(&self, state: &mut Self::State, action: &Self::Action) -> Result<(), GameError>;

    /// The result of the game, or `None` while it is still in progress.
    fn outcome(&self, state: &Self::State) -> Option<Outcome>;

    /// A renderer-agnostic snapshot of the table from `perspective`'s point of
    /// view (`None` for a neutral, all-knowing spectator).
    fn view(&self, state: &Self::State, perspective: Option<PlayerId>) -> TableView;

    /// The action that rewinds one step of a multi-step decision (undo a
    /// partial selection), or `None` when there is nothing to cancel. A
    /// presentation layer can bind this to Escape. Defaults to `None`, so games
    /// without staged input need not implement it.
    fn cancel_action(&self, _state: &Self::State) -> Option<Self::Action> {
        None
    }

    /// A stable identifier for the **scenario / context** this state belongs to (a menu, a specific
    /// battle, a campaign). A presentation layer keeps a *separate, local* undo history per key and
    /// remembers each session's state, so leaving a scenario and coming back resumes it where you
    /// left off ("sticky"), and undo never crosses out of the current one. Navigation between
    /// sessions (picking a scenario, returning to a menu) is the key changing. Defaults to 0 — a
    /// flat game is a single sticky session.
    fn session_key(&self, _state: &Self::State) -> u64 {
        0
    }

    /// The **guide's recommended** next action — an on-script suggestion a presentation layer can
    /// highlight (and detect deviation from). `None` = no guidance. Defaults to `None`.
    fn suggest(&self, _state: &Self::State) -> Option<Self::Action> {
        None
    }

    /// Whether `action` is the guide's suggested action from `state` — so a presentation layer can
    /// flag a **deviation** (taking a different action) without requiring `Action: PartialEq` on the
    /// trait. The game, which knows how to compare its own actions, owns this. Defaults to `false`.
    fn is_suggested(&self, _state: &Self::State, _action: &Self::Action) -> bool {
        false
    }

    /// Whether the game is asking the host application to quit (e.g. the player
    /// chose "Exit" from a menu). A presentation layer can poll this and close
    /// the window. Defaults to `false`.
    fn exit_requested(&self, _state: &Self::State) -> bool {
        false
    }

    /// Whether `action` is the one that asks the host application to quit. A
    /// presentation layer that cannot honor a quit — e.g. a browser tab, where
    /// terminating the app just freezes the canvas — can use this to hide the
    /// action entirely. Defaults to `false`.
    fn is_exit_action(&self, _state: &Self::State, _action: &Self::Action) -> bool {
        false
    }

    /// A browsable rules reference (the in-app encyclopedia): keyword / procedure entries the
    /// presentation layer can surface in an overlay so rules are discoverable in context.
    /// Defaults to empty (a game with no reference simply shows none).
    fn reference(&self) -> Vec<RefEntry> {
        Vec::new()
    }
}
