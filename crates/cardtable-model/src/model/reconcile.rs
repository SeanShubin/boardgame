//! **Reconcile a saved board against the pristine board the current code builds** — the load-time seam
//! that decides what a software update preserves and what it refreshes.
//!
//! A save conflates two kinds of state. *Arrangement* — where each card sits, its facing, its size, how
//! stacks are split, where piles were dragged — encodes the player's decisions: the code cannot invent
//! it, so it must be preserved. *Content* — what a card says (detail/panel/type/kind/recipe) and which
//! cards exist at all — is a pure function of the code: persisting it just caches a projection that goes
//! stale the moment the code changes (a rules card from an old build "could never have come into
//! existence under the updated code"). The rule here is mechanical, keyed off [`CardKind`]:
//!
//! - **Physical** cards (Regular / Zone / Header): the code owns their content, the save owns their
//!   arrangement. Every physical card in the result is drawn from the pristine board's supply, matched
//!   by `(front title, card type)` on the sum of stack quantities (PC.2). A card the current code no
//!   longer provisions cannot come back — wherever the player had carried it — and a card the code newly
//!   provisions appears at its pristine home.
//! - **Virtual** cards (session records — e.g. a battle's round logs): their content *is* the record the
//!   player earned, so the save owns them wholly; they are re-minted verbatim.
//! - **Utility** cards (the System deck's controls): neither side — skipped here, torn down and rebuilt
//!   every launch by the renderer.
//!
//! Piles match by their **label path** from the root (k-th same-labeled sibling to k-th). A matched pile
//! keeps the code's `layout`/`projection` (rules of presentation) and the save's position, open/closed
//! state, and child order (the player's arrangement). A saved pile with no pristine counterpart — a
//! Victory record, a character deck — is grafted. A pristine pile the save never mentions (a deck the
//! update added) simply stays as built; a deck the update deleted has no supply, so its cards vanish.
//!
//! Invariant: the result's physical card total equals the pristine board's exactly — reconciliation can
//! neither create nor destroy physical cards relative to what the current code provisions.

use std::collections::{HashMap, HashSet, VecDeque};

use super::{Board, CardId, CardKind, Face, Node, PileId};

/// Every pile in tree (pre-)order: parent before children, children in pile order. Deterministic, so
/// the supply is consumed in a stable order.
fn piles_in_tree_order(board: &Board) -> Vec<PileId> {
    let mut out = Vec::new();
    let mut stack = vec![board.root_id()];
    while let Some(p) = stack.pop() {
        out.push(p);
        let subs = board.pile(p).map(|q| q.subpiles()).unwrap_or_default();
        for s in subs.into_iter().rev() {
            stack.push(s);
        }
    }
    out
}

/// Rebuild `saved`'s arrangement on top of `pristine` (the board the *current* code builds) — see the
/// module docs for what is preserved and what is refreshed. Consumes `pristine` as the base; `saved` is
/// only read. Total, never panics: anything in the save the pristine board cannot express is dropped,
/// anything in the pristine board the save does not claim stays at its pristine home.
pub fn reconcile(pristine: Board, saved: &Board) -> Board {
    let mut out = pristine;
    let saved_piles = piles_in_tree_order(saved);

    // --- Phase 1: piles — match saved piles to pristine ones by label path, graft the rest. ---------
    // Matching runs against a snapshot of the pristine sibling lists, so piles grafted along the way can
    // never shift which pristine pile the k-th same-labeled saved sibling pairs with.
    let pristine_subpiles: HashMap<PileId, Vec<PileId>> = piles_in_tree_order(&out)
        .into_iter()
        .map(|p| (p, out.pile(p).map(|q| q.subpiles()).unwrap_or_default()))
        .collect();
    let mut pile_map: HashMap<PileId, PileId> = HashMap::new();
    let mut grafted: HashSet<PileId> = HashSet::new();
    pile_map.insert(saved.root_id(), out.root_id());
    for &sp in &saved_piles {
        let Some(&c) = pile_map.get(&sp) else {
            continue;
        };
        let mut occurrence: HashMap<String, usize> = HashMap::new();
        for child in saved.pile(sp).map(|q| q.subpiles()).unwrap_or_default() {
            let Some(cp) = saved.pile(child) else {
                continue;
            };
            let k = {
                let e = occurrence.entry(cp.label.clone()).or_insert(0);
                let k = *e;
                *e += 1;
                k
            };
            let candidate = pristine_subpiles
                .get(&c)
                .map(|subs| {
                    subs.iter()
                        .filter(|&&s| out.pile(s).is_some_and(|q| q.label == cp.label))
                        .copied()
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
                .get(k)
                .copied();
            let m = match candidate {
                Some(m) => m,
                None => {
                    // Session-created (a record deck, a character deck): graft it, presentation and all.
                    let Ok(m) = out.add_pile(c, cp.label.clone()) else {
                        continue;
                    };
                    grafted.insert(child);
                    let _ = out.set_layout(m, cp.layout());
                    m
                }
            };
            let _ = out.set_pile_pos(m, cp.pos().x, cp.pos().y);
            pile_map.insert(child, m);
        }
    }

    // --- Phase 2: cards — draw every saved physical card from the pristine supply; re-mint records. --
    // The supply: every physical pristine card by content key, in tree order. Quantities are consumed
    // stack by stack, so a save that split a x12 bank across three piles claims 12 in total (PC.2).
    let mut supply: HashMap<(String, String), VecDeque<CardId>> = HashMap::new();
    for p in piles_in_tree_order(&out) {
        for card in out.pile(p).map(|q| q.cards()).unwrap_or_default() {
            let Some(c) = out.card(card) else {
                continue;
            };
            if c.is_physical() {
                supply
                    .entry((c.front_title().to_string(), c.card_type().to_string()))
                    .or_default()
                    .push_back(card);
            }
        }
    }
    let mut card_map: HashMap<CardId, CardId> = HashMap::new();
    for &sp in &saved_piles {
        let Some(&dest) = pile_map.get(&sp) else {
            continue;
        };
        for sc_id in saved.pile(sp).map(|q| q.cards()).unwrap_or_default() {
            let Some(sc) = saved.card(sc_id) else {
                continue;
            };
            match sc.kind() {
                // System controls: rebuilt each launch, never carried over.
                CardKind::Utility(_) => {}
                // Session records: the content IS the state — re-mint verbatim.
                CardKind::Virtual => {
                    let Ok(id) = out.add_card(
                        dest,
                        Face::Up {
                            title: sc.front_title().to_string(),
                        },
                        None,
                    ) else {
                        continue;
                    };
                    let _ = out.set_card_kind(id, CardKind::Virtual);
                    if !sc.card_type().is_empty() {
                        let _ = out.set_card_type(id, sc.card_type().to_string());
                    }
                    if !sc.detail().is_empty() {
                        let _ = out.set_card_detail(id, sc.detail().to_vec());
                    }
                    if !sc.panel().is_empty() {
                        let _ = out.set_card_panel(id, sc.panel().to_vec());
                    }
                    let _ = out.set_card_quantity(id, sc.quantity());
                    if sc.is_face_down() {
                        let _ = out.flip_down(id);
                    }
                    let _ = out.set_size_clamped(id, sc.size());
                    let _ = out.set_card_pos(id, sc.pos().x, sc.pos().y);
                    card_map.insert(sc_id, id);
                }
                // Physical: arrangement from the save, the card itself from the pristine supply.
                _ => {
                    let key = (sc.front_title().to_string(), sc.card_type().to_string());
                    let mut want = sc.quantity();
                    let mut placed: Option<CardId> = None;
                    while want > 0 {
                        let Some(stack) = supply.get_mut(&key).and_then(|d| d.front().copied())
                        else {
                            break; // supply exhausted (or the key no longer exists): the rest vanishes
                        };
                        let avail = out.card(stack).map(|c| c.quantity()).unwrap_or(0);
                        let take = want.min(avail).max(1);
                        let chunk = if take >= avail {
                            supply.get_mut(&key).expect("key present").pop_front();
                            stack
                        } else {
                            match out.split_off(stack, take) {
                                Ok(twin) => twin,
                                Err(_) => break,
                            }
                        };
                        let at = out.pile(dest).map(|p| p.children().len()).unwrap_or(0);
                        if out.move_card(chunk, dest, at).is_err() {
                            break;
                        }
                        match placed {
                            None => placed = Some(chunk),
                            // Fold follow-up chunks into the first, re-forming the saved stack (the
                            // sum is conserved: the chunk's quantity moves onto the first card).
                            Some(first) => {
                                let total =
                                    out.card(first).map(|c| c.quantity()).unwrap_or(1) + take;
                                let _ = out.set_card_quantity(first, total);
                                let _ = out.remove_card(chunk);
                            }
                        }
                        want -= take;
                    }
                    if let Some(first) = placed {
                        if sc.is_face_down() {
                            let _ = out.flip_down(first);
                        }
                        let _ = out.set_size_clamped(first, sc.size());
                        let _ = out.set_card_pos(first, sc.pos().x, sc.pos().y);
                        card_map.insert(sc_id, first);
                    }
                }
            }
        }
    }

    // --- Phase 3: order — each reconciled pile's children follow the saved order. -------------------
    // Whatever the save does not mention (cards the update added, still at their pristine home) keeps
    // its current relative order *below* the saved block, so e.g. a pile's Zone label stays on top.
    for &sp in &saved_piles {
        let Some(&c) = pile_map.get(&sp) else {
            continue;
        };
        let block: Vec<Node> = saved
            .pile(sp)
            .map(|q| q.children().to_vec())
            .unwrap_or_default()
            .iter()
            .filter_map(|n| match *n {
                Node::Card(id) => card_map.get(&id).copied().map(Node::Card),
                Node::Pile(id) => pile_map.get(&id).copied().map(Node::Pile),
            })
            .collect();
        let in_block: HashSet<Node> = block.iter().copied().collect();
        let mut desired: Vec<Node> = out
            .pile(c)
            .map(|p| p.children().to_vec())
            .unwrap_or_default()
            .into_iter()
            .filter(|n| !in_block.contains(n))
            .collect();
        desired.extend(block);
        let _ = out.set_child_order(c, desired);
    }

    // --- Phase 4: re-link what points at ids — reflects (character decks), grafted projections. -----
    for &sp in &saved_piles {
        let Some(&m) = pile_map.get(&sp) else {
            continue;
        };
        if let Some(r) = saved.pile(sp).and_then(|p| p.reflects())
            && let Some(&nr) = card_map.get(&r)
        {
            let _ = out.set_reflects(m, Some(nr));
        }
        // A matched pile keeps the code's projection; a grafted one carries the save's, re-mapped.
        if grafted.contains(&sp) {
            let sources: Vec<PileId> = saved
                .pile(sp)
                .map(|p| p.projection().to_vec())
                .unwrap_or_default()
                .into_iter()
                .filter_map(|s| pile_map.get(&s).copied())
                .collect();
            if !sources.is_empty() {
                let _ = out.set_projection(m, sources);
            }
        }
    }

    // --- Phase 5: attention — restore focus (nearest surviving ancestor), then exact open/closed. ---
    let mut f = saved.focus_id();
    loop {
        if let Some(&m) = pile_map.get(&f) {
            let _ = out.focus(m);
            break;
        }
        match saved.pile(f).and_then(|p| p.parent()) {
            Some(parent) => f = parent,
            None => {
                let _ = out.focus(out.root_id());
                break;
            }
        }
    }
    // `focus` derives every pile's collapsed state from the focus path; re-apply the saved flags on top
    // so any pile the session had opened or closed by hand comes back exactly as left.
    for &sp in &saved_piles {
        if let (Some(&m), Some(p)) = (pile_map.get(&sp), saved.pile(sp)) {
            let _ = out.set_collapsed(m, p.collapsed);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::{Arrangement, CardKind, Face, Layout, Size, Utility};
    use super::*;

    /// The pristine table "the current code" builds: a Rules deck (one card, `new text`) and an empty
    /// Hand.
    fn pristine() -> Board {
        let mut b = Board::new();
        let rules = b.add_pile(b.root_id(), "Rules").unwrap();
        b.add_pile(b.root_id(), "Hand").unwrap();
        let alpha = b
            .add_card(
                rules,
                Face::Up {
                    title: "Alpha".into(),
                },
                None,
            )
            .unwrap();
        b.set_card_type(alpha, "rule").unwrap();
        b.set_card_detail(alpha, vec!["new text".into()]).unwrap();
        b
    }

    fn find_pile(b: &Board, label: &str) -> PileId {
        piles_in_tree_order(b)
            .into_iter()
            .find(|&p| b.pile(p).is_some_and(|q| q.label == label))
            .unwrap_or_else(|| panic!("no pile labeled {label}"))
    }

    fn find_card(b: &Board, title: &str) -> CardId {
        piles_in_tree_order(b)
            .into_iter()
            .flat_map(|p| b.pile(p).map(|q| q.cards()).unwrap_or_default())
            .find(|&c| b.card(c).is_some_and(|k| k.front_title() == title))
            .unwrap_or_else(|| panic!("no card titled {title}"))
    }

    fn physical_total(b: &Board) -> usize {
        b.physical_card_count(b.root_id())
    }

    /// The headline case: the player moved a rules card, then the code's text for it changed. The card
    /// stays where the player put it, but says what the current code says.
    #[test]
    fn moved_card_keeps_its_place_and_takes_the_code_content() {
        let mut saved = pristine();
        let alpha = find_card(&saved, "Alpha");
        let hand = find_pile(&saved, "Hand");
        saved
            .set_card_detail(alpha, vec!["old text".into()])
            .unwrap();
        saved.move_card(alpha, hand, 0).unwrap();

        let out = reconcile(pristine(), &saved);
        let hand = find_pile(&out, "Hand");
        let cards = out.pile(hand).unwrap().cards();
        assert_eq!(cards.len(), 1, "the moved card is in the Hand");
        let c = out.card(cards[0]).unwrap();
        assert_eq!(c.name(), "Alpha");
        assert_eq!(
            c.detail(),
            ["new text".to_string()],
            "content is the code's"
        );
        assert!(
            out.pile(find_pile(&out, "Rules"))
                .unwrap()
                .cards()
                .is_empty(),
            "and it left the Rules deck"
        );
    }

    /// A card the update deleted cannot come back — even carried in the player's Hand — and the
    /// physical total still matches what the current code provisions.
    #[test]
    fn deleted_card_vanishes_wherever_it_was_carried() {
        let mut saved = pristine();
        let hand = find_pile(&saved, "Hand");
        let ghost = saved
            .add_card(
                hand,
                Face::Up {
                    title: "Retired Rule".into(),
                },
                None,
            )
            .unwrap();
        saved.set_card_type(ghost, "rule").unwrap();

        let out = reconcile(pristine(), &saved);
        let everywhere: Vec<CardId> = piles_in_tree_order(&out)
            .into_iter()
            .flat_map(|p| out.pile(p).map(|q| q.cards()).unwrap_or_default())
            .collect();
        assert!(
            everywhere
                .iter()
                .all(|&c| out.card(c).unwrap().front_title() != "Retired Rule"),
            "the retired card is gone"
        );
        assert_eq!(physical_total(&out), physical_total(&pristine()));
    }

    /// A card the update added appears at its pristine home — at the bottom, under the saved order.
    #[test]
    fn added_card_appears_at_its_pristine_home() {
        let saved = pristine();
        let mut newer = pristine();
        let rules = find_pile(&newer, "Rules");
        let beta = newer
            .add_card(
                rules,
                Face::Up {
                    title: "Beta".into(),
                },
                None,
            )
            .unwrap();
        newer.set_card_type(beta, "rule").unwrap();

        let out = reconcile(newer, &saved);
        let rules = find_pile(&out, "Rules");
        let names: Vec<String> = out
            .pile(rules)
            .unwrap()
            .cards()
            .into_iter()
            .map(|c| out.card(c).unwrap().name().to_string())
            .collect();
        assert_eq!(names, ["Beta", "Alpha"], "new card below the saved block");
    }

    /// A session record — a grafted pile holding a Virtual card — survives verbatim: its content is the
    /// player's record, not the code's.
    #[test]
    fn virtual_records_survive_verbatim() {
        let mut saved = pristine();
        let hand = find_pile(&saved, "Hand");
        let victory = saved.add_pile(hand, "Victory").unwrap();
        let log = saved
            .add_card(
                victory,
                Face::Up {
                    title: "Round 1".into(),
                },
                None,
            )
            .unwrap();
        saved.set_card_kind(log, CardKind::Virtual).unwrap();
        saved.set_card_type(log, "log").unwrap();
        saved
            .set_card_panel(log, vec!["Raider fells the Husk".into()])
            .unwrap();

        let out = reconcile(pristine(), &saved);
        let victory = find_pile(&out, "Victory");
        let cards = out.pile(victory).unwrap().cards();
        assert_eq!(cards.len(), 1);
        let c = out.card(cards[0]).unwrap();
        assert_eq!(c.name(), "Round 1");
        assert_eq!(c.kind(), CardKind::Virtual);
        assert_eq!(c.panel(), ["Raider fells the Husk".to_string()]);
    }

    /// A split stack reconciles on the sum (PC.2): the save's distribution is rebuilt from the pristine
    /// supply, and the physical total is exactly the pristine one.
    #[test]
    fn split_stacks_reconcile_on_the_sum() {
        let mut base = pristine();
        let rules = find_pile(&base, "Rules");
        let stack = base
            .add_card(rules, Face::Up { title: "6".into() }, None)
            .unwrap();
        base.set_card_type(stack, "number").unwrap();
        base.set_card_quantity(stack, 12).unwrap();

        let mut saved = base.clone();
        let hand = find_pile(&saved, "Hand");
        let three = saved.split_off(stack, 3).unwrap();
        saved.move_card(three, hand, 0).unwrap();

        let out = reconcile(base.clone(), &saved);
        let in_hand = out.pile(find_pile(&out, "Hand")).unwrap().cards();
        assert_eq!(in_hand.len(), 1);
        assert_eq!(out.card(in_hand[0]).unwrap().quantity(), 3);
        let in_rules: u32 = out
            .pile(find_pile(&out, "Rules"))
            .unwrap()
            .cards()
            .into_iter()
            .filter(|&c| out.card(c).unwrap().name() == "6")
            .map(|c| out.card(c).unwrap().quantity())
            .sum();
        assert_eq!(in_rules, 9, "the rest stays at the pristine home");
        assert_eq!(physical_total(&out), physical_total(&base));
    }

    /// Facing and size are the player's; size clamps to what the *new* content supports.
    #[test]
    fn facing_kept_and_size_clamped_to_new_content() {
        let mut saved = pristine();
        let alpha = find_card(&saved, "Alpha");
        saved.flip_down(alpha).unwrap();
        saved.set_size_clamped(alpha, Size::Medium).unwrap();

        // Same code: down + Medium survive (the pristine card still has detail).
        let out = reconcile(pristine(), &saved);
        let c = out.card(find_card(&out, "Alpha")).unwrap();
        assert!(c.is_face_down(), "still face down");
        assert_eq!(c.size(), Size::Medium);

        // An update that dropped the card's detail: the saved Medium clamps back to Small.
        let mut slimmer = pristine();
        let a = find_card(&slimmer, "Alpha");
        slimmer.set_card_detail(a, Vec::new()).unwrap();
        let out = reconcile(slimmer, &saved);
        let c = out.card(find_card(&out, "Alpha")).unwrap();
        assert_eq!(c.size(), Size::Small, "size clamps to the new content");
    }

    /// A matched pile keeps the player's position but takes the code's layout — presentation rules are
    /// content, where a deck sits on the felt is arrangement.
    #[test]
    fn matched_pile_keeps_player_pos_takes_code_layout() {
        let mut saved = pristine();
        let rules = find_pile(&saved, "Rules");
        saved.set_pile_pos(rules, 500, 600).unwrap();
        saved
            .set_layout(
                rules,
                Layout {
                    arrangement: Arrangement::List,
                    editable: true,
                },
            )
            .unwrap();

        let mut newer = pristine();
        let grid = Layout {
            arrangement: Arrangement::Grid { columns: 3 },
            editable: false,
        };
        newer.set_layout(find_pile(&newer, "Rules"), grid).unwrap();

        let out = reconcile(newer, &saved);
        let rules = out.pile(find_pile(&out, "Rules")).unwrap();
        assert_eq!((rules.pos().x, rules.pos().y), (500, 600), "player's spot");
        assert_eq!(rules.layout(), grid, "code's layout");
    }

    /// The saved child order is the player's arrangement — it wins inside a matched pile.
    #[test]
    fn child_order_follows_the_save() {
        let mut base = pristine();
        let rules = find_pile(&base, "Rules");
        for t in ["Bravo", "Charlie"] {
            let c = base
                .add_card(rules, Face::Up { title: t.into() }, None)
                .unwrap();
            base.set_card_type(c, "rule").unwrap();
        }

        let mut saved = base.clone();
        saved.reorder(rules, 2, 0).unwrap(); // Charlie to the bottom

        let out = reconcile(base, &saved);
        let names: Vec<String> = out
            .pile(find_pile(&out, "Rules"))
            .unwrap()
            .cards()
            .into_iter()
            .map(|c| out.card(c).unwrap().name().to_string())
            .collect();
        assert_eq!(names, ["Charlie", "Alpha", "Bravo"]);
    }

    /// Focus comes back where the player left it (remapped), and the selection does not persist.
    #[test]
    fn focus_is_remapped_and_selection_cleared() {
        let mut saved = pristine();
        let hand = find_pile(&saved, "Hand");
        saved.focus(hand).unwrap();
        let alpha = find_card(&saved, "Alpha");
        saved.select(alpha).unwrap();

        let out = reconcile(pristine(), &saved);
        assert_eq!(out.focus_id(), find_pile(&out, "Hand"));
        assert!(out.selection().is_empty());
    }

    /// Utility cards never carry over — the System deck is rebuilt each launch.
    #[test]
    fn utility_cards_are_skipped() {
        let mut saved = pristine();
        let system = saved.add_pile(saved.root_id(), "System").unwrap();
        let quit = saved
            .add_card(
                system,
                Face::Up {
                    title: "Exit".into(),
                },
                None,
            )
            .unwrap();
        saved
            .set_card_kind(quit, CardKind::Utility(Utility::Exit))
            .unwrap();

        let out = reconcile(pristine(), &saved);
        assert!(
            out.pile(find_pile(&out, "System"))
                .unwrap()
                .cards()
                .is_empty(),
            "the grafted System pile carries no cards (the renderer rebuilds it)"
        );
    }

    /// A character deck (a grafted pile whose `reflects` names its hero card) is re-linked to the
    /// re-minted hero card's new id.
    #[test]
    fn reflects_is_relinked_to_the_reminted_card() {
        let mut base = pristine();
        let heroes = base.add_pile(base.root_id(), "Heroes").unwrap();
        let hero = base
            .add_card(
                heroes,
                Face::Up {
                    title: "Raider".into(),
                },
                None,
            )
            .unwrap();
        base.set_card_type(hero, "hero").unwrap();

        let mut saved = base.clone();
        let deck = saved.add_pile(saved.root_id(), "Raider").unwrap();
        saved.move_card(hero, deck, 0).unwrap();
        saved.set_reflects(deck, Some(hero)).unwrap();

        let out = reconcile(base, &saved);
        let deck = find_pile(&out, "Raider");
        let linked = out.pile(deck).unwrap().reflects().expect("re-linked");
        assert_eq!(out.card(linked).unwrap().front_title(), "Raider");
        assert_eq!(
            out.pile_of(linked),
            Some(deck),
            "the linked card is the one in the deck"
        );
    }
}
