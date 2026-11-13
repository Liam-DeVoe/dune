//! The game's enums: open newtypes over `i32` with named constants.
//!
//! Any `i32` is a value of every enum: event JSON may hold `13` for a `Faction`, and an
//! undefined value prints as its number. Savegames and clients rely on this, so the enums are
//! not closed Rust `enum`s:
//!
//! ```
//! use treachery_engine::enumerations::Faction;
//! let f = Faction::Fremen;
//! assert_eq!(f.0, 10);
//! assert!(Faction::Atreides > Faction::Fremen); // ordered by value
//! assert_eq!(Faction(13).to_string(), "13"); // undefined values are allowed
//! match f {
//!     Faction::Fremen => {}
//!     _ => unreachable!(),             // a `_` arm is always required
//! }
//! ```
//!
//! `Display` prints the member name, or the number, or a `[Flags]` combination
//! (`"Cross, ToHomeworld"`), as the log texts always have.

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

/// Implemented by every type generated with [`game_enum!`].
pub trait GameEnum: Copy + Eq + Ord + std::hash::Hash + fmt::Debug + 'static {
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
/// game_enum! {
///     /// C# `BattleFactKind`
///     BattleFactKind { Strength = 0, Winner = 1, }
///     get_values [Strength, Winner]
/// }
/// ```
macro_rules! game_enum {
    ($(#[$m:meta])* flags $name:ident { $($var:ident = $val:expr,)* } get_values [$($gv:ident),* $(,)?]) => {
        $crate::enum_type::game_enum!(@impl true, $(#[$m])* $name { $($var = $val,)* } [$($gv),*]);
    };
    ($(#[$m:meta])* $name:ident { $($var:ident = $val:expr,)* } get_values [$($gv:ident),* $(,)?]) => {
        $crate::enum_type::game_enum!(@impl false, $(#[$m])* $name { $($var = $val,)* } [$($gv),*]);
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

            pub const INFO: $crate::enum_type::EnumTypeInfo = $crate::enum_type::EnumTypeInfo {
                name: stringify!($name),
                is_flags: $flags,
                declared: &[$((stringify!($var), $val)),*],
            };

            /// `Enum.IsDefined`
            pub fn is_defined(self) -> bool {
                Self::INFO.name_of(self.0).is_some()
            }
        }

        impl $crate::enum_type::GameEnum for $name {
            const INFO: &'static $crate::enum_type::EnumTypeInfo = &$name::INFO;
            const ALL: &'static [Self] = $name::ALL;
            fn value(self) -> i32 { self.0 }
            fn from_value(value: i32) -> Self { $name(value) }
        }

        /// `ToString()` with .NET semantics.
        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(&<$name as $crate::enum_type::GameEnum>::INFO.format(self.0))
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

        impl From<$name> for $crate::messages::expression::ExpressionElement {
            fn from(v: $name) -> Self {
                $crate::messages::expression::ExpressionElement::Enum($crate::messages::expression::EnumValue::of(v))
            }
        }
    };
}
pub(crate) use game_enum;
