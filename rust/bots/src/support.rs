//! Helpers that only the bots need.
//!
//! The casts the bots apply to `ClairVoyancePlayed.Parameter1`/`Parameter2`, which hold any
//! value (a `Part`), since the player who asks may put anything there. They follow the .NET
//! unboxing rules. A cast that .NET refused is an `Err` with the message of the .NET exception
//! (probed on .NET 10), and a bot that needs the value makes no decision:
//!
//! - an unboxing cast (`(TreacheryCardType)o`, `(Faction)o`, `(int)o`, `(float)o`) fails with
//!   NullReferenceException for null;
//! - an enum or `int` unboxes to any enum or to `int`: .NET allows unboxing between an enum and
//!   its underlying type, and between enums of the same underlying type (every game enum is
//!   `int`). `(Faction)(object)TreacheryCardType.Laser` is `Faction.Fremen` (10);
//! - `(float)o` needs a boxed `float` exactly;
//! - a reference cast (`(IHero)o`) gives null for null, and the object for a `Leader`, `Messiah`
//!   or `TreacheryCard`;
//! - anything else fails with InvalidCastException
//!   "Unable to cast object of type 'X' to type 'Y'.".

use crate::prelude::*;
use treachery_engine::messages::part::Part;
use treachery_engine::enum_type::GameEnum;

/// The runtime type name of a boxed value (`o.GetType().FullName`).
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

/// A cast that .NET refused, with the message of its exception.
pub type CastResult<T> = Result<T, String>;

fn invalid_cast<T>(o: &Part, to: &str) -> CastResult<T> {
    Err(format!("Unable to cast object of type '{}' to type '{}'.", boxed_type_name(o), to))
}

fn null_reference<T>() -> CastResult<T> {
    Err("Object reference not set to an instance of an object.".to_string())
}

/// `(E)o` for an enum type `E`: `None` for the absent value of a closed enum, or a number that names no member.
pub fn unbox_enum<E: GameEnum>(o: &Part) -> CastResult<Option<E>> {
    match (o, o.enum_value()) {
        (Part::Null, _) => null_reference(),
        (_, Some((_, value))) => Ok(E::from_value(value)),
        (Part::Number(i), _) => Ok(E::from_value(*i)),
        (other, _) => invalid_cast(other, &format!("Treachery.Shared.{}", E::INFO.name)),
    }
}

/// `(int)o`
pub fn unbox_int(o: &Part) -> CastResult<i32> {
    match (o, o.enum_value()) {
        (Part::Null, _) => null_reference(),
        (_, Some((_, value))) => Ok(value),
        (Part::Number(i), _) => Ok(*i),
        (other, _) => invalid_cast(other, "System.Int32"),
    }
}

/// `(float)o`, for the dial of a Clairvoyance question.
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

/// `(IHero)o`: a reference cast, so null stays null.
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
