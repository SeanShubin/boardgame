//! Stats and the **pile → bar → pool** defense model.
//!
//! Combat has **one damage channel** (Spec §2.2): every attack deals untyped **Might** into the
//! **health pile**; each time the pile clears the **bar** (Toughness) one Health card flips face down.
//! Empty the health pool and the Actor is **down** — the game's single kill-condition (Charter #13).
//! There is **no cut** today — Armor and damage *types* are deferred to the later gear system
//! (`future-possibilities.md` §7). The old inner **Fear/Spirit** channel was collapsed out (2026).
//! See `docs/games/deckbound/notes/form-and-defeat.md`.

/// The **health pool**: a stack of generic Health cards, each absorbing `toughness` damage. The only
/// maintained meter. `max`/`remaining` are the **Vitality** count; `toughness` the per-card magnitude.
#[derive(Clone, Debug)]
pub struct Health {
    pub max: u32,
    pub remaining: u32,
    pub toughness: u32,
}

impl Health {
    pub fn new(count: u32, toughness: u32) -> Self {
        Self {
            max: count,
            remaining: count,
            toughness: toughness.max(1),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.remaining == 0
    }
}

/// Everything that defends an Actor. **Toughness** (the bar) is a passive stat read off the Form; only
/// the **health pool** is a maintained meter. `health_pile` is a round-scoped accumulator that clears at
/// round end.
#[derive(Clone, Debug)]
pub struct Defense {
    pub health: Health,

    // round-scoped pile (cleared at round end)
    pub health_pile: u32,
}

/// What a single hit did.
#[derive(Clone, Copy, Debug, Default)]
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
        let mut out = HitOutcome {
            through: raw,
            ..Default::default()
        };
        self.health_pile += raw;
        while self.health_pile >= self.health.toughness && self.health.remaining > 0 {
            self.health.remaining -= 1;
            self.health_pile -= self.health.toughness;
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
        if self.health.remaining < self.health.max {
            self.health.remaining += 1;
            1
        } else {
            0
        }
    }

    /// Round end: partial (sub-bar) damage clears.
    pub fn end_round(&mut self) {
        self.health_pile = 0;
    }
}

/// The offensive stats (Spec §2.4): the flat strike force **Might**, the Tempo **count** (Cadence), and
/// the per-Tempo-card **grade** (Finesse — the magnitude weighed in a crossing or evade contest).
#[derive(Clone, Copy, Debug, Default)]
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
    fn emptying_the_pool_downs_the_actor() {
        let mut d = Defense::new(2, 1); // vitality 2, toughness 1
        assert!(!d.take(1).down);
        let o = d.take(1);
        assert!(o.down);
        assert!(d.is_down());
    }
}
