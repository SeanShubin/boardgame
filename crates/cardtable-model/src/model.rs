//! The pure pile/card model and its behaviors. Dependency-free: no `contract`, no Bevy.
//!
//! A [`Tableau`] is a tree of [`Pile`]s. Each pile holds an ordered list of cards and an ordered
//! list of child piles (so a pile can contain piles — the recursive zoom of the card-table UI). A
//! card always lives in exactly one pile (the "every card has a physical place" principle): operations
//! preserve that invariant or return a [`TableauError`] without mutating anything.

use std::collections::HashMap;

/// A stable handle to a card within a [`Tableau`].
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct CardId(pub u64);

/// A stable handle to a pile within a [`Tableau`].
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct PileId(pub u64);

/// A **child of a pile** — one entry in its single ordered [`children`](Pile::children) list. A pile's
/// contents are a homogeneous sequence of nodes, each either a leaf [`Card`] or a nested [`Pile`]; both
/// are positioned, sized, movable, and shoved *uniformly* (see [`Tableau::separate`]). The card-vs-pile
/// distinction only decides leaf behavior — a card grows, a pile drills in — not whether it can move.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Node {
    Card(CardId),
    Pile(PileId),
}

impl Node {
    /// The card this node is, if it is a leaf card.
    pub fn card(self) -> Option<CardId> {
        match self {
            Node::Card(id) => Some(id),
            Node::Pile(_) => None,
        }
    }

    /// The pile this node is, if it is a nested pile.
    pub fn pile(self) -> Option<PileId> {
        match self {
            Node::Pile(id) => Some(id),
            Node::Card(_) => None,
        }
    }
}

/// A 2-D position on the table surface, in pixels from its top-left. The card-table is a physical
/// space, so a pile has a *place*; the renderer draws it there and drag-to-place updates it.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
}

impl Pos {
    /// Squared distance to `other` — used to order [`Tableau::separate`]'s outward wavefront without a
    /// square root (only the ordering matters).
    fn dist_sq(self, other: Pos) -> f32 {
        (self.x - other.x).powi(2) + (self.y - other.y).powi(2)
    }
}

/// The model's own, minimal card face — independent of any game or of `contract::CardFace`, so the
/// core stays dependency-free. The [`binding`](crate::binding) module maps between the two.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Face {
    /// Face up, showing at least a title.
    Up { title: String },
    /// Face down — only the back is visible, but the card **remembers its front**
    /// (spec `physical-cards.md` PC.2: a flip is reversible, and a packed face-down
    /// card restores to the right card). [`Card::name`] still reports `""` while down
    /// (face-down cards stay anonymous in views); [`Card::front_title`] reads the
    /// remembered front regardless of facing.
    Down { title: String },
}

/// How large a card is currently drawn — the three planned card sizes. Default [`Size::Small`], the
/// compact form. A card grows only as far as it has content for: [`Size::Medium`] needs `detail`,
/// [`Size::Large`] needs `panel`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Size {
    /// **Small** — the compact form: just the name and type (plus a quantity when several of a kind
    /// stack). What decks and their contents (e.g. location cards) render as. The default.
    #[default]
    Small,
    /// **Medium** — a full individual card face: name and type over its detail lines (stats / rules).
    Medium,
    /// **Large** — a document-sized panel for special content with no physical-card counterpart, e.g.
    /// documentation or a combat log.
    Large,
}

/// What a card *is*, which decides what a single click on it means (the renderer reads this).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CardKind {
    /// An ordinary card (may still carry `detail` / a `panel`).
    Regular,
    /// A card whose job is to name its pile — and the zone you enter by drilling into that pile.
    Zone,
    /// A user-interface card that performs an action when clicked.
    Utility(Utility),
    /// A **row header** in an [`Arrangement::Rows`] pile: the first card of a row, naming it. Not a
    /// content card — it isn't dragged or counted.
    Header,
    /// A **virtual** readout — a card that shows game state (e.g. a combat log) rather than being a
    /// physical tabletop card. It renders and expands like a Regular card, but is **not counted** in the
    /// physical tally ([`Tableau::physical_card_count`]): it exists only in software.
    Virtual,
}

/// The action a [`CardKind::Utility`] card performs when clicked — e.g. a card in the System deck.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Utility {
    /// Quit the application (desktop only).
    Exit,
    /// Start over from a pristine table, discarding this session's changes and the saved game.
    StartOver,
}

/// A **recipe**: the structured content a starting kit yields when it equips a character. It carries the
/// stat *values* (`[Might, Vitality, Toughness, Cadence, Finesse]`, the [`catalog::STATS`](crate::catalog::STATS)
/// order) and the name of the ability the character gains. Building a character deck instantiates it —
/// one card per stat named `"{Stat} {value}"` plus one ability card — each looking its description up in
/// the [`catalog`](crate::catalog), so the kit stores only the numbers and everything else is authored once.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Recipe {
    /// The five stat values, in `[Might, Vitality, Toughness, Cadence, Finesse]` order.
    pub stats: [u8; 5],
    /// The ability the character gains — a name in [`catalog::ABILITIES`](crate::catalog::ABILITIES).
    pub ability: String,
}

/// The default [`Card::quantity`] for a card that doesn't record one (a single physical card).
fn default_quantity() -> u32 {
    1
}

/// A single card and its place in the tableau. Beyond its `face`, a card carries the content for the
/// larger render [`Size`]s (`detail`, `panel`) and a [`CardKind`] that gives a click its meaning.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Card {
    /// This card's stable id.
    pub id: CardId,
    /// What the card shows (its name is the face title).
    pub face: Face,
    /// If `Some(i)`, this card maps to the legal action at index `i` in the source view — i.e. a
    /// click here is a legal move, not just a look. `None` means "viewable but not actionable".
    pub actionable: Option<usize>,
    detail: Vec<String>,
    panel: Vec<String>,
    kind: CardKind,
    /// The card's **type** — a short category shown on the card and used as the deck's top-card label
    /// (e.g. "Location", "Adventurer"). Every card has one; empty means untyped (no badge drawn).
    card_type: String,
    size: Size,
    home: PileId,
    /// Free position on the drilled-in surface and rendered footprint — used only by an
    /// [`Arrangement::Free`] deck, where cards are placed and shoved like the top-level piles.
    pos: Pos,
    footprint: Pos,
    /// This card's **recipe** — the structured content it yields when combined. A starting kit carries a
    /// [`Recipe`] (the character's stat values + ability); an ordinary card (e.g. a hero identity) has none.
    recipe: Option<Recipe>,
    /// **Quantity** — how many identical physical cards this one stack stands for (spec
    /// `canon/2-spec/physical-cards.md` PC.2/PC.5): a run of duplicates in the same pile is one card that
    /// carries its count (`6 ×12`). Default 1. **Conservation is on the sum** of quantities, so
    /// [`card_count`](Tableau::card_count) totals them; splitting/merging a stack preserves that sum.
    #[serde(default = "default_quantity")]
    quantity: u32,
    /// **Pairing target key** — if `Some(k)`, another card can be paired onto this one (the renderer's
    /// drag-drop / tap-then-tap target). `None` = not a target. Mirrors `contract::CardView::pair_key`.
    /// Skipped from the save when absent, so a card without pairings serializes exactly as before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pair_key: Option<u32>,
    /// **Pairings** *from* this card — `(target pair_key, actionable index)`: completing the pairing
    /// performs the source view's legal action at that index. Empty = pairs onto nothing. Mirrors
    /// `contract::CardView::pairings`. Skipped from the save when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pairings: Vec<(u32, usize)>,
}

impl Card {
    /// The pile this card currently lives in.
    pub fn home(&self) -> PileId {
        self.home
    }

    /// How many identical physical cards this stack represents (`×N`, spec `physical-cards.md` PC.2) —
    /// at least 1.
    pub fn quantity(&self) -> u32 {
        self.quantity.max(1)
    }

    /// Whether clicking this card performs a legal move.
    pub fn is_actionable(&self) -> bool {
        self.actionable.is_some()
    }

    /// The card's name — its face title, or empty when face down (a face-down card is anonymous in
    /// views; use [`front_title`](Card::front_title) to read its remembered front).
    pub fn name(&self) -> &str {
        match &self.face {
            Face::Up { title } => title,
            Face::Down { .. } => "",
        }
    }

    /// The card's **front title regardless of facing** — the remembered face even while down
    /// (spec `physical-cards.md` PC.2). Use this to identify a face-down card (e.g. which hero a
    /// flipped day-clock copy is) or to restore it on flip-up; use [`name`](Card::name) for display.
    pub fn front_title(&self) -> &str {
        match &self.face {
            Face::Up { title } | Face::Down { title } => title,
        }
    }

    /// Is this card face down?
    pub fn is_face_down(&self) -> bool {
        matches!(self.face, Face::Down { .. })
    }

    /// Body lines shown at [`Size::Medium`].
    pub fn detail(&self) -> &[String] {
        &self.detail
    }

    /// Panel lines shown at [`Size::Large`].
    pub fn panel(&self) -> &[String] {
        &self.panel
    }

    /// This card's **pairing target key**, if it is a pairing target (something can be paired onto it).
    pub fn pair_key(&self) -> Option<u32> {
        self.pair_key
    }

    /// The **pairings** from this card — `(target key, actionable index)`: pairing this card onto the
    /// target with that key performs the source view's legal action at that index.
    pub fn pairings(&self) -> &[(u32, usize)] {
        &self.pairings
    }

    /// What the card is (drives click meaning).
    pub fn kind(&self) -> CardKind {
        self.kind
    }

    /// The card's type — a short category (e.g. "Location"). Empty when untyped.
    pub fn card_type(&self) -> &str {
        &self.card_type
    }

    /// How large the card is drawn right now.
    pub fn size(&self) -> Size {
        self.size
    }

    /// The card's free position on the drilled-in surface (an [`Arrangement::Free`] deck only).
    pub fn pos(&self) -> Pos {
        self.pos
    }

    /// The card's rendered footprint (`x` = width, `y` = height), fed back by the renderer for shoving.
    pub fn footprint(&self) -> Pos {
        self.footprint
    }

    /// The [`Recipe`] this card yields when combined (a kit's recipe), or `None` for an ordinary card.
    pub fn recipe(&self) -> Option<&Recipe> {
        self.recipe.as_ref()
    }

    /// Whether the card has more than a name to show, so a click can grow it.
    pub fn is_expandable(&self) -> bool {
        !self.detail.is_empty() || !self.panel.is_empty()
    }

    /// Whether this is a **physical** tabletop card — the single source of truth for the physical tally
    /// ([`Tableau::physical_card_count`]). Software cards are not physical: a [`Utility`](CardKind::Utility)
    /// control (the System deck's Start Over / Exit) and a [`Virtual`](CardKind::Virtual) readout (a combat
    /// log) both exist only in software, so neither is counted.
    pub fn is_physical(&self) -> bool {
        !matches!(self.kind(), CardKind::Utility(_) | CardKind::Virtual)
    }
}

/// Whether two cards group as one entry in the Name view: same face (up/down), name, and type.
fn same_type(a: &Card, b: &Card) -> bool {
    a.is_face_down() == b.is_face_down() && a.name() == b.name() && a.card_type == b.card_type
}

// ---- shared box geometry (piles and cards separate against these) ----------------------------

/// The centre point of a box `(pos, size)`.
fn box_center(pos: Pos, size: Pos) -> Pos {
    Pos {
        x: pos.x + size.x * 0.5,
        y: pos.y + size.y * 0.5,
    }
}

/// Clamp a box of `size` at `pos` fully inside `surface` (top-left origin). A box larger than the
/// surface pins to the top-left.
fn clamp_box(pos: Pos, size: Pos, surface: Pos) -> Pos {
    let max_x = (surface.x - size.x).max(0.0);
    let max_y = (surface.y - size.y).max(0.0);
    Pos {
        x: pos.x.clamp(0.0, max_x),
        y: pos.y.clamp(0.0, max_y),
    }
}

/// Total area by which a box `(pos, size)` overlaps the `locked` boxes.
fn overlap_area(pos: Pos, size: Pos, locked: &[(Pos, Pos)]) -> f32 {
    let mut total = 0.0;
    for &(lp, lsz) in locked {
        let ox = (pos.x + size.x).min(lp.x + lsz.x) - pos.x.max(lp.x);
        let oy = (pos.y + size.y).min(lp.y + lsz.y) - pos.y.max(lp.y);
        if ox > 0.01 && oy > 0.01 {
            total += ox * oy;
        }
    }
    total
}

/// The position nearest `cur` for a box of `size` that is fully inside `surface` *and* clear of every
/// `locked` box; if none is fully clear, the in-bounds spot of least total overlap. The free region for
/// an axis-aligned box among static boxes is rectilinear, so its nearest point lies on a candidate
/// coordinate line: the box's own coordinate, each locked box's near/far edge in configuration space, or
/// a wall. We test that grid of lines (straight slides *and* go-around-a-corner spots) and keep the
/// clear one closest to where the box already is.
fn place_clear_of(cur: Pos, size: Pos, locked: &[(Pos, Pos)], surface: Pos) -> Pos {
    let max_x = (surface.x - size.x).max(0.0);
    let max_y = (surface.y - size.y).max(0.0);
    let cx = cur.x.clamp(0.0, max_x);
    let cy = cur.y.clamp(0.0, max_y);
    let mut xs = vec![cx, 0.0, max_x];
    let mut ys = vec![cy, 0.0, max_y];
    for &(lp, lsz) in locked {
        xs.push(lp.x - size.x); // just left of the locked box
        xs.push(lp.x + lsz.x); // just right of it
        ys.push(lp.y - size.y); // just above it
        ys.push(lp.y + lsz.y); // just below it
    }
    xs.retain(|&x| x >= 0.0 && x <= max_x);
    ys.retain(|&y| y >= 0.0 && y <= max_y);

    let mut best_clear: Option<(f32, Pos)> = None; // (dist², pos)
    let mut best_any: Option<(f32, f32, Pos)> = None; // (overlap area, dist², pos)
    for &x in &xs {
        for &y in &ys {
            let pos = Pos { x, y };
            let overlap = overlap_area(pos, size, locked);
            let dist_sq = (x - cur.x).powi(2) + (y - cur.y).powi(2);
            if overlap <= 0.0 {
                if best_clear.is_none_or(|(d, _)| dist_sq < d) {
                    best_clear = Some((dist_sq, pos));
                }
            } else if best_any.is_none_or(|(o, d, _)| overlap < o || (overlap == o && dist_sq < d))
            {
                best_any = Some((overlap, dist_sq, pos));
            }
        }
    }
    best_clear
        .map(|(_, p)| p)
        .or(best_any.map(|(_, _, p)| p))
        .unwrap_or(Pos { x: cx, y: cy })
}

/// Lock-as-you-go separation of `boxes` (`(pos, size)`) inside `surface`, pinning `anchor` and shoving
/// the rest clear nearest-first (a wavefront outward from the anchor). Returns each box's settled
/// position, index-aligned with `boxes`. The shared core of [`Tableau::separate`] (piles) and
/// [`Tableau::separate`] (cards): because each box is placed clear of all already-settled boxes
/// and never disturbed afterward, no two overlap once the space allows it. Terminates in one placement
/// per box.
fn separate_boxes(
    boxes: &[(Pos, Pos)],
    anchor: usize,
    surface: Pos,
    pinned: &[(Pos, Pos)],
) -> Vec<Pos> {
    let mut result: Vec<Pos> = boxes.iter().map(|&(p, _)| p).collect();
    if boxes.is_empty() {
        return result;
    }
    // Priority order, lock-as-you-go: whoever is placed first wins its spot; everyone after settles clear.
    let mut locked: Vec<(Pos, Pos)> = Vec::with_capacity(pinned.len() + boxes.len());
    // (1) Pinned fixtures — highest priority. Placed *through the same clear-rule* (not dumped raw), so a
    // lower-priority pinned box yields to a higher one and two pinned boxes can never overlap.
    for &(p, s) in pinned {
        let pos = place_clear_of(p, s, &locked, surface);
        locked.push((pos, s));
    }
    // (2) The anchor (the just-dropped / just-changed box) — clear of the pinned, but nothing else moves it.
    let (anchor_pos, anchor_size) = boxes[anchor];
    let anchor_center = box_center(anchor_pos, anchor_size);
    let anchor_pos = place_clear_of(anchor_pos, anchor_size, &locked, surface);
    result[anchor] = anchor_pos;
    locked.push((anchor_pos, anchor_size));
    // (3) The rest — fan out nearest-first from the anchor, each clear of everything already locked.
    let mut order: Vec<usize> = (0..boxes.len()).filter(|&i| i != anchor).collect();
    order.sort_by(|&i, &j| {
        let di = box_center(boxes[i].0, boxes[i].1).dist_sq(anchor_center);
        let dj = box_center(boxes[j].0, boxes[j].1).dist_sq(anchor_center);
        di.partial_cmp(&dj)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(i.cmp(&j))
    });
    for i in order {
        let pos = place_clear_of(boxes[i].0, boxes[i].1, &locked, surface);
        result[i] = pos;
        locked.push((pos, boxes[i].1));
    }
    result
}

/// How a pile arranges its contents when you drill into it — the shape half of a [`Layout`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Arrangement {
    /// **One-dimensional**: an ordered list that wraps across rows to fit the width. A card's place is
    /// a single linear index; reordering slides it along that sequence. The default.
    #[default]
    List,
    /// **Two-dimensional**: a grid `columns` wide, where a card's row *and* column are both meaningful
    /// and kept as cards come and go.
    Grid { columns: usize },
    /// **Unordered**: cards are freely positioned and dragged at will — order is irrelevant. Overlaps
    /// shove apart ([`separate`](Tableau::separate)), exactly as the top-level piles do on
    /// the table, whatever each card's current size. The layout the System deck uses.
    Free,
    /// **Rows**: a stack of horizontal rows. Each row is led by a [`Header`](CardKind::Header) card that
    /// names it; the rest of the row's cards follow as a horizontal **fan** — overlapped so only each
    /// card's left-edge sliver shows, with the tapped card pulled fully to the front to examine it (the
    /// header is not part of the fan). See [`Tableau::row_groups`]. Used by the inn.
    Rows,
}

/// How a pile presents its contents: an [`Arrangement`] (1-D list or 2-D grid) plus whether the
/// contents are `editable` (can be dragged to reorder). The four combinations share one layout path —
/// only these two switches differ. Default: an editable list (the original behaviour).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Layout {
    pub arrangement: Arrangement,
    pub editable: bool,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            arrangement: Arrangement::List,
            editable: true,
        }
    }
}

/// A pile of cards (and, optionally, nested piles). Collapsed = shown as a compact, counted pile;
/// not collapsed = fanned out and attended to.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Pile {
    /// This pile's stable id.
    pub id: PileId,
    /// A human-readable label (e.g. "Hand", "Vanguard").
    pub label: String,
    /// `true` = compact pile; `false` = fanned/attended.
    pub collapsed: bool,
    parent: Option<PileId>,
    /// This pile's contents, in one ordered list: cards and nested piles interleaved, positioned and
    /// shoved uniformly. [`cards`](Pile::cards) / [`subpiles`](Pile::subpiles) are filtered views of it.
    children: Vec<Node>,
    pos: Pos,
    size: Pos,
    layout: Layout,
    /// If non-empty, this pile is a **projection**: instead of its own contents, drilling into it shows
    /// the [content cards](Tableau::content_cards) of these source piles (grouped, still owned by the
    /// sources). A projection gathers relevant cards from several decks into one view without moving them.
    projection: Vec<PileId>,
    /// If set, this pile is a **character deck** for the hero whose [`CardId`] this is (the deck's own
    /// identity/label card). [`equip_character`](Tableau::equip_character) sets it and
    /// [`unequip_character`](Tableau::unequip_character) removes the deck; the party reconcile
    /// ([`sync_party`](Tableau::sync_party)) reads it to know who is active. Nothing else carries this.
    reflects: Option<CardId>,
}

impl Pile {
    /// This pile's parent, or `None` for the root.
    pub fn parent(&self) -> Option<PileId> {
        self.parent
    }

    /// This pile's contents in order — cards and nested piles interleaved.
    pub fn children(&self) -> &[Node] {
        &self.children
    }

    /// The cards directly in this pile, in order (bottom to top) — the card nodes of [`children`](Pile::children).
    pub fn cards(&self) -> Vec<CardId> {
        self.children.iter().filter_map(|n| n.card()).collect()
    }

    /// The piles nested directly in this pile, in order — the pile nodes of [`children`](Pile::children).
    pub fn subpiles(&self) -> Vec<PileId> {
        self.children.iter().filter_map(|n| n.pile()).collect()
    }

    /// This pile's position on the table surface.
    pub fn pos(&self) -> Pos {
        self.pos
    }

    /// This pile's rendered size (`x` = width, `y` = height), fed back by the renderer after layout.
    pub fn size(&self) -> Pos {
        self.size
    }

    /// How this pile arranges its contents when drilled into, and whether they can be rearranged.
    pub fn layout(&self) -> Layout {
        self.layout
    }

    /// The source piles this pile **projects** (shows the cards of). Empty for an ordinary pile.
    pub fn projection(&self) -> &[PileId] {
        &self.projection
    }

    /// The hero this pile is a **character deck** for, or `None` for an ordinary pile. See
    /// [`Tableau::equip_character`].
    pub fn reflects(&self) -> Option<CardId> {
        self.reflects
    }
}

/// Why a pile/card operation could not be performed. Operations that return this leave the tree
/// unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TableauError {
    /// No card with this id exists.
    UnknownCard(CardId),
    /// No pile with this id exists.
    UnknownPile(PileId),
    /// An index was outside the valid range for the operation.
    IndexOutOfRange,
    /// A pile move that would break the tree — moving the root, or moving a pile into itself or one
    /// of its own descendants (a cycle).
    InvalidMove,
    /// A bank ran out of the card an assembly asked for — a provisioning shortfall (raise the supply,
    /// spec `physical-cards.md` PC.5). Carries the requested card's name.
    BankEmpty(String),
}

/// A tree of piles and cards, plus the current attention state (which pile is focused, which cards
/// are selected). All behaviors are methods here; the renderer reads the resulting state to draw.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Tableau {
    piles: HashMap<PileId, Pile>,
    cards: HashMap<CardId, Card>,
    root: PileId,
    focus: PileId,
    selection: Vec<CardId>,
    next_pile: u64,
    next_card: u64,
    // Renderer-fed, transient: not persisted — re-reported every frame, so a save round-trips without them.
    #[serde(skip, default = "unbounded_surface")]
    surface: Pos,
    /// **Pinned** rectangles `(top-left, size)` — the fixed felt fixtures (the centered zone title, the
    /// Back card) that freely-placed content must settle clear of. In [`separate`] they take top priority:
    /// placed first, so nothing overrides them; they never move for a card. Fed by the renderer each frame.
    #[serde(skip)]
    pinned: Vec<(Pos, Pos)>,
}

/// The starting (effectively unbounded) surface used until the renderer reports the real size — also the
/// value a deserialized [`Tableau`] gets, since `surface` is not persisted.
fn unbounded_surface() -> Pos {
    Pos { x: 1.0e6, y: 1.0e6 }
}

impl Default for Tableau {
    fn default() -> Self {
        Self::new()
    }
}

impl Tableau {
    /// A new tree with a single, open (fanned) root pile and nothing in it.
    pub fn new() -> Self {
        let root = PileId(0);
        let mut piles = HashMap::new();
        piles.insert(
            root,
            Pile {
                id: root,
                label: "table".to_string(),
                collapsed: false,
                parent: None,
                children: Vec::new(),
                pos: Pos::default(),
                size: Pos::default(),
                layout: Layout::default(),
                projection: Vec::new(),
                reflects: None,
            },
        );
        Self {
            piles,
            cards: HashMap::new(),
            root,
            focus: root,
            selection: Vec::new(),
            next_pile: 1,
            next_card: 0,
            // Effectively unbounded until the renderer reports the real table size, so piles aren't
            // jammed to the origin before the first layout.
            surface: unbounded_surface(),
            pinned: Vec::new(),
        }
    }

    // --- construction -----------------------------------------------------------------------

    /// Adds an empty pile under `parent`. New piles start collapsed (the unattended default).
    pub fn add_pile(
        &mut self,
        parent: PileId,
        label: impl Into<String>,
    ) -> Result<PileId, TableauError> {
        if !self.piles.contains_key(&parent) {
            return Err(TableauError::UnknownPile(parent));
        }
        let id = PileId(self.next_pile);
        self.next_pile += 1;
        self.piles.insert(
            id,
            Pile {
                id,
                label: label.into(),
                collapsed: true,
                parent: Some(parent),
                children: Vec::new(),
                pos: Pos::default(),
                size: Pos::default(),
                layout: Layout::default(),
                projection: Vec::new(),
                reflects: None,
            },
        );
        self.piles
            .get_mut(&parent)
            .expect("parent checked above")
            .children
            .push(Node::Pile(id));
        Ok(id)
    }

    /// Adds a card on top of `pile`.
    pub fn add_card(
        &mut self,
        pile: PileId,
        face: Face,
        actionable: Option<usize>,
    ) -> Result<CardId, TableauError> {
        if !self.piles.contains_key(&pile) {
            return Err(TableauError::UnknownPile(pile));
        }
        let id = CardId(self.next_card);
        self.next_card += 1;
        self.cards.insert(
            id,
            Card {
                id,
                face,
                actionable,
                detail: Vec::new(),
                panel: Vec::new(),
                kind: CardKind::Regular,
                card_type: String::new(),
                size: Size::Small,
                home: pile,
                pos: Pos::default(),
                footprint: Pos::default(),
                recipe: None,
                quantity: 1,
                pair_key: None,
                pairings: Vec::new(),
            },
        );
        self.piles
            .get_mut(&pile)
            .expect("pile checked above")
            .children
            .push(Node::Card(id));
        Ok(id)
    }

    // --- queries ----------------------------------------------------------------------------

    /// The root pile's id.
    pub fn root_id(&self) -> PileId {
        self.root
    }

    /// The currently focused (fanned, picked-up) pile.
    pub fn focus_id(&self) -> PileId {
        self.focus
    }

    /// The pile with this id, if any.
    pub fn pile(&self, id: PileId) -> Option<&Pile> {
        self.piles.get(&id)
    }

    /// The card with this id, if any.
    pub fn card(&self, id: CardId) -> Option<&Card> {
        self.cards.get(&id)
    }

    /// Total number of **physical** cards — the **sum of stack quantities** (spec `physical-cards.md`
    /// PC.2: conservation is on this sum, so a `6 ×12` bank counts as 12). Splitting or merging a stack
    /// preserves it. See [`stack_count`](Self::stack_count) for the number of distinct stacks.
    pub fn card_count(&self) -> usize {
        self.cards.values().map(|c| c.quantity() as usize).sum()
    }

    /// The number of distinct card **stacks** (a `6 ×12` bank is one stack) — as opposed to
    /// [`card_count`](Self::card_count), the physical total.
    pub fn stack_count(&self) -> usize {
        self.cards.len()
    }

    /// The selected cards, in selection order.
    pub fn selection(&self) -> &[CardId] {
        &self.selection
    }

    /// Whether `card` is currently selected.
    pub fn is_selected(&self, card: CardId) -> bool {
        self.selection.contains(&card)
    }

    // --- selection --------------------------------------------------------------------------

    /// Selects `card` (idempotent — selecting an already-selected card is a no-op).
    pub fn select(&mut self, card: CardId) -> Result<(), TableauError> {
        if !self.cards.contains_key(&card) {
            return Err(TableauError::UnknownCard(card));
        }
        if !self.selection.contains(&card) {
            self.selection.push(card);
        }
        Ok(())
    }

    /// Removes `card` from the selection (no-op if it was not selected).
    pub fn deselect(&mut self, card: CardId) {
        self.selection.retain(|c| *c != card);
    }

    /// Clears the entire selection.
    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    // --- reordering & moving ----------------------------------------------------------------

    /// Reorders a child within `pile` so the node at `from` ends up at index `to`. Indices are into the
    /// pile's [`children`](Pile::children) (cards and sub-piles alike); both must be in range, else
    /// [`TableauError::IndexOutOfRange`] and no change.
    pub fn reorder(&mut self, pile: PileId, from: usize, to: usize) -> Result<(), TableauError> {
        let d = self
            .piles
            .get_mut(&pile)
            .ok_or(TableauError::UnknownPile(pile))?;
        if from >= d.children.len() || to >= d.children.len() {
            return Err(TableauError::IndexOutOfRange);
        }
        let node = d.children.remove(from);
        d.children.insert(to, node);
        Ok(())
    }

    /// Moves `card` out of its current pile and into `to_deck` at index `at`. `at` may be anywhere
    /// from `0` to the destination's card count (an `at` past the end errors). Count is conserved and
    /// the card's [`home`](Card::home) is updated. On any error the tree is left unchanged.
    pub fn move_card(
        &mut self,
        card: CardId,
        to_deck: PileId,
        at: usize,
    ) -> Result<(), TableauError> {
        let from_deck = self
            .cards
            .get(&card)
            .ok_or(TableauError::UnknownCard(card))?
            .home;
        if !self.piles.contains_key(&to_deck) {
            return Err(TableauError::UnknownPile(to_deck));
        }
        if at > self.piles[&to_deck].children.len() {
            return Err(TableauError::IndexOutOfRange);
        }
        // Remove from source (home invariant guarantees it is present).
        let pos = self.piles[&from_deck]
            .children
            .iter()
            .position(|n| *n == Node::Card(card))
            .expect("home invariant: card is in its home pile");
        self.piles
            .get_mut(&from_deck)
            .expect("from exists")
            .children
            .remove(pos);
        // Insert into destination, clamping for the same-pile shift after removal.
        let dst = self.piles.get_mut(&to_deck).expect("to checked above");
        let at = at.min(dst.children.len());
        dst.children.insert(at, Node::Card(card));
        self.cards.get_mut(&card).expect("card checked above").home = to_deck;
        Ok(())
    }

    /// Removes `card` from its home pile and from the tableau entirely — it is discarded, not moved
    /// (e.g. a reusable kit's copy when its pairing is taken apart).
    pub fn remove_card(&mut self, card: CardId) -> Result<(), TableauError> {
        let home = self
            .cards
            .get(&card)
            .ok_or(TableauError::UnknownCard(card))?
            .home;
        if let Some(p) = self.piles.get_mut(&home) {
            p.children.retain(|&n| n != Node::Card(card));
        }
        self.cards.remove(&card);
        self.selection.retain(|&c| c != card);
        Ok(())
    }

    /// Moves a whole pile under `new_parent` at index `at` — re-parenting it, or reordering it when
    /// the parent is unchanged. Rejects moving the root, or moving a pile into itself or one of its own
    /// descendants (each would break the tree): [`TableauError::InvalidMove`], leaving the tree unchanged.
    pub fn move_pile(
        &mut self,
        pile: PileId,
        new_parent: PileId,
        at: usize,
    ) -> Result<(), TableauError> {
        if pile == self.root {
            return Err(TableauError::InvalidMove);
        }
        if !self.piles.contains_key(&pile) {
            return Err(TableauError::UnknownPile(pile));
        }
        if !self.piles.contains_key(&new_parent) {
            return Err(TableauError::UnknownPile(new_parent));
        }
        // Moving a pile onto itself or into one of its descendants would create a cycle.
        if self.is_ancestor_or_self(pile, new_parent) {
            return Err(TableauError::InvalidMove);
        }
        let old_parent = self.piles[&pile]
            .parent
            .expect("a non-root pile has a parent");
        let pos = self.piles[&old_parent]
            .children
            .iter()
            .position(|n| *n == Node::Pile(pile))
            .expect("child invariant: pile is in its parent's children");
        self.piles
            .get_mut(&old_parent)
            .expect("old parent exists")
            .children
            .remove(pos);
        let dest = self
            .piles
            .get_mut(&new_parent)
            .expect("new parent checked above");
        let at = at.min(dest.children.len());
        dest.children.insert(at, Node::Pile(pile));
        self.piles
            .get_mut(&pile)
            .expect("pile checked above")
            .parent = Some(new_parent);
        Ok(())
    }

    // --- focus / zoom -----------------------------------------------------------------------

    /// Focuses `pile`: it (and its ancestors, the path back to the root) are fanned open; every other
    /// pile collapses into a compact pile. This is "attend to one set, collapse the unattended".
    pub fn focus(&mut self, pile: PileId) -> Result<(), TableauError> {
        if !self.piles.contains_key(&pile) {
            return Err(TableauError::UnknownPile(pile));
        }
        self.focus = pile;
        self.apply_collapse();
        Ok(())
    }

    /// Zooms out one level — focus moves to the current focus's parent. A no-op at the root.
    pub fn zoom_out(&mut self) {
        if let Some(parent) = self.piles[&self.focus].parent {
            self.focus = parent;
            self.apply_collapse();
        }
    }

    /// Sets how `pile` arranges its contents and whether they can be reordered (see [`Layout`]).
    pub fn set_layout(&mut self, pile: PileId, layout: Layout) -> Result<(), TableauError> {
        self.piles
            .get_mut(&pile)
            .ok_or(TableauError::UnknownPile(pile))?
            .layout = layout;
        Ok(())
    }

    /// Makes `pile` a **projection** of `sources` — drilling into it shows those piles' cards (see
    /// [`projection_groups`](Self::projection_groups)) rather than its own. Pass an empty vec to clear.
    pub fn set_projection(
        &mut self,
        pile: PileId,
        sources: Vec<PileId>,
    ) -> Result<(), TableauError> {
        self.piles
            .get_mut(&pile)
            .ok_or(TableauError::UnknownPile(pile))?
            .projection = sources;
        Ok(())
    }

    /// The cards a projection `pile` shows, grouped by source: `(source pile, its content cards)` for
    /// each source in [`projection`](Pile::projection), skipping unknown sources. The cards keep their
    /// home (the source) — a projection displays them, it doesn't own them. Empty for a normal pile.
    pub fn projection_groups(&self, pile: PileId) -> Vec<(PileId, Vec<CardId>)> {
        let Some(p) = self.piles.get(&pile) else {
            return Vec::new();
        };
        p.projection
            .iter()
            .filter(|&&src| self.piles.contains_key(&src))
            .map(|&src| (src, self.content_cards(src).to_vec()))
            .collect()
    }

    /// The rows of an [`Arrangement::Rows`] pile: each `(header, cards)` where `header` is a
    /// [`Header`](CardKind::Header) card and `cards` are the cards on that row. The pile's header cards
    /// (in order) name the rows; each of the first *projected* rows shows a
    /// [`projection`](Pile::projection) source's cards, and the last header's row shows the pile's own
    /// non-header content. (For the inn: **Hero** → the Identity deck, **Kit** → the Kit deck, **Active**
    /// → the recruited pairs the inn owns.)
    pub fn row_groups(&self, pile: PileId) -> Vec<(CardId, Vec<CardId>)> {
        let Some(p) = self.piles.get(&pile) else {
            return Vec::new();
        };
        let headers: Vec<CardId> = p
            .cards()
            .into_iter()
            .filter(|c| {
                self.cards
                    .get(c)
                    .is_some_and(|k| k.kind == CardKind::Header)
            })
            .collect();
        let own: Vec<CardId> = p
            .cards()
            .into_iter()
            .filter(|c| {
                self.cards
                    .get(c)
                    .is_some_and(|k| k.kind != CardKind::Header)
            })
            .collect();
        let projected = self.projection_groups(pile);
        headers
            .iter()
            .enumerate()
            .map(|(i, &header)| {
                let cards = if i < projected.len() {
                    projected[i].1.clone()
                } else if i + 1 == headers.len() {
                    own.clone()
                } else {
                    Vec::new()
                };
                (header, cards)
            })
            .collect()
    }

    /// Draw **one** card named `name` out of `bank` onto the top of `dest`, returning the single card's
    /// id — a **split** of the bank stack (spec `physical-cards.md` PC.2): if the bank stack has quantity
    /// `>1` it is decremented and a fresh `×1` card is placed in `dest`; if it is the last one, the whole
    /// stack moves. Either way the **sum** of quantities is conserved. Errors
    /// [`BankEmpty`](TableauError::BankEmpty) if the bank holds no such card.
    fn draw_named_from(
        &mut self,
        bank: PileId,
        dest: PileId,
        name: &str,
    ) -> Result<CardId, TableauError> {
        let stack = self
            .content_cards(bank)
            .into_iter()
            .find(|&c| self.card(c).is_some_and(|k| k.name() == name))
            .ok_or_else(|| TableauError::BankEmpty(name.to_string()))?;
        let at = self.piles[&dest].children.len();
        if self.cards[&stack].quantity() <= 1 {
            self.move_card(stack, dest, at)?;
            return Ok(stack);
        }
        // Split one off the stack: decrement the bank, mint a `×1` twin in `dest` (sum unchanged).
        self.cards.get_mut(&stack).expect("stack").quantity -= 1;
        let face = self.cards[&stack].face.clone();
        let ctype = self.cards[&stack].card_type.clone();
        let detail = self.cards[&stack].detail.clone();
        let one = self.add_card(dest, face, None)?;
        self.set_card_type(one, ctype)?;
        if !detail.is_empty() {
            self.set_card_detail(one, detail)?;
        }
        Ok(one)
    }

    /// Return `card` to `bank` — a **merge** (spec `physical-cards.md` PC.2): if the bank already holds a
    /// stack of the same name, `card`'s quantity is folded into it and `card` is removed; otherwise `card`
    /// simply moves to the bank. The **sum** of quantities is conserved.
    fn return_one(&mut self, card: CardId, bank: PileId) -> Result<(), TableauError> {
        let name = self
            .card(card)
            .map(|c| c.name().to_string())
            .unwrap_or_default();
        let q = self.card(card).map(|c| c.quantity()).unwrap_or(1);
        let existing = self
            .content_cards(bank)
            .into_iter()
            .find(|&c| c != card && self.card(c).is_some_and(|k| k.name() == name));
        if let Some(stack) = existing {
            self.cards.get_mut(&stack).expect("stack").quantity += q;
            self.remove_card(card)?;
        } else {
            let at = self.piles[&bank].children.len();
            self.move_card(card, bank, at)?;
        }
        Ok(())
    }

    /// **Equip** `hero` with a kit `recipe` into a new character deck — conservation-clean (spec
    /// `physical-cards.md` PC.2): every card is **moved** from a bank, nothing minted. Bottom→top the deck
    /// spells each stat as a **stat-name card then a number card** (e.g. `Might` then `6`, drawn from
    /// `stats_bank` / `numbers_bank`), then the **ability** card (from `abilities_bank`), then the hero's
    /// **own identity card** moved on top as the deck's [`Zone`](CardKind::Zone) label (it leaves the
    /// Identity deck). The kit card is a reusable spec — it is **not** touched. Returns the character deck.
    #[allow(clippy::too_many_arguments)]
    pub fn equip_character(
        &mut self,
        hero_name: &str,
        recipe: &Recipe,
        heroes: PileId,
        stats_bank: PileId,
        numbers_bank: PileId,
        abilities_bank: PileId,
        home_location: PileId,
        progress: PileId,
    ) -> Result<PileId, TableauError> {
        let deck = self.add_pile(self.root, hero_name.to_string())?;
        // Assemble the stats + ability from the banks (conservation-clean, no mint).
        for (&(stat, _desc), &value) in crate::catalog::STATS.iter().zip(recipe.stats.iter()) {
            self.draw_named_from(stats_bank, deck, stat)?;
            self.draw_named_from(numbers_bank, deck, &value.to_string())?;
        }
        self.draw_named_from(abilities_bank, deck, &recipe.ability)?;
        // Deal the hero's four copies off the Heroes `×4` stack (`draw_named_from` splits one off each
        // time). Two go to the character deck: a **rank marker** (a Regular hero card that sits with the
        // character, to declare rank in combat — wired later), then the deck's **Zone label** on top (the
        // stats tracker + the pile's `reflects`). The Heroes stack empties after all four.
        self.draw_named_from(heroes, deck, hero_name)?; // rank marker
        let label = self.draw_named_from(heroes, deck, hero_name)?;
        self.set_card_kind(label, CardKind::Zone)?;
        self.set_layout(
            deck,
            Layout {
                arrangement: Arrangement::List,
                editable: true,
            },
        )?;
        self.set_pile_pos(deck, 40.0, 320.0)?;
        self.piles.get_mut(&deck).expect("just created").reflects = Some(label);
        // Station on the board: copy 3 stands at the home location (map position), copy 4 is a face-up
        // move marker on Progress (face-up = hasn't moved today). `draw_named_from` mints face-up copies.
        self.draw_named_from(heroes, home_location, hero_name)?;
        self.draw_named_from(heroes, progress, hero_name)?;
        Ok(deck)
    }

    /// **Un-equip** a character `deck` — the inverse of [`equip_character`], conservation-clean: return
    /// each borrowed stat/number/ability card to its bank (`return_one`, merged, PC.2), and gather the
    /// hero's **four copies** — the deck's Zone label, its rank marker, its map position, and its Progress
    /// move marker — back onto the Heroes stack (re-forming the `×4`). Removes the now-empty character-deck
    /// **pile**. The stationed copies are found across the table by name (PC.1).
    pub fn unequip_character(
        &mut self,
        deck: PileId,
        heroes: PileId,
        stats_bank: PileId,
        numbers_bank: PileId,
        abilities_bank: PileId,
    ) -> Result<(), TableauError> {
        let name = self
            .zone_card(deck)
            .and_then(|c| self.card(c))
            .map(|c| c.front_title().to_string())
            .unwrap_or_default();
        // The deck's Zone label (a hero copy) → back to the Heroes stack as a plain Regular card.
        if let Some(hero) = self.zone_card(deck) {
            self.set_card_kind(hero, CardKind::Regular)?;
            self.return_one(hero, heroes)?;
        }
        // The deck's content: stat/number/ability → banks; the rank-marker hero copy → the Heroes stack.
        for c in self.content_cards(deck) {
            match self.card(c).map(|k| k.card_type()) {
                Some("stat") => self.return_one(c, stats_bank)?,
                Some("number") => self.return_one(c, numbers_bank)?,
                Some("ability") => self.return_one(c, abilities_bank)?,
                _ => self.return_one(c, heroes)?, // the rank marker (a hero copy)
            }
        }
        self.remove_pile(deck)?;
        // The stationed copies (map position + Progress move marker), wherever they stand, → Heroes.
        let strays: Vec<CardId> = self
            .cards
            .iter()
            .filter(|(_, c)| {
                c.card_type() == "hero" && c.front_title() == name && c.home() != heroes
            })
            .map(|(&id, _)| id)
            .collect();
        for c in strays {
            self.flip_up(c)?;
            self.return_one(c, heroes)?;
        }
        Ok(())
    }

    /// **Instantiate** an encounter's foes as real cards in `arena`, split off the `bestiary` `×N` stacks —
    /// conservation-clean (spec `physical-cards.md` PC.2): each instance is *drawn* from its Bestiary stack
    /// (decrement + `×1` twin, via [`draw_named_from`](Self::draw_named_from)), nothing minted. Foes are
    /// **virtual at rest** — a location holds only an `encounter` header — so the roster comes from the
    /// catalog: `place_label` names the encounter and [`encounter_foes`](crate::catalog::encounter_foes)
    /// gives its `(creature, quantity)` list, so a `×2` keystone fields two cards. Returns the dealt foe
    /// card ids (in roster order). A place with no encounter (the inn) yields an empty vec. This is the
    /// combat-time counterpart to the at-rest Bestiary supply that manual combat draws its units from.
    pub fn instantiate_encounter_foes(
        &mut self,
        bestiary: PileId,
        arena: PileId,
        place_label: &str,
    ) -> Result<Vec<CardId>, TableauError> {
        let Some(encounter) = crate::catalog::encounter_for(place_label) else {
            return Ok(Vec::new());
        };
        let mut dealt = Vec::new();
        for (creature, qty) in crate::catalog::encounter_foes(encounter) {
            for _ in 0..qty {
                dealt.push(self.draw_named_from(bestiary, arena, creature.name)?);
            }
        }
        Ok(dealt)
    }

    /// **Return** instantiated foe `cards` to the `bestiary` — the inverse of
    /// [`instantiate_encounter_foes`](Self::instantiate_encounter_foes), conservation-clean: normalize each
    /// card (face-up, [`Regular`](CardKind::Regular)) then [`return_one`](Self::return_one) it, merging back
    /// into its `×N` Bestiary stack. Mirrors the teardown [`unequip_character`](Self::unequip_character)
    /// does for a hero's copies. A card that no longer exists (already discarded) is skipped.
    pub fn return_foes_to_bestiary(
        &mut self,
        cards: &[CardId],
        bestiary: PileId,
    ) -> Result<(), TableauError> {
        for &c in cards {
            if self.card(c).is_none() {
                continue;
            }
            self.flip_up(c)?;
            self.set_card_kind(c, CardKind::Regular)?;
            self.return_one(c, bestiary)?;
        }
        Ok(())
    }

    /// Recover a recruited character's [`Recipe`] from its `deck` — the inverse *read* of
    /// [`equip_character`]. The recipe is not stored on the deck, so this parses it back from the cards:
    /// pair each `"stat"` card with the `"number"` card that follows it (its value), placed by the stat's
    /// index in [`catalog::STATS`](crate::catalog::STATS), and take the `"ability"` card's name. Returns
    /// `None` unless the deck is a complete build (all five stat values + an ability). Used to turn a
    /// party member back into `[Might, Vitality, Toughness, Cadence, Finesse]` + ability for combat.
    pub fn character_recipe(&self, deck: PileId) -> Option<Recipe> {
        let mut stats = [0u8; 5];
        let mut seen = [false; 5];
        let mut ability: Option<String> = None;
        let mut pending: Option<usize> = None; // the stat card awaiting its number
        for c in self.content_cards(deck) {
            let card = self.card(c)?;
            match card.card_type() {
                "stat" => {
                    pending = crate::catalog::STATS
                        .iter()
                        .position(|&(n, _)| n == card.name());
                }
                "number" => {
                    if let Some(i) = pending.take() {
                        stats[i] = card.name().parse().ok()?;
                        seen[i] = true;
                    }
                }
                "ability" => ability = Some(card.name().to_string()),
                _ => {}
            }
        }
        if seen.iter().all(|&b| b) {
            ability.map(|ability| Recipe { stats, ability })
        } else {
            None
        }
    }

    /// Removes `pile` and everything under it — its cards, and recursively its sub-piles — and unlinks it
    /// from its parent. The root cannot be removed.
    pub fn remove_pile(&mut self, pile: PileId) -> Result<(), TableauError> {
        if pile == self.root {
            return Err(TableauError::UnknownPile(pile));
        }
        let p = self
            .piles
            .get(&pile)
            .ok_or(TableauError::UnknownPile(pile))?;
        let parent = p.parent;
        let subpiles = p.subpiles();
        let cards = p.cards();
        for sub in subpiles {
            self.remove_pile(sub)?;
        }
        for card in cards {
            self.cards.remove(&card);
            self.selection.retain(|&c| c != card);
        }
        if let Some(parent) = parent
            && let Some(pp) = self.piles.get_mut(&parent)
        {
            pp.children.retain(|&n| n != Node::Pile(pile));
        }
        self.piles.remove(&pile);
        Ok(())
    }

    /// Move a character's **map position** copy to `to_location` and spend its move: flip its Progress
    /// move marker face-down ([`mark_moved`](Tableau::mark_moved)). Returns `true` when that was the
    /// **last** character still to move — the day is over, and the caller then
    /// [`advance_day`](Tableau::advance_day)s. Conservation-clean (a move + a flip).
    pub fn move_character(
        &mut self,
        position: CardId,
        to_location: PileId,
        progress: PileId,
    ) -> Result<bool, TableauError> {
        let name = self
            .card(position)
            .map(|c| c.front_title().to_string())
            .unwrap_or_default();
        self.move_card(position, to_location, 0)?;
        self.mark_moved(progress, &name)?;
        Ok(self.day_is_over(progress))
    }

    /// Sets `pile`'s collapsed flag directly (a manual fan/collapse, independent of focus).
    pub fn set_collapsed(&mut self, pile: PileId, collapsed: bool) -> Result<(), TableauError> {
        self.piles
            .get_mut(&pile)
            .ok_or(TableauError::UnknownPile(pile))?
            .collapsed = collapsed;
        Ok(())
    }

    /// Sets `pile`'s position on the table surface (drag-to-place commits here).
    pub fn set_pile_pos(&mut self, pile: PileId, x: f32, y: f32) -> Result<(), TableauError> {
        self.piles
            .get_mut(&pile)
            .ok_or(TableauError::UnknownPile(pile))?
            .pos = Pos { x, y };
        Ok(())
    }

    /// Records `pile`'s rendered size (the renderer feeds this back after layout, for [`separate`]).
    pub fn set_pile_size(&mut self, pile: PileId, w: f32, h: f32) -> Result<(), TableauError> {
        self.piles
            .get_mut(&pile)
            .ok_or(TableauError::UnknownPile(pile))?
            .size = Pos { x: w, y: h };
        Ok(())
    }

    // --- card content & render size ---------------------------------------------------------

    /// Sets a card's [`Size::Medium`] body lines.
    pub fn set_card_detail(
        &mut self,
        card: CardId,
        lines: Vec<String>,
    ) -> Result<(), TableauError> {
        self.cards
            .get_mut(&card)
            .ok_or(TableauError::UnknownCard(card))?
            .detail = lines;
        Ok(())
    }

    /// Sets a card's [`Size::Large`] panel lines (e.g. a log).
    pub fn set_card_panel(&mut self, card: CardId, lines: Vec<String>) -> Result<(), TableauError> {
        self.cards
            .get_mut(&card)
            .ok_or(TableauError::UnknownCard(card))?
            .panel = lines;
        Ok(())
    }

    /// Marks a card as a **pairing target** with `key` (something can be paired onto it).
    pub fn set_card_pair_key(&mut self, card: CardId, key: u32) -> Result<(), TableauError> {
        self.cards
            .get_mut(&card)
            .ok_or(TableauError::UnknownCard(card))?
            .pair_key = Some(key);
        Ok(())
    }

    /// Sets a card's **pairings** — `(target pair_key, actionable index)` entries.
    pub fn set_card_pairings(
        &mut self,
        card: CardId,
        pairings: Vec<(u32, usize)>,
    ) -> Result<(), TableauError> {
        self.cards
            .get_mut(&card)
            .ok_or(TableauError::UnknownCard(card))?
            .pairings = pairings;
        Ok(())
    }

    /// Sets what a card *is* (its click meaning).
    pub fn set_card_kind(&mut self, card: CardId, kind: CardKind) -> Result<(), TableauError> {
        self.cards
            .get_mut(&card)
            .ok_or(TableauError::UnknownCard(card))?
            .kind = kind;
        Ok(())
    }

    /// Sets a card's [`type`](Card::card_type) — a short category shown on the card (e.g. "Location").
    pub fn set_card_type(
        &mut self,
        card: CardId,
        card_type: impl Into<String>,
    ) -> Result<(), TableauError> {
        self.cards
            .get_mut(&card)
            .ok_or(TableauError::UnknownCard(card))?
            .card_type = card_type.into();
        Ok(())
    }

    /// Sets a card's free position on the drilled-in surface (an [`Arrangement::Free`] deck).
    pub fn set_card_pos(&mut self, card: CardId, x: f32, y: f32) -> Result<(), TableauError> {
        self.cards
            .get_mut(&card)
            .ok_or(TableauError::UnknownCard(card))?
            .pos = Pos { x, y };
        Ok(())
    }

    /// Records a card's rendered footprint (the renderer feeds this back after layout, for shoving).
    pub fn set_card_footprint(&mut self, card: CardId, w: f32, h: f32) -> Result<(), TableauError> {
        self.cards
            .get_mut(&card)
            .ok_or(TableauError::UnknownCard(card))?
            .footprint = Pos { x: w, y: h };
        Ok(())
    }

    /// Sets a card's [`recipe`](Card::recipe) — the [`Recipe`] it yields when combined (a kit).
    pub fn set_card_recipe(&mut self, card: CardId, recipe: Recipe) -> Result<(), TableauError> {
        self.cards
            .get_mut(&card)
            .ok_or(TableauError::UnknownCard(card))?
            .recipe = Some(recipe);
        Ok(())
    }

    /// Sets a card's [`quantity`](Card::quantity) — how many identical physical cards this one stack
    /// stands for (spec `physical-cards.md` PC.2: a `6 ×12` bank). Used at **setup** to deal a stack;
    /// in play, quantities change only by the split/merge inside a move. Floored at 1.
    pub fn set_card_quantity(&mut self, card: CardId, quantity: u32) -> Result<(), TableauError> {
        self.cards
            .get_mut(&card)
            .ok_or(TableauError::UnknownCard(card))?
            .quantity = quantity.max(1);
        Ok(())
    }

    /// Sets a card's [`Face`] outright — the one post-creation mutation of facing (spec
    /// `physical-cards.md` PC.2). Prefer [`flip_down`](Tableau::flip_down) / [`flip_up`](Tableau::flip_up),
    /// which preserve the front title so the flip is reversible; this is the raw setter.
    pub fn set_face(&mut self, card: CardId, face: Face) -> Result<(), TableauError> {
        self.cards
            .get_mut(&card)
            .ok_or(TableauError::UnknownCard(card))?
            .face = face;
        Ok(())
    }

    /// Flip a card **face down**, remembering its front so [`flip_up`](Tableau::flip_up) restores it
    /// (PC.2 — damage landing on a Health card, a day-clock copy spent by a move). A no-op if already
    /// down.
    pub fn flip_down(&mut self, card: CardId) -> Result<(), TableauError> {
        let c = self
            .cards
            .get_mut(&card)
            .ok_or(TableauError::UnknownCard(card))?;
        if let Face::Up { title } = &c.face {
            c.face = Face::Down {
                title: title.clone(),
            };
        }
        Ok(())
    }

    /// Flip a card **face up**, restoring the front it remembered while down (PC.2 — Recover on a Health
    /// card, the day clock re-standing every copy at day's end). A no-op if already up.
    pub fn flip_up(&mut self, card: CardId) -> Result<(), TableauError> {
        let c = self
            .cards
            .get_mut(&card)
            .ok_or(TableauError::UnknownCard(card))?;
        if let Face::Down { title } = &c.face {
            c.face = Face::Up {
                title: title.clone(),
            };
        }
        Ok(())
    }

    // --- The day clock (spec `physical-cards.md` PC.5) ------------------------------------------------
    // **Progress** is the day clock: a `Day Passed ×N` **count** stack (type `event`; its quantity is the
    // current day) plus one face-up `hero` **move marker** per active character (face-up = hasn't moved
    // today). A move flips a marker down; once every marker is down the day is over and
    // [`advance_day`](Tableau::advance_day) stands them back up and grows the count by one. Count and
    // markers are told apart by card type. All conservation-safe: only flips and one move.

    /// Whether card `c` on the Progress deck is a **hero move marker** (as opposed to the `event` count).
    fn is_move_marker(&self, c: CardId) -> bool {
        self.card(c).is_some_and(|k| k.card_type() == "hero")
    }

    /// Spend a character's move — flip the **face-up** hero marker on `progress` whose front title is
    /// `character` face-down ("has moved today"). A no-op if none matches (already down, or not tracked).
    pub fn mark_moved(&mut self, progress: PileId, character: &str) -> Result<(), TableauError> {
        let target = self.content_cards(progress).into_iter().find(|&c| {
            self.is_move_marker(c)
                && self
                    .card(c)
                    .is_some_and(|card| !card.is_face_down() && card.front_title() == character)
        });
        if let Some(c) = target {
            self.flip_down(c)?;
        }
        Ok(())
    }

    /// Is the day over — has **every** hero move marker on `progress` been spent (face-down)? No markers
    /// (no active party) is never "over".
    pub fn day_is_over(&self, progress: PileId) -> bool {
        let markers: Vec<CardId> = self
            .content_cards(progress)
            .into_iter()
            .filter(|&c| self.is_move_marker(c))
            .collect();
        !markers.is_empty()
            && markers
                .iter()
                .all(|&c| self.card(c).is_some_and(|card| card.is_face_down()))
    }

    /// Advance to the next day: stand every hero move marker on `progress` back up (leaving the count
    /// alone), and draw **one** `Day Passed` card from `reserve` onto `progress` — merging into the count
    /// stack, so the day is that stack's **quantity** (PC.5, no number cap). A no-op on the draw if the
    /// reserve is spent (the game ran past its provisioned length — a bound to raise).
    pub fn advance_day(&mut self, progress: PileId, reserve: PileId) -> Result<(), TableauError> {
        for c in self.content_cards(progress) {
            if self.is_move_marker(c) {
                self.flip_up(c)?;
            }
        }
        // Draw one Day Passed off the reserve and merge it into the Progress count stack (split & merge, PC.2).
        if let Some(&stack) = self.content_cards(reserve).last() {
            let name = self
                .card(stack)
                .map(|c| c.name().to_string())
                .unwrap_or_default();
            let drawn = self.draw_named_from(reserve, progress, &name)?;
            let merged_into = self
                .content_cards(progress)
                .into_iter()
                .find(|&c| c != drawn && self.card(c).is_some_and(|k| k.name() == name));
            if let Some(existing) = merged_into {
                let q = self.card(drawn).map(|c| c.quantity()).unwrap_or(1);
                self.cards.get_mut(&existing).expect("stack").quantity += q;
                self.remove_card(drawn)?;
            }
        }
        Ok(())
    }

    /// The current day — the **sum of the `event` count quantities** on `progress` (PC.5: count the Day
    /// Passes cards; the hero move markers are not counted).
    pub fn current_day(&self, progress: PileId) -> usize {
        self.content_cards(progress)
            .iter()
            .filter(|&&c| self.card(c).is_some_and(|k| k.card_type() == "event"))
            .map(|&c| self.card(c).map_or(0, |k| k.quantity() as usize))
            .sum()
    }

    /// Advances a card to the next render size it has content for, wrapping back to [`Size::Small`]:
    /// `Name → Card` (if it has detail) `→ Full` (if it has a panel) `→ Name`. A name-only card stays
    /// at `Name`. This is the "click to grow, click again to shrink" cycle.
    pub fn cycle_card_size(&mut self, card: CardId) -> Result<(), TableauError> {
        let c = self
            .cards
            .get_mut(&card)
            .ok_or(TableauError::UnknownCard(card))?;
        let has_detail = !c.detail.is_empty();
        let has_panel = !c.panel.is_empty();
        c.size = match c.size {
            Size::Small if has_detail => Size::Medium,
            Size::Small if has_panel => Size::Large,
            Size::Medium if has_panel => Size::Large,
            _ => Size::Small,
        };
        Ok(())
    }

    /// Group a pile's cards into runs for the Name view: adjacent cards that are at [`Size::Small`] and
    /// of the same *type* (same face-up/down and name) collapse into one entry, returned as
    /// `(representative card, quantity)`. A card grown past `Name` is its own run of 1 (it renders
    /// individually), and it breaks any run around it. The renderer shows "Name ×N" when quantity > 1.
    pub fn name_runs(&self, pile: PileId) -> Vec<(CardId, usize)> {
        let Some(p) = self.piles.get(&pile) else {
            return Vec::new();
        };
        let mut runs: Vec<(CardId, usize)> = Vec::new();
        for cid in p.cards() {
            let card = &self.cards[&cid];
            // Each stack contributes its own quantity (PC.2 `×N`), so a `6 ×12` alone reads "6 ×12".
            let q = card.quantity() as usize;
            if card.size == Size::Small
                && let Some(&(prev, _)) = runs.last()
                && self.cards[&prev].size == Size::Small
                && same_type(&self.cards[&prev], card)
            {
                runs.last_mut().expect("just checked non-empty").1 += q;
            } else {
                runs.push((cid, q));
            }
        }
        runs
    }

    /// Records the table surface size (the renderer feeds this back after layout). Decks are kept
    /// within `0..=surface-size` — the borders act as walls. See [`place_pile`] and [`separate`].
    pub fn set_surface(&mut self, w: f32, h: f32) {
        self.surface = Pos { x: w, y: h };
    }

    /// Set the felt's **pinned** rectangles — the fixed fixtures (centered title, Back) that content is
    /// shoved clear of, highest priority. Ordered highest-priority-first; fed by the renderer each frame.
    pub fn set_pinned(&mut self, pinned: Vec<(Pos, Pos)>) {
        self.pinned = pinned;
    }

    /// The current table surface size (as last reported by the renderer). Used to lay cards out into a
    /// grid of as many columns as fit.
    pub fn surface(&self) -> Pos {
        self.surface
    }

    /// The index of a card within its home pile's [`children`](Pile::children) (0 = bottom), if it
    /// exists. Drives its grid cell — the same child ordering the renderer lays out.
    pub fn card_index(&self, card: CardId) -> Option<usize> {
        let home = self.cards.get(&card)?.home;
        self.piles[&home]
            .children
            .iter()
            .position(|n| *n == Node::Card(card))
    }

    /// The pile's **zone card** — the [`CardKind::Zone`] card that *names* the pile rather than being one
    /// of its contents — if it has one. Identified by **kind**, wherever it sits among the children (not by
    /// position), so a sub-pile added after it can't demote the label to content. The zone card is the
    /// pile's label: it titles the zone you drill into, and is neither counted nor shown among the contents.
    /// (The last such card wins, so an append-a-label idiom still names the pile.)
    pub fn zone_card(&self, pile: PileId) -> Option<CardId> {
        let p = self.piles.get(&pile)?;
        p.children
            .iter()
            .rev()
            .filter_map(|n| n.card())
            .find(|c| self.cards.get(c).is_some_and(|k| k.kind == CardKind::Zone))
    }

    /// The pile's **content** cards — every card except a trailing [`zone_card`](Self::zone_card).
    /// This is what drilling into the pile shows, and what its deck count reflects.
    pub fn content_cards(&self, pile: PileId) -> Vec<CardId> {
        let zone = self.zone_card(pile);
        match self.piles.get(&pile) {
            Some(p) => p
                .children
                .iter()
                .filter_map(|n| n.card())
                .filter(|&c| Some(c) != zone)
                .collect(),
            None => Vec::new(),
        }
    }

    /// The number of **physical** cards `pile` holds — the count you'd get holding the deck at a
    /// tabletop — counted recursively through its nested sub-piles and shown in a zone's `(N)` title
    /// prefix. Every card physically *here* counts once, by its [`quantity`](Card::quantity): the pile's
    /// own title (zone) card, row headers, and ordinary cards alike (a `Day Passed ×11` stack is 11).
    /// Two things don't count, because they aren't physical cards:
    /// - **Projected** cards keep their home in the deck the pile borrows from (the inn borrows Identity
    ///   and Kit) and are counted there, so no card is counted twice.
    /// - A **software-only deck** — one holding [`Utility`] action cards (the System deck's Start Over /
    ///   Exit are app controls, not cards you'd hold) — is skipped whole, its label included.
    ///
    /// So summing this over every top-level deck on the table yields the game's real physical card total.
    pub fn physical_card_count(&self, pile: PileId) -> usize {
        let Some(p) = self.piles.get(&pile) else {
            return 0;
        };
        // A deck that holds Utility action cards is software-only (the System deck) — its whole self is
        // software, including its own label, so it contributes zero.
        let is_software_only = p
            .children
            .iter()
            .filter_map(|n| n.card().and_then(|c| self.cards.get(&c)))
            .any(|k| matches!(k.kind(), CardKind::Utility(_)));
        if is_software_only {
            return 0;
        }
        // Otherwise count each **physical** card (see [`Card::is_physical`]) — a Virtual readout (a combat
        // log) sitting among real cards is skipped, the same predicate that excludes a System control.
        p.children
            .iter()
            .map(|node| match node {
                Node::Card(c) => self
                    .cards
                    .get(c)
                    .filter(|k| k.is_physical())
                    .map_or(0, |k| k.quantity() as usize),
                Node::Pile(sub) => self.physical_card_count(*sub),
            })
            .sum()
    }

    /// Places `pile` at `(x, y)`, clamped inside the surface (the borders shove it back in), returning
    /// the position actually used. Drag-to-place calls this each move.
    pub fn place_pile(&mut self, pile: PileId, x: f32, y: f32) -> Result<Pos, TableauError> {
        let size = self
            .piles
            .get(&pile)
            .ok_or(TableauError::UnknownPile(pile))?
            .size;
        let pos = clamp_box(Pos { x, y }, size, self.surface);
        self.piles.get_mut(&pile).expect("checked above").pos = pos;
        Ok(pos)
    }

    /// The movable children of `pile` — every child *except* a trailing [`zone_card`](Self::zone_card),
    /// which is the pile's label, not a thing on the felt. This is the single set that [`separate`] shoves
    /// and the renderer lays out, so a nested pile and a card are first-class alike.
    pub fn movable_children(&self, pile: PileId) -> Vec<Node> {
        let zone = self.zone_card(pile).map(Node::Card);
        self.piles.get(&pile).map_or(Vec::new(), |p| {
            p.children
                .iter()
                .copied()
                .filter(|&n| Some(n) != zone)
                .collect()
        })
    }

    /// The box (`pos`, `size`) a node occupies on the felt: a card by its [`footprint`](Card::footprint),
    /// a pile by its rendered [`size`](Pile::size). `None` if the node is unknown.
    fn node_box(&self, node: Node) -> Option<(Pos, Pos)> {
        match node {
            Node::Card(c) => self.cards.get(&c).map(|k| (k.pos, k.footprint)),
            Node::Pile(p) => self.piles.get(&p).map(|d| (d.pos, d.size)),
        }
    }

    /// Sets a node's position — a card's [`pos`](Card::pos) or a pile's [`pos`](Pile::pos).
    fn set_node_pos(&mut self, node: Node, pos: Pos) {
        match node {
            Node::Card(c) => {
                if let Some(k) = self.cards.get_mut(&c) {
                    k.pos = pos;
                }
            }
            Node::Pile(p) => {
                if let Some(d) = self.piles.get_mut(&p) {
                    d.pos = pos;
                }
            }
        }
    }

    /// Pushes the overlapping **children of `pile`** apart so the felt reads as physical objects — cards
    /// and nested piles *alike*, since both are [`movable_children`](Self::movable_children). Priority order
    /// (see [`separate_boxes`]): the felt's [pinned](Self::set_pinned) fixtures first, then the dropped
    /// `anchor`, then the rest fanning outward — each settling clear of everyone above it, all kept inside
    /// the surface. No-op if the pile or anchor is unknown. This is the one shove — the root and any drilled
    /// zone are just piles whose children get separated.
    pub fn separate(&mut self, pile: PileId, anchor: Node) {
        let nodes = self.movable_children(pile);
        let Some(anchor_idx) = nodes.iter().position(|&n| n == anchor) else {
            return;
        };
        let boxes: Vec<(Pos, Pos)> = nodes
            .iter()
            .map(|&n| self.node_box(n).unwrap_or_default())
            .collect();
        let settled = separate_boxes(&boxes, anchor_idx, self.surface, &self.pinned);
        for (&n, pos) in nodes.iter().zip(settled) {
            self.set_node_pos(n, pos);
        }
    }

    /// Lay `pile`'s movable children out in a **left-to-right, wrapping row** with an exact constant `gap`
    /// between their bounding boxes and from the left / top (`top`) edges. Each child is placed by its own
    /// box — a pile's [`size`](Pile::size), a card's [`footprint`](Card::footprint) — so the gaps are
    /// identical even when widths differ (which fixed pitches can't guarantee). The tidy default layout.
    /// Children not yet sized (box width `< 1`) are skipped so they don't collapse the row before layout.
    pub fn arrange_row(&mut self, pile: PileId, gap: f32, top: f32) {
        let nodes = self.movable_children(pile);
        let (mut x, mut y, mut row_h) = (gap, top, 0.0_f32);
        for node in nodes {
            let (_, size) = self.node_box(node).unwrap_or_default();
            if size.x < 1.0 {
                continue; // not laid out yet
            }
            // Wrap to the next row when this child would run past the right edge.
            if x > gap && x + size.x + gap > self.surface.x {
                x = gap;
                y += row_h + gap;
                row_h = 0.0;
            }
            self.set_node_pos(node, Pos { x, y });
            x += size.x + gap;
            row_h = row_h.max(size.y);
        }
    }

    /// Footprint-aware layout positions for a **structured** deck's movable children, in child order — the
    /// size-aware replacement for a fixed grid, and the reason `List`/`Grid` decks reflow when a card
    /// grows. `List` flows left-to-right, wrapping at the surface width, each child one `gap` after the
    /// previous by its *actual* box; a row is as tall as its tallest child. `Grid { columns }` aligns
    /// **both** axes: a column is as wide as its widest child, a row as tall as its tallest, so a grown
    /// card pushes its whole column and row. Because it reads live sizes, re-running it (the renderer does,
    /// every frame) is itself the reflow — no separate shove. A child not yet laid out (box `< 1`) uses
    /// `default` so the first frame is sane. Empty for non-structured arrangements (`Free`/`Rows`
    /// place their contents their own way).
    pub fn structured_positions(
        &self,
        pile: PileId,
        gap: f32,
        top: f32,
        default: Pos,
    ) -> Vec<(Node, Pos)> {
        let nodes = self.movable_children(pile);
        if nodes.is_empty() {
            return Vec::new();
        }
        let size = |n: Node| -> Pos {
            let (_, s) = self.node_box(n).unwrap_or_default();
            if s.x < 1.0 || s.y < 1.0 { default } else { s }
        };
        match self.pile(pile).map(|p| p.layout().arrangement) {
            Some(Arrangement::List) => {
                let mut out = Vec::with_capacity(nodes.len());
                let (mut x, mut y, mut row_h) = (gap, top, 0.0_f32);
                for n in nodes {
                    let s = size(n);
                    if x > gap && x + s.x + gap > self.surface.x {
                        x = gap;
                        y += row_h + gap;
                        row_h = 0.0;
                    }
                    out.push((n, Pos { x, y }));
                    x += s.x + gap;
                    row_h = row_h.max(s.y);
                }
                out
            }
            Some(Arrangement::Grid { columns }) => {
                let cols = columns.max(1);
                let rows = nodes.len().div_ceil(cols);
                // Column widths and row heights are the max box in that column / row (so both axes align).
                let mut col_w = vec![0.0_f32; cols];
                let mut row_h = vec![0.0_f32; rows];
                for (i, &n) in nodes.iter().enumerate() {
                    let s = size(n);
                    col_w[i % cols] = col_w[i % cols].max(s.x);
                    row_h[i / cols] = row_h[i / cols].max(s.y);
                }
                let mut col_x = vec![gap; cols];
                for c in 1..cols {
                    col_x[c] = col_x[c - 1] + col_w[c - 1] + gap;
                }
                let mut row_y = vec![top; rows];
                for r in 1..rows {
                    row_y[r] = row_y[r - 1] + row_h[r - 1] + gap;
                }
                nodes
                    .iter()
                    .enumerate()
                    .map(|(i, &n)| {
                        (
                            n,
                            Pos {
                                x: col_x[i % cols],
                                y: row_y[i / cols],
                            },
                        )
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    fn apply_collapse(&mut self) {
        let focus = self.focus;
        let ids: Vec<PileId> = self.piles.keys().copied().collect();
        for id in ids {
            let on_path = self.is_ancestor_or_self(id, focus);
            self.piles.get_mut(&id).expect("id from keys").collapsed = !on_path;
        }
    }

    /// Whether `anc` is `node` or an ancestor of `node` (walking parents up from `node`).
    fn is_ancestor_or_self(&self, anc: PileId, node: PileId) -> bool {
        let mut cur = node;
        loop {
            if cur == anc {
                return true;
            }
            match self.piles[&cur].parent {
                Some(p) => cur = p,
                None => return false,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// root → hand, pile; with a couple of cards in each. Returns (tree, hand, pile, c0, c1).
    fn fixture() -> (Tableau, PileId, PileId, CardId, CardId) {
        let mut t = Tableau::new();
        let root = t.root_id();
        let hand = t.add_pile(root, "Hand").unwrap();
        let pile = t.add_pile(root, "Pile").unwrap();
        let c0 = t
            .add_card(
                hand,
                Face::Up {
                    title: "Knight".into(),
                },
                Some(2),
            )
            .unwrap();
        let c1 = t
            .add_card(
                hand,
                Face::Down {
                    title: "hidden".into(),
                },
                None,
            )
            .unwrap();
        (t, hand, pile, c0, c1)
    }

    #[test]
    fn new_tree_has_open_root_and_no_cards() {
        let t = Tableau::new();
        assert_eq!(t.focus_id(), t.root_id());
        assert!(!t.pile(t.root_id()).unwrap().collapsed);
        assert_eq!(t.card_count(), 0);
    }

    #[test]
    fn add_records_home_and_actionable() {
        let (t, hand, _deck, c0, c1) = fixture();
        assert_eq!(t.card_count(), 2);
        assert_eq!(t.card(c0).unwrap().home(), hand);
        assert!(t.card(c0).unwrap().is_actionable());
        assert_eq!(t.card(c0).unwrap().actionable, Some(2));
        assert!(!t.card(c1).unwrap().is_actionable());
        assert_eq!(t.pile(hand).unwrap().cards(), &[c0, c1]);
    }

    #[test]
    fn select_is_idempotent_and_reversible() {
        let (mut t, _hand, _deck, c0, _c1) = fixture();
        t.select(c0).unwrap();
        t.select(c0).unwrap();
        assert_eq!(t.selection(), &[c0]);
        assert!(t.is_selected(c0));
        t.deselect(c0);
        assert!(!t.is_selected(c0));
        t.select(c0).unwrap();
        t.clear_selection();
        assert!(t.selection().is_empty());
    }

    #[test]
    fn select_unknown_card_errors() {
        let mut t = Tableau::new();
        assert_eq!(
            t.select(CardId(999)),
            Err(TableauError::UnknownCard(CardId(999)))
        );
    }

    #[test]
    fn reorder_moves_within_deck() {
        let (mut t, hand, _deck, c0, c1) = fixture();
        t.reorder(hand, 0, 1).unwrap();
        assert_eq!(t.pile(hand).unwrap().cards(), &[c1, c0]);
        // no-op same index
        t.reorder(hand, 1, 1).unwrap();
        assert_eq!(t.pile(hand).unwrap().cards(), &[c1, c0]);
    }

    #[test]
    fn reorder_out_of_range_errors_and_leaves_order() {
        let (mut t, hand, _deck, c0, c1) = fixture();
        assert_eq!(t.reorder(hand, 0, 5), Err(TableauError::IndexOutOfRange));
        assert_eq!(t.pile(hand).unwrap().cards(), &[c0, c1]);
    }

    #[test]
    fn move_card_conserves_count_and_updates_home() {
        let (mut t, hand, pile, c0, c1) = fixture();
        t.move_card(c0, pile, 0).unwrap();
        assert_eq!(t.card_count(), 2);
        assert_eq!(t.pile(hand).unwrap().cards(), &[c1]);
        assert_eq!(t.pile(pile).unwrap().cards(), &[c0]);
        assert_eq!(t.card(c0).unwrap().home(), pile);
    }

    /// PC.2 — flipping is the reversible facing mutation: a face-down card stays anonymous in views
    /// (`name() == ""`) yet **remembers its front** (`front_title`), so `flip_up` restores it and the
    /// remembered front survives a RON round-trip. Flips are idempotent no-ops when already at the target.
    #[test]
    fn flip_is_reversible_and_remembers_its_front() {
        let mut t = Tableau::new();
        let root = t.root_id();
        let p = t.add_pile(root, "P").unwrap();
        let c = t
            .add_card(
                p,
                Face::Up {
                    title: "Vael".into(),
                },
                None,
            )
            .unwrap();
        assert!(!t.card(c).unwrap().is_face_down());
        assert_eq!(t.card(c).unwrap().name(), "Vael");

        t.flip_down(c).unwrap();
        assert!(t.card(c).unwrap().is_face_down());
        assert_eq!(
            t.card(c).unwrap().name(),
            "",
            "face-down is anonymous in views"
        );
        assert_eq!(
            t.card(c).unwrap().front_title(),
            "Vael",
            "but it remembers its front"
        );
        t.flip_down(c).unwrap(); // idempotent
        assert_eq!(t.card(c).unwrap().front_title(), "Vael");

        t.flip_up(c).unwrap();
        assert_eq!(
            t.card(c).unwrap().name(),
            "Vael",
            "flip_up restores the front"
        );
        t.flip_up(c).unwrap(); // idempotent
        assert_eq!(t.card(c).unwrap().name(), "Vael");

        // A face-down card round-trips through RON with its remembered front intact (packable, PC.3).
        t.flip_down(c).unwrap();
        let text = ron::to_string(&t).expect("serialize");
        let back: Tableau = ron::from_str(&text).expect("deserialize");
        assert!(back.card(c).unwrap().is_face_down());
        assert_eq!(back.card(c).unwrap().front_title(), "Vael");
    }

    /// PC.5 — the day clock: each character's copy flips down as it moves; when all are down the day is
    /// over; advancing stands them back up and grows the counted day track by one. And PC.2 — the total
    /// card count is **conserved** across every move and the advance (only flips + one draw, no mint).
    #[test]
    fn day_clock_advances_when_every_character_has_moved() {
        let mut t = Tableau::new();
        let root = t.root_id();
        // Progress: the day **count** (a `Day Passed` event card, quantity = the day) plus one face-up
        // `hero` move marker per active character (front = hero name).
        let progress = t.add_pile(root, "Progress").unwrap();
        let count = t
            .add_card(
                progress,
                Face::Up {
                    title: "Day Passed".into(),
                },
                None,
            )
            .unwrap();
        t.set_card_type(count, "event").unwrap();
        for h in ["Vael", "Bram", "Isolde"] {
            let c = t
                .add_card(progress, Face::Up { title: h.into() }, None)
                .unwrap();
            t.set_card_type(c, "hero").unwrap();
        }
        // A reserve of `Day Passed` cards to draw from as the days advance.
        let reserve = t.add_pile(root, "Events").unwrap();
        for _ in 0..5 {
            let e = t
                .add_card(
                    reserve,
                    Face::Up {
                        title: "Day Passed".into(),
                    },
                    None,
                )
                .unwrap();
            t.set_card_type(e, "event").unwrap();
        }
        let total = t.card_count(); // the closed deck after setup

        assert_eq!(t.current_day(progress), 1);
        assert!(!t.day_is_over(progress));

        t.mark_moved(progress, "Vael").unwrap();
        t.mark_moved(progress, "Bram").unwrap();
        assert!(!t.day_is_over(progress), "one character still to move");
        t.mark_moved(progress, "Isolde").unwrap();
        assert!(
            t.day_is_over(progress),
            "everyone has moved — the day is over"
        );

        t.advance_day(progress, reserve).unwrap();
        assert_eq!(
            t.current_day(progress),
            2,
            "the day is the count's quantity"
        );
        assert_eq!(
            t.content_cards(reserve).len(),
            4,
            "one Day Passed was drawn"
        );
        assert!(
            !t.day_is_over(progress),
            "a new day — everyone can move again"
        );
        assert!(
            t.content_cards(progress)
                .iter()
                .filter(|&&c| t.card(c).unwrap().card_type() == "hero")
                .all(|&c| !t.card(c).unwrap().is_face_down()),
            "every marker stood back up"
        );

        assert_eq!(
            t.card_count(),
            total,
            "conservation: no card minted or destroyed in play"
        );
    }

    /// PC.2 — equipping a character **assembles** it from the banks (a stat-name card then a number card
    /// per stat, then the ability, labelled by the hero's own identity card), **moving** cards, never
    /// minting; un-equipping returns every card to its bank and the hero to Identity. The card count is
    /// conserved throughout, and the kit spec is never consumed.
    #[test]
    fn equip_assembles_from_banks_and_unequip_returns_them_conserving_cards() {
        let mut t = Tableau::new();
        let root = t.root_id();
        let heroes = t.add_pile(root, "Heroes").unwrap();
        let hero = t
            .add_card(
                heroes,
                Face::Up {
                    title: "Vael".into(),
                },
                None,
            )
            .unwrap();
        t.set_card_type(hero, "hero").unwrap();
        t.set_card_quantity(hero, 4).unwrap(); // the four copies recruiting deals out
        // Where the stationed copies go.
        let home = t.add_pile(root, "Home").unwrap();
        let progress = t.add_pile(root, "Progress").unwrap();
        // Banks with many copies (PC.5): two of each stat name, four of each digit, one ability.
        let stats = t.add_pile(root, "Stats").unwrap();
        for (name, _) in crate::catalog::STATS {
            for _ in 0..2 {
                let c = t
                    .add_card(stats, Face::Up { title: name.into() }, None)
                    .unwrap();
                t.set_card_type(c, "stat").unwrap();
            }
        }
        let numbers = t.add_pile(root, "Numbers").unwrap();
        for d in 1..=9 {
            for _ in 0..4 {
                let c = t
                    .add_card(
                        numbers,
                        Face::Up {
                            title: d.to_string(),
                        },
                        None,
                    )
                    .unwrap();
                t.set_card_type(c, "number").unwrap();
            }
        }
        let abilities = t.add_pile(root, "Abilities").unwrap();
        let ab = t
            .add_card(
                abilities,
                Face::Up {
                    title: "Alpha Strike".into(),
                },
                None,
            )
            .unwrap();
        t.set_card_type(ab, "ability").unwrap();

        let total = t.card_count();
        let recipe = Recipe {
            stats: [6, 3, 1, 1, 1],
            ability: "Alpha Strike".into(),
        };
        let deck = t
            .equip_character(
                "Vael", &recipe, heroes, stats, numbers, abilities, home, progress,
            )
            .unwrap();

        // The deck spells each stat as name+number, then the ability, then a hero **rank marker**; the
        // deck's Zone label is another hero copy.
        let content: Vec<String> = t
            .content_cards(deck)
            .iter()
            .map(|&c| t.card(c).unwrap().name().to_string())
            .collect();
        assert_eq!(
            content,
            [
                "Might",
                "6",
                "Vitality",
                "3",
                "Toughness",
                "1",
                "Cadence",
                "1",
                "Finesse",
                "1",
                "Alpha Strike",
                "Vael"
            ]
        );
        assert_eq!(t.card(t.zone_card(deck).unwrap()).unwrap().name(), "Vael");
        // The four copies were dealt: none left in Heroes; one at home; one on Progress.
        assert!(
            t.content_cards(heroes)
                .iter()
                .all(|&c| t.card(c).unwrap().name() != "Vael"),
            "the Heroes stack emptied"
        );
        assert_eq!(t.content_cards(home).len(), 1, "position copy at home");
        assert_eq!(
            t.content_cards(progress).len(),
            1,
            "move marker on Progress"
        );
        assert_eq!(
            t.card_count(),
            total,
            "assembly + dealing minted nothing (PC.2)"
        );

        // Un-equip: everything returns — cards to banks, the four hero copies re-form the ×4 Heroes stack.
        t.unequip_character(deck, heroes, stats, numbers, abilities)
            .unwrap();
        assert!(t.pile(deck).is_none(), "character deck removed");
        let restacked = t
            .content_cards(heroes)
            .iter()
            .find(|&&c| t.card(c).unwrap().name() == "Vael")
            .map(|&c| t.card(c).unwrap().quantity());
        assert_eq!(restacked, Some(4), "four copies merged back to ×4");
        assert!(t.content_cards(home).is_empty(), "position copy returned");
        assert!(t.content_cards(progress).is_empty(), "move marker returned");
        // The returned cards merged back into the banks — the physical totals are restored (5 names ×2 =
        // 10, one ability), even though they now sit as fewer, larger stacks.
        let bank_sum = |t: &Tableau, p: PileId| -> u32 {
            t.content_cards(p)
                .iter()
                .map(|&c| t.card(c).unwrap().quantity())
                .sum()
        };
        assert_eq!(bank_sum(&t, stats), 10, "stat-name bank restored (sum)");
        assert_eq!(bank_sum(&t, abilities), 1, "ability returned (sum)");
        assert_eq!(
            t.card_count(),
            total,
            "conservation across equip + un-equip"
        );
    }

    #[test]
    fn move_card_inserts_at_index() {
        let (mut t, hand, pile, c0, c1) = fixture();
        let c2 = t
            .add_card(
                pile,
                Face::Down {
                    title: "hidden".into(),
                },
                None,
            )
            .unwrap();
        let c3 = t
            .add_card(
                pile,
                Face::Down {
                    title: "hidden".into(),
                },
                None,
            )
            .unwrap();
        // move c0 from hand into pile between c2 and c3
        t.move_card(c0, pile, 1).unwrap();
        assert_eq!(t.pile(pile).unwrap().cards(), &[c2, c0, c3]);
        assert_eq!(t.pile(hand).unwrap().cards(), &[c1]);
    }

    #[test]
    fn move_card_errors_leave_tree_unchanged() {
        let (mut t, hand, pile, c0, c1) = fixture();
        assert_eq!(
            t.move_card(CardId(999), pile, 0),
            Err(TableauError::UnknownCard(CardId(999)))
        );
        assert_eq!(
            t.move_card(c0, PileId(999), 0),
            Err(TableauError::UnknownPile(PileId(999)))
        );
        assert_eq!(t.move_card(c0, pile, 9), Err(TableauError::IndexOutOfRange));
        // unchanged
        assert_eq!(t.pile(hand).unwrap().cards(), &[c0, c1]);
        assert_eq!(t.card(c0).unwrap().home(), hand);
    }

    #[test]
    fn move_pile_nests_under_a_new_parent() {
        let (mut t, hand, pile, _c0, _c1) = fixture();
        let root = t.root_id();
        // nest `hand` inside `pile`
        t.move_pile(hand, pile, 0).unwrap();
        assert_eq!(t.pile(hand).unwrap().parent(), Some(pile));
        assert!(t.pile(pile).unwrap().subpiles().contains(&hand));
        assert!(!t.pile(root).unwrap().subpiles().contains(&hand));
    }

    #[test]
    fn move_pile_reorders_among_siblings() {
        let (mut t, hand, pile, _c0, _c1) = fixture();
        let root = t.root_id();
        // root.subpiles starts as [hand, pile]; move `pile` to the front (same parent = reorder).
        assert_eq!(t.pile(root).unwrap().subpiles(), &[hand, pile]);
        t.move_pile(pile, root, 0).unwrap();
        assert_eq!(t.pile(root).unwrap().subpiles(), &[pile, hand]);
    }

    #[test]
    fn move_pile_rejects_cycles_and_root() {
        let (mut t, hand, pile, _c0, _c1) = fixture();
        let root = t.root_id();
        let nested = t.add_pile(hand, "Nested").unwrap();
        // into itself, into a descendant, and moving the root: all rejected, tree unchanged.
        assert_eq!(t.move_pile(hand, hand, 0), Err(TableauError::InvalidMove));
        assert_eq!(t.move_pile(hand, nested, 0), Err(TableauError::InvalidMove));
        assert_eq!(t.move_pile(root, pile, 0), Err(TableauError::InvalidMove));
        assert_eq!(t.pile(hand).unwrap().parent(), Some(root));
        assert_eq!(t.pile(nested).unwrap().parent(), Some(hand));
    }

    /// Pinned fixtures outrank even the dropped anchor: a pile dropped onto the (title) pinned rect is
    /// nudged clear of it, and with no pinned it keeps the spot it was dropped at.
    #[test]
    fn separate_pins_take_priority_over_the_dropped_anchor() {
        let mut t = Tableau::new();
        t.set_surface(1000.0, 1000.0);
        let root = t.root_id();
        let a = t.add_pile(root, "A").unwrap();
        t.set_pile_size(a, 100.0, 100.0).unwrap();
        t.set_pile_pos(a, 200.0, 40.0).unwrap();
        // A pinned fixture (the centered title) right where the pile was dropped.
        t.set_pinned(vec![(Pos { x: 180.0, y: 20.0 }, Pos { x: 160.0, y: 60.0 })]);

        t.separate(root, Node::Pile(a));
        let p = t.pile(a).unwrap().pos();
        let ox = (p.x + 100.0).min(180.0 + 160.0) - p.x.max(180.0);
        let oy = (p.y + 100.0).min(20.0 + 60.0) - p.y.max(20.0);
        assert!(
            ox <= 0.02 || oy <= 0.02,
            "the dropped pile must yield to the pinned fixture, got {p:?}"
        );

        // No pinned → the dropped pile keeps its spot (nothing outranks it).
        t.set_pinned(vec![]);
        t.set_pile_pos(a, 200.0, 40.0).unwrap();
        t.separate(root, Node::Pile(a));
        assert_eq!(t.pile(a).unwrap().pos(), Pos { x: 200.0, y: 40.0 });
    }

    /// Two 100×100 piles overlapping; the anchor stays, the other slides clear on the x axis.
    #[test]
    fn separate_keeps_anchor_and_clears_overlap() {
        let mut t = Tableau::new();
        let root = t.root_id();
        let a = t.add_pile(root, "A").unwrap();
        let b = t.add_pile(root, "B").unwrap();
        for d in [a, b] {
            t.set_pile_size(d, 100.0, 100.0).unwrap();
        }
        t.set_pile_pos(a, 0.0, 0.0).unwrap();
        t.set_pile_pos(b, 20.0, 0.0).unwrap(); // overlaps a by 80px on x

        t.separate(t.root_id(), Node::Pile(a));

        assert_eq!(t.pile(a).unwrap().pos(), Pos { x: 0.0, y: 0.0 }); // anchor unmoved
        let bp = t.pile(b).unwrap().pos();
        assert!(bp.x >= 100.0 - 0.02, "b should clear a on x, got {bp:?}");
        assert!(
            (bp.y - 0.0).abs() < 0.02,
            "b should not drift on y, got {bp:?}"
        );
    }

    /// `arrange_row` lays movable children in a left-to-right row with the exact constant gap from the
    /// left/top edges and between every adjacent box — even when widths differ.
    #[test]
    fn arrange_row_places_children_at_an_exact_constant_gap() {
        let mut t = Tableau::new();
        let root = t.root_id();
        t.set_surface(1000.0, 800.0);
        let a = t.add_pile(root, "A").unwrap();
        let b = t.add_pile(root, "B").unwrap();
        let c = t.add_pile(root, "C").unwrap();
        t.set_pile_size(a, 100.0, 60.0).unwrap();
        t.set_pile_size(b, 140.0, 60.0).unwrap(); // wider than the others
        t.set_pile_size(c, 100.0, 60.0).unwrap();

        t.arrange_row(root, 12.0, 52.0);

        // First box: one gap in from the left, at the given top.
        assert_eq!(t.pile(a).unwrap().pos(), Pos { x: 12.0, y: 52.0 });
        // Next box: previous left + previous width + gap = 12 + 100 + 12 = 124 (exact, despite widths).
        assert_eq!(t.pile(b).unwrap().pos(), Pos { x: 124.0, y: 52.0 });
        // And past the *wider* B: 124 + 140 + 12 = 276.
        assert_eq!(t.pile(c).unwrap().pos(), Pos { x: 276.0, y: 52.0 });
    }

    /// `arrange_row` wraps to a new row (one gap below the tallest box) when a child would run past the edge.
    #[test]
    fn arrange_row_wraps_at_the_surface_edge() {
        let mut t = Tableau::new();
        let root = t.root_id();
        t.set_surface(300.0, 800.0); // only room for two 100-wide boxes per row
        let ids: Vec<_> = (0..3)
            .map(|i| {
                let p = t.add_pile(root, format!("D{i}")).unwrap();
                t.set_pile_size(p, 100.0, 60.0).unwrap();
                p
            })
            .collect();

        t.arrange_row(root, 12.0, 52.0);

        assert_eq!(t.pile(ids[0]).unwrap().pos(), Pos { x: 12.0, y: 52.0 });
        assert_eq!(t.pile(ids[1]).unwrap().pos(), Pos { x: 124.0, y: 52.0 });
        // Third wraps: back to the left gap, one row down (52 + 60 + 12 = 124).
        assert_eq!(t.pile(ids[2]).unwrap().pos(), Pos { x: 12.0, y: 124.0 });
    }

    /// `structured_positions` lays a List out **footprint-aware**: a wider (e.g. grown) card shifts the
    /// cards after it, so expanding a card reflows the deck instead of overlapping.
    #[test]
    fn structured_positions_list_flows_by_footprint() {
        let mut t = Tableau::new();
        let deck = t.add_pile(t.root_id(), "D").unwrap();
        t.set_surface(1000.0, 800.0);
        let mk = |t: &mut Tableau, name: &str, w: f32| {
            let c = t
                .add_card(deck, Face::Up { title: name.into() }, None)
                .unwrap();
            t.set_card_footprint(c, w, 60.0).unwrap();
            c
        };
        let (a, b, c) = (
            mk(&mut t, "a", 100.0),
            mk(&mut t, "b", 200.0),
            mk(&mut t, "c", 100.0),
        );
        t.set_layout(
            deck,
            Layout {
                arrangement: Arrangement::List,
                editable: true,
            },
        )
        .unwrap();

        let pos: std::collections::HashMap<_, _> = t
            .structured_positions(deck, 10.0, 10.0, Pos { x: 100.0, y: 100.0 })
            .into_iter()
            .collect();
        assert_eq!(pos[&Node::Card(a)], Pos { x: 10.0, y: 10.0 }); // first, at the gap
        assert_eq!(pos[&Node::Card(b)], Pos { x: 120.0, y: 10.0 }); // after a: 10 + 100 + 10
        assert_eq!(pos[&Node::Card(c)], Pos { x: 330.0, y: 10.0 }); // after the WIDE b: 120 + 200 + 10
    }

    /// `structured_positions` aligns a Grid on **both** axes: a wider card widens its whole column.
    #[test]
    fn structured_positions_grid_aligns_columns() {
        let mut t = Tableau::new();
        let deck = t.add_pile(t.root_id(), "G").unwrap();
        t.set_surface(1000.0, 800.0);
        let ids: Vec<_> = (0..4)
            .map(|i| {
                let c = t
                    .add_card(
                        deck,
                        Face::Up {
                            title: format!("{i}"),
                        },
                        None,
                    )
                    .unwrap();
                t.set_card_footprint(c, 100.0, 60.0).unwrap();
                c
            })
            .collect();
        t.set_card_footprint(ids[2], 180.0, 60.0).unwrap(); // bottom-left card widens column 0
        t.set_layout(
            deck,
            Layout {
                arrangement: Arrangement::Grid { columns: 2 },
                editable: false,
            },
        )
        .unwrap();

        let pos: std::collections::HashMap<_, _> = t
            .structured_positions(deck, 10.0, 10.0, Pos { x: 100.0, y: 100.0 })
            .into_iter()
            .collect();
        // 2 cols [0 1 / 2 3]: col0 width = max(100,180) = 180, so col1 starts at 10 + 180 + 10 = 200.
        assert_eq!(pos[&Node::Card(ids[0])], Pos { x: 10.0, y: 10.0 });
        assert_eq!(pos[&Node::Card(ids[1])], Pos { x: 200.0, y: 10.0 }); // pushed right by the wide col 0
        assert_eq!(pos[&Node::Card(ids[2])], Pos { x: 10.0, y: 80.0 }); // row 1: 10 + 60 + 10
        assert_eq!(pos[&Node::Card(ids[3])], Pos { x: 200.0, y: 80.0 });
    }

    /// Three stacked piles: the anchor holds and the chain pushes the rest apart (no pair overlaps).
    #[test]
    fn separate_resolves_chain_reactions() {
        let mut t = Tableau::new();
        let root = t.root_id();
        let ids: Vec<_> = (0..3)
            .map(|i| t.add_pile(root, format!("D{i}")).unwrap())
            .collect();
        for &d in &ids {
            t.set_pile_size(d, 100.0, 100.0).unwrap();
            t.set_pile_pos(d, 0.0, 0.0).unwrap(); // all stacked at the origin
        }
        let anchor = ids[0];
        t.separate(t.root_id(), Node::Pile(anchor));

        assert_eq!(t.pile(anchor).unwrap().pos(), Pos { x: 0.0, y: 0.0 });
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let p = t.pile(ids[i]).unwrap().pos();
                let q = t.pile(ids[j]).unwrap().pos();
                let ox = (p.x + 100.0).min(q.x + 100.0) - p.x.max(q.x);
                let oy = (p.y + 100.0).min(q.y + 100.0) - p.y.max(q.y);
                assert!(
                    ox <= 0.02 || oy <= 0.02,
                    "piles {i},{j} still overlap: {p:?} {q:?}"
                );
            }
        }
    }

    /// place_pile clamps to the surface — the borders shove a pile back inside.
    #[test]
    fn place_pile_clamps_to_surface() {
        let mut t = Tableau::new();
        let root = t.root_id();
        let a = t.add_pile(root, "A").unwrap();
        t.set_pile_size(a, 100.0, 100.0).unwrap();
        t.set_surface(300.0, 200.0);

        assert_eq!(
            t.place_pile(a, 500.0, 500.0).unwrap(),
            Pos { x: 200.0, y: 100.0 }
        );
        assert_eq!(
            t.place_pile(a, -50.0, -50.0).unwrap(),
            Pos { x: 0.0, y: 0.0 }
        );
    }

    /// In a bounded surface, separate keeps every pile fully inside (residual overlap is tolerated).
    #[test]
    fn separate_keeps_decks_within_bounds() {
        let mut t = Tableau::new();
        let root = t.root_id();
        let ids: Vec<_> = (0..3)
            .map(|i| t.add_pile(root, format!("D{i}")).unwrap())
            .collect();
        for &d in &ids {
            t.set_pile_size(d, 100.0, 100.0).unwrap();
            t.set_pile_pos(d, 250.0, 0.0).unwrap(); // stacked near the right wall
        }
        t.set_surface(300.0, 200.0);
        t.separate(t.root_id(), Node::Pile(ids[0]));

        for &d in &ids {
            let p = t.pile(d).unwrap().pos();
            assert!(p.x >= -0.02 && p.x <= 200.02, "x out of bounds: {p:?}");
            assert!(p.y >= -0.02 && p.y <= 100.02, "y out of bounds: {p:?}");
        }
    }

    /// A pile wedged between the anchor and a wall can't go the "natural" way, so it must route around
    /// (here: downward) and end up clear and in bounds — not deadlocked against the wall.
    #[test]
    fn separate_routes_around_a_wall_instead_of_deadlocking() {
        let mut t = Tableau::new();
        let root = t.root_id();
        let a = t.add_pile(root, "A").unwrap(); // anchor, top-left corner
        let b = t.add_pile(root, "B").unwrap();
        t.set_pile_size(a, 100.0, 100.0).unwrap();
        t.set_pile_size(b, 100.0, 100.0).unwrap();
        t.set_surface(150.0, 300.0); // only 50px right of A — B (100 wide) can't slide right
        t.set_pile_pos(a, 0.0, 0.0).unwrap();
        t.set_pile_pos(b, 10.0, 0.0).unwrap(); // overlapping A, pinned near the right wall

        t.separate(t.root_id(), Node::Pile(a));

        assert_eq!(t.pile(a).unwrap().pos(), Pos { x: 0.0, y: 0.0 }); // anchor held
        let (ap, asz) = (t.pile(a).unwrap().pos(), t.pile(a).unwrap().size());
        let (bp, bsz) = (t.pile(b).unwrap().pos(), t.pile(b).unwrap().size());
        assert!(
            bp.x >= -0.02 && bp.x <= 50.02 && bp.y >= -0.02,
            "b out of bounds: {bp:?}"
        );
        let ox = (ap.x + asz.x).min(bp.x + bsz.x) - ap.x.max(bp.x);
        let oy = (ap.y + asz.y).min(bp.y + bsz.y) - ap.y.max(bp.y);
        assert!(
            ox <= 0.02 || oy <= 0.02,
            "a and b must not overlap: {ap:?} {bp:?}"
        );
    }

    /// Assert no two of `ids` overlap (each 100×100). Shared by the dense-pack cases.
    fn assert_no_overlaps(t: &Tableau, ids: &[PileId]) {
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let p = t.pile(ids[i]).unwrap().pos();
                let q = t.pile(ids[j]).unwrap().pos();
                let ox = (p.x + 100.0).min(q.x + 100.0) - p.x.max(q.x);
                let oy = (p.y + 100.0).min(q.y + 100.0) - p.y.max(q.y);
                assert!(
                    ox <= 0.02 || oy <= 0.02,
                    "piles {i},{j} overlap: {p:?} {q:?}"
                );
            }
        }
    }

    /// Regression for the old pairwise relaxation: a dense stack that its 4-candidate, either-pile
    /// moves could leave overlapping within the pass cap. Lock-as-you-go must fully separate all six.
    #[test]
    fn separate_fully_clears_a_dense_stack() {
        let mut t = Tableau::new();
        let root = t.root_id();
        let ids: Vec<_> = (0..6)
            .map(|i| t.add_pile(root, format!("D{i}")).unwrap())
            .collect();
        for (i, &d) in ids.iter().enumerate() {
            t.set_pile_size(d, 100.0, 100.0).unwrap();
            // Tightly clustered so every pile overlaps several others.
            t.set_pile_pos(d, i as f32 * 12.0, i as f32 * 9.0).unwrap();
        }
        t.set_surface(1000.0, 1000.0);
        t.separate(t.root_id(), Node::Pile(ids[0]));

        assert_eq!(t.pile(ids[0]).unwrap().pos(), Pos { x: 0.0, y: 0.0 }); // anchor held
        assert_no_overlaps(&t, &ids);
    }

    /// Regression for the old terminal wall-clamp, which could shove a pile onto another with no pass
    /// left to fix it. Here the surface is exactly two piles wide/tall, so the clamp path is exercised;
    /// lock-as-you-go folds the walls into placement, so nothing overlaps after clamping.
    #[test]
    fn separate_no_overlap_after_wall_clamp() {
        let mut t = Tableau::new();
        let root = t.root_id();
        let ids: Vec<_> = (0..4)
            .map(|i| t.add_pile(root, format!("D{i}")).unwrap())
            .collect();
        for &d in &ids {
            t.set_pile_size(d, 100.0, 100.0).unwrap();
            t.set_pile_pos(d, 150.0, 150.0).unwrap(); // all stacked at the surface's center
        }
        t.set_surface(200.0, 200.0); // a 2×2 grid of 100×100 cells is the only clear packing
        t.separate(t.root_id(), Node::Pile(ids[0]));

        for &d in &ids {
            let p = t.pile(d).unwrap().pos();
            assert!(
                p.x >= -0.02 && p.x <= 100.02 && p.y >= -0.02 && p.y <= 100.02,
                "pile out of bounds: {p:?}"
            );
        }
        assert_no_overlaps(&t, &ids);
    }

    #[test]
    fn card_size_cycles_through_supported_sizes() {
        let mut t = Tableau::new();
        let root = t.root_id();
        let p = t.add_pile(root, "P").unwrap();
        let c = t
            .add_card(
                p,
                Face::Up {
                    title: "Knight".into(),
                },
                None,
            )
            .unwrap();

        // Name-only card: not expandable, cycling stays at Name.
        assert!(!t.card(c).unwrap().is_expandable());
        t.cycle_card_size(c).unwrap();
        assert_eq!(t.card(c).unwrap().size(), Size::Small);

        // With detail: Name -> Card -> Name.
        t.set_card_detail(c, vec!["Might 4".into(), "Vitality 6".into()])
            .unwrap();
        assert!(t.card(c).unwrap().is_expandable());
        t.cycle_card_size(c).unwrap();
        assert_eq!(t.card(c).unwrap().size(), Size::Medium);
        t.cycle_card_size(c).unwrap();
        assert_eq!(t.card(c).unwrap().size(), Size::Small);

        // With a panel too: Name -> Card -> Full -> Name.
        t.set_card_panel(c, vec!["round 1".into()]).unwrap();
        t.cycle_card_size(c).unwrap();
        assert_eq!(t.card(c).unwrap().size(), Size::Medium);
        t.cycle_card_size(c).unwrap();
        assert_eq!(t.card(c).unwrap().size(), Size::Large);
        t.cycle_card_size(c).unwrap();
        assert_eq!(t.card(c).unwrap().size(), Size::Small);
    }

    #[test]
    fn name_runs_group_adjacent_identical_name_cards() {
        let mut t = Tableau::new();
        let root = t.root_id();
        let p = t.add_pile(root, "Quiver").unwrap();
        let a1 = t
            .add_card(
                p,
                Face::Up {
                    title: "Arrow".into(),
                },
                None,
            )
            .unwrap();
        t.add_card(
            p,
            Face::Up {
                title: "Arrow".into(),
            },
            None,
        )
        .unwrap();
        t.add_card(
            p,
            Face::Up {
                title: "Arrow".into(),
            },
            None,
        )
        .unwrap();
        let bow = t
            .add_card(
                p,
                Face::Up {
                    title: "Bow".into(),
                },
                None,
            )
            .unwrap();
        let a4 = t
            .add_card(
                p,
                Face::Up {
                    title: "Arrow".into(),
                },
                None,
            )
            .unwrap();

        // [Arrow×3, Bow, Arrow]
        assert_eq!(t.name_runs(p), vec![(a1, 3), (bow, 1), (a4, 1)]);

        // Growing the first Arrow breaks its run; the two later Arrows still group.
        t.set_card_detail(a1, vec!["1 damage".into()]).unwrap();
        t.cycle_card_size(a1).unwrap(); // a1 now Size::Medium
        let runs = t.name_runs(p);
        assert_eq!(runs[0], (a1, 1)); // expanded card stands alone
        assert_eq!(runs.last().unwrap().1, 1); // trailing lone Arrow
    }

    #[test]
    fn focus_fans_path_and_collapses_the_rest() {
        let mut t = Tableau::new();
        let root = t.root_id();
        let a = t.add_pile(root, "A").unwrap();
        let b = t.add_pile(a, "B").unwrap();
        let sib = t.add_pile(root, "Sibling").unwrap();

        t.focus(b).unwrap();
        assert_eq!(t.focus_id(), b);
        assert!(!t.pile(b).unwrap().collapsed); // focus open
        assert!(!t.pile(a).unwrap().collapsed); // ancestor open
        assert!(!t.pile(root).unwrap().collapsed); // root open
        assert!(t.pile(sib).unwrap().collapsed); // unattended collapsed

        t.zoom_out(); // focus -> a
        assert_eq!(t.focus_id(), a);
        assert!(!t.pile(a).unwrap().collapsed);
        assert!(t.pile(b).unwrap().collapsed); // no longer on the path
    }

    #[test]
    fn layout_defaults_to_editable_list_and_can_be_set() {
        let mut t = Tableau::new();
        let root = t.root_id();
        let p = t.add_pile(root, "Locations").unwrap();
        // Piles start as editable 1-D lists (the original behaviour).
        assert_eq!(t.pile(p).unwrap().layout(), Layout::default());
        assert_eq!(t.pile(p).unwrap().layout().arrangement, Arrangement::List);
        assert!(t.pile(p).unwrap().layout().editable);

        // A fixed 2-D grid: three columns, not reorderable.
        let grid = Layout {
            arrangement: Arrangement::Grid { columns: 3 },
            editable: false,
        };
        t.set_layout(p, grid).unwrap();
        assert_eq!(t.pile(p).unwrap().layout(), grid);
        assert_eq!(
            t.set_layout(PileId(999), grid),
            Err(TableauError::UnknownPile(PileId(999)))
        );
    }

    #[test]
    fn separate_cards_clears_overlapping_cards_and_holds_anchor() {
        let mut t = Tableau::new();
        let root = t.root_id();
        let p = t.add_pile(root, "Scatter").unwrap();
        t.set_layout(
            p,
            Layout {
                arrangement: Arrangement::Free,
                editable: true,
            },
        )
        .unwrap();
        t.set_surface(1000.0, 1000.0);
        let a = t.add_card(p, Face::Up { title: "A".into() }, None).unwrap();
        let b = t.add_card(p, Face::Up { title: "B".into() }, None).unwrap();
        for c in [a, b] {
            t.set_card_footprint(c, 100.0, 100.0).unwrap();
        }
        t.set_card_pos(a, 0.0, 0.0).unwrap();
        t.set_card_pos(b, 20.0, 0.0).unwrap(); // overlaps a by 80px on x

        t.separate(p, Node::Card(a));

        assert_eq!(t.card(a).unwrap().pos(), Pos { x: 0.0, y: 0.0 }); // anchor held
        let bp = t.card(b).unwrap().pos();
        let ox = (0.0f32 + 100.0).min(bp.x + 100.0) - 0.0f32.max(bp.x);
        let oy = (0.0f32 + 100.0).min(bp.y + 100.0) - 0.0f32.max(bp.y);
        assert!(ox <= 0.02 || oy <= 0.02, "cards must not overlap: {bp:?}");
    }

    #[test]
    fn projection_shows_sources_and_equip_recruits() {
        let mut t = Tableau::new();
        let root = t.root_id();
        let identity = t.add_pile(root, "Identity").unwrap();
        let hero = t
            .add_card(
                identity,
                Face::Up {
                    title: "Vael".into(),
                },
                None,
            )
            .unwrap();
        t.set_card_type(hero, "hero").unwrap();
        t.set_card_quantity(hero, 4).unwrap(); // the four copies recruiting deals out
        let home = t.add_pile(root, "Home").unwrap();
        let progress = t.add_pile(root, "Progress").unwrap();
        let kits = t.add_pile(root, "Starting Kit").unwrap();
        let kit = t
            .add_card(
                kits,
                Face::Up {
                    title: "Skirmisher".into(),
                },
                None,
            )
            .unwrap();
        t.set_card_recipe(
            kit,
            Recipe {
                stats: [2, 3, 1, 2, 1],
                ability: "Jab".into(),
            },
        )
        .unwrap();
        // The inn projects both decks (a pure view — no copies).
        let inn = t.add_pile(root, "Inn").unwrap();
        t.set_projection(inn, vec![identity, kits]).unwrap();
        assert_eq!(
            t.projection_groups(inn),
            vec![(identity, vec![hero]), (kits, vec![kit])]
        );

        // Banks to assemble from, then recruit via `equip` (drag hero onto kit).
        let stats = t.add_pile(root, "Stats").unwrap();
        for (name, _) in crate::catalog::STATS {
            let c = t
                .add_card(stats, Face::Up { title: name.into() }, None)
                .unwrap();
            t.set_card_type(c, "stat").unwrap();
            t.set_card_quantity(c, 3).unwrap();
        }
        let numbers = t.add_pile(root, "Numbers").unwrap();
        for d in 1..=9 {
            let c = t
                .add_card(
                    numbers,
                    Face::Up {
                        title: d.to_string(),
                    },
                    None,
                )
                .unwrap();
            t.set_card_type(c, "number").unwrap();
            t.set_card_quantity(c, 5).unwrap();
        }
        let abilities = t.add_pile(root, "Abilities").unwrap();
        let ab = t
            .add_card(
                abilities,
                Face::Up {
                    title: "Jab".into(),
                },
                None,
            )
            .unwrap();
        t.set_card_type(ab, "ability").unwrap();

        let recipe = t.card(kit).unwrap().recipe().unwrap().clone();
        let character = t
            .equip_character(
                "Vael", &recipe, identity, stats, numbers, abilities, home, progress,
            )
            .unwrap();
        assert!(t.content_cards(identity).is_empty()); // the ×4 stack fully dealt out
        assert_eq!(
            t.card(t.zone_card(character).unwrap()).unwrap().name(),
            "Vael"
        );
        let content: Vec<String> = t
            .content_cards(character)
            .iter()
            .map(|&c| t.card(c).unwrap().name().to_string())
            .collect();
        // Each stat spelled name+number, then the ability, then the hero rank marker — assembled from the
        // banks + the Heroes stack (PC.2, no mint).
        assert_eq!(
            content,
            [
                "Might",
                "2",
                "Vitality",
                "3",
                "Toughness",
                "1",
                "Cadence",
                "2",
                "Finesse",
                "1",
                "Jab",
                "Vael"
            ]
        );
        assert_eq!(t.card(kit).unwrap().name(), "Skirmisher"); // kit spec untouched (reusable)
    }

    #[test]
    fn zone_card_names_the_pile_and_is_not_content() {
        let mut t = Tableau::new();
        let root = t.root_id();
        let p = t.add_pile(root, "Locations").unwrap();
        let a = t
            .add_card(
                p,
                Face::Up {
                    title: "Alpha".into(),
                },
                None,
            )
            .unwrap();
        let b = t
            .add_card(
                p,
                Face::Up {
                    title: "Bravo".into(),
                },
                None,
            )
            .unwrap();
        // No zone card yet: content is every card.
        assert_eq!(t.zone_card(p), None);
        assert_eq!(t.content_cards(p), &[a, b]);

        // Cap the pile with a Zone card: it is the zone card, and drops out of the content.
        let z = t
            .add_card(
                p,
                Face::Up {
                    title: "Location".into(),
                },
                None,
            )
            .unwrap();
        t.set_card_kind(z, CardKind::Zone).unwrap();
        assert_eq!(t.zone_card(p), Some(z));
        assert_eq!(t.content_cards(p), &[a, b]);
    }

    #[test]
    fn zoom_out_at_root_is_noop() {
        let mut t = Tableau::new();
        t.zoom_out();
        assert_eq!(t.focus_id(), t.root_id());
    }

    #[test]
    fn every_card_lives_in_exactly_one_deck() {
        let (t, _hand, _deck, _c0, _c1) = fixture();
        for (id, card) in [(_c0, t.card(_c0).unwrap()), (_c1, t.card(_c1).unwrap())] {
            let homes = t.piles.values().filter(|d| d.cards().contains(&id)).count();
            assert_eq!(homes, 1, "card must live in exactly one pile");
            assert!(t.pile(card.home()).unwrap().cards().contains(&id));
        }
    }

    /// Conservation guard: in the sample table every card is a **real member** of some pile — owned as a
    /// child, not merely shown. A projection (the inn) *displays* other decks' cards but never lists them
    /// among its own children, so a projected card is still counted at its true home. Thus no card can
    /// exist only as a projection with no real home.
    #[test]
    fn sample_table_has_no_projection_only_cards() {
        let t = crate::fixtures::sample_table();
        // Walk every pile and tally who owns each card (a pile owns exactly the Card nodes in its children).
        let mut owners: HashMap<CardId, usize> = HashMap::new();
        for pile in t.piles.values() {
            for cid in pile.cards() {
                *owners.entry(cid).or_default() += 1;
                // The owned card's recorded home points back to this very pile.
                assert_eq!(
                    t.cards[&cid].home, pile.id,
                    "card {cid:?} home must match its owning pile"
                );
            }
        }
        // Every card is owned exactly once...
        for (&id, &count) in &owners {
            assert_eq!(count, 1, "card {id:?} is owned by more than one pile");
        }
        // ...and every card **stack** in the tableau is owned somewhere — none is projection-only.
        assert_eq!(owners.len(), t.stack_count());
        for id in t.cards.keys() {
            assert!(
                owners.contains_key(id),
                "card {id:?} exists only as a projection, with no real (non-projection) home"
            );
        }
    }
}
