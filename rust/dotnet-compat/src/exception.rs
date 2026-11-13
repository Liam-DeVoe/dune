//! .NET exceptions.
//!
//! The parsing functions return a [`CsException`] as their error. Every other API
//! that throws in .NET panics here instead, with the message
//! `"<type name>: <.NET message>"`: calling it so that .NET would throw is a bug in
//! the caller.

/// A .NET exception as a value: its type name (`"FormatException"`) and message.
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

/// Panics with the exception .NET would throw here.
#[track_caller]
pub fn throw(type_name: &'static str, message: String) -> ! {
    panic!("{type_name}: {message}")
}

/// Panics with `e`.
#[track_caller]
pub fn throw_exception(e: CsException) -> ! {
    panic!("{e}")
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

/// The text of a panic payload (from `catch_unwind`), for tests and crash reports.
pub fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else {
        "panic".to_string()
    }
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
