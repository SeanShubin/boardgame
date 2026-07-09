//! A renderer-agnostic snapshot of the table.
//!
//! A [`Game`](crate::Game) turns its private state into a [`TableView`]: a plain
//! description of what is on the table and whose turn it is. A presentation
//! layer draws a `TableView` without knowing the rules of any particular game,
//! and a game produces one without knowing how it will be drawn. This is the
//! seam that lets one renderer display every game.

use crate::player::PlayerId;

/// Everything a presentation layer needs to draw the table once.
#[derive(Clone, Debug, Default)]
pub struct TableView {
    /// A short, human-readable description of the current situation, e.g.
    /// "Player 0's turn" or "Game over — Player 1 wins!".
    pub status: String,
    /// The piles on the table, in the order they should be presented.
    pub zones: Vec<ZoneView>,
    /// Optional **prose** content (a rules page, a briefing, a log). When non-empty, a renderer
    /// should present it as a formatted, scrollable **reading pane** in place of the card board —
    /// long text belongs in flowing prose, not in fixed-size cards.
    pub prose: Vec<ProseLine>,
    /// Optional **spatial map** (a world board of tiles, with movable tokens). When set, a renderer
    /// draws it as a tiled grid in place of the card board — for a strategic/overworld layer.
    pub map: Option<MapView>,
    /// Optional **event feed** — a running play-by-play (combat resolution, world events), oldest
    /// first. When non-empty, a renderer should present it as a scrolling side panel distinct from
    /// the one-line `status` caption.
    pub log: Vec<String>,
}

/// A spatial map a renderer can draw as a tiled board (a world of locations, §8). Tiles sit at grid
/// coordinates; a renderer that can't draw boards may fall back to listing the tiles as cards.
#[derive(Clone, Debug, Default)]
pub struct MapView {
    /// `true` = offset-hex field (6 neighbours); `false` = square grid (4 neighbours).
    pub hex: bool,
    /// The tiles, in any order (positioned by their coordinates).
    pub tiles: Vec<MapTile>,
}

/// One tile of a [`MapView`].
#[derive(Clone, Debug, Default)]
pub struct MapTile {
    /// Grid column / row.
    pub col: i32,
    pub row: i32,
    /// What the tile shows face-up; `None` means face-down (undiscovered — fog).
    pub label: Option<String>,
    /// An optional second line (e.g. currency type / clear status).
    pub sub: Option<String>,
    /// Colour hint — e.g. [`Accent::Suggested`] for the guide's next move, [`Accent::Good`] cleared.
    pub accent: Accent,
    /// If set, clicking the tile performs the legal action at this index (move / enter).
    pub action: Option<usize>,
    /// Token labels standing on this tile (party pieces); empty if none.
    pub tokens: Vec<String>,
}

/// One line of a prose reading pane, with a role a renderer can style.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProseLine {
    /// A section heading.
    Heading(String),
    /// A bold sub-heading / keyword (e.g. a glossary term).
    Term(String),
    /// A body paragraph (wraps to the pane width).
    Body(String),
    /// Vertical breathing room between blocks.
    Gap,
    /// A small comparison grid — e.g. a "what beats what" RPS chart. A renderer draws it as an
    /// aligned table of cells (proportional fonts can't align an ASCII grid in flowing text).
    Grid(Grid),
}

/// A comparison grid: `headers` are the column labels; each [`GridRow`] is a labelled row of
/// [`GridCell`]s. Row *i*, column *j* reads "row beats / loses to / ties column".
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Grid {
    /// Column headers (the corner above the row labels is left blank by the renderer).
    pub headers: Vec<String>,
    /// The grid's rows, top to bottom.
    pub rows: Vec<GridRow>,
}

/// One labelled row of a [`Grid`].
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct GridRow {
    /// The row's label, shown in the leftmost column.
    pub label: String,
    /// The row's cells, left to right, aligned under [`Grid::headers`].
    pub cells: Vec<GridCell>,
}

/// One cell of a [`Grid`]: short text plus a colour hint.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct GridCell {
    /// The cell's text (e.g. "win", "lose", "trade", "—").
    pub text: String,
    /// A colour hint so a renderer can tint outcomes (e.g. [`Accent::Good`] for a win).
    pub accent: Accent,
}

impl GridCell {
    /// A cell with text and an accent.
    pub fn new(text: impl Into<String>, accent: Accent) -> Self {
        Self {
            text: text.into(),
            accent,
        }
    }
}

/// A single pile of cards as it should appear to the viewer.
#[derive(Clone, Debug)]
pub struct ZoneView {
    /// A label for the pile, e.g. "Deck (24)".
    pub label: String,
    /// A hint for how the cards should be arranged.
    pub layout: Layout,
    /// The player this pile belongs to, if any.
    pub owner: Option<PlayerId>,
    /// The cards in the pile, in presentation order.
    pub cards: Vec<CardView>,
    /// **Nested sub-zones** — piles that live *inside* this one (a card-table drills into them). Empty
    /// for a flat zone. A renderer that can't nest (e.g. the button `tabletop`) may ignore this and draw
    /// only `cards`; a card-table renderer presents each sub-zone as its own drill-in pile. Additive: a
    /// game that never nests leaves it empty and every existing renderer is unaffected.
    pub zones: Vec<ZoneView>,
}

/// A hint for how a renderer should arrange the cards in a zone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// Overlapping pile (a deck or discard).
    Stack,
    /// A straight, evenly spaced row.
    Row,
    /// An overlapping fan, as a hand is held.
    Fan,
}

/// A colour hint for a card, so a renderer can tell allies from foes, flag a
/// warning, or highlight a selection — without knowing any game's rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Accent {
    /// No special meaning.
    #[default]
    Neutral,
    /// Belongs to the viewing player (an ally).
    Ally,
    /// An opponent or hazard.
    Foe,
    /// Something to watch — exposed, panicked, low.
    Warn,
    /// A benefit — a buff, a banked gain.
    Good,
    /// The current focus of a decision.
    Selected,
    /// A guide's **recommended** choice — the on-script next action (campaign guidance, §8).
    Suggested,
}

/// A single card as it should appear to the viewer.
#[derive(Clone, Debug, PartialEq)]
pub struct CardView {
    /// Whether the card's face is visible, and what it shows.
    pub face: CardFace,
    /// If set, this card is **clickable** and selecting it performs the legal action at this
    /// index (into the same `legal_actions` list the renderer draws buttons from). A renderer
    /// should render such a card as interactive and **omit** that action from any button list,
    /// so a choice never appears as both a card and a button.
    pub action: Option<usize>,
    /// How many identical physical cards this one stack stands for (drawn as a `×N` badge). Default 1;
    /// a renderer that doesn't show stacks may ignore it.
    pub quantity: u32,
}

/// The visible side of a card.
///
/// A face-up card is laid out like a collectible-card-game card — a title bar,
/// an optional type line, a body of stat / rules lines, and a corner badge
/// (the "power/toughness" spot) — minus art and flavour, so the space goes to
/// information.
#[derive(Clone, Debug, PartialEq)]
pub enum CardFace {
    /// Face up.
    Up {
        /// The card's name, shown in the title bar.
        title: String,
        /// An optional type line beneath the title (e.g. "Knight · Front").
        type_line: Option<String>,
        /// Stat / rules lines in the card body, top to bottom.
        body: Vec<String>,
        /// Optional **reading-panel** lines, shown when the card is enlarged (e.g. a combat log or a
        /// rules blurb). Distinct from `body`: a card-table renderer may show `body` on the small face
        /// and reveal `panel` only when the card is grown. Empty for an ordinary card.
        panel: Vec<String>,
        /// An optional corner badge (e.g. a health total "5/8").
        corner: Option<String>,
        /// A colour hint for the card.
        accent: Accent,
    },
    /// Face down: only the card back is shown.
    Down,
}

impl CardView {
    /// A face-up card showing only a title.
    pub fn up(title: impl Into<String>) -> Self {
        Self {
            face: CardFace::Up {
                title: title.into(),
                type_line: None,
                body: Vec::new(),
                panel: Vec::new(),
                corner: None,
                accent: Accent::Neutral,
            },
            action: None,
            quantity: 1,
        }
    }

    /// A face-up card showing a title and a numeric corner badge.
    pub fn up_valued(title: impl Into<String>, value: i32) -> Self {
        Self::up(title).corner(value.to_string())
    }

    /// A face-down card.
    pub fn down() -> Self {
        Self {
            face: CardFace::Down,
            action: None,
            quantity: 1,
        }
    }

    /// Bind this card to the legal action at `index`, making it clickable (and removing that
    /// action from the button list). Chainable with the other builders.
    pub fn action(mut self, index: usize) -> Self {
        self.action = Some(index);
        self
    }

    /// Set the type line (no-op on a face-down card).
    pub fn typed(mut self, type_line: impl Into<String>) -> Self {
        if let CardFace::Up { type_line: t, .. } = &mut self.face {
            *t = Some(type_line.into());
        }
        self
    }

    /// Set the body lines (no-op on a face-down card).
    pub fn body(mut self, lines: Vec<String>) -> Self {
        if let CardFace::Up { body, .. } = &mut self.face {
            *body = lines;
        }
        self
    }

    /// Set the corner badge (no-op on a face-down card).
    pub fn corner(mut self, text: impl Into<String>) -> Self {
        if let CardFace::Up { corner, .. } = &mut self.face {
            *corner = Some(text.into());
        }
        self
    }

    /// Set the reading-panel lines (no-op on a face-down card).
    pub fn panel(mut self, lines: Vec<String>) -> Self {
        if let CardFace::Up { panel, .. } = &mut self.face {
            *panel = lines;
        }
        self
    }

    /// Set how many identical cards this stack stands for (the `×N` badge). Chainable.
    pub fn times(mut self, quantity: u32) -> Self {
        self.quantity = quantity;
        self
    }

    /// Set the colour accent (no-op on a face-down card).
    pub fn accent(mut self, a: Accent) -> Self {
        if let CardFace::Up { accent, .. } = &mut self.face {
            *accent = a;
        }
        self
    }
}
