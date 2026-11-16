//! Helpers that only the bots need.
//!
//! The casts the bots apply to `ClairVoyancePlayed.Parameter1`/`Parameter2`, which hold any
//! value (a `Part`). They follow the .NET unboxing rules and throw the .NET
//! exceptions and messages (probed on .NET 10):
//!
//! - an unboxing cast (`(TreacheryCardType)o`, `(Faction)o`, `(int)o`, `(float)o`) throws
//!   NullReferenceException for null;
//! - an enum or `int` unboxes to any enum or to `int`: .NET allows unboxing between an enum and
//!   its underlying type, and between enums of the same underlying type (every game enum is
//!   `int`). `(Faction)(object)TreacheryCardType.Laser` is `Faction.Fremen` (10);
//! - `(float)o` needs a boxed `float` exactly;
//! - a reference cast (`(IHero)o`) gives null for null, and the object for a `Leader`, `Messiah`
//!   or `TreacheryCard`;
//! - anything else throws InvalidCastException
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
        Part::Float(_) => "System.Single".to_string(),
        Part::Double(_) => "System.Double".to_string(),
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

fn invalid_cast(o: &Part, to: &str) -> ! {
    panic!("Unable to cast object of type '{}' to type '{}'.", boxed_type_name(o), to)
}

fn null_reference() -> ! {
    panic!("Object reference not set to an instance of an object.")
}

/// `(E)o` for an enum type `E`: `None` for the absent value of a closed enum, or a number that names no member.
pub fn unbox_enum<E: GameEnum>(o: &Part) -> Option<E> {
    match o {
        Part::Null => null_reference(),
        other if other.enum_value().is_some() => E::from_value(other.enum_value().expect("an enum").1),
        Part::Number(i) => E::from_value(*i),
        other => invalid_cast(other, &format!("Treachery.Shared.{}", E::INFO.name)),
    }
}

/// `(int)o`
pub fn unbox_int(o: &Part) -> i32 {
    match o {
        Part::Null => null_reference(),
        other if other.enum_value().is_some() => other.enum_value().expect("an enum").1,
        Part::Number(i) => *i,
        other => invalid_cast(other, "System.Int32"),
    }
}

/// `(float)o`
pub fn unbox_float(o: &Part) -> f32 {
    match o {
        Part::Null => null_reference(),
        Part::Float(f) => *f,
        other => invalid_cast(other, "System.Single"),
    }
}

/// `(IHero)o`: a reference cast, so null stays null.
pub fn cast_hero(o: &Part) -> Option<Hero> {
    match o {
        Part::Null => None,
        Part::Leader(_) | Part::Messiah | Part::Card(_) => o.hero(),
        other => invalid_cast(other, "Treachery.Shared.IHero"),
    }
}
