//! The pure pile/card model and its behaviors. Dependency-free: no `contract`, no Bevy.
//!
//! A [`Tableau`] is a tree of [`Pile`]s. Each pile holds an ordered list of cards and an ordered
//! list of child piles (so a pile can contain piles — the recursive zoom of the card-table UI). A
//! card always lives in exactly one pile (the "every card has a physical place" principle): operations
//! preserve that invariant or return a [`TableauError`] without mutating anything.

use std::collections::HashMap;

/// A stable handle to a card within a [`Tableau`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CardId(pub u64);

/// A stable handle to a pile within a [`Tableau`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PileId(pub u64);

/// A 2-D position on the table surface, in pixels from its top-left. The card-table is a physical
/// space, so a pile has a *place*; the renderer draws it there and drag-to-place updates it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Face {
    /// Face up, showing at least a title.
    Up { title: String },
    /// Face down (only the back is visible).
    Down,
}

/// How large a card is currently drawn. Default [`Size::Name`] — the smallest. A card grows only as
/// far as it has content for: [`Size::Card`] needs `detail`, [`Size::Full`] needs `panel`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Size {
    /// Just the name (plus a quantity when several of a kind stack). The default.
    #[default]
    Name,
    /// A full card face: name + detail lines (stats / rules).
    Card,
    /// A large utility panel with no physical-card counterpart (e.g. a combat log).
    Full,
}

/// What a card *is*, which decides what a single click on it means (the renderer reads this).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardKind {
    /// An ordinary card (may still carry `detail` / a `panel`).
    Regular,
    /// A card whose job is to name its pile — and the zone you enter by drilling into that pile.
    Zone,
    /// A user-interface card that performs an action when clicked.
    Utility(Utility),
}

/// The action a [`CardKind::Utility`] card performs when clicked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Utility {
    /// Go up one zone level.
    Back,
    /// Quit the application (desktop only).
    Exit,
}

/// A single card and its place in the tableau. Beyond its `face`, a card carries the content for the
/// larger render [`Size`]s (`detail`, `panel`) and a [`CardKind`] that gives a click its meaning.
#[derive(Clone, Debug, PartialEq, Eq)]
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
}

impl Card {
    /// The pile this card currently lives in.
    pub fn home(&self) -> PileId {
        self.home
    }

    /// Whether clicking this card performs a legal move.
    pub fn is_actionable(&self) -> bool {
        self.actionable.is_some()
    }

    /// The card's name — its face title, or empty when face down.
    pub fn name(&self) -> &str {
        match &self.face {
            Face::Up { title } => title,
            Face::Down => "",
        }
    }

    /// Body lines shown at [`Size::Card`].
    pub fn detail(&self) -> &[String] {
        &self.detail
    }

    /// Panel lines shown at [`Size::Full`].
    pub fn panel(&self) -> &[String] {
        &self.panel
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

    /// Whether the card has more than a name to show, so a click can grow it.
    pub fn is_expandable(&self) -> bool {
        !self.detail.is_empty() || !self.panel.is_empty()
    }
}

/// Whether two cards group as one entry in the Name view: same face (up/down), name, and type.
fn same_type(a: &Card, b: &Card) -> bool {
    matches!(a.face, Face::Down) == matches!(b.face, Face::Down)
        && a.name() == b.name()
        && a.card_type == b.card_type
}

/// A pile of cards (and, optionally, nested piles). Collapsed = shown as a compact, counted pile;
/// not collapsed = fanned out and attended to.
#[derive(Clone, Debug, PartialEq)]
pub struct Pile {
    /// This pile's stable id.
    pub id: PileId,
    /// A human-readable label (e.g. "Hand", "Vanguard").
    pub label: String,
    /// `true` = compact pile; `false` = fanned/attended.
    pub collapsed: bool,
    parent: Option<PileId>,
    cards: Vec<CardId>,
    subpiles: Vec<PileId>,
    pos: Pos,
    size: Pos,
}

impl Pile {
    /// This pile's parent, or `None` for the root.
    pub fn parent(&self) -> Option<PileId> {
        self.parent
    }

    /// The cards directly in this pile, in order (bottom to top).
    pub fn cards(&self) -> &[CardId] {
        &self.cards
    }

    /// The piles nested directly in this pile, in order.
    pub fn subpiles(&self) -> &[PileId] {
        &self.subpiles
    }

    /// This pile's position on the table surface.
    pub fn pos(&self) -> Pos {
        self.pos
    }

    /// This pile's rendered size (`x` = width, `y` = height), fed back by the renderer after layout.
    pub fn size(&self) -> Pos {
        self.size
    }
}

/// Why a pile/card operation could not be performed. Operations that return this leave the tree
/// unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
}

/// A tree of piles and cards, plus the current attention state (which pile is focused, which cards
/// are selected). All behaviors are methods here; the renderer reads the resulting state to draw.
#[derive(Clone, Debug)]
pub struct Tableau {
    piles: HashMap<PileId, Pile>,
    cards: HashMap<CardId, Card>,
    root: PileId,
    focus: PileId,
    selection: Vec<CardId>,
    next_pile: u64,
    next_card: u64,
    surface: Pos,
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
                cards: Vec::new(),
                subpiles: Vec::new(),
                pos: Pos::default(),
                size: Pos::default(),
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
            surface: Pos { x: 1.0e6, y: 1.0e6 },
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
                cards: Vec::new(),
                subpiles: Vec::new(),
                pos: Pos::default(),
                size: Pos::default(),
            },
        );
        self.piles
            .get_mut(&parent)
            .expect("parent checked above")
            .subpiles
            .push(id);
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
                size: Size::Name,
                home: pile,
            },
        );
        self.piles
            .get_mut(&pile)
            .expect("pile checked above")
            .cards
            .push(id);
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

    /// Total number of cards across every pile (conserved by [`move_card`](Self::move_card)).
    pub fn card_count(&self) -> usize {
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

    /// Reorders a card within `pile` so the card at `from` ends up at index `to`. Both indices must
    /// be in range for the pile's cards, else [`TableauError::IndexOutOfRange`] and no change.
    pub fn reorder(&mut self, pile: PileId, from: usize, to: usize) -> Result<(), TableauError> {
        let d = self
            .piles
            .get_mut(&pile)
            .ok_or(TableauError::UnknownPile(pile))?;
        if from >= d.cards.len() || to >= d.cards.len() {
            return Err(TableauError::IndexOutOfRange);
        }
        let id = d.cards.remove(from);
        d.cards.insert(to, id);
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
        if at > self.piles[&to_deck].cards.len() {
            return Err(TableauError::IndexOutOfRange);
        }
        // Remove from source (home invariant guarantees it is present).
        let pos = self.piles[&from_deck]
            .cards
            .iter()
            .position(|c| *c == card)
            .expect("home invariant: card is in its home pile");
        self.piles
            .get_mut(&from_deck)
            .expect("from exists")
            .cards
            .remove(pos);
        // Insert into destination, clamping for the same-pile shift after removal.
        let dst = self.piles.get_mut(&to_deck).expect("to checked above");
        let at = at.min(dst.cards.len());
        dst.cards.insert(at, card);
        self.cards.get_mut(&card).expect("card checked above").home = to_deck;
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
            .subpiles
            .iter()
            .position(|d| *d == pile)
            .expect("child invariant: pile is in its parent's subpiles");
        self.piles
            .get_mut(&old_parent)
            .expect("old parent exists")
            .subpiles
            .remove(pos);
        let dest = self
            .piles
            .get_mut(&new_parent)
            .expect("new parent checked above");
        let at = at.min(dest.subpiles.len());
        dest.subpiles.insert(at, pile);
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

    /// Sets a card's [`Size::Card`] body lines.
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

    /// Sets a card's [`Size::Full`] panel lines (e.g. a log).
    pub fn set_card_panel(&mut self, card: CardId, lines: Vec<String>) -> Result<(), TableauError> {
        self.cards
            .get_mut(&card)
            .ok_or(TableauError::UnknownCard(card))?
            .panel = lines;
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

    /// Advances a card to the next render size it has content for, wrapping back to [`Size::Name`]:
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
            Size::Name if has_detail => Size::Card,
            Size::Name if has_panel => Size::Full,
            Size::Card if has_panel => Size::Full,
            _ => Size::Name,
        };
        Ok(())
    }

    /// Group a pile's cards into runs for the Name view: adjacent cards that are at [`Size::Name`] and
    /// of the same *type* (same face-up/down and name) collapse into one entry, returned as
    /// `(representative card, quantity)`. A card grown past `Name` is its own run of 1 (it renders
    /// individually), and it breaks any run around it. The renderer shows "Name ×N" when quantity > 1.
    pub fn name_runs(&self, pile: PileId) -> Vec<(CardId, usize)> {
        let Some(p) = self.piles.get(&pile) else {
            return Vec::new();
        };
        let mut runs: Vec<(CardId, usize)> = Vec::new();
        for &cid in &p.cards {
            let card = &self.cards[&cid];
            if card.size == Size::Name
                && let Some(&(prev, _)) = runs.last()
                && self.cards[&prev].size == Size::Name
                && same_type(&self.cards[&prev], card)
            {
                runs.last_mut().expect("just checked non-empty").1 += 1;
            } else {
                runs.push((cid, 1));
            }
        }
        runs
    }

    /// Records the table surface size (the renderer feeds this back after layout). Decks are kept
    /// within `0..=surface-size` — the borders act as walls. See [`place_pile`] and [`separate`].
    pub fn set_surface(&mut self, w: f32, h: f32) {
        self.surface = Pos { x: w, y: h };
    }

    /// The current table surface size (as last reported by the renderer). Used to lay cards out into a
    /// grid of as many columns as fit.
    pub fn surface(&self) -> Pos {
        self.surface
    }

    /// The index of a card within its home pile (0 = bottom), if it exists. Drives its grid cell.
    pub fn card_index(&self, card: CardId) -> Option<usize> {
        let home = self.cards.get(&card)?.home;
        self.piles[&home].cards.iter().position(|c| *c == card)
    }

    /// Places `pile` at `(x, y)` but clamps it inside the surface (the borders shove it back in),
    /// returning the position actually used. Drag-to-place calls this each move.
    pub fn place_pile(&mut self, pile: PileId, x: f32, y: f32) -> Result<Pos, TableauError> {
        if !self.piles.contains_key(&pile) {
            return Err(TableauError::UnknownPile(pile));
        }
        self.piles.get_mut(&pile).expect("checked above").pos = Pos { x, y };
        self.clamp_within(pile);
        Ok(self.piles[&pile].pos)
    }

    /// Clamp one pile's position inside the surface (top-left at `0,0`, bottom-right at `surface`).
    /// Returns whether it had to move — that "had to move" is what makes a wall a participant in
    /// [`separate`]'s relaxation. A pile wider/taller than the surface is pinned to the top-left.
    fn clamp_within(&mut self, pile: PileId) -> bool {
        let surface = self.surface;
        let d = self.piles.get_mut(&pile).expect("pile id from subpiles");
        let max_x = (surface.x - d.size.x).max(0.0);
        let max_y = (surface.y - d.size.y).max(0.0);
        let nx = d.pos.x.clamp(0.0, max_x);
        let ny = d.pos.y.clamp(0.0, max_y);
        let changed = nx != d.pos.x || ny != d.pos.y;
        d.pos.x = nx;
        d.pos.y = ny;
        changed
    }

    /// Pushes overlapping **top-level** piles apart so the table reads as physical objects, keeping
    /// every pile inside the surface. Lock-as-you-go, outward from the `anchor` (the just-placed pile):
    ///
    /// 1. Pin the anchor at its in-bounds position and treat it as immovable.
    /// 2. Take the remaining piles nearest-first by center distance from the anchor (ties by id) — a
    ///    wavefront, so the drop reads as shoving neighbours outward in a cascade.
    /// 3. Place each one at the position nearest its own current spot that is fully inside the surface
    ///    **and** clear of every already-locked pile, then lock it and never move it again.
    ///
    /// Because each pile is placed clear of all locked piles and is never disturbed afterward, no two
    /// piles overlap once the space allows it — the guarantee the old pairwise relaxation lacked. When
    /// the table is genuinely too crowded (or a pile is larger than the surface), a pile settles at the
    /// in-bounds spot of least overlap instead of looping. Terminates in one placement per pile.
    pub fn separate(&mut self, anchor: PileId) {
        let mut ids = self.piles[&self.root].subpiles.clone();
        ids.retain(|&id| id != anchor);
        // Outward wavefront: nearest to the anchor settles first, ties broken by id for determinism.
        let anchor_center = self.center(anchor);
        ids.sort_by(|&p, &q| {
            let dp = self.center(p).dist_sq(anchor_center);
            let dq = self.center(q).dist_sq(anchor_center);
            dp.partial_cmp(&dq)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(p.cmp(&q))
        });

        // Lock the anchor at its in-bounds position; it is a fixed obstacle for everything else.
        self.clamp_within(anchor);
        let mut locked: Vec<PileId> = vec![anchor];

        for id in ids {
            let pos = self.place_clear_of(id, &locked);
            self.piles.get_mut(&id).expect("pile id from subpiles").pos = pos;
            locked.push(id);
        }
    }

    /// The center point of a pile's AABB — the metric [`separate`] orders its wavefront by.
    fn center(&self, pile: PileId) -> Pos {
        let d = &self.piles[&pile];
        Pos {
            x: d.pos.x + d.size.x * 0.5,
            y: d.pos.y + d.size.y * 0.5,
        }
    }

    /// The position nearest `pile`'s current spot that is fully inside the surface *and* clear of every
    /// pile in `locked`. If nothing is fully clear (the space is too crowded, or `pile` is larger than
    /// the surface), returns the in-bounds spot of least total overlap — best-effort degradation.
    ///
    /// The free region for an axis-aligned box among static boxes is rectilinear, so its nearest point
    /// lies on a candidate coordinate line: the pile's own coordinate, each locked pile's near/far edge
    /// in configuration space (the edge offset by this pile's size), or a wall. We test the grid of
    /// those `x`/`y` lines — which contains both the straight slides and the go-around-a-corner spots —
    /// and keep the clear one closest to where the pile already is.
    fn place_clear_of(&self, pile: PileId, locked: &[PileId]) -> Pos {
        let cur = self.piles[&pile].pos;
        let sz = self.piles[&pile].size;
        let surface = self.surface;
        let max_x = (surface.x - sz.x).max(0.0);
        let max_y = (surface.y - sz.y).max(0.0);
        let cx = cur.x.clamp(0.0, max_x);
        let cy = cur.y.clamp(0.0, max_y);

        // Candidate coordinate lines, kept in bounds. Start from the pile's own clamped position so
        // "already clear → stay put" falls out for free.
        let mut xs = vec![cx, 0.0, max_x];
        let mut ys = vec![cy, 0.0, max_y];
        for &l in locked {
            let lp = self.piles[&l].pos;
            let lsz = self.piles[&l].size;
            xs.push(lp.x - sz.x); // this pile just left of l
            xs.push(lp.x + lsz.x); // just right of l
            ys.push(lp.y - sz.y); // just above l
            ys.push(lp.y + lsz.y); // just below l
        }
        xs.retain(|&x| x >= 0.0 && x <= max_x);
        ys.retain(|&y| y >= 0.0 && y <= max_y);

        let mut best_clear: Option<(f32, Pos)> = None; // (dist², pos)
        let mut best_any: Option<(f32, f32, Pos)> = None; // (overlap area, dist², pos)
        for &x in &xs {
            for &y in &ys {
                let overlap = self.overlap_area(x, y, sz, locked);
                let dist_sq = (x - cur.x).powi(2) + (y - cur.y).powi(2);
                if overlap <= 0.0 {
                    if best_clear.is_none_or(|(d, _)| dist_sq < d) {
                        best_clear = Some((dist_sq, Pos { x, y }));
                    }
                } else if best_any
                    .is_none_or(|(o, d, _)| overlap < o || (overlap == o && dist_sq < d))
                {
                    best_any = Some((overlap, dist_sq, Pos { x, y }));
                }
            }
        }
        best_clear
            .map(|(_, p)| p)
            .or(best_any.map(|(_, _, p)| p))
            .unwrap_or(Pos { x: cx, y: cy })
    }

    /// Total area by which `pile` (size `sz`) at `(x, y)` overlaps the piles in `locked`.
    fn overlap_area(&self, x: f32, y: f32, sz: Pos, locked: &[PileId]) -> f32 {
        let mut total = 0.0;
        for &l in locked {
            let lp = self.piles[&l].pos;
            let lsz = self.piles[&l].size;
            let ox = (x + sz.x).min(lp.x + lsz.x) - x.max(lp.x);
            let oy = (y + sz.y).min(lp.y + lsz.y) - y.max(lp.y);
            if ox > 0.01 && oy > 0.01 {
                total += ox * oy;
            }
        }
        total
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
        let c1 = t.add_card(hand, Face::Down, None).unwrap();
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

    #[test]
    fn move_card_inserts_at_index() {
        let (mut t, hand, pile, c0, c1) = fixture();
        let c2 = t.add_card(pile, Face::Down, None).unwrap();
        let c3 = t.add_card(pile, Face::Down, None).unwrap();
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

        t.separate(a);

        assert_eq!(t.pile(a).unwrap().pos(), Pos { x: 0.0, y: 0.0 }); // anchor unmoved
        let bp = t.pile(b).unwrap().pos();
        assert!(bp.x >= 100.0 - 0.02, "b should clear a on x, got {bp:?}");
        assert!(
            (bp.y - 0.0).abs() < 0.02,
            "b should not drift on y, got {bp:?}"
        );
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
        t.separate(anchor);

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
        t.separate(ids[0]);

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

        t.separate(a);

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
        t.separate(ids[0]);

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
        t.separate(ids[0]);

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
        assert_eq!(t.card(c).unwrap().size(), Size::Name);

        // With detail: Name -> Card -> Name.
        t.set_card_detail(c, vec!["Might 4".into(), "Vitality 6".into()])
            .unwrap();
        assert!(t.card(c).unwrap().is_expandable());
        t.cycle_card_size(c).unwrap();
        assert_eq!(t.card(c).unwrap().size(), Size::Card);
        t.cycle_card_size(c).unwrap();
        assert_eq!(t.card(c).unwrap().size(), Size::Name);

        // With a panel too: Name -> Card -> Full -> Name.
        t.set_card_panel(c, vec!["round 1".into()]).unwrap();
        t.cycle_card_size(c).unwrap();
        assert_eq!(t.card(c).unwrap().size(), Size::Card);
        t.cycle_card_size(c).unwrap();
        assert_eq!(t.card(c).unwrap().size(), Size::Full);
        t.cycle_card_size(c).unwrap();
        assert_eq!(t.card(c).unwrap().size(), Size::Name);
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
        t.cycle_card_size(a1).unwrap(); // a1 now Size::Card
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
}
