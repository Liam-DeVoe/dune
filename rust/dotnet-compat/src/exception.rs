//! C# exceptions as Rust panics.
//!
//! A .NET API that throws is modeled as a panic whose payload is a
//! [`CsException`] carrying the exception's type name and exact .NET 10
//! message. The engine's replayer (`treachery_engine::cs::catch`) turns such a
//! panic back into a value. Every throwing API in this crate panics this way.

use std::panic;

/// A C# exception: its type name (`"InvalidOperationException"`) and message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsException {
    pub type_name: &'static str,
    pub message: String,
}

impl std::fmt::Display for CsException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.type_name, self.message)
    }
}

/// Raises a C# exception. Prefer the [`throw!`](crate::throw) macro.
#[track_caller]
pub fn throw(type_name: &'static str, message: String) -> ! {
    panic::panic_any(CsException { type_name, message })
}

/// Raises `e`.
#[track_caller]
pub fn throw_exception(e: CsException) -> ! {
    panic::panic_any(e)
}

/// `throw new X(message)`:
///
/// ```should_panic
/// dotnet_compat::throw!(InvalidOperationException, "Cannot draw from an empty deck");
/// ```
///
/// The first argument is the C# exception type name; the rest is `format!`
/// arguments for the message.
#[macro_export]
macro_rules! throw {
    ($ty:ident) => {
        $crate::exception::throw(stringify!($ty), String::new())
    };
    ($ty:ident, $($arg:tt)+) => {
        $crate::exception::throw(stringify!($ty), format!($($arg)+))
    };
}

/// If `payload` (from `catch_unwind`) is a C# exception, returns it.
pub fn as_cs_exception(payload: &(dyn std::any::Any + Send)) -> Option<&CsException> {
    payload.downcast_ref::<CsException>()
}

// Messages shared by several APIs (exact .NET 10 texts).
pub const NO_ELEMENTS: &str = "Sequence contains no elements";
pub const NO_MATCH: &str = "Sequence contains no matching element";
pub const MORE_THAN_ONE_ELEMENT: &str = "Sequence contains more than one element";
pub const MORE_THAN_ONE_MATCH: &str = "Sequence contains more than one matching element";
pub const ARITHMETIC_OVERFLOW: &str = "Arithmetic operation resulted in an overflow.";

#[track_caller]
pub(crate) fn invalid_operation(message: &str) -> ! {
    throw("InvalidOperationException", message.to_string())
}

#[track_caller]
pub(crate) fn overflow() -> ! {
    throw("OverflowException", ARITHMETIC_OVERFLOW.to_string())
}

/// C# `ToString()` of a value, for exception messages that embed it (such as
/// `Dictionary.Add`'s "Key: X"). Implement it for every dictionary key type.
pub trait CsToString {
    fn cs_to_string(&self) -> String;
}

macro_rules! cs_to_string_via_display {
    ($($t:ty),*) => {
        $(impl CsToString for $t {
            fn cs_to_string(&self) -> String {
                self.to_string()
            }
        })*
    };
}
cs_to_string_via_display!(i8, i16, i32, i64, u8, u16, u32, u64, usize, isize, char, String, str);

impl CsToString for bool {
    fn cs_to_string(&self) -> String {
        if *self { "True".into() } else { "False".into() }
    }
}
impl CsToString for f32 {
    fn cs_to_string(&self) -> String {
        crate::math::format_f32(*self)
    }
}
impl CsToString for f64 {
    fn cs_to_string(&self) -> String {
        crate::math::format_f64(*self)
    }
}
impl<T: CsToString + ?Sized> CsToString for &T {
    fn cs_to_string(&self) -> String {
        (**self).cs_to_string()
    }
}
