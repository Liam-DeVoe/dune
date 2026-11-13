//! How a C# enum is represented in Rust. Support module (no C# counterpart).
//!
//! A C# enum is an `int` with names attached. Any `int` is a valid value: JSON
//! deserialization accepts `13` for a `Faction`, casts like `(Faction)x` never
//! fail, and `ToString()` of an undefined value prints the number. The fuzzer
//! sends such values, so a closed Rust `enum` would reject inputs that C#
//! accepts. Each C# enum therefore becomes an open newtype over `i32`:
//!
//! ```
//! use treachery_engine::enumerations::Faction;
//! let f = Faction::Fremen;              // associated const, same name as in C#
//! assert_eq!(f.0, 10);
//! assert!(Faction::Atreides > Faction::Fremen); // ordered by value, like C#
//! assert_eq!(Faction(13).to_string(), "13"); // undefined values are allowed
//! match f {
//!     Faction::Fremen => {}
//!     _ => unreachable!(),             // a `_` arm is always required
//! }
//! ```

use std::fmt;

/// Static description of a C# enum type.
#[derive(Debug)]
pub struct EnumTypeInfo {
    /// C# type name, e.g. `"Faction"`.
    pub name: &'static str,
    /// `[Flags]` attribute present.
    pub is_flags: bool,
    /// Members in declaration order.
    pub declared: &'static [(&'static str, i32)],
}

impl EnumTypeInfo {
    /// `Enum.GetName`: the name of the first declared member with this value.
    pub fn name_of(&self, value: i32) -> Option<&'static str> {
        self.declared.iter().find(|(_, v)| *v == value).map(|(n, _)| *n)
    }

    /// `Enum.ToString()` with .NET semantics, including `[Flags]` combinations
    /// (`"Cross, ToHomeworld"`) and undefined values (printed as a number).
    pub fn format(&self, value: i32) -> String {
        if let Some(n) = self.name_of(value) {
            return n.to_string();
        }
        if !self.is_flags || value == 0 {
            return value.to_string();
        }
        // .NET: walk members from the highest value down, taking every member
        // whose bits are all set; fail if bits remain. Names are printed in
        // ascending value order.
        let mut members: Vec<(&'static str, u32)> =
            self.declared.iter().map(|(n, v)| (*n, *v as u32)).collect();
        members.sort_by_key(|(_, v)| *v);
        members.dedup_by_key(|(_, v)| *v);
        let mut remaining = value as u32;
        let mut taken = Vec::new();
        for (n, v) in members.iter().rev() {
            if *v != 0 && remaining & v == *v {
                remaining -= v;
                taken.push(*n);
            }
        }
        if remaining != 0 {
            return value.to_string();
        }
        taken.reverse();
        taken.join(", ")
    }
}

/// Implemented by every type generated with [`net_enum!`].
pub trait NetEnum: Copy + Eq + Ord + std::hash::Hash + fmt::Debug + 'static {
    const INFO: &'static EnumTypeInfo;
    /// `Enum.GetValues<T>()` order (sorted by unsigned value).
    const ALL: &'static [Self];
    fn value(self) -> i32;
    fn from_value(value: i32) -> Self;
}

/// Declares a C# enum as an open `i32` newtype. Used by the generated
/// `enumerations.rs`; use it by hand for enums declared in other C# files.
///
/// ```ignore
/// net_enum! {
///     /// C# `BattleFactKind`
///     BattleFactKind { Strength = 0, Winner = 1, }
///     get_values [Strength, Winner]
/// }
/// ```
macro_rules! net_enum {
    ($(#[$m:meta])* flags $name:ident { $($var:ident = $val:expr,)* } get_values [$($gv:ident),* $(,)?]) => {
        $crate::compat::net_enum::net_enum!(@impl true, $(#[$m])* $name { $($var = $val,)* } [$($gv),*]);
    };
    ($(#[$m:meta])* $name:ident { $($var:ident = $val:expr,)* } get_values [$($gv:ident),* $(,)?]) => {
        $crate::compat::net_enum::net_enum!(@impl false, $(#[$m])* $name { $($var = $val,)* } [$($gv),*]);
    };
    (@impl $flags:expr, $(#[$m:meta])* $name:ident { $($var:ident = $val:expr,)* } [$($gv:ident),*]) => {
        $(#[$m])*
        #[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, ::serde::Serialize, ::serde::Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub i32);

        #[allow(non_upper_case_globals)]
        impl $name {
            $(pub const $var: $name = $name($val);)*

            /// `Enum.GetValues` order.
            pub const ALL: &'static [$name] = &[$($name::$gv),*];

            pub const INFO: $crate::compat::net_enum::EnumTypeInfo = $crate::compat::net_enum::EnumTypeInfo {
                name: stringify!($name),
                is_flags: $flags,
                declared: &[$((stringify!($var), $val)),*],
            };

            /// `Enum.IsDefined`
            pub fn is_defined(self) -> bool {
                Self::INFO.name_of(self.0).is_some()
            }
        }

        impl $crate::compat::net_enum::NetEnum for $name {
            const INFO: &'static $crate::compat::net_enum::EnumTypeInfo = &$name::INFO;
            const ALL: &'static [Self] = $name::ALL;
            fn value(self) -> i32 { self.0 }
            fn from_value(value: i32) -> Self { $name(value) }
        }

        /// `ToString()` with .NET semantics.
        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(&<$name as $crate::compat::net_enum::NetEnum>::INFO.format(self.0))
            }
        }

        impl ::std::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}::{}", stringify!($name), self)
            }
        }

        /// `Comparer.Default` on a boxed enum compares the underlying values.
        impl $crate::compat::random_ext::CsComparable for $name {
            fn cs_compare(&self, other: &Self) -> ::std::cmp::Ordering { self.0.cmp(&other.0) }
            fn cs_equals(&self, other: &Self) -> bool { self.0 == other.0 }
        }

        impl $crate::messages::expression::CsType for $name {
            const IS_VALUE_TYPE: bool = true;
            fn cs_type_name() -> String { format!("Treachery.Shared.{}", stringify!($name)) }
        }

        /// `ToString()`, for exception messages that embed a key (`Dictionary.Add`, the indexer).
        impl dotnet_compat::exception::CsToString for $name {
            fn cs_to_string(&self) -> String {
                self.to_string()
            }
        }

        /// State dump: `"<ToString()>(<number>)"` (trace-format.md 7.3).
        impl $crate::canon::Canon for $name {
            fn canon(&self, w: &mut $crate::canon::CanonWriter) {
                $crate::canon::write_enum(w, &<$name as $crate::compat::net_enum::NetEnum>::INFO.format(self.0), self.0)
            }
        }

        impl $crate::canon::CanonShape for $name {
            fn canon_shape(_: &mut $crate::canon::Registry) -> $crate::canon::Shape {
                $crate::canon::Shape::kind("str")
            }
        }

        impl From<$name> for $crate::messages::expression::ExpressionElement {
            fn from(v: $name) -> Self {
                $crate::messages::expression::ExpressionElement::Enum($crate::messages::expression::EnumValue::of(v))
            }
        }
    };
}
pub(crate) use net_enum;
