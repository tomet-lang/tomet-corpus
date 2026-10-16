//! SplitMix64: tiny, fast, and fixed forever -- the same seed yields the same
//! stream on every platform and release, which is the property benchmarks need.

#[derive(Debug, Clone)]
pub(crate) struct Rng(u64);

impl Rng {
    /// Independent stream for `(seed, index)`, so any document can be
    /// generated on its own without replaying the ones before it.
    pub(crate) fn for_index(seed: u64, index: u64) -> Self {
        let mut r = Rng(seed ^ index.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        // Decorrelate neighbouring indices.
        r.next_u64();
        r.next_u64();
        r
    }

    pub(crate) fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..n` (`n > 0`). Modulo bias is irrelevant at these sizes.
    pub(crate) fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    /// Uniform in `lo..=hi`.
    pub(crate) fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }

    /// True with probability `permille / 1000`.
    pub(crate) fn chance(&mut self, permille: u16) -> bool {
        self.below(1000) < permille as usize
    }

    pub(crate) fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }

    /// Index into `weights`, proportionally. All-zero weights return 0.
    pub(crate) fn weighted(&mut self, weights: &[u32]) -> usize {
        let total: u64 = weights.iter().map(|&w| w as u64).sum();
        if total == 0 {
            return 0;
        }
        let mut roll = self.next_u64() % total;
        for (i, &w) in weights.iter().enumerate() {
            if roll < w as u64 {
                return i;
            }
            roll -= w as u64;
        }
        weights.len() - 1
    }
}
