//! The game's enums.
//!
//! A closed enum is a Rust `enum` with the C# member values as discriminants. Decoding a number
//! that names no member fails. Where the C# enum had a `None = 0` member for "no value", the Rust
//! enum leaves it out and the game uses `Option` instead. Event JSON still writes absence as 0,
//! and texts and views still name it "None" (`EnumTypeInfo::absent`).
//!
//! ```
//! use treachery_engine::enumerations::Faction;
//! use treachery_engine::enum_type::{GameEnum, name_or_absent};
//! let f = Faction::Fremen;
//! assert_eq!(f.value(), 10);
//! assert!(Faction::Atreides > Faction::Fremen); // ordered by value
//! assert_eq!(Faction::from_value(13), None); // undefined values don't exist
//! assert_eq!(name_or_absent::<Faction>(None), "None");
//! ```
//!
//! `Display` prints the member name, as the log texts always have. `ShipmentPermission`, a C#
//! `[Flags]` enum, is a struct of three bools that implements [`GameEnum`] by hand.

use std::fmt;

/// Static description of a C# enum type.
#[derive(Debug)]
pub struct EnumTypeInfo {
    /// C# type name, e.g. `"Faction"`.
    pub name: &'static str,
    /// `[Flags]` attribute present.
    pub is_flags: bool,
    /// Members in declaration order. For a closed enum this includes the `None = 0` member the
    /// Rust type leaves out (see `absent`).
    pub declared: &'static [(&'static str, i32)],
    /// The C# type had a `None = 0` member for "no value". In Rust absence is `Option::None`,
    /// written as 0 in event JSON and named "None" in texts and views.
    pub absent: bool,
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
    /// `Enum.GetValues<T>()` order (sorted by value). A closed enum leaves out its absent
    /// member.
    const ALL: &'static [Self];
    fn value(self) -> i32;
    /// The member with this value: `None` for a number that names no member, and for the
    /// absent member of a closed enum.
    fn from_value(value: i32) -> Option<Self>;
}

/// The number an optional enum is written as in event JSON: absent is 0.
pub fn value_or_absent<T: GameEnum>(v: Option<T>) -> i32 {
    v.map_or(0, T::value)
}

/// Reads an optional enum from its number in event JSON: 0 is absent, and a number that names no
/// member is an error.
pub fn from_value_or_absent<T: GameEnum>(value: i32) -> Result<Option<T>, String> {
    if value == 0 && T::INFO.absent {
        return Ok(None);
    }
    T::from_value(value).map(Some).ok_or_else(|| undefined_value::<T>(value))
}

/// The error for a number that names no member of `T`.
pub fn undefined_value<T: GameEnum>(value: i32) -> String {
    format!("{value} is not a valid {}", T::INFO.name)
}

/// How texts and views name an optional enum: the member name, or "None" when absent.
pub fn name_or_absent<T: GameEnum>(v: Option<T>) -> String {
    T::INFO.format(value_or_absent(v))
}

/// Declares a C# enum. Used by `enumerations.rs`; use it by hand for enums declared elsewhere.
///
/// A closed enum (`closed`) is a Rust `enum`. `absent None = 0;` declares the C# member for "no
/// value", which the Rust type leaves out. `get_values` lists the other members in value order.
///
/// ```ignore
/// game_enum! {
///     closed BattleFactKind { Strength = 0, Winner = 1, }
///     get_values [Strength, Winner]
/// }
/// game_enum! {
///     closed Faction { absent None = 0; Fremen = 10, Atreides = 20, }
///     get_values [Fremen, Atreides]
/// }
/// ```
macro_rules! game_enum {
    ($(#[$m:meta])* closed $name:ident { absent $none:ident = 0; $($var:ident = $val:literal,)* } get_values [$($gv:ident),* $(,)?]) => {
        $crate::enum_type::game_enum!(@closed true, $(#[$m])* $name { ($none = 0,) $($var = $val,)* } [$($var = $val),*] [$($gv),*]);

        /// Absent is the C# `None` member, which the skin names.
        impl From<Option<$name>> for $crate::messages::expression::ExpressionElement {
            fn from(v: Option<$name>) -> Self {
                $crate::messages::expression::ExpressionElement::Enum($crate::messages::expression::EnumValue {
                    ty: &$name::INFO,
                    value: $crate::enum_type::value_or_absent(v),
                })
            }
        }

        /// Absent is written as 0, as C# wrote its `None` member.
        impl $crate::json::ToJson for Option<$name> {
            fn write_json(&self, w: &mut $crate::json::JsonWriter) {
                w.int($crate::enum_type::value_or_absent(*self) as i64)
            }
        }

        impl $crate::json::FromJson for Option<$name> {
            fn from_json(v: &$crate::json::JsonValue) -> Result<Self, $crate::json::JsonError> {
                let n = <i32 as $crate::json::FromJson>::from_json(v)?;
                $crate::enum_type::from_value_or_absent(n).map_err(|e| $crate::json::JsonError::new("JsonException", e))
            }
        }
    };
    ($(#[$m:meta])* closed $name:ident { $($var:ident = $val:literal,)* } get_values [$($gv:ident),* $(,)?]) => {
        $crate::enum_type::game_enum!(@closed false, $(#[$m])* $name { () $($var = $val,)* } [$($var = $val),*] [$($gv),*]);

        impl $crate::messages::expression::NullElement for $name {}
        impl $crate::json::JsonNullable for $name {}

        /// C# `default(T)`: the member with value 0.
        impl Default for $name {
            fn default() -> Self {
                <$name as $crate::enum_type::GameEnum>::from_value(0).expect(concat!(stringify!($name), " has no member 0"))
            }
        }
    };
    (@closed $absent:expr, $(#[$m:meta])* $name:ident { ($($none:ident = 0,)?) $($dvar:ident = $dval:literal,)* } [$($var:ident = $val:literal),*] [$($gv:ident),*]) => {
        $(#[$m])*
        #[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, ::serde::Serialize, ::serde::Deserialize)]
        #[repr(i32)]
        pub enum $name {
            $($var = $val,)*
        }

        impl $name {
            /// `Enum.GetValues` order, without the absent member.
            pub const ALL: &'static [$name] = &[$($name::$gv),*];

            pub const INFO: $crate::enum_type::EnumTypeInfo = $crate::enum_type::EnumTypeInfo {
                name: stringify!($name),
                is_flags: false,
                declared: &[$((stringify!($none), 0),)? $((stringify!($dvar), $dval)),*],
                absent: $absent,
            };
        }

        impl $crate::enum_type::GameEnum for $name {
            const INFO: &'static $crate::enum_type::EnumTypeInfo = &$name::INFO;
            const ALL: &'static [Self] = $name::ALL;
            fn value(self) -> i32 { self as i32 }
            fn from_value(value: i32) -> Option<Self> {
                $(if value == $val { return Some($name::$var); })*
                None
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(match self { $($name::$var => stringify!($var),)* })
            }
        }

        $crate::enum_type::game_enum!(@common $name);
    };
    (@common $name:ident) => {
        impl ::std::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}::{}", stringify!($name), self)
            }
        }

        impl $crate::random::RankKey for $name {
            fn rank(&self, other: &Self) -> ::std::cmp::Ordering { self.cmp(other) }
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
