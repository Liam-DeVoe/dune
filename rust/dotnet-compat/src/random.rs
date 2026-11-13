//! `System.Random` constructed with a seed.
//!
//! `new Random(seed)` in .NET 6+ (including .NET 10) uses `Net5CompatSeedImpl`,
//! which wraps `CompatPrng`, Knuth's subtractive generator as shipped in .NET
//! Framework. This is a line-by-line port of
//! `src/libraries/System.Private.CoreLib/src/System/Random.CompatImpl.cs`.
//! All arithmetic is C# `unchecked` (wrapping).

/// `System.Random` constructed with a seed (the legacy Net5CompatSeedImpl /
/// Knuth subtractive generator). Must yield the exact .NET sequence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetRandom {
    seed_array: [i32; 56],
    inext: usize,
    inextp: usize,
}

impl NetRandom {
    /// `new Random(seed)`
    pub fn new(seed: i32) -> Self {
        let mut seed_array = [0i32; 56];
        let subtraction = if seed == i32::MIN { i32::MAX } else { seed.abs() };
        let mut mj: i32 = 161_803_398i32.wrapping_sub(subtraction);
        seed_array[55] = mj;
        let mut mk: i32 = 1;
        let mut ii: usize = 0;
        for _ in 1..55 {
            ii += 21;
            if ii >= 55 {
                ii -= 55;
            }
            seed_array[ii] = mk;
            mk = mj.wrapping_sub(mk);
            if mk < 0 {
                mk = mk.wrapping_add(i32::MAX);
            }
            mj = seed_array[ii];
        }
        for _ in 1..5 {
            for i in 1..56 {
                let mut n = i + 30;
                if n >= 55 {
                    n -= 55;
                }
                seed_array[i] = seed_array[i].wrapping_sub(seed_array[1 + n]);
                if seed_array[i] < 0 {
                    seed_array[i] = seed_array[i].wrapping_add(i32::MAX);
                }
            }
        }
        NetRandom { seed_array, inext: 0, inextp: 21 }
    }

    /// `CompatPrng.InternalSample()`: a value in `[0, int.MaxValue)`.
    fn internal_sample(&mut self) -> i32 {
        let mut loc_inext = self.inext + 1;
        if loc_inext >= 56 {
            loc_inext = 1;
        }
        let mut loc_inextp = self.inextp + 1;
        if loc_inextp >= 56 {
            loc_inextp = 1;
        }
        let mut ret = self.seed_array[loc_inext].wrapping_sub(self.seed_array[loc_inextp]);
        if ret == i32::MAX {
            ret -= 1;
        }
        if ret < 0 {
            ret = ret.wrapping_add(i32::MAX);
        }
        self.seed_array[loc_inext] = ret;
        self.inext = loc_inext;
        self.inextp = loc_inextp;
        ret
    }

    /// `CompatPrng.Sample()`
    fn sample(&mut self) -> f64 {
        self.internal_sample() as f64 * (1.0 / i32::MAX as f64)
    }

    /// `CompatPrng.GetSampleForLargeRange()`
    fn sample_for_large_range(&mut self) -> f64 {
        let mut result = self.internal_sample();
        let negative = self.internal_sample() % 2 == 0;
        if negative {
            result = -result;
        }
        let mut d = result as f64;
        d += (i32::MAX - 1) as f64;
        d /= (2u32.wrapping_mul(i32::MAX as u32).wrapping_sub(1)) as f64;
        d
    }

    /// `Random.Next()`: `[0, int.MaxValue)`.
    pub fn next(&mut self) -> i32 {
        self.internal_sample()
    }

    /// `Random.Next(maxValue)`; panics if max < 0, as .NET throws.
    /// `Next(0)` returns 0 but still consumes one sample, as in .NET.
    pub fn next_max(&mut self, max_value: i32) -> i32 {
        if max_value < 0 {
            crate::exception::throw(
                "ArgumentOutOfRangeException",
                format!("maxValue ('{max_value}') must be a non-negative value. (Parameter 'maxValue')\nActual value was {max_value}."),
            );
        }
        (self.sample() * max_value as f64) as i32
    }

    /// `Random.Next(minValue, maxValue)`; throws if min > max.
    /// `Next(x, x)` returns x but still consumes a sample, as in .NET.
    pub fn next_range(&mut self, min_value: i32, max_value: i32) -> i32 {
        if min_value > max_value {
            crate::exception::throw(
                "ArgumentOutOfRangeException",
                "'minValue' cannot be greater than maxValue. (Parameter 'minValue')".to_string(),
            );
        }
        let range = max_value as i64 - min_value as i64;
        if range <= i32::MAX as i64 {
            ((self.sample() * range as f64) as i32).wrapping_add(min_value)
        } else {
            (((self.sample_for_large_range() * range as f64) as i64).wrapping_add(min_value as i64)) as i32
        }
    }

    /// `Random.NextDouble()`: `[0, 1)`.
    pub fn next_double(&mut self) -> f64 {
        self.sample()
    }
}
