//! §2.4–§2.6 — **stats-as-deck, the positional tree.** A character's [`Form`] (the flat
//! `Vec<StatCard>` summed by the engine, §2.3) *renders* as a **tree of decks** whose leaves are
//! **base-2 denomination cards**, and a leaf's meaning is its **path** (§2.6). This module derives
//! that canonical tree from a Form's totals — it is the **static Form tableau** (§2.6 GUARANTEE:
//! tableau only, never a shuffled pile), additive over the engine's summed read-path: the engine
//! still reads `Offense`/`Defense` (commutative sums, §5.5); the tree is the *legible* projection.
//!
//! Three locked rules it realizes:
//! - **§2.4 two suits** — every stat is **Quantity** (a count) or **Power** (a per-card magnitude).
//!   A leaf prints only *(suit, value)*; which stat it feeds is fixed by the **deck** it sits in.
//! - **§2.5 base-2 denominations** — suit cards come in 1, 2, 4, 8, … with **at most one of each**,
//!   so every value has **exactly one** representation (its binary expansion) and costs
//!   **popcount(value)** cards.
//! - **§2.6 positional notation** — root (bare identity) → **stat decks** (Health · Tempo · Might) →
//!   **suit decks** (Quantity · Power) → denomination **leaves**; a deck's face is the **sum** of its
//!   contents; only leaves carry values.

use crate::form::Form;

/// §2.4 — the two suits. **Quantity** = breadth (how many cards); **Power** = depth (per-card
/// magnitude). Quantity appears only on pooled stats; every stat has a Power.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Suit {
    Quantity,
    Power,
}

/// §2.6 — a **stat deck**: the middle tier of the Form tree (a physical grouping that maps to a
/// pooled resource, or the lone flat magnitude). Its children are suit decks; the leaves below them
/// carry the values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Deck {
    /// Health = Vitality (Quantity) × Toughness (Power) — the per-encounter pool (§2.2 / §2.7).
    Health,
    /// Tempo = Cadence (Quantity) × Finesse (Power) — the per-round pool (§3).
    Tempo,
    /// Might — the lone flat magnitude: a Power-only deck, no Quantity (§2.4).
    Might,
}

/// The five named stats (§2.4). Each is a **(deck × suit)** cell; **Might** is the lone Power-only flat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stat {
    Vitality,
    Toughness,
    Cadence,
    Finesse,
    Might,
}

impl Stat {
    /// The **(deck, suit)** cell this stat occupies — the §2.6 path that gives a leaf its meaning.
    pub fn cell(self) -> (Deck, Suit) {
        match self {
            Stat::Vitality => (Deck::Health, Suit::Quantity),
            Stat::Toughness => (Deck::Health, Suit::Power),
            Stat::Cadence => (Deck::Tempo, Suit::Quantity),
            Stat::Finesse => (Deck::Tempo, Suit::Power),
            Stat::Might => (Deck::Might, Suit::Power),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Stat::Vitality => "Vitality",
            Stat::Toughness => "Toughness",
            Stat::Cadence => "Cadence",
            Stat::Finesse => "Finesse",
            Stat::Might => "Might",
        }
    }
}

/// §2.5 — the **base-2 denomination leaves** of a value: its binary expansion, ascending (1, 2, 4,
/// …), **at most one of each**. The sum is the value and the count is `popcount(value)`. `0` → no
/// leaves (an empty suit deck).
pub fn denominations(value: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut bit = 1u32;
    let mut v = value;
    while v != 0 {
        if v & 1 == 1 {
            out.push(bit);
        }
        v >>= 1;
        bit <<= 1;
    }
    out
}

/// §2.5 — a value's **card-cost**: `popcount(value)` (set bits = denomination leaves). Doubling a
/// stat is **+1 card** (one new denomination); a sparse value stays sparse (O(log V)).
pub fn card_cost(value: u32) -> u32 {
    value.count_ones()
}

/// A suit deck (the leaf tier's parent): its `suit`, the stat it feeds (via its path), and the
/// **denomination leaves** that sum to the stat's value. The deck **face** is `value` (= the sum).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SuitDeck {
    pub suit: Suit,
    pub stat: Stat,
    pub value: u32,
    pub leaves: Vec<u32>,
}

impl SuitDeck {
    fn new(suit: Suit, stat: Stat, value: u32) -> Self {
        SuitDeck {
            suit,
            stat,
            value,
            leaves: denominations(value),
        }
    }
}

/// A stat deck (the middle tier): its `deck` and its (one or two) suit decks. Only **leaves** carry
/// values — a stat deck is pure position (§2.6), its face the sum of its suit decks' faces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatDeck {
    pub deck: Deck,
    pub suits: Vec<SuitDeck>,
}

/// §2.6 — the **Form tree**: the three stat decks (Health · Tempo · Might) derived from a [`Form`]'s
/// commutative totals. The engine reads the summed `Offense`/`Defense`; this is the same numbers as a
/// navigable, base-2, positional tableau (the leaves you audit, the faces you act on).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormTree {
    pub decks: Vec<StatDeck>,
}

impl FormTree {
    /// Derive the canonical tree from a Form (its stats summed commutatively, §5.5). Health carries
    /// both suits (Vitality × Toughness), Tempo both (Cadence × Finesse), Might the lone Power flat.
    pub fn of(form: &Form) -> Self {
        let o = form.offense();
        let d = form.defense();
        FormTree {
            decks: vec![
                StatDeck {
                    deck: Deck::Health,
                    suits: vec![
                        SuitDeck::new(Suit::Quantity, Stat::Vitality, d.health.max()),
                        SuitDeck::new(Suit::Power, Stat::Toughness, d.health.toughness()),
                    ],
                },
                StatDeck {
                    deck: Deck::Tempo,
                    suits: vec![
                        SuitDeck::new(Suit::Quantity, Stat::Cadence, o.cadence),
                        SuitDeck::new(Suit::Power, Stat::Finesse, o.finesse),
                    ],
                },
                StatDeck {
                    deck: Deck::Might,
                    suits: vec![SuitDeck::new(Suit::Power, Stat::Might, o.might)],
                },
            ],
        }
    }

    /// The value of one stat, read off the tree (the suit-deck face for that stat). Round-trips the
    /// engine totals — a regression guard that the tableau equals the summed Form (§2.6 deck-face =
    /// sum).
    pub fn stat(&self, stat: Stat) -> u32 {
        self.decks
            .iter()
            .flat_map(|sd| &sd.suits)
            .find(|s| s.stat == stat)
            .map(|s| s.value)
            .unwrap_or(0)
    }

    /// The whole Form's card-cost: total denomination leaves across every suit deck (§2.5 popcount).
    pub fn total_cards(&self) -> u32 {
        self.decks
            .iter()
            .flat_map(|sd| &sd.suits)
            .map(|s| s.leaves.len() as u32)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::form::StatCard;

    #[test]
    fn base2_denominations_are_the_unique_binary_expansion() {
        // §2.5: 18 = 16 + 2, the *only* representation; cost = popcount = 2.
        assert_eq!(denominations(18), vec![2, 16]);
        assert_eq!(card_cost(18), 2);
        assert_eq!(denominations(0), Vec::<u32>::new()); // empty deck
        assert_eq!(denominations(1), vec![1]);
        assert_eq!(denominations(7), vec![1, 2, 4]); // popcount 3
        // Round-trip + one-of-each: every value's leaves are distinct powers of two summing to it.
        for v in 0u32..=64 {
            let d = denominations(v);
            assert_eq!(d.iter().sum::<u32>(), v, "leaves sum to the value");
            assert!(
                d.iter().all(|x| x.is_power_of_two()),
                "every leaf is a power of two"
            );
            let mut sorted = d.clone();
            sorted.dedup();
            assert_eq!(
                sorted.len(),
                d.len(),
                "no denomination repeats (one of each)"
            );
            assert_eq!(card_cost(v), d.len() as u32, "cost = popcount = leaf count");
        }
    }

    #[test]
    fn stat_cells_are_the_locked_deck_x_suit_map() {
        // §2.4 / §2.6: each stat sits at exactly its (deck, suit) path; Might is the Power-only flat.
        assert_eq!(Stat::Vitality.cell(), (Deck::Health, Suit::Quantity));
        assert_eq!(Stat::Toughness.cell(), (Deck::Health, Suit::Power));
        assert_eq!(Stat::Cadence.cell(), (Deck::Tempo, Suit::Quantity));
        assert_eq!(Stat::Finesse.cell(), (Deck::Tempo, Suit::Power));
        assert_eq!(Stat::Might.cell(), (Deck::Might, Suit::Power));
    }

    #[test]
    fn form_tree_face_equals_the_summed_form() {
        // §2.6 deck-face = sum: the tableau derived from a Form must read back the engine totals.
        let form = Form::new(vec![
            StatCard {
                name: "base".into(),
                might: 6,
                vitality: 10,
                toughness: 3,
                cadence: 5,
                finesse: 2,
            },
            StatCard {
                name: "reward".into(),
                toughness: 1,
                cadence: 2,
                ..Default::default()
            },
        ]);
        let tree = FormTree::of(&form);
        // Faces equal the summed stats.
        assert_eq!(tree.stat(Stat::Vitality), 10);
        assert_eq!(tree.stat(Stat::Toughness), 4); // 3 + 1
        assert_eq!(tree.stat(Stat::Cadence), 7); // 5 + 2
        assert_eq!(tree.stat(Stat::Finesse), 2);
        assert_eq!(tree.stat(Stat::Might), 6);
        // A leaf's value sums to its suit-deck face (§2.6).
        for sd in &tree.decks {
            for suit in &sd.suits {
                assert_eq!(
                    suit.leaves.iter().sum::<u32>(),
                    suit.value,
                    "{:?} {:?} leaves sum to the face",
                    sd.deck,
                    suit.suit
                );
            }
        }
        // Card-cost = popcount over all stats: V10(2)+T4(1)+C7(3)+F2(1)+M6(2) = 9.
        assert_eq!(tree.total_cards(), 2 + 1 + 3 + 1 + 2);
    }

    #[test]
    fn might_deck_is_power_only_no_quantity() {
        // §2.4: Might is the lone flat — its deck carries a Power suit and no Quantity.
        let form = Form::new(vec![StatCard {
            might: 5,
            ..Default::default()
        }]);
        let tree = FormTree::of(&form);
        let might = tree
            .decks
            .iter()
            .find(|d| d.deck == Deck::Might)
            .expect("a Might deck");
        assert_eq!(might.suits.len(), 1, "Might has Power only — no Quantity");
        assert_eq!(might.suits[0].suit, Suit::Power);
    }
}
