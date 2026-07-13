//! A **game-agnostic scene** — a pure-data description of a full-screen modal the game asks the renderer to
//! draw in place of the normal table (a combat arena is the one use today). The game builds it; the renderer
//! draws it *blind*. Nothing here names a rank, a tempo, a phase, or any other game concept: the renderer
//! knows only tracks, tiles, rows, arrows and text. All game meaning (what lights up, what a badge says, who
//! may strike whom) is decided by the game when it fills these fields, so the renderer stays generic.
//!
//! This is the third thing a [`BoardGame`](crate::seam::BoardGame) can declare, beside intentions and
//! affordances: when [`scene`](crate::seam::BoardGame::scene) returns `Some`, the renderer draws it instead
//! of the felt. Interaction still flows through the ordinary seam verbs — a tile's `tappable` routes taps to
//! `tap_intention`, a `draggable` tile dropped into a [`Row`]'s `drop_pile` routes to `drop_intention`.

use crate::model::{CardId, PileId};

/// A full-screen modal scene: progress tracks down the side, a heading + prompt, a body of card tiles, and
/// an optional attention-arrow overlay and text log. The footer controls are the zone's ordinary
/// affordances; the scene only says which of them render disabled.
pub struct Scene {
    /// Left-sidebar progress tracks (each a titled vertical list with one item marked current).
    pub tracks: Vec<Track>,
    /// The large heading over the body (e.g. a round counter). Empty = draw none.
    pub heading: String,
    /// A muted instruction line under the heading. Empty = draw none.
    pub prompt: String,
    /// The main content: card tiles arranged in rows.
    pub body: SceneBody,
    /// Directed **associations** between cards ("this card relates to those, now"), overlaid as animated
    /// arrows. What the association *means* (a target, a link, a pairing) is the game's business, not the
    /// renderer's.
    pub links: Vec<Link>,
    /// The **decision the game is asking for right now** — its options, drawn as small cards just above the
    /// [`log`](Scene::log), each carrying its own consequence. Empty = the game is asking nothing.
    ///
    /// Clicking one is sent back through
    /// [`choice_intention`](crate::BoardGame::choice_intention). The renderer draws them and does not know
    /// what any of them *mean*; everything a player needs to choose is in the [`Choice`] itself.
    pub choices: Vec<Choice>,
    /// A text panel under the body: un-indented lines are section headers, leading-space lines are entries.
    /// Empty = draw no panel.
    pub log: Vec<String>,
    /// A standing **legend** card in the sidebar: what the abbreviations on the tiles actually mean. Same text
    /// convention as [`log`](Scene::log) — un-indented lines are headers, leading-space lines are entries.
    /// Empty = draw none.
    ///
    /// Tiles are cramped, so they abbreviate ("M 7  F 2  T 1"). An abbreviation the player cannot expand is
    /// just noise, and it does not belong in a manual: the meaning has to be *on the table*, next to the thing
    /// it explains.
    pub legend: Vec<String>,
    /// Indices (into the focused zone's affordance list) of footer controls that render **disabled** —
    /// present but inert (e.g. a "Start" that is not yet legal).
    pub disabled_controls: Vec<usize>,
}

/// One option in the decision the game is currently asking for — a small card the player can take.
///
/// A choice **carries its own consequence**. A label alone ("Strike Back") only names an action; what a
/// player actually needs is what it will *do to them* ("spend 1 tempo, deal 7 back"). Put that on the card
/// and the decision can be made from the screen, without knowing the rules.
///
/// The same applies to an option that is **not** available: it is still shown, with `why_not` saying what
/// stops it. "Why can I not strike back?" must be answerable by looking, not by reading the source — an
/// absent option teaches nothing, and a silently-missing one reads as a bug.
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    /// The action's name, e.g. `"Strike Back"`.
    pub label: String,
    /// What taking it costs and does, e.g. `"spend 1 tempo, deal 7 back"`. Empty = nothing to add.
    pub consequence: String,
    /// Why it cannot be taken, e.g. `"the blow was ranged - nothing to answer"`. Empty = it can be taken.
    /// A choice with a reason is drawn inert, *and shows the reason*.
    pub why_not: String,
    /// This is the option currently staged — what will happen if the player commits as things stand.
    pub chosen: bool,
}

impl Choice {
    /// A choice the player can take.
    pub fn new(label: impl Into<String>, consequence: impl Into<String>) -> Self {
        Choice {
            label: label.into(),
            consequence: consequence.into(),
            why_not: String::new(),
            chosen: false,
        }
    }

    /// Mark this the staged option.
    pub fn chosen(mut self, chosen: bool) -> Self {
        self.chosen = chosen;
        self
    }

    /// Bar this option, saying **why**. It is still shown — inert, with the reason.
    pub fn barred(mut self, why_not: impl Into<String>) -> Self {
        self.why_not = why_not.into();
        self
    }

    /// Whether the player may take it.
    pub fn enabled(&self) -> bool {
        self.why_not.is_empty()
    }
}

/// A left-sidebar progress track: a titled list of steps with the current one highlighted.
pub struct Track {
    pub title: String,
    pub items: Vec<TrackItem>,
}

/// One step in a [`Track`].
pub struct TrackItem {
    pub label: String,
    /// The step the scene is currently on (drawn highlighted).
    pub current: bool,
}

/// How the scene's tiles are laid out.
pub enum SceneBody {
    /// **Assignment rows** — each row is a drop zone (a pile) with a label and its tiles; a `draggable` tile
    /// dropped into another row's `drop_pile` moves there. (A formation being arranged.)
    Rows(Vec<Row>),
    /// **Two-sided lanes** — each lane has a label, a left group and a right group of tiles; the renderer
    /// aligns the divider across lanes. (Two sides facing off.)
    Lanes(Vec<Lane>),
}

/// One assignment row: a labeled drop zone over `drop_pile`, holding its tiles.
pub struct Row {
    pub label: String,
    /// What this row is *for*, in the game's own words — drawn under the label. The renderer knows only that
    /// it is text belonging to the row; the game decides whether that is a tactic, a rule, or nothing at all.
    /// Empty = no hint.
    pub hint: String,
    pub drop_pile: PileId,
    pub tiles: Vec<Tile>,
}

/// One face-off lane: a label, then a left group and a right group of tiles either side of a divider.
pub struct Lane {
    pub label: String,
    pub left: Vec<Tile>,
    pub right: Vec<Tile>,
}

/// Which side of a face-off a tile belongs to — drives its base accent/face only (no game meaning).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Team {
    Left,
    Right,
}

/// A tile's attention state this scene — drives its ring and dimming (the renderer maps each to a look).
/// These are *emphasis* levels, not game states: the game decides which of its situations map to each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Highlight {
    /// A normal, un-highlighted tile.
    Idle,
    /// Nothing to act on here now — it recedes.
    Dim,
    /// A legal thing to act on this step — an "available" cue.
    Available,
    /// The current choice / a tile with a staged action — the brightest cue.
    Active,
    /// Inert / out of play — drawn the hardest-receded.
    Spent,
}

/// The tone of a [`Badge`] line — a rules-neutral color role (emphasis + hue) the renderer resolves to a
/// palette color. The game maps its own meanings onto these; the renderer never learns what they signify.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Ordinary secondary text.
    Muted,
    /// A caution / "this does nothing" cue.
    Warn,
    /// A positive / confirmed cue.
    Good,
    /// A cool-hued category accent.
    Cool,
    /// A warm-hued category accent.
    Warm,
    /// A faded / inert line.
    Faded,
}

/// A short text line under a tile's title, with a color tone.
pub struct Badge {
    pub text: String,
    pub tone: Tone,
}

/// One card tile in a scene: a card, a title, its side and attention state, badge lines, and which input
/// verbs it accepts. Carries no game meaning — the game has already decided every field.
pub struct Tile {
    /// The physical card this tile stands for (its screen rect is tracked by id for arrow endpoints).
    pub card: CardId,
    pub title: String,
    pub team: Team,
    pub highlight: Highlight,
    /// Text lines under the title (a stat bar, capability cues, a staged-plan line).
    pub badges: Vec<Badge>,
    /// The player may drag this tile into another [`Row`]'s `drop_pile`.
    pub draggable: bool,
    /// A single tap on this tile is a game action (routed to `tap_intention`).
    pub tappable: bool,
}

/// A directed **association** between two cards — "this card relates to that one right now" — that the
/// renderer draws as a flowing arrow. The renderer knows only that the two cards are linked, never what the
/// link signifies (the game reads it as a target, a pairing, whatever it likes).
pub struct Link {
    pub from: CardId,
    pub to: CardId,
    /// A committed association (denser, "confirmed" color) vs a merely-possible one (sparser, "available").
    pub confirmed: bool,
    /// A broad association — the renderer fans the dots into several parallel threads.
    pub broad: bool,
}
