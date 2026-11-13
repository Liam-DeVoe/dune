//! Helpers that only the bots need.
//!
//! The casts the bots apply to `ClairVoyancePlayed.Parameter1`/`Parameter2`, which hold any
//! value (an `ExpressionElement`). They follow the .NET unboxing rules and throw the .NET
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
use treachery_engine::compat::cs;
use treachery_engine::compat::net_enum::NetEnum;

/// The runtime type name of a boxed value (`o.GetType().FullName`).
pub fn boxed_type_name(o: &ExpressionElement) -> String {
    match o {
        ExpressionElement::Null => "null".to_string(),
        ExpressionElement::String(_) => "System.String".to_string(),
        ExpressionElement::Bool(_) => "System.Boolean".to_string(),
        ExpressionElement::Int(_) => "System.Int32".to_string(),
        ExpressionElement::Long(_) => "System.Int64".to_string(),
        ExpressionElement::Float(_) => "System.Single".to_string(),
        ExpressionElement::Double(_) => "System.Double".to_string(),
        ExpressionElement::Enum(v) => format!("Treachery.Shared.{}", v.ty.name),
        ExpressionElement::Location(_) => "Treachery.Shared.Location".to_string(),
        ExpressionElement::Territory(_) => "Treachery.Shared.Territory".to_string(),
        ExpressionElement::Hero(Hero::Leader(_)) => "Treachery.Shared.Leader".to_string(),
        ExpressionElement::Hero(Hero::Messiah) => "Treachery.Shared.Messiah".to_string(),
        ExpressionElement::Hero(Hero::TreacheryCard(_)) => "Treachery.Shared.TreacheryCard".to_string(),
        ExpressionElement::ResourceCard(_) => "Treachery.Shared.ResourceCard".to_string(),
        ExpressionElement::Payment(_) => "Treachery.Shared.Payment".to_string(),
        ExpressionElement::Message(_) => "Treachery.Shared.Message".to_string(),
        ExpressionElement::MessagePart(_) => "Treachery.Shared.MessagePart".to_string(),
        ExpressionElement::Exception(e) => format!("System.{}", e.type_name),
        ExpressionElement::Player(_) => "Treachery.Shared.Player".to_string(),
        ExpressionElement::List { type_name, .. } => type_name.clone(),
    }
}

fn invalid_cast(o: &ExpressionElement, to: &str) -> ! {
    cs::throw(
        "InvalidCastException",
        format!("Unable to cast object of type '{}' to type '{}'.", boxed_type_name(o), to),
    )
}

fn null_reference() -> ! {
    cs::throw("NullReferenceException", cs::default_message("NullReferenceException").to_string())
}

/// `(E)o` for an enum type `E`.
pub fn unbox_enum<E: NetEnum>(o: &ExpressionElement) -> E {
    match o {
        ExpressionElement::Null => null_reference(),
        ExpressionElement::Enum(v) => E::from_value(v.value),
        ExpressionElement::Int(i) => E::from_value(*i),
        other => invalid_cast(other, &format!("Treachery.Shared.{}", E::INFO.name)),
    }
}

/// `(int)o`
pub fn unbox_int(o: &ExpressionElement) -> i32 {
    match o {
        ExpressionElement::Null => null_reference(),
        ExpressionElement::Enum(v) => v.value,
        ExpressionElement::Int(i) => *i,
        other => invalid_cast(other, "System.Int32"),
    }
}

/// `(float)o`
pub fn unbox_float(o: &ExpressionElement) -> f32 {
    match o {
        ExpressionElement::Null => null_reference(),
        ExpressionElement::Float(f) => *f,
        other => invalid_cast(other, "System.Single"),
    }
}

/// `(IHero)o`: a reference cast, so null stays null.
pub fn cast_hero(o: &ExpressionElement) -> Option<Hero> {
    match o {
        ExpressionElement::Null => None,
        ExpressionElement::Hero(h) => h.opt(),
        other => invalid_cast(other, "Treachery.Shared.IHero"),
    }
}
