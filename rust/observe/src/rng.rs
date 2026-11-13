//! The recorder's own random numbers: SplitMix64. Only the recorder draws from it (game
//! configuration, fuzz choices, bot order); the engine and the bots keep their own generators,
//! seeded from values written into the trace.

#[derive(Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    pub fn f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform in [0, n); 0 when n is 0.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next_u64() % n as u64) as usize }
    }

    /// Uniform in [lo, hi).
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo { lo } else { lo + self.below((hi - lo) as usize) as i32 }
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.f64() < p
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() { None } else { Some(&items[self.below(items.len())]) }
    }

    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for n in (1..items.len()).rev() {
            let k = self.below(n + 1);
            items.swap(n, k);
        }
    }

    /// A non-negative i32 seed for another generator.
    pub fn seed(&mut self) -> i32 {
        (self.next_u64() & 0x7FFF_FFFF) as i32
    }
}

/// The seed of game `index` of a run, from the run's seed and the mode.
pub fn game_seed(mode: &str, seed: u64, index: u64) -> u64 {
    let salt: u64 = if mode == "fuzz" { 0x4655_5A5A } else { 0x424F_5453 };
    let mut r = Rng::new(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ index.wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ salt);
    r.next_u64()
}
