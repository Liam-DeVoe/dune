//! Helpers that only the bots need.
//!
//! The casts the bots apply to `ClairVoyancePlayed.Parameter1`/`Parameter2`, which hold any
//! value (a `Part`), since the player who asks may put anything there. A cast that fails is an
//! `Err` with a message, and a bot that needs the value makes no decision:
//!
//! - a cast to an enum, an `int` or a strength (`unbox_enum`, `unbox_int`, `unbox_strength`)
//!   fails for null;
//! - an enum or `int` casts to any enum or to `int` by its number, so a `TreacheryCardType.Laser`
//!   read as a `Faction` is `Faction.Fremen` (10);
//! - a strength needs a strength exactly;
//! - a hero cast (`cast_hero`) gives `None` for null, and the hero for a leader, the Messiah or a
//!   treachery card;
//! - anything else fails with "Unable to cast object of type 'X' to type 'Y'.".

use crate::prelude::*;
use treachery_engine::messages::part::Part;
use treachery_engine::enum_type::GameEnum;

/// The type name that cast error messages give for a value.
pub fn boxed_type_name(o: &Part) -> String {
    match o {
        Part::Null => "null".to_string(),
        Part::Text(_) => "System.String".to_string(),
        Part::Bool(_) => "System.Boolean".to_string(),
        Part::Number(_) => "System.Int32".to_string(),
        Part::Strength(_) => "System.Single".to_string(),
        Part::Location(_) => "Treachery.Shared.Location".to_string(),
        Part::Territory(_) => "Treachery.Shared.Territory".to_string(),
        Part::Leader(_) => "Treachery.Shared.Leader".to_string(),
        Part::Messiah => "Treachery.Shared.Messiah".to_string(),
        Part::Card(_) => "Treachery.Shared.TreacheryCard".to_string(),
        Part::SpiceCard(_) => "Treachery.Shared.ResourceCard".to_string(),
        Part::Payment { .. } => "Treachery.Shared.Payment".to_string(),
        Part::Message(_) => "Treachery.Shared.Message".to_string(),
        Part::Phrase(_) => "Treachery.Shared.MessagePart".to_string(),
        Part::Error(_) => "Treachery.Shared.GameError".to_string(),
        Part::List(_) => "System.Collections.Generic.List`1[System.Object]".to_string(),
        other => match other.enum_value() {
            Some((info, _)) => format!("Treachery.Shared.{}", info.name),
            None => unreachable!("every part that isn't an enum is named above"),
        },
    }
}

/// A cast that failed, with its error message.
pub type CastResult<T> = Result<T, String>;

fn invalid_cast<T>(o: &Part, to: &str) -> CastResult<T> {
    Err(format!("Unable to cast object of type '{}' to type '{}'.", boxed_type_name(o), to))
}

fn null_reference<T>() -> CastResult<T> {
    Err("Object reference not set to an instance of an object.".to_string())
}

/// A parameter as an enum `E`: `None` for the absent value of a closed enum, or a number that names no member.
pub fn unbox_enum<E: GameEnum>(o: &Part) -> CastResult<Option<E>> {
    match (o, o.enum_value()) {
        (Part::Null, _) => null_reference(),
        (_, Some((_, value))) => Ok(E::from_value(value)),
        (Part::Number(i), _) => Ok(E::from_value(*i)),
        (other, _) => invalid_cast(other, &format!("Treachery.Shared.{}", E::INFO.name)),
    }
}

/// A parameter as an `int`.
pub fn unbox_int(o: &Part) -> CastResult<i32> {
    match (o, o.enum_value()) {
        (Part::Null, _) => null_reference(),
        (_, Some((_, value))) => Ok(value),
        (Part::Number(i), _) => Ok(*i),
        (other, _) => invalid_cast(other, "System.Int32"),
    }
}

/// A parameter as a strength, for the dial of a Clairvoyance question.
pub fn unbox_strength(o: &Part) -> CastResult<HalfPoints> {
    match o {
        Part::Null => null_reference(),
        Part::Strength(h) => Ok(*h),
        other => invalid_cast(other, "System.Single"),
    }
}

/// A strength as the `float` the bots' estimates compute with. Exact: a strength is a whole
/// number of halves.
pub fn f32_of(h: HalfPoints) -> f32 {
    h.halves() as f32 / 2.0
}

/// A `float` the bots rank by in `RandomPick`: by `total_cmp`, with -0 equal to 0. The engine
/// ranks only integers and enums.
#[derive(Clone, Copy, Debug)]
pub struct FloatKey(pub f32);

impl treachery_engine::random::RankKey for FloatKey {
    fn rank(&self, other: &Self) -> std::cmp::Ordering {
        (self.0 + 0.0).total_cmp(&(other.0 + 0.0))
    }
}

/// Orders floats with NaN below everything (and equal to NaN), and -0 equal to 0.
pub fn cmp_nan_first(a: f32, b: f32) -> std::cmp::Ordering {
    a.partial_cmp(&b).unwrap_or_else(|| b.is_nan().cmp(&a.is_nan()))
}

/// The larger of two floats: NaN if `a` is NaN, else `b` if `b` is NaN, and 0 of 0 and -0.
pub fn max_or_nan(a: f32, b: f32) -> f32 {
    if a != b {
        if a.is_nan() {
            return a;
        }
        return if b < a { a } else { b };
    }
    if b.is_sign_negative() { a } else { b }
}

/// The smaller of two floats: NaN if `a` is NaN, else `b` if `b` is NaN, and -0 of 0 and -0.
pub fn min_or_nan(a: f32, b: f32) -> f32 {
    if a != b {
        if a.is_nan() {
            return a;
        }
        return if a < b { a } else { b };
    }
    if a.is_sign_negative() { a } else { b }
}

/// A parameter as a hero: null gives `None`.
pub fn cast_hero(o: &Part) -> CastResult<Option<Hero>> {
    match o {
        Part::Null => Ok(None),
        Part::Leader(_) | Part::Messiah | Part::Card(_) => Ok(o.hero()),
        other => invalid_cast(other, "Treachery.Shared.IHero"),
    }
}

#[cfg(test)]
mod tests {
    use super::FloatKey;
    use treachery_engine::random::{new_rng, RandomPick};

    /// -0 and 0 tie, so both are drawn; -1 never is.
    #[test]
    fn float_keys_rank_negative_zero_with_zero() {
        let mut random = new_rng(5);
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..1000 {
            let items = [(0, -0.0f32), (1, 0.0f32), (2, -1.0f32)];
            seen.insert(items.iter().random_highest(&mut random, |x| FloatKey(x.1)).unwrap().0);
        }
        assert_eq!(seen.into_iter().collect::<Vec<_>>(), [0, 1]);
    }
}
