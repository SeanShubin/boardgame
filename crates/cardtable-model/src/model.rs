//! The pure deck/card model and its behaviors. Dependency-free: no `contract`, no Bevy.
//!
//! A [`DeckTree`] is a tree of [`Deck`]s. Each deck holds an ordered list of cards and an ordered
//! list of child decks (so a deck can contain decks — the recursive zoom of the card-table UI). A
//! card always lives in exactly one deck (the "every card has a physical place" principle): operations
//! preserve that invariant or return a [`DeckError`] without mutating anything.

use std::collections::HashMap;

/// A stable handle to a card within a [`DeckTree`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CardId(pub u64);

/// A stable handle to a deck within a [`DeckTree`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DeckId(pub u64);

/// A 2-D position on the table surface, in pixels from its top-left. The card-table is a physical
/// space, so a deck has a *place*; the renderer draws it there and drag-to-place updates it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
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

/// A single card and its place in the tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Card {
    /// This card's stable id.
    pub id: CardId,
    /// What the card shows.
    pub face: Face,
    /// If `Some(i)`, this card maps to the legal action at index `i` in the source view — i.e. a
    /// click here is a legal move, not just a look. `None` means "viewable but not actionable".
    pub actionable: Option<usize>,
    home: DeckId,
}

impl Card {
    /// The deck this card currently lives in.
    pub fn home(&self) -> DeckId {
        self.home
    }

    /// Whether clicking this card performs a legal move.
    pub fn is_actionable(&self) -> bool {
        self.actionable.is_some()
    }
}

/// A pile of cards (and, optionally, nested decks). Collapsed = shown as a compact, counted pile;
/// not collapsed = fanned out and attended to.
#[derive(Clone, Debug, PartialEq)]
pub struct Deck {
    /// This deck's stable id.
    pub id: DeckId,
    /// A human-readable label (e.g. "Hand", "Vanguard").
    pub label: String,
    /// `true` = compact pile; `false` = fanned/attended.
    pub collapsed: bool,
    parent: Option<DeckId>,
    cards: Vec<CardId>,
    subdecks: Vec<DeckId>,
    pos: Pos,
    size: Pos,
}

impl Deck {
    /// This deck's parent, or `None` for the root.
    pub fn parent(&self) -> Option<DeckId> {
        self.parent
    }

    /// The cards directly in this deck, in order (bottom to top).
    pub fn cards(&self) -> &[CardId] {
        &self.cards
    }

    /// The decks nested directly in this deck, in order.
    pub fn subdecks(&self) -> &[DeckId] {
        &self.subdecks
    }

    /// This deck's position on the table surface.
    pub fn pos(&self) -> Pos {
        self.pos
    }

    /// This deck's rendered size (`x` = width, `y` = height), fed back by the renderer after layout.
    pub fn size(&self) -> Pos {
        self.size
    }
}

/// Why a deck/card operation could not be performed. Operations that return this leave the tree
/// unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeckError {
    /// No card with this id exists.
    UnknownCard(CardId),
    /// No deck with this id exists.
    UnknownDeck(DeckId),
    /// An index was outside the valid range for the operation.
    IndexOutOfRange,
    /// A deck move that would break the tree — moving the root, or moving a deck into itself or one
    /// of its own descendants (a cycle).
    InvalidMove,
}

/// A tree of decks and cards, plus the current attention state (which deck is focused, which cards
/// are selected). All behaviors are methods here; the renderer reads the resulting state to draw.
#[derive(Clone, Debug)]
pub struct DeckTree {
    decks: HashMap<DeckId, Deck>,
    cards: HashMap<CardId, Card>,
    root: DeckId,
    focus: DeckId,
    selection: Vec<CardId>,
    next_deck: u64,
    next_card: u64,
    surface: Pos,
}

impl Default for DeckTree {
    fn default() -> Self {
        Self::new()
    }
}

impl DeckTree {
    /// A new tree with a single, open (fanned) root deck and nothing in it.
    pub fn new() -> Self {
        let root = DeckId(0);
        let mut decks = HashMap::new();
        decks.insert(
            root,
            Deck {
                id: root,
                label: "table".to_string(),
                collapsed: false,
                parent: None,
                cards: Vec::new(),
                subdecks: Vec::new(),
                pos: Pos::default(),
                size: Pos::default(),
            },
        );
        Self {
            decks,
            cards: HashMap::new(),
            root,
            focus: root,
            selection: Vec::new(),
            next_deck: 1,
            next_card: 0,
            // Effectively unbounded until the renderer reports the real table size, so decks aren't
            // jammed to the origin before the first layout.
            surface: Pos { x: 1.0e6, y: 1.0e6 },
        }
    }

    // --- construction -----------------------------------------------------------------------

    /// Adds an empty deck under `parent`. New decks start collapsed (the unattended default).
    pub fn add_deck(
        &mut self,
        parent: DeckId,
        label: impl Into<String>,
    ) -> Result<DeckId, DeckError> {
        if !self.decks.contains_key(&parent) {
            return Err(DeckError::UnknownDeck(parent));
        }
        let id = DeckId(self.next_deck);
        self.next_deck += 1;
        self.decks.insert(
            id,
            Deck {
                id,
                label: label.into(),
                collapsed: true,
                parent: Some(parent),
                cards: Vec::new(),
                subdecks: Vec::new(),
                pos: Pos::default(),
                size: Pos::default(),
            },
        );
        self.decks
            .get_mut(&parent)
            .expect("parent checked above")
            .subdecks
            .push(id);
        Ok(id)
    }

    /// Adds a card on top of `deck`.
    pub fn add_card(
        &mut self,
        deck: DeckId,
        face: Face,
        actionable: Option<usize>,
    ) -> Result<CardId, DeckError> {
        if !self.decks.contains_key(&deck) {
            return Err(DeckError::UnknownDeck(deck));
        }
        let id = CardId(self.next_card);
        self.next_card += 1;
        self.cards.insert(
            id,
            Card {
                id,
                face,
                actionable,
                home: deck,
            },
        );
        self.decks
            .get_mut(&deck)
            .expect("deck checked above")
            .cards
            .push(id);
        Ok(id)
    }

    // --- queries ----------------------------------------------------------------------------

    /// The root deck's id.
    pub fn root_id(&self) -> DeckId {
        self.root
    }

    /// The currently focused (fanned, picked-up) deck.
    pub fn focus_id(&self) -> DeckId {
        self.focus
    }

    /// The deck with this id, if any.
    pub fn deck(&self, id: DeckId) -> Option<&Deck> {
        self.decks.get(&id)
    }

    /// The card with this id, if any.
    pub fn card(&self, id: CardId) -> Option<&Card> {
        self.cards.get(&id)
    }

    /// Total number of cards across every deck (conserved by [`move_card`](Self::move_card)).
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
    pub fn select(&mut self, card: CardId) -> Result<(), DeckError> {
        if !self.cards.contains_key(&card) {
            return Err(DeckError::UnknownCard(card));
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

    /// Reorders a card within `deck` so the card at `from` ends up at index `to`. Both indices must
    /// be in range for the deck's cards, else [`DeckError::IndexOutOfRange`] and no change.
    pub fn reorder(&mut self, deck: DeckId, from: usize, to: usize) -> Result<(), DeckError> {
        let d = self
            .decks
            .get_mut(&deck)
            .ok_or(DeckError::UnknownDeck(deck))?;
        if from >= d.cards.len() || to >= d.cards.len() {
            return Err(DeckError::IndexOutOfRange);
        }
        let id = d.cards.remove(from);
        d.cards.insert(to, id);
        Ok(())
    }

    /// Moves `card` out of its current deck and into `to_deck` at index `at`. `at` may be anywhere
    /// from `0` to the destination's card count (an `at` past the end errors). Count is conserved and
    /// the card's [`home`](Card::home) is updated. On any error the tree is left unchanged.
    pub fn move_card(&mut self, card: CardId, to_deck: DeckId, at: usize) -> Result<(), DeckError> {
        let from_deck = self
            .cards
            .get(&card)
            .ok_or(DeckError::UnknownCard(card))?
            .home;
        if !self.decks.contains_key(&to_deck) {
            return Err(DeckError::UnknownDeck(to_deck));
        }
        if at > self.decks[&to_deck].cards.len() {
            return Err(DeckError::IndexOutOfRange);
        }
        // Remove from source (home invariant guarantees it is present).
        let pos = self.decks[&from_deck]
            .cards
            .iter()
            .position(|c| *c == card)
            .expect("home invariant: card is in its home deck");
        self.decks
            .get_mut(&from_deck)
            .expect("from exists")
            .cards
            .remove(pos);
        // Insert into destination, clamping for the same-deck shift after removal.
        let dst = self.decks.get_mut(&to_deck).expect("to checked above");
        let at = at.min(dst.cards.len());
        dst.cards.insert(at, card);
        self.cards.get_mut(&card).expect("card checked above").home = to_deck;
        Ok(())
    }

    /// Moves a whole deck under `new_parent` at index `at` — re-parenting it, or reordering it when
    /// the parent is unchanged. Rejects moving the root, or moving a deck into itself or one of its own
    /// descendants (each would break the tree): [`DeckError::InvalidMove`], leaving the tree unchanged.
    pub fn move_deck(
        &mut self,
        deck: DeckId,
        new_parent: DeckId,
        at: usize,
    ) -> Result<(), DeckError> {
        if deck == self.root {
            return Err(DeckError::InvalidMove);
        }
        if !self.decks.contains_key(&deck) {
            return Err(DeckError::UnknownDeck(deck));
        }
        if !self.decks.contains_key(&new_parent) {
            return Err(DeckError::UnknownDeck(new_parent));
        }
        // Moving a deck onto itself or into one of its descendants would create a cycle.
        if self.is_ancestor_or_self(deck, new_parent) {
            return Err(DeckError::InvalidMove);
        }
        let old_parent = self.decks[&deck]
            .parent
            .expect("a non-root deck has a parent");
        let pos = self.decks[&old_parent]
            .subdecks
            .iter()
            .position(|d| *d == deck)
            .expect("child invariant: deck is in its parent's subdecks");
        self.decks
            .get_mut(&old_parent)
            .expect("old parent exists")
            .subdecks
            .remove(pos);
        let dest = self
            .decks
            .get_mut(&new_parent)
            .expect("new parent checked above");
        let at = at.min(dest.subdecks.len());
        dest.subdecks.insert(at, deck);
        self.decks
            .get_mut(&deck)
            .expect("deck checked above")
            .parent = Some(new_parent);
        Ok(())
    }

    // --- focus / zoom -----------------------------------------------------------------------

    /// Focuses `deck`: it (and its ancestors, the path back to the root) are fanned open; every other
    /// deck collapses into a compact pile. This is "attend to one set, collapse the unattended".
    pub fn focus(&mut self, deck: DeckId) -> Result<(), DeckError> {
        if !self.decks.contains_key(&deck) {
            return Err(DeckError::UnknownDeck(deck));
        }
        self.focus = deck;
        self.apply_collapse();
        Ok(())
    }

    /// Zooms out one level — focus moves to the current focus's parent. A no-op at the root.
    pub fn zoom_out(&mut self) {
        if let Some(parent) = self.decks[&self.focus].parent {
            self.focus = parent;
            self.apply_collapse();
        }
    }

    /// Sets `deck`'s collapsed flag directly (a manual fan/collapse, independent of focus).
    pub fn set_collapsed(&mut self, deck: DeckId, collapsed: bool) -> Result<(), DeckError> {
        self.decks
            .get_mut(&deck)
            .ok_or(DeckError::UnknownDeck(deck))?
            .collapsed = collapsed;
        Ok(())
    }

    /// Sets `deck`'s position on the table surface (drag-to-place commits here).
    pub fn set_deck_pos(&mut self, deck: DeckId, x: f32, y: f32) -> Result<(), DeckError> {
        self.decks
            .get_mut(&deck)
            .ok_or(DeckError::UnknownDeck(deck))?
            .pos = Pos { x, y };
        Ok(())
    }

    /// Records `deck`'s rendered size (the renderer feeds this back after layout, for [`separate`]).
    pub fn set_deck_size(&mut self, deck: DeckId, w: f32, h: f32) -> Result<(), DeckError> {
        self.decks
            .get_mut(&deck)
            .ok_or(DeckError::UnknownDeck(deck))?
            .size = Pos { x: w, y: h };
        Ok(())
    }

    /// Records the table surface size (the renderer feeds this back after layout). Decks are kept
    /// within `0..=surface-size` — the borders act as walls. See [`place_deck`] and [`separate`].
    pub fn set_surface(&mut self, w: f32, h: f32) {
        self.surface = Pos { x: w, y: h };
    }

    /// Places `deck` at `(x, y)` but clamps it inside the surface (the borders shove it back in),
    /// returning the position actually used. Drag-to-place calls this each move.
    pub fn place_deck(&mut self, deck: DeckId, x: f32, y: f32) -> Result<Pos, DeckError> {
        if !self.decks.contains_key(&deck) {
            return Err(DeckError::UnknownDeck(deck));
        }
        self.decks.get_mut(&deck).expect("checked above").pos = Pos { x, y };
        self.clamp_within(deck);
        Ok(self.decks[&deck].pos)
    }

    /// Clamp one deck's position inside the surface (top-left at `0,0`, bottom-right at `surface`).
    /// Returns whether it had to move — that "had to move" is what makes a wall a participant in
    /// [`separate`]'s relaxation. A deck wider/taller than the surface is pinned to the top-left.
    fn clamp_within(&mut self, deck: DeckId) -> bool {
        let surface = self.surface;
        let d = self.decks.get_mut(&deck).expect("deck id from subdecks");
        let max_x = (surface.x - d.size.x).max(0.0);
        let max_y = (surface.y - d.size.y).max(0.0);
        let nx = d.pos.x.clamp(0.0, max_x);
        let ny = d.pos.y.clamp(0.0, max_y);
        let changed = nx != d.pos.x || ny != d.pos.y;
        d.pos.x = nx;
        d.pos.y = ny;
        changed
    }

    /// Pushes overlapping **top-level** decks apart so the table reads as physical objects. `anchor`
    /// (the just-placed deck) stays put; every deck it overlaps slides away, cascading to resolve
    /// secondary overlaps (chain reactions). Iterative and capped, so a hopelessly crowded table
    /// settles with some residual overlap rather than looping forever. Uses each deck's `pos`/`size`.
    pub fn separate(&mut self, anchor: DeckId) {
        const MAX_PASSES: usize = 64;
        const EPS: f32 = 0.01;
        let mut ids = self.decks[&self.root].subdecks.clone();
        ids.sort(); // stable order → deterministic settling
        for _ in 0..MAX_PASSES {
            let mut moved = false;
            // The borders shove decks back inside first. A deck pinned against a wall can't yield, so
            // the next overlap pass pushes its *neighbour* instead — that's how the wall propagates.
            for &id in &ids {
                if self.clamp_within(id) {
                    moved = true;
                }
            }
            for i in 0..ids.len() {
                for j in (i + 1)..ids.len() {
                    let (a, b) = (ids[i], ids[j]);
                    let (ap, asz) = (self.decks[&a].pos, self.decks[&a].size);
                    let (bp, bsz) = (self.decks[&b].pos, self.decks[&b].size);
                    let overlap_x = (ap.x + asz.x).min(bp.x + bsz.x) - ap.x.max(bp.x);
                    let overlap_y = (ap.y + asz.y).min(bp.y + bsz.y) - ap.y.max(bp.y);
                    if overlap_x <= EPS || overlap_y <= EPS {
                        continue; // not actually overlapping
                    }
                    // Minimum-translation: separate along the axis of least overlap.
                    let (mut push_x, mut push_y) = (0.0, 0.0);
                    if overlap_x < overlap_y {
                        let dir = if bp.x + bsz.x / 2.0 >= ap.x + asz.x / 2.0 {
                            1.0
                        } else {
                            -1.0
                        };
                        push_x = dir * overlap_x;
                    } else {
                        let dir = if bp.y + bsz.y / 2.0 >= ap.y + asz.y / 2.0 {
                            1.0
                        } else {
                            -1.0
                        };
                        push_y = dir * overlap_y;
                    }
                    match (a == anchor, b == anchor) {
                        (true, true) => {} // distinct ids, unreachable
                        (true, false) => self.shift(b, push_x, push_y),
                        (false, true) => self.shift(a, -push_x, -push_y),
                        (false, false) => {
                            self.shift(a, -push_x / 2.0, -push_y / 2.0);
                            self.shift(b, push_x / 2.0, push_y / 2.0);
                        }
                    }
                    moved = true;
                }
            }
            if !moved {
                break;
            }
        }
        // If we stopped at the pass cap mid-push, make sure nothing ends up poking through a wall.
        for &id in &ids {
            self.clamp_within(id);
        }
    }

    fn shift(&mut self, deck: DeckId, dx: f32, dy: f32) {
        let pos = &mut self
            .decks
            .get_mut(&deck)
            .expect("deck id from subdecks")
            .pos;
        pos.x += dx;
        pos.y += dy;
    }

    fn apply_collapse(&mut self) {
        let focus = self.focus;
        let ids: Vec<DeckId> = self.decks.keys().copied().collect();
        for id in ids {
            let on_path = self.is_ancestor_or_self(id, focus);
            self.decks.get_mut(&id).expect("id from keys").collapsed = !on_path;
        }
    }

    /// Whether `anc` is `node` or an ancestor of `node` (walking parents up from `node`).
    fn is_ancestor_or_self(&self, anc: DeckId, node: DeckId) -> bool {
        let mut cur = node;
        loop {
            if cur == anc {
                return true;
            }
            match self.decks[&cur].parent {
                Some(p) => cur = p,
                None => return false,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// root → hand, deck; with a couple of cards in each. Returns (tree, hand, deck, c0, c1).
    fn fixture() -> (DeckTree, DeckId, DeckId, CardId, CardId) {
        let mut t = DeckTree::new();
        let root = t.root_id();
        let hand = t.add_deck(root, "Hand").unwrap();
        let deck = t.add_deck(root, "Deck").unwrap();
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
        (t, hand, deck, c0, c1)
    }

    #[test]
    fn new_tree_has_open_root_and_no_cards() {
        let t = DeckTree::new();
        assert_eq!(t.focus_id(), t.root_id());
        assert!(!t.deck(t.root_id()).unwrap().collapsed);
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
        assert_eq!(t.deck(hand).unwrap().cards(), &[c0, c1]);
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
        let mut t = DeckTree::new();
        assert_eq!(
            t.select(CardId(999)),
            Err(DeckError::UnknownCard(CardId(999)))
        );
    }

    #[test]
    fn reorder_moves_within_deck() {
        let (mut t, hand, _deck, c0, c1) = fixture();
        t.reorder(hand, 0, 1).unwrap();
        assert_eq!(t.deck(hand).unwrap().cards(), &[c1, c0]);
        // no-op same index
        t.reorder(hand, 1, 1).unwrap();
        assert_eq!(t.deck(hand).unwrap().cards(), &[c1, c0]);
    }

    #[test]
    fn reorder_out_of_range_errors_and_leaves_order() {
        let (mut t, hand, _deck, c0, c1) = fixture();
        assert_eq!(t.reorder(hand, 0, 5), Err(DeckError::IndexOutOfRange));
        assert_eq!(t.deck(hand).unwrap().cards(), &[c0, c1]);
    }

    #[test]
    fn move_card_conserves_count_and_updates_home() {
        let (mut t, hand, deck, c0, c1) = fixture();
        t.move_card(c0, deck, 0).unwrap();
        assert_eq!(t.card_count(), 2);
        assert_eq!(t.deck(hand).unwrap().cards(), &[c1]);
        assert_eq!(t.deck(deck).unwrap().cards(), &[c0]);
        assert_eq!(t.card(c0).unwrap().home(), deck);
    }

    #[test]
    fn move_card_inserts_at_index() {
        let (mut t, hand, deck, c0, c1) = fixture();
        let c2 = t.add_card(deck, Face::Down, None).unwrap();
        let c3 = t.add_card(deck, Face::Down, None).unwrap();
        // move c0 from hand into deck between c2 and c3
        t.move_card(c0, deck, 1).unwrap();
        assert_eq!(t.deck(deck).unwrap().cards(), &[c2, c0, c3]);
        assert_eq!(t.deck(hand).unwrap().cards(), &[c1]);
    }

    #[test]
    fn move_card_errors_leave_tree_unchanged() {
        let (mut t, hand, deck, c0, c1) = fixture();
        assert_eq!(
            t.move_card(CardId(999), deck, 0),
            Err(DeckError::UnknownCard(CardId(999)))
        );
        assert_eq!(
            t.move_card(c0, DeckId(999), 0),
            Err(DeckError::UnknownDeck(DeckId(999)))
        );
        assert_eq!(t.move_card(c0, deck, 9), Err(DeckError::IndexOutOfRange));
        // unchanged
        assert_eq!(t.deck(hand).unwrap().cards(), &[c0, c1]);
        assert_eq!(t.card(c0).unwrap().home(), hand);
    }

    #[test]
    fn move_deck_nests_under_a_new_parent() {
        let (mut t, hand, deck, _c0, _c1) = fixture();
        let root = t.root_id();
        // nest `hand` inside `deck`
        t.move_deck(hand, deck, 0).unwrap();
        assert_eq!(t.deck(hand).unwrap().parent(), Some(deck));
        assert!(t.deck(deck).unwrap().subdecks().contains(&hand));
        assert!(!t.deck(root).unwrap().subdecks().contains(&hand));
    }

    #[test]
    fn move_deck_reorders_among_siblings() {
        let (mut t, hand, deck, _c0, _c1) = fixture();
        let root = t.root_id();
        // root.subdecks starts as [hand, deck]; move `deck` to the front (same parent = reorder).
        assert_eq!(t.deck(root).unwrap().subdecks(), &[hand, deck]);
        t.move_deck(deck, root, 0).unwrap();
        assert_eq!(t.deck(root).unwrap().subdecks(), &[deck, hand]);
    }

    #[test]
    fn move_deck_rejects_cycles_and_root() {
        let (mut t, hand, deck, _c0, _c1) = fixture();
        let root = t.root_id();
        let nested = t.add_deck(hand, "Nested").unwrap();
        // into itself, into a descendant, and moving the root: all rejected, tree unchanged.
        assert_eq!(t.move_deck(hand, hand, 0), Err(DeckError::InvalidMove));
        assert_eq!(t.move_deck(hand, nested, 0), Err(DeckError::InvalidMove));
        assert_eq!(t.move_deck(root, deck, 0), Err(DeckError::InvalidMove));
        assert_eq!(t.deck(hand).unwrap().parent(), Some(root));
        assert_eq!(t.deck(nested).unwrap().parent(), Some(hand));
    }

    /// Two 100×100 decks overlapping; the anchor stays, the other slides clear on the x axis.
    #[test]
    fn separate_keeps_anchor_and_clears_overlap() {
        let mut t = DeckTree::new();
        let root = t.root_id();
        let a = t.add_deck(root, "A").unwrap();
        let b = t.add_deck(root, "B").unwrap();
        for d in [a, b] {
            t.set_deck_size(d, 100.0, 100.0).unwrap();
        }
        t.set_deck_pos(a, 0.0, 0.0).unwrap();
        t.set_deck_pos(b, 20.0, 0.0).unwrap(); // overlaps a by 80px on x

        t.separate(a);

        assert_eq!(t.deck(a).unwrap().pos(), Pos { x: 0.0, y: 0.0 }); // anchor unmoved
        let bp = t.deck(b).unwrap().pos();
        assert!(bp.x >= 100.0 - 0.02, "b should clear a on x, got {bp:?}");
        assert!(
            (bp.y - 0.0).abs() < 0.02,
            "b should not drift on y, got {bp:?}"
        );
    }

    /// Three stacked decks: the anchor holds and the chain pushes the rest apart (no pair overlaps).
    #[test]
    fn separate_resolves_chain_reactions() {
        let mut t = DeckTree::new();
        let root = t.root_id();
        let ids: Vec<_> = (0..3)
            .map(|i| t.add_deck(root, format!("D{i}")).unwrap())
            .collect();
        for &d in &ids {
            t.set_deck_size(d, 100.0, 100.0).unwrap();
            t.set_deck_pos(d, 0.0, 0.0).unwrap(); // all stacked at the origin
        }
        let anchor = ids[0];
        t.separate(anchor);

        assert_eq!(t.deck(anchor).unwrap().pos(), Pos { x: 0.0, y: 0.0 });
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let p = t.deck(ids[i]).unwrap().pos();
                let q = t.deck(ids[j]).unwrap().pos();
                let ox = (p.x + 100.0).min(q.x + 100.0) - p.x.max(q.x);
                let oy = (p.y + 100.0).min(q.y + 100.0) - p.y.max(q.y);
                assert!(
                    ox <= 0.02 || oy <= 0.02,
                    "decks {i},{j} still overlap: {p:?} {q:?}"
                );
            }
        }
    }

    /// place_deck clamps to the surface — the borders shove a deck back inside.
    #[test]
    fn place_deck_clamps_to_surface() {
        let mut t = DeckTree::new();
        let root = t.root_id();
        let a = t.add_deck(root, "A").unwrap();
        t.set_deck_size(a, 100.0, 100.0).unwrap();
        t.set_surface(300.0, 200.0);

        assert_eq!(
            t.place_deck(a, 500.0, 500.0).unwrap(),
            Pos { x: 200.0, y: 100.0 }
        );
        assert_eq!(
            t.place_deck(a, -50.0, -50.0).unwrap(),
            Pos { x: 0.0, y: 0.0 }
        );
    }

    /// In a bounded surface, separate keeps every deck fully inside (residual overlap is tolerated).
    #[test]
    fn separate_keeps_decks_within_bounds() {
        let mut t = DeckTree::new();
        let root = t.root_id();
        let ids: Vec<_> = (0..3)
            .map(|i| t.add_deck(root, format!("D{i}")).unwrap())
            .collect();
        for &d in &ids {
            t.set_deck_size(d, 100.0, 100.0).unwrap();
            t.set_deck_pos(d, 250.0, 0.0).unwrap(); // stacked near the right wall
        }
        t.set_surface(300.0, 200.0);
        t.separate(ids[0]);

        for &d in &ids {
            let p = t.deck(d).unwrap().pos();
            assert!(p.x >= -0.02 && p.x <= 200.02, "x out of bounds: {p:?}");
            assert!(p.y >= -0.02 && p.y <= 100.02, "y out of bounds: {p:?}");
        }
    }

    #[test]
    fn focus_fans_path_and_collapses_the_rest() {
        let mut t = DeckTree::new();
        let root = t.root_id();
        let a = t.add_deck(root, "A").unwrap();
        let b = t.add_deck(a, "B").unwrap();
        let sib = t.add_deck(root, "Sibling").unwrap();

        t.focus(b).unwrap();
        assert_eq!(t.focus_id(), b);
        assert!(!t.deck(b).unwrap().collapsed); // focus open
        assert!(!t.deck(a).unwrap().collapsed); // ancestor open
        assert!(!t.deck(root).unwrap().collapsed); // root open
        assert!(t.deck(sib).unwrap().collapsed); // unattended collapsed

        t.zoom_out(); // focus -> a
        assert_eq!(t.focus_id(), a);
        assert!(!t.deck(a).unwrap().collapsed);
        assert!(t.deck(b).unwrap().collapsed); // no longer on the path
    }

    #[test]
    fn zoom_out_at_root_is_noop() {
        let mut t = DeckTree::new();
        t.zoom_out();
        assert_eq!(t.focus_id(), t.root_id());
    }

    #[test]
    fn every_card_lives_in_exactly_one_deck() {
        let (t, _hand, _deck, _c0, _c1) = fixture();
        for (id, card) in [(_c0, t.card(_c0).unwrap()), (_c1, t.card(_c1).unwrap())] {
            let homes = t.decks.values().filter(|d| d.cards().contains(&id)).count();
            assert_eq!(homes, 1, "card must live in exactly one deck");
            assert!(t.deck(card.home()).unwrap().cards().contains(&id));
        }
    }
}
