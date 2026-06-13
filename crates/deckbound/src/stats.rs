//! Numeric vocabulary for Deckbound combat: damage types, armor, and the
//! health-as-cards `Body` model.
//!
//! All numbers are first-pass and tunable; they mirror the design appendix
//! (`docs/games/deckbound/design/rulebook.md`). Damage resolves as
//! `(raw − armor[type]) / toughness` cards flipped — so a heavy hit barely
//! scratches a high-toughness foe, and armor that does nothing to a type lets
//! it through whole.

/// The kind of damage a blow carries. Armor stops some types and not others.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DamageType {
    Sharp,
    Blunt,
    Heat,
    Cold,
}

impl DamageType {
    /// A short display name.
    pub fn name(self) -> &'static str {
        match self {
            DamageType::Sharp => "sharp",
            DamageType::Blunt => "blunt",
            DamageType::Heat => "heat",
            DamageType::Cold => "cold",
        }
    }
}

/// Per-type damage reduction — one suit of armor. Reductions never stack across
/// sources; this models a single plate / hide.
#[derive(Clone, Debug, Default)]
pub struct Armor {
    reductions: Vec<(DamageType, u32)>,
}

impl Armor {
    /// Armor that stops nothing.
    pub fn none() -> Self {
        Self::default()
    }

    /// Armor with the given per-type reductions.
    pub fn new(reductions: Vec<(DamageType, u32)>) -> Self {
        Self { reductions }
    }

    /// How much damage of `ty` this armor absorbs (0 if it does nothing to it).
    pub fn reduce(&self, ty: DamageType) -> u32 {
        self.reductions
            .iter()
            .find(|(t, _)| *t == ty)
            .map(|(_, v)| *v)
            .unwrap_or(0)
    }

    /// Whether this armor reduces anything at all.
    pub fn is_some(&self) -> bool {
        !self.reductions.is_empty()
    }
}

/// Health as a stack of cards. `remaining` of `max` cards are still standing;
/// each absorbs `toughness` damage (after armor) before it flips. The number of
/// cards a hit flips is `floor(damage_after_armor / toughness)`.
#[derive(Clone, Debug)]
pub struct Body {
    pub max: u32,
    pub remaining: u32,
    pub toughness: u32,
}

impl Body {
    /// `count` cards, each of the given toughness (clamped to at least 1).
    pub fn new(count: u32, toughness: u32) -> Self {
        Self {
            max: count,
            remaining: count,
            toughness: toughness.max(1),
        }
    }

    /// Whether every card has been flipped — the actor is down.
    pub fn is_down(&self) -> bool {
        self.remaining == 0
    }

    /// Damage already reduced by armor, expressed as cards flipped here.
    pub fn flips_for(&self, after_armor: u32) -> u32 {
        (after_armor / self.toughness).min(self.remaining)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toughness_divides_damage_into_flipped_cards() {
        let body = Body::new(8, 3);
        // A heat hit of 5 against toughness 3 flips one card (5 / 3 = 1).
        assert_eq!(body.flips_for(5), 1);
        // Against toughness 1 the same 5 would flip five.
        assert_eq!(Body::new(8, 1).flips_for(5), 5);
    }

    #[test]
    fn armor_stops_some_types_and_not_others() {
        let plate = Armor::new(vec![(DamageType::Sharp, 4), (DamageType::Blunt, 3)]);
        assert_eq!(plate.reduce(DamageType::Sharp), 4);
        assert_eq!(plate.reduce(DamageType::Heat), 0); // heat passes whole
    }
}
