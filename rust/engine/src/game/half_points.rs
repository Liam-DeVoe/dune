//! Battle strength in half points, and halving whole amounts without floats.

/// A battle strength or a dial, counted in halves: a force without spice support counts half,
/// so every strength is a multiple of 0.5. It prints as `13` or `2.5`, the way C# printed the
/// `float` it replaces.
///
/// It is signed because a total can go below zero: a Bureaucrat's penalty is subtracted from the
/// opponent's total whatever it is (`Battle::determine_skill_penalty`), and a total of -1 loses
/// to a total of 0. Saturating at zero would turn some lost battles into ties.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HalfPoints(i32);

impl HalfPoints {
    pub const ZERO: HalfPoints = HalfPoints(0);
    /// What one force at half strength adds per point of its full strength.
    pub const HALF: HalfPoints = HalfPoints(1);

    /// `halves` half points.
    pub const fn from_halves(halves: i32) -> HalfPoints {
        HalfPoints(halves)
    }

    /// `points` whole points.
    pub const fn whole(points: i32) -> HalfPoints {
        HalfPoints(points.wrapping_mul(2))
    }

    pub const fn halves(self) -> i32 {
        self.0
    }

    /// The whole points, rounded down (`Math.Floor`).
    pub const fn floor(self) -> i32 {
        self.0.div_euclid(2)
    }

    /// Half of this strength, for a force that counts half. `None` if that isn't a whole number
    /// of halves.
    pub fn halved(self) -> Option<HalfPoints> {
        (self.0 % 2 == 0).then_some(HalfPoints(self.0 / 2))
    }
}

impl std::ops::Add for HalfPoints {
    type Output = HalfPoints;
    fn add(self, other: HalfPoints) -> HalfPoints {
        HalfPoints(self.0.wrapping_add(other.0))
    }
}

impl std::ops::AddAssign for HalfPoints {
    fn add_assign(&mut self, other: HalfPoints) {
        *self = *self + other;
    }
}

impl std::ops::Sub for HalfPoints {
    type Output = HalfPoints;
    fn sub(self, other: HalfPoints) -> HalfPoints {
        HalfPoints(self.0.wrapping_sub(other.0))
    }
}

/// The strength of `n` forces of this strength each.
impl std::ops::Mul<i32> for HalfPoints {
    type Output = HalfPoints;
    fn mul(self, n: i32) -> HalfPoints {
        HalfPoints(self.0.wrapping_mul(n))
    }
}

/// `13`, `2.5`, `-0.5`.
impl std::fmt::Display for HalfPoints {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let abs = self.0.unsigned_abs();
        if abs % 2 == 0 {
            write!(f, "{sign}{}", abs / 2)
        } else {
            write!(f, "{sign}{}.5", abs / 2)
        }
    }
}

/// Half of `n`, rounded up (`(int)Math.Ceiling(0.5 * n)`).
pub const fn half_rounded_up(n: i32) -> i32 {
    n.div_euclid(2) + n.rem_euclid(2)
}

/// Half of `n`, rounded toward zero (`(int)(0.5f * n)`).
pub const fn half_rounded_toward_zero(n: i32) -> i32 {
    n / 2
}
