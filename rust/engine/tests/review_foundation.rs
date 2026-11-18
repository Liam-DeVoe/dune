//! Edge cases of the foundation layer (messages, random helpers, JSON, map lookups). Each test
//! failed until its bug was fixed (see rust/docs/escapes.md), except the ones marked as guards.

use treachery_engine::enumerations::*;
use treachery_engine::messages::part::Part;

// ---------------------------------------------------------------- enumerations.rs


// ---------------------------------------------------------------- extension_methods.rs

// ---------------------------------------------------------------- part.rs / message.rs

/// A strength prints whole numbers without a decimal point, halves with ".5", and the sign before
/// both.
#[test]
fn strength_plain_text() {
    use treachery_engine::game::HalfPoints;
    for (halves, text) in [(0, "0"), (1, "0.5"), (5, "2.5"), (26, "13"), (-1, "-0.5"), (-2, "-1"), (-7, "-3.5"), (20_000_001, "10000000.5")] {
        assert_eq!(Part::Strength(HalfPoints::from_halves(halves)).plain_text(), text);
    }
}

/// Halving rounds as float `ceil`, truncation and `floor` do, negative amounts included.
#[test]
fn halves_round_like_float_ceil_truncate_and_floor() {
    use treachery_engine::game::half_points::{half_rounded_toward_zero, half_rounded_up};
    use treachery_engine::game::HalfPoints;
    for n in -9..=9 {
        assert_eq!(half_rounded_up(n), (0.5 * n as f64).ceil() as i32, "{n}");
        assert_eq!(half_rounded_toward_zero(n), (0.5f32 * n as f32) as i32, "{n}");
        assert_eq!(HalfPoints::from_halves(n).floor(), (n as f64 / 2.0).floor() as i32, "{n}");
    }
    assert_eq!(half_rounded_up(i32::MAX), 1 << 30);
    assert_eq!(half_rounded_up(i32::MIN), -(1 << 30));
}

/// A game error prints as its message.
#[test]
fn game_error_plain_text() {
    let e = treachery_engine::error::GameError::new("Cannot draw from an empty deck");
    assert_eq!(Part::Error(e).plain_text(), "Cannot draw from an empty deck");
}

#[test]
fn list_plain_text() {
    let l = Part::from(vec![Faction::Atreides, Faction::Fremen]);
    assert_eq!(l.plain_text(), "[Atreides, Fremen]");
}
