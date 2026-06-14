//! Health as cards.
//!
//! A `Body` is a little stack of cards; a hit flips `floor(damage / toughness)`
//! of them. With toughness 1 (the common case for the duel sandbox) damage maps
//! one-to-one to cards. First-pass, tunable.

#[derive(Clone, Debug)]
pub struct Body {
    pub max: u32,
    pub remaining: u32,
    pub toughness: u32,
}

impl Body {
    /// `count` cards, each absorbing `toughness` damage (clamped to >= 1).
    pub fn new(count: u32, toughness: u32) -> Self {
        Self {
            max: count,
            remaining: count,
            toughness: toughness.max(1),
        }
    }

    pub fn is_down(&self) -> bool {
        self.remaining == 0
    }

    /// How many cards a `raw` hit flips (capped at what remains).
    pub fn flips_for(&self, raw: u32) -> u32 {
        (raw / self.toughness).min(self.remaining)
    }

    /// Apply a `raw` hit; returns the number of cards flipped.
    pub fn take(&mut self, raw: u32) -> u32 {
        let flips = self.flips_for(raw);
        self.remaining -= flips;
        flips
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toughness_divides_damage() {
        let mut body = Body::new(8, 2);
        assert_eq!(body.take(5), 2); // 5 / 2 = 2 cards
        assert_eq!(body.remaining, 6);
    }

    #[test]
    fn a_body_goes_down_and_caps() {
        let mut body = Body::new(3, 1);
        assert_eq!(body.take(10), 3); // capped at remaining
        assert!(body.is_down());
    }
}
