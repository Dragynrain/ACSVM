// SPDX-License-Identifier: GPL-3.0-or-later

//! The game's single source of randomness: SplitMix64, seeded once.
//!
//! Every random decision in the rules draws from one [`Rng`] in a fixed
//! order, so a game is replayed exactly from its seed and command log. The
//! generator is written out here (rather than taken from a crate) so that its
//! output can never change under us with a dependency update.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..n` (`n > 0`), without modulo bias.
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0, "below(0)");
        let n = n as u64;
        let zone = u64::MAX - u64::MAX % n;
        loop {
            let v = self.next_u64();
            if v < zone {
                return (v % n) as u32;
            }
        }
    }

    /// True with probability `percent`/100. Draws even when `percent` is 0
    /// or 100, so callers decide whether a roll happens at all.
    pub fn percent(&mut self, percent: u8) -> bool {
        self.below(100) < percent as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_stream() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn splitmix_reference_value() {
        // First output of SplitMix64 seeded with 0 (published test vector).
        assert_eq!(Rng::new(0).next_u64(), 0xE220_A839_7B1D_CDAF);
    }

    #[test]
    fn percent_rate_is_close() {
        let mut r = Rng::new(7);
        let hits = (0..10_000).filter(|_| r.percent(30)).count();
        assert!((2_800..3_200).contains(&hits), "{hits}");
        assert!(!(0..100).any(|_| r.percent(0)));
        assert!((0..100).all(|_| r.percent(100)));
    }
}
