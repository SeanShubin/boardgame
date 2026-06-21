//! Stats and the **cut → bar → pool** defense model.
//!
//! Every attack is one of two **channels** — outer **Body** (physical, including conjured
//! elements) or inner **Fear** (Spirit). Each resolves the same way: subtract a per-source **cut**
//! (Armor outer, Ward inner), accumulate into the round's **pile**, then test the **bar** — only the
//! Body channel has a **pool** (Health cards) behind it; the inner (Fear) channel **breaks** on one
//! crossing. *(The Mind / Confusion channel was removed 2026-06-20 with the Tempo/Focus merge — Spec
//! §2 / §3.2.)* See `docs/games/deckbound/notes/form-and-defeat.md`.

use std::collections::BTreeMap;

use serde::Deserialize;

/// A damage type. Physical/elemental types are outer (met by Armor); Fear is inner (met by Ward).
/// `Confusion` is retained as a type but now flows through the **Fear** channel (the Mind channel is
/// gone) — a mental attack on the will.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub enum DamageType {
    Blunt,
    Sharp,
    Pierce,
    Heat,
    Cold,
    Lightning,
    Fear,
    Confusion,
}

/// Which defense channel a damage type flows through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    /// Outer: Armor → Toughness → the Body pool.
    Body,
    /// Inner: Ward(vs-fear) → Resolve, no pool.
    Fear,
}

impl DamageType {
    pub fn channel(self) -> Channel {
        match self {
            // Fear and the former Confusion both attack the will (the Mind channel is gone, §3.2).
            DamageType::Fear | DamageType::Confusion => Channel::Fear,
            _ => Channel::Body,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DamageType::Blunt => "blunt",
            DamageType::Sharp => "sharp",
            DamageType::Pierce => "pierce",
            DamageType::Heat => "heat",
            DamageType::Cold => "cold",
            DamageType::Lightning => "lightning",
            DamageType::Fear => "fear",
            DamageType::Confusion => "confusion",
        }
    }
}

/// The Body **pool**: a stack of generic Health cards, each absorbing `toughness`
/// damage. The only maintained meter.
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

/// The will break an inner (Fear) crossing produces. Tiers scale with how far the
/// pile clears the bar (past R / 2R / 3R) — first-pass knob.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Break {
    /// Spirit: held this round, can't press.
    Freeze,
    /// Spirit: routed — flees.
    Flee,
    /// Spirit: the lethal extreme — bleeds into the Body pool.
    ScaredToDeath,
}

/// Everything that defends an Actor. The bar (Resolve) and cuts (Armor, Ward) are **passive
/// stats**; only the Body pool is a maintained meter. The `*_pile` fields are round-scoped
/// accumulators that clear at round end.
#[derive(Clone, Debug)]
pub struct Defense {
    pub body: Health,
    /// Outer per-source cut by type.
    pub armor: BTreeMap<DamageType, u32>,
    /// Inner per-source cut by type (Fear).
    pub ward: BTreeMap<DamageType, u32>,
    /// Spirit bar — fear must exceed it.
    pub resolve: u32,

    // round-scoped piles (cleared at round end)
    pub body_pile: u32,
    pub fear_pile: u32,
    // this-round break flag (lifted at round end)
    pub will_break: Option<Break>,
}

/// What a single hit did.
#[derive(Clone, Copy, Debug, Default)]
pub struct HitOutcome {
    /// Damage that got **through** the cut (Armor/Ward) into the round's pile — what "accumulates"
    /// before any health card turns. 0 = the blow was turned aside entirely.
    pub through: u32,
    /// Health cards turned **face down** by this hit (the body pile crossing toughness).
    pub cards_flipped: u32,
    pub broke: Option<Break>,
    /// The Body pool emptied — the Actor is knocked out.
    pub down: bool,
}

impl Defense {
    pub fn new(body_count: u32, toughness: u32, resolve: u32) -> Self {
        Self {
            body: Health::new(body_count, toughness),
            armor: BTreeMap::new(),
            ward: BTreeMap::new(),
            resolve,
            body_pile: 0,
            fear_pile: 0,
            will_break: None,
        }
    }

    /// Body gone → out of the fight.
    pub fn is_down(&self) -> bool {
        self.body.is_empty()
    }

    fn armor_cut(&self, dtype: DamageType, precision: u32) -> u32 {
        let raw = self.armor.get(&dtype).copied().unwrap_or(0);
        // Precision bypasses some Armor.
        raw.saturating_sub(precision)
    }

    fn ward_cut(&self, dtype: DamageType) -> u32 {
        self.ward.get(&dtype).copied().unwrap_or(0)
    }

    /// Apply one `raw`-magnitude hit of `dtype`, with attacker `precision`.
    /// Routes through the channel's cut → bar → pool/break.
    pub fn take(&mut self, raw: u32, dtype: DamageType, precision: u32) -> HitOutcome {
        let mut out = HitOutcome::default();
        match dtype.channel() {
            Channel::Body => {
                let eff = raw.saturating_sub(self.armor_cut(dtype, precision));
                out.through = eff;
                self.body_pile += eff;
                while self.body_pile >= self.body.toughness && self.body.remaining > 0 {
                    self.body.remaining -= 1;
                    self.body_pile -= self.body.toughness;
                    out.cards_flipped += 1;
                }
                if self.body.is_empty() {
                    out.down = true;
                }
            }
            Channel::Fear => {
                let eff = raw.saturating_sub(self.ward_cut(DamageType::Fear));
                out.through = eff;
                self.fear_pile += eff;
                if self.fear_pile > self.resolve {
                    let tier = will_tier(self.fear_pile, self.resolve);
                    self.will_break = Some(tier);
                    out.broke = Some(tier);
                    if tier == Break::ScaredToDeath {
                        // Bleeds into the Body pool: flip a card.
                        if self.body.remaining > 0 {
                            self.body.remaining -= 1;
                            out.cards_flipped += 1;
                        }
                        if self.body.is_empty() {
                            out.down = true;
                        }
                    }
                }
            }
        }
        out
    }

    /// **Recover**: turn one face-down Health card back up (§5 card-state) — the inverse of a card
    /// flipping down. Clears the current sub-card pile first, then restores one whole card if any are
    /// down. Returns the number of cards turned back up (0 if already at full Body). A down Actor with
    /// a card restored is no longer down.
    pub fn recover_card(&mut self) -> u32 {
        self.body_pile = 0;
        if self.body.remaining < self.body.max {
            self.body.remaining += 1;
            1
        } else {
            0
        }
    }

    /// Round end: partial (sub-bar) damage clears, and this-round breaks lift.
    /// A `ScaredToDeath` already bled into the (permanent) Body pool, so only the
    /// transient will flag resets.
    pub fn end_round(&mut self) {
        self.body_pile = 0;
        self.fear_pile = 0;
        self.will_break = None;
    }
}

/// Tier a will break by how far the fear pile clears Resolve (past R / 2R / 3R).
fn will_tier(pile: u32, resolve: u32) -> Break {
    if pile > resolve.saturating_mul(3) {
        Break::ScaredToDeath
    } else if pile > resolve.saturating_mul(2) {
        Break::Flee
    } else {
        Break::Freeze
    }
}

/// The offensive stats: how hard, how precise, how fast, the force of fear, and the **grade** of
/// each Tempo card (Daring, §3 — the magnitude that decides a gauntlet crossing).
#[derive(Clone, Copy, Debug, Default)]
pub struct Offense {
    pub power: u32,
    pub precision: u32,
    /// The **count** of Tempo cards (§3): how many you start each round with.
    pub speed: u32,
    /// The **grade** of each Tempo card (§3): its Daring — the magnitude weighed in a gauntlet
    /// crossing (the Infiltrator's slip-grade). Irrelevant to a strike's force.
    pub daring: u32,
    /// The Controller's fear-projection (Bone): scales an inner (Fear-channel) attack, mirroring
    /// Strike↔Body (§2.2). Named for its Role — a Controller projects **Dread** (Charter #12).
    pub dread: u32,
    /// The Support's force-multiplier (Salt): each augment it plays — Mend / Rally / Haste / Empower —
    /// gains +Inspiration on its magnitude (§2.4), mirroring how an attack gains Strike/Dread. The
    /// Salt role's signature stat (Charter #12).
    pub inspiration: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn knight() -> Defense {
        let mut d = Defense::new(8, 2, 4);
        d.armor.insert(DamageType::Sharp, 3);
        d.armor.insert(DamageType::Heat, 0);
        d
    }

    #[test]
    fn armor_then_toughness_gates_body_damage() {
        let mut d = knight();
        // Sharp 4 - armor 3 = 1 into the pile; below toughness 2 → no card flips.
        let o = d.take(4, DamageType::Sharp, 0);
        assert_eq!(o.cards_flipped, 0);
        // Another sharp 4 → pile 2 → one card flips.
        let o = d.take(4, DamageType::Sharp, 0);
        assert_eq!(o.cards_flipped, 1);
    }

    #[test]
    fn heat_bypasses_plate() {
        let mut d = knight(); // heat armor 0
        let o = d.take(6, DamageType::Heat, 0); // 6 / toughness 2 = 3 cards
        assert_eq!(o.cards_flipped, 3);
    }

    #[test]
    fn precision_shaves_armor() {
        let mut d = knight();
        // Sharp 4, precision 3 → armor 3-3=0 cut → 4 into pile → 2 cards.
        let o = d.take(4, DamageType::Sharp, 3);
        assert_eq!(o.cards_flipped, 2);
    }

    #[test]
    fn partial_body_damage_clears_at_round_end() {
        let mut d = knight();
        d.take(4, DamageType::Sharp, 0); // pile 1, no flip
        assert_eq!(d.body_pile, 1);
        d.end_round();
        assert_eq!(d.body_pile, 0);
    }

    #[test]
    fn fear_must_exceed_resolve_and_tiers_up() {
        let mut d = knight(); // resolve 4
        assert!(d.take(4, DamageType::Fear, 0).broke.is_none()); // 4 !> 4
        d.end_round();
        assert_eq!(d.take(5, DamageType::Fear, 0).broke, Some(Break::Freeze)); // >R
        d.end_round();
        assert_eq!(d.take(9, DamageType::Fear, 0).broke, Some(Break::Flee)); // >2R
        d.end_round();
        assert_eq!(
            d.take(13, DamageType::Fear, 0).broke,
            Some(Break::ScaredToDeath)
        ); // >3R
    }

    #[test]
    fn ward_shaves_each_fright() {
        let mut d = knight();
        d.ward.insert(DamageType::Fear, 2);
        // Three Fear-3 hits, each shaved to 1 → pile 3 ≤ resolve 4 → holds.
        for _ in 0..3 {
            assert!(d.take(3, DamageType::Fear, 0).broke.is_none());
        }
        assert_eq!(d.fear_pile, 3);
    }
}
