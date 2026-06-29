//! A small, dependency-free, deterministic pseudo-random number generator.
//!
//! Games seed an [`Rng`] from a `u64` so that an entire game can be replayed
//! bit-for-bit from its seed. This is the SplitMix64 algorithm, which is fast,
//! has no external dependencies, and is more than adequate for shuffling.

/// A seeded SplitMix64 generator.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// Creates a generator with the given seed. The same seed always produces
    /// the same sequence.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Returns the next 64-bit value in the sequence.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Returns a value in `0..bound`. Returns `0` when `bound` is `0`.
    pub fn below(&mut self, bound: usize) -> usize {
        if bound == 0 {
            return 0;
        }
        (self.next_u64() % bound as u64) as usize
    }

    /// Shuffles `items` in place using an unbiased Fisher-Yates shuffle.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        let len = items.len();
        if len < 2 {
            return;
        }
        for i in (1..len).rev() {
            let j = self.below(i + 1);
            items.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = Rng::new(1);
        let mut b = Rng::new(2);
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn shuffle_is_a_permutation() {
        let mut rng = Rng::new(7);
        let mut items: Vec<u32> = (0..50).collect();
        rng.shuffle(&mut items);
        let mut sorted = items.clone();
        sorted.sort();
        assert_eq!(sorted, (0..50).collect::<Vec<_>>());
    }

    #[test]
    fn shuffle_is_deterministic() {
        let mut a = Rng::new(99);
        let mut b = Rng::new(99);
        let mut x: Vec<u32> = (0..20).collect();
        let mut y = x.clone();
        a.shuffle(&mut x);
        b.shuffle(&mut y);
        assert_eq!(x, y);
    }
}
