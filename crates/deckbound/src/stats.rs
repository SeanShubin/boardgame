//! Stats and the **pile → bar → pool** defense model.
//!
//! Combat has **one damage channel** (Spec §2.2): every attack deals untyped **Might** into the
//! **health pile**; each time the pile clears the **bar** (Toughness) one Health card flips face down.
//! Empty the health pool and the Actor is **down** — the game's single kill-condition (Charter #13).
//! There is **no cut** today — Armor and damage *types* are deferred to the later gear system
//! (`future-possibilities.md` §7). The old inner **Fear/Spirit** channel was collapsed out (2026).
//! See `docs/games/deckbound/notes/form-and-defeat.md`.

/// A single **Health card** in the pool (§2.4 Power). A card is **face-up** while it is still absorbing
/// blows and **face-down** (`down`) once a pile clears its bar and flips it. `toughness` is this card's
/// per-card bar — the magnitude the pile must clear to flip *it*. Today every card in a pool shares one
/// toughness (a uniform deck), but the field is per-card so a future mixed wall can vary it.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct HealthCard {
    /// §2.4 Power: this card's per-card bar (the damage the pile must clear to flip it).
    pub toughness: u32,
    /// Facing: `false` = face-up (intact), `true` = face-down (flipped/spent).
    pub down: bool,
}

/// The **health pool**: a 1D deck of [`HealthCard`]s, front-to-back. Flips happen **front-first** (the
/// front-most face-up card flips down); Recover turns the front-most face-down card back up. The only
/// maintained defensive meter — Vitality is the card *count*, Toughness the per-card *bar*.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Health {
    /// The deck, front (index 0) to back. Damage flips face-up cards front-first.
    pub cards: Vec<HealthCard>,
}

impl Health {
    /// `count` face-up cards, each with `toughness` (floored at 1) — matches the old constructor.
    pub fn new(count: u32, toughness: u32) -> Self {
        let toughness = toughness.max(1);
        Self {
            cards: (0..count)
                .map(|_| HealthCard {
                    toughness,
                    down: false,
                })
                .collect(),
        }
    }

    /// **Vitality** count still standing — the number of face-up cards (the old `remaining`).
    pub fn remaining(&self) -> u32 {
        self.cards.iter().filter(|c| !c.down).count() as u32
    }

    /// Total deck size — the old `max`.
    pub fn max(&self) -> u32 {
        self.cards.len() as u32
    }

    /// The current **bar** (Toughness): the toughness of the front-most **face-up** card. The deck is
    /// uniform today, so the choice of card is behavior-identical to the old single `toughness`; we pick
    /// the front-most face-up card (the one a hit would flip next) so `take_with_toughness` reads the bar
    /// of the card it is about to flip. If no card is face-up (the Actor is down), fall back to the front
    /// card's toughness — still floored at 1, and never used to flip since the loop stops at empty.
    pub fn toughness(&self) -> u32 {
        self.cards
            .iter()
            .find(|c| !c.down)
            .or_else(|| self.cards.first())
            .map(|c| c.toughness)
            .unwrap_or(1)
            .max(1)
    }

    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    /// Turn the front-most **face-up** card **face-down** (a flip). Returns whether one was flipped.
    pub fn flip_down(&mut self) -> bool {
        if let Some(c) = self.cards.iter_mut().find(|c| !c.down) {
            c.down = true;
            true
        } else {
            false
        }
    }

    /// Turn the front-most **face-down** card **face-up** (Recover). Returns whether one was turned up.
    pub fn turn_up(&mut self) -> bool {
        if let Some(c) = self.cards.iter_mut().find(|c| c.down) {
            c.down = false;
            true
        } else {
            false
        }
    }

    /// Turn up to `amt` face-down cards back up (front-first); returns how many were turned. A heal that
    /// exceeds the down cards simply stops (clamped to `max`, like the old `(remaining + amt).min(max)`).
    pub fn heal(&mut self, amt: u32) -> u32 {
        let mut turned = 0;
        while turned < amt && self.turn_up() {
            turned += 1;
        }
        turned
    }

    /// Replace the deck with `count` face-up cards, each at this pool's current toughness (the front
    /// card's, or 1 if empty) — a wholesale Vitality reset for balance probes.
    pub fn set_count(&mut self, count: u32) {
        let toughness = self.cards.first().map(|c| c.toughness).unwrap_or(1).max(1);
        *self = Health::new(count, toughness);
    }

    /// Set **every** card's per-card bar to `toughness` (floored at 1).
    pub fn set_toughness(&mut self, toughness: u32) {
        let toughness = toughness.max(1);
        for c in &mut self.cards {
            c.toughness = toughness;
        }
    }

    /// Append `n` face-up cards at the current toughness (the front card's, or 1 if empty).
    pub fn add_cards(&mut self, n: u32) {
        let toughness = self.cards.first().map(|c| c.toughness).unwrap_or(1).max(1);
        for _ in 0..n {
            self.cards.push(HealthCard {
                toughness,
                down: false,
            });
        }
    }

    /// Add `extra` to **every** card's per-card bar.
    pub fn add_toughness(&mut self, extra: u32) {
        for c in &mut self.cards {
            c.toughness += extra;
        }
    }
}

/// Everything that defends an Actor. **Toughness** (the bar) is a passive stat read off the Form; only
/// the **health pool** is a maintained meter. `health_pile` is a **per-phase** accumulator (§4.6): a
/// landed hit banks its Might here, the pile flips a Health card each time it clears Toughness, and the
/// pile **wipes at every phase boundary** — sub-threshold damage never crosses into the next phase
/// (only Health persists, §2.1). See [`Defense::clear_pile`].
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Defense {
    pub health: Health,

    /// §4.6 per-phase pile: the Might banked toward the next Health flip in the **current phase**.
    /// Cleared at each phase boundary by [`clear_pile`](Defense::clear_pile) (was round-scoped, §2.2 →
    /// §4.6 per-phase).
    pub health_pile: u32,
}

/// What a single hit did.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HitOutcome {
    /// Damage that accumulated into the round's pile — what "accumulates" before any health card
    /// turns. (No cut today, so this equals the raw blow.)
    pub through: u32,
    /// Health cards turned **face down** by this hit (the health pile crossing toughness).
    pub cards_flipped: u32,
    /// The health pool emptied — the Actor is knocked out.
    pub down: bool,
}

impl Defense {
    pub fn new(vitality: u32, toughness: u32) -> Self {
        Self {
            health: Health::new(vitality, toughness),
            health_pile: 0,
        }
    }

    /// Health gone → out of the fight.
    pub fn is_down(&self) -> bool {
        self.health.is_empty()
    }

    /// Apply one `raw`-magnitude (untyped Might) hit. Accumulate into the round's pile → each time the
    /// pile clears the bar (Toughness), flip one Health card. No cut, no types (Spec §2.2).
    pub fn take(&mut self, raw: u32) -> HitOutcome {
        self.take_with_toughness(raw, self.health.toughness())
    }

    /// As [`take`](Defense::take), but the per-card **wall** is `bar` rather than the bare Toughness —
    /// the call site folds in any **Guard** tokens (+Toughness this round, §10) so a braced wall is
    /// harder to crack. `bar` is floored at 1 (a zero wall would flip every card at once).
    pub fn take_with_toughness(&mut self, raw: u32, bar: u32) -> HitOutcome {
        let bar = bar.max(1);
        let mut out = HitOutcome {
            through: raw,
            ..Default::default()
        };
        self.health_pile += raw;
        while self.health_pile >= bar && self.health.flip_down() {
            self.health_pile -= bar;
            out.cards_flipped += 1;
        }
        if self.health.is_empty() {
            out.down = true;
        }
        out
    }

    /// **Recover**: turn one face-down Health card back up (§5 card-state) — the inverse of a card
    /// flipping down. Clears the current sub-card pile first, then restores one whole card if any are
    /// down. Returns the number of cards turned back up (0 if already at full health). A down Actor with
    /// a card restored is no longer down.
    pub fn recover_card(&mut self) -> u32 {
        self.health_pile = 0;
        if self.health.turn_up() { 1 } else { 0 }
    }

    /// §4.6 **phase boundary**: the sub-threshold pile wipes — banked damage that did not flip a
    /// Health card does **not** carry into the next phase (only Health persists, §2.1). This is the
    /// single place the per-phase accumulator is reset; the round boundary (the Lull) is just the last
    /// such wipe of the round.
    pub fn clear_pile(&mut self) {
        self.health_pile = 0;
    }

    /// Round end (the Lull): partial (sub-bar) damage clears. Identical to [`clear_pile`](Defense::clear_pile)
    /// — kept as a named round-boundary call site (§4.6 Lull); the pile already wiped at each in-round
    /// phase boundary, so by the Lull there is nothing sub-threshold left to clear.
    pub fn end_round(&mut self) {
        self.clear_pile();
    }
}

/// The offensive stats (Spec §2.4): the flat strike force **Might**, the Tempo **count** (Cadence), and
/// the per-Tempo-card **grade** (Finesse — the magnitude weighed in a crossing or evade contest).
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Offense {
    /// Flat strike force (Power-only magnitude, §2.4). Formerly `power`.
    pub might: u32,
    /// The **count** of Tempo cards (§3): how many you start each round with.
    pub cadence: u32,
    /// The **grade** of each Tempo card (§3): its Finesse — the magnitude weighed in a crossing or
    /// evade contest. Irrelevant to a strike's force.
    pub finesse: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn knight() -> Defense {
        // Vitality 8, Toughness 2.
        Defense::new(8, 2)
    }

    #[test]
    fn might_accumulates_into_the_pile_and_toughness_gates_flips() {
        let mut d = knight();
        // Might 1 into the pile; below toughness 2 → no card flips.
        let o = d.take(1);
        assert_eq!(o.cards_flipped, 0);
        // Another Might 1 → pile 2 → one card flips.
        let o = d.take(1);
        assert_eq!(o.cards_flipped, 1);
    }

    #[test]
    fn a_big_hit_flips_several_cards() {
        let mut d = knight(); // toughness 2
        let o = d.take(6); // 6 / 2 = 3 cards
        assert_eq!(o.cards_flipped, 3);
    }

    #[test]
    fn partial_damage_clears_at_round_end() {
        let mut d = knight();
        d.take(1); // pile 1, no flip
        assert_eq!(d.health_pile, 1);
        d.end_round();
        assert_eq!(d.health_pile, 0);
    }

    #[test]
    fn health_deck_flips_front_first_and_recovers_like_the_old_counts() {
        // A uniform deck of 3 cards, bar 2 — must behave exactly like the old {max:3, remaining:3}.
        let mut h = Health::new(3, 2);
        assert_eq!(h.max(), 3);
        assert_eq!(h.remaining(), 3);
        assert_eq!(h.toughness(), 2);
        assert!(!h.is_empty());

        // Flip the front-most face-up card.
        assert!(h.flip_down());
        assert!(h.cards[0].down, "front card flips first");
        assert!(!h.cards[1].down);
        assert_eq!(h.remaining(), 2);
        assert_eq!(h.max(), 3, "max is the deck size, unchanged by flips");
        assert_eq!(
            h.toughness(),
            2,
            "bar still reads from the next face-up card"
        );

        // Flip the rest; once none are up the pool is empty and flip_down reports failure.
        assert!(h.flip_down());
        assert!(h.flip_down());
        assert_eq!(h.remaining(), 0);
        assert!(h.is_empty());
        assert!(!h.flip_down(), "no face-up card left to flip");

        // turn_up recovers front-first (the card at index 0 comes back up first).
        assert!(h.turn_up());
        assert!(!h.cards[0].down, "front card recovers first");
        assert_eq!(h.remaining(), 1);
        assert!(!h.is_empty());

        // Driving the deck through Defense::take_with_toughness matches the old remaining-- loop.
        let mut d = Defense::new(4, 2); // 4 cards, bar 2
        let o = d.take(5); // pile 5 / bar 2 = 2 flips, 1 left in pile
        assert_eq!(o.cards_flipped, 2);
        assert_eq!(d.health.remaining(), 2);
        assert_eq!(d.health_pile, 1);
        // Recover clears the pile and turns one card up.
        assert_eq!(d.recover_card(), 1);
        assert_eq!(d.health.remaining(), 3);
        assert_eq!(d.health_pile, 0);
    }

    #[test]
    fn emptying_the_pool_downs_the_actor() {
        let mut d = Defense::new(2, 1); // vitality 2, toughness 1
        assert!(!d.take(1).down);
        let o = d.take(1);
        assert!(o.down);
        assert!(d.is_down());
    }
}
