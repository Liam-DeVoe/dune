//! String parsing with .NET's grammar, results and exception messages.
//!
//! `None` stands for a C# `null` string. Each function throws (panics with a
//! [`CsException`]) exactly like .NET, with the exact .NET 10 message under the
//! invariant culture. The `try_` variants return the exception instead.

use crate::exception::{CsException, throw_exception};
use crate::math::{self, ParseError};

fn null(param: &str) -> CsException {
    CsException { type_name: "ArgumentNullException", message: format!("Value cannot be null. (Parameter '{param}')") }
}
fn format(s: &str) -> CsException {
    CsException { type_name: "FormatException", message: format!("The input string '{s}' was not in a correct format.") }
}
fn overflow_int32() -> CsException {
    CsException { type_name: "OverflowException", message: "Value was either too large or too small for an Int32.".into() }
}

/// `int.Parse(s)` (NumberStyles.Integer, invariant signs): optional leading
/// and trailing whitespace, optional sign, ASCII digits, trailing `'\0'`s.
pub fn try_int_parse(s: Option<&str>) -> Result<i32, CsException> {
    let s = s.ok_or_else(|| null("s"))?;
    math::parse_i32(s).map_err(|e| match e {
        ParseError::Format => format(s),
        ParseError::Overflow => overflow_int32(),
    })
}

/// `Convert.ToInt32(string)`: like `int.Parse`, but null gives 0.
pub fn try_convert_to_int32(s: Option<&str>) -> Result<i32, CsException> {
    match s {
        None => Ok(0),
        Some(_) => try_int_parse(s),
    }
}

/// `float.Parse(s, CultureInfo.InvariantCulture)`. Accepts thousands
/// separators (`"1,5"` is 15), exponents, `Infinity`/`NaN` in any case;
/// out-of-range values become ±Infinity.
pub fn try_float_parse_invariant(s: Option<&str>) -> Result<f32, CsException> {
    let s = s.ok_or_else(|| null("s"))?;
    math::parse_f32(s).map_err(|_| format(s))
}

/// `double.Parse(s, CultureInfo.InvariantCulture)`; see [`float_parse_invariant`].
pub fn try_double_parse_invariant(s: Option<&str>) -> Result<f64, CsException> {
    let s = s.ok_or_else(|| null("s"))?;
    math::parse_f64(s).map_err(|_| format(s))
}

/// `Enum.Parse<T>(s)` for an enum with underlying type `int`. `members` lists
/// every (name, value) pair of the enum.
///
/// - Leading whitespace is skipped. Empty input throws ArgumentException.
/// - Input starting with a digit, `-` or `+` is first tried as an integer;
///   any int is accepted, defined or not. Overflow throws.
/// - Otherwise (or if that fails) the input is split on `,`, each piece is
///   trimmed and matched case-sensitively, and the values are OR'ed together,
///   whether or not the enum is `[Flags]`.
pub fn try_enum_parse(s: Option<&str>, members: &[(&str, i32)]) -> Result<i32, CsException> {
    let s = s.ok_or_else(|| null("value"))?;
    let v = s.trim_start();
    if v.is_empty() {
        return Err(CsException {
            type_name: "ArgumentException",
            message: "Must specify valid information for parsing in the string. (Parameter 'value')".into(),
        });
    }
    let c = v.chars().next().unwrap();
    if c.is_ascii_digit() || c == '-' || c == '+' {
        match math::parse_i32(v) {
            Ok(x) => return Ok(x),
            Err(ParseError::Overflow) => return Err(overflow_int32()),
            Err(ParseError::Format) => {}
        }
    }
    let not_found =
        || CsException { type_name: "ArgumentException", message: format!("Requested value '{v}' was not found.") };
    let mut result = 0i32;
    for piece in v.split(',') {
        let piece = piece.trim();
        match members.iter().find(|(name, _)| *name == piece) {
            Some((_, value)) => result |= value,
            None => return Err(not_found()),
        }
    }
    Ok(result)
}

/// [`try_int_parse`], throwing.
#[track_caller]
pub fn int_parse(s: Option<&str>) -> i32 {
    try_int_parse(s).unwrap_or_else(|e| throw_exception(e))
}

/// [`try_convert_to_int32`], throwing.
#[track_caller]
pub fn convert_to_int32(s: Option<&str>) -> i32 {
    try_convert_to_int32(s).unwrap_or_else(|e| throw_exception(e))
}

/// [`try_float_parse_invariant`], throwing.
#[track_caller]
pub fn float_parse_invariant(s: Option<&str>) -> f32 {
    try_float_parse_invariant(s).unwrap_or_else(|e| throw_exception(e))
}

/// [`try_enum_parse`], throwing.
#[track_caller]
pub fn enum_parse(s: Option<&str>, members: &[(&str, i32)]) -> i32 {
    try_enum_parse(s, members).unwrap_or_else(|e| throw_exception(e))
}
