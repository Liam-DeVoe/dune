//! Numeric helpers with .NET semantics (Math.Round default is banker's
//! rounding, float->int casts truncate, etc.).
//!
//! Things that need no helper (verified against .NET 10, see docs/compat.md):
//! - `(int)someDouble` / `(int)someFloat` is Rust `as i32`: truncation toward
//!   zero, NaN -> 0, out of range saturates (.NET 9+ on x64 and arm64).
//! - `Math.Ceiling`/`Math.Floor` take a double. `Math.Ceiling(0.5f * x)` is
//!   `((0.5f32 * x as f32) as f64).ceil()`: the multiply happens in f32.
//! - C# `float` arithmetic is real IEEE f32 arithmetic; use `f32` in Rust.
//! - Integer `/` and `%` truncate toward zero in both languages.
//! - C# integer arithmetic is unchecked (wraps) unless inside `checked`; Rust
//!   debug builds panic on overflow. Where wrapping can happen, use
//!   `wrapping_add` etc.
//! - `int.ToString()` is Rust `to_string()` (invariant culture).

use std::cmp::Ordering;

/// `Math.Round(double)` (MidpointRounding.ToEven)
pub fn round_f64(x: f64) -> f64 {
    x.round_ties_even()
}
/// `MathF.Round(float)` (ToEven). Note: C# `Math.Round(someFloat)` binds to
/// `Math.Round(double)`; use `round_f64(x as f64)` for that.
pub fn round_f32(x: f32) -> f32 {
    x.round_ties_even()
}

/// `System.MidpointRounding`
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MidpointRounding {
    ToEven = 0,
    AwayFromZero = 1,
    ToZero = 2,
    ToNegativeInfinity = 3,
    ToPositiveInfinity = 4,
}

/// `BitDecrement(0.5)`
const ALMOST_HALF: f64 = 0.49999999999999994;

/// `Math.Round(double, MidpointRounding)`
pub fn round_f64_mode(x: f64, mode: MidpointRounding) -> f64 {
    match mode {
        MidpointRounding::ToEven => x.round_ties_even(),
        MidpointRounding::AwayFromZero => (x + ALMOST_HALF.copysign(x)).trunc(),
        MidpointRounding::ToZero => x.trunc(),
        MidpointRounding::ToNegativeInfinity => x.floor(),
        MidpointRounding::ToPositiveInfinity => x.ceil(),
    }
}

/// `Math.Round(double, int digits)` (ToEven)
pub fn round_f64_digits(x: f64, digits: i32) -> f64 {
    round_f64_digits_mode(x, digits, MidpointRounding::ToEven)
}

/// `Math.Round(double, int digits, MidpointRounding)`. Throws
/// ArgumentOutOfRangeException for digits outside 0..=15.
pub fn round_f64_digits_mode(x: f64, digits: i32, mode: MidpointRounding) -> f64 {
    if !(0..=15).contains(&digits) {
        crate::exception::throw(
            "ArgumentOutOfRangeException",
            "Rounding digits must be between 0 and 15, inclusive. (Parameter 'digits')".to_string(),
        );
    }
    if x.abs() < 1e16 {
        let power10 = 10f64.powi(digits);
        let v = x * power10;
        let v = round_f64_mode(v, mode);
        v / power10
    } else {
        x
    }
}

/// `Math.Abs(int)`; throws OverflowException for int.MinValue.
pub fn abs_i32(x: i32) -> i32 {
    match x.checked_abs() {
        Some(v) => v,
        None => crate::exception::throw(
            "OverflowException",
            "Negating the minimum value of a twos complement number is invalid.".to_string(),
        ),
    }
}

/// `(int)x` for a double: truncates, NaN -> 0, saturates out of range.
pub fn f64_to_i32(x: f64) -> i32 {
    x as i32
}
/// `(int)x` for a float: truncates, NaN -> 0, saturates out of range.
pub fn f32_to_i32(x: f32) -> i32 {
    x as i32
}

/// `double.CompareTo(double)` / `Comparer<double>.Default`: a total order in
/// which NaN is smaller than everything (and equal to NaN) and -0.0 == 0.0.
/// This is what OrderBy/Max/Min/Sort use for double keys.
pub fn cmp_f64(a: f64, b: f64) -> Ordering {
    if a < b {
        Ordering::Less
    } else if a > b {
        Ordering::Greater
    } else if a == b {
        Ordering::Equal
    } else if a.is_nan() {
        if b.is_nan() { Ordering::Equal } else { Ordering::Less }
    } else {
        Ordering::Greater
    }
}
/// `float.CompareTo(float)`; see [`cmp_f64`].
pub fn cmp_f32(a: f32, b: f32) -> Ordering {
    cmp_f64(a as f64, b as f64)
}

/// `double.Equals(double)` (and `object.Equals` on boxed doubles): like `==`
/// except NaN equals NaN.
pub fn equals_f64(a: f64, b: f64) -> bool {
    a == b || (a.is_nan() && b.is_nan())
}
/// `float.Equals(float)`; see [`equals_f64`].
pub fn equals_f32(a: f32, b: f32) -> bool {
    a == b || (a.is_nan() && b.is_nan())
}

/// `Math.Max(double, double)`: NaN if either is NaN; max(-0.0, 0.0) = 0.0.
/// (Rust's `f64::max` ignores NaN.) NaN payloads are not modeled: on arm64
/// .NET returns a quieted NaN when given a signaling one; this returns the
/// NaN input unchanged.
pub fn math_max_f64(a: f64, b: f64) -> f64 {
    if a != b {
        if a.is_nan() {
            return a;
        }
        return if b < a { a } else { b };
    }
    if b.is_sign_negative() { a } else { b }
}
/// `Math.Min(double, double)`: NaN if either is NaN; min(-0.0, 0.0) = -0.0.
pub fn math_min_f64(a: f64, b: f64) -> f64 {
    if a != b {
        if a.is_nan() {
            return a;
        }
        return if a < b { a } else { b };
    }
    if a.is_sign_negative() { a } else { b }
}
/// `Math.Max(float, float)`
pub fn math_max_f32(a: f32, b: f32) -> f32 {
    if a != b {
        if a.is_nan() {
            return a;
        }
        return if b < a { a } else { b };
    }
    if b.is_sign_negative() { a } else { b }
}
/// `Math.Min(float, float)`
pub fn math_min_f32(a: f32, b: f32) -> f32 {
    if a != b {
        if a.is_nan() {
            return a;
        }
        return if a < b { a } else { b };
    }
    if a.is_sign_negative() { a } else { b }
}

// ---------------------------------------------------------------------------
// Formatting
// ---------------------------------------------------------------------------

/// Formats `.NET`-style "G" with the shortest round-trip digits.
/// `digits` are the significant digits (no leading/trailing zeros, nonempty),
/// `scale` is the decimal exponent with value = 0.d1d2... * 10^scale.
fn format_general(negative: bool, digits: &str, scale: i32, min_max_digits: i32) -> String {
    let n_max_digits = (digits.len() as i32).max(min_max_digits);
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    let d = digits.as_bytes();
    let mut di = 0usize;
    let mut dig_pos = scale;
    let mut scientific = false;
    if dig_pos > n_max_digits || dig_pos < -3 {
        dig_pos = 1;
        scientific = true;
    }
    if dig_pos > 0 {
        while dig_pos > 0 {
            out.push(if di < d.len() { d[di] as char } else { '0' });
            if di < d.len() {
                di += 1;
            }
            dig_pos -= 1;
        }
    } else {
        out.push('0');
    }
    if di < d.len() || dig_pos < 0 {
        out.push('.');
        while dig_pos < 0 {
            out.push('0');
            dig_pos += 1;
        }
        while di < d.len() {
            out.push(d[di] as char);
            di += 1;
        }
    }
    if scientific {
        let e = scale - 1;
        out.push('E');
        out.push(if e < 0 { '-' } else { '+' });
        out.push_str(&format!("{:02}", e.unsigned_abs()));
    }
    out
}

/// Shortest round-trip digits of `x` (finite, > 0) as (digits, scale) with
/// value = 0.d1d2... * 10^scale, chosen like .NET's Dragon4: among the
/// shortest strings that round-trip, the closest one, and on an exact tie the
/// one with an even last digit. Rust's formatter rounds such ties up, so they
/// are fixed here using the exact decimal expansion.
fn shortest_digits(shortest_e: String, exact_e: String) -> (String, i32) {
    let split = |e: &str| -> (String, i32) {
        let (mant, exp) = e.split_once('e').unwrap();
        let exp: i32 = exp.parse().unwrap();
        let digits: String = mant.chars().filter(|c| c.is_ascii_digit()).collect();
        (digits.trim_end_matches('0').to_string(), exp + 1)
    };
    let (digits, scale) = split(&shortest_e);
    let (exact, exact_scale) = split(&exact_e);
    let n = digits.len();
    if exact.len() == n + 1 && exact.ends_with('5') {
        // Exact tie between exact[..n] and exact[..n] + 1 ulp-of-digit.
        let lower = &exact[..n];
        let last = lower.as_bytes()[n - 1] - b'0';
        if last % 2 == 0 {
            return (lower.trim_end_matches('0').to_string(), exact_scale);
        }
        // Round up with carry.
        let mut bytes = lower.as_bytes().to_vec();
        let mut i = n;
        loop {
            if i == 0 {
                bytes.insert(0, b'1');
                let s = String::from_utf8(bytes).unwrap();
                return (s.trim_end_matches('0').to_string(), exact_scale + 1);
            }
            i -= 1;
            if bytes[i] == b'9' {
                bytes[i] = b'0';
            } else {
                bytes[i] += 1;
                break;
            }
        }
        let s = String::from_utf8(bytes).unwrap();
        return (s.trim_end_matches('0').to_string(), exact_scale);
    }
    (digits, scale)
}

/// `double.ToString()` / `double.ToString(CultureInfo.InvariantCulture)` /
/// string interpolation of a double, under the invariant culture: shortest
/// round-trip digits, scientific notation ("1E+17", "1E-05") when the decimal
/// exponent is >= max(17, digit count) or < -4.
pub fn format_f64(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity".into() } else { "-Infinity".into() };
    }
    let negative = x.is_sign_negative();
    if x == 0.0 {
        return if negative { "-0".into() } else { "0".into() };
    }
    let (digits, scale) = shortest_digits(format!("{:e}", x.abs()), format!("{:.800e}", x.abs()));
    format_general(negative, &digits, scale, 17)
}

/// `float.ToString()` under the invariant culture; like [`format_f64`] with a
/// threshold of 9 digits ("1E+09").
pub fn format_f32(x: f32) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity".into() } else { "-Infinity".into() };
    }
    let negative = x.is_sign_negative();
    if x == 0.0 {
        return if negative { "-0".into() } else { "0".into() };
    }
    let (digits, scale) = shortest_digits(format!("{:e}", x.abs()), format!("{:.160e}", x.abs()));
    format_general(negative, &digits, scale, 9)
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// `double.NaN` as .NET produces it (sign bit set, bits 0xFFF8000000000000).
/// Rust's `f64::NAN` has the sign bit clear. Only observable through bit
/// patterns (`BitConverter`, `IsNegative`).
pub const NET_NAN_F64: f64 = f64::from_bits(0xFFF8_0000_0000_0000);
/// `float.NaN` as .NET produces it (bits 0xFFC00000).
pub const NET_NAN_F32: f32 = f32::from_bits(0xFFC0_0000);

/// Why a .NET `Parse` call would throw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// FormatException
    Format,
    /// OverflowException
    Overflow,
}

/// .NET `IsRichese`: U+0009..U+000D and space.
fn is_richese(c: u16) -> bool {
    c == 0x20 || (0x09..=0x0D).contains(&c)
}
fn is_digit(c: u16) -> bool {
    (b'0' as u16..=b'9' as u16).contains(&c)
}
/// .NET `TrailingZeros`: the rest of the input is all '\0'.
fn trailing_zeros(s: &[u16], from: usize) -> bool {
    s[from..].iter().all(|&c| c == 0)
}

/// `int.Parse(s)` (NumberStyles.Integer) with the invariant culture's signs.
pub fn parse_i32(s: &str) -> Result<i32, ParseError> {
    let s: Vec<u16> = s.encode_utf16().collect();
    let mut i = 0;
    while i < s.len() && is_richese(s[i]) {
        i += 1;
    }
    let mut negative = false;
    if i < s.len() && (s[i] == '-' as u16 || s[i] == '+' as u16) {
        negative = s[i] == '-' as u16;
        i += 1;
    }
    if i >= s.len() || !is_digit(s[i]) {
        return Err(ParseError::Format);
    }
    let mut value: i64 = 0;
    let mut overflow = false;
    while i < s.len() && is_digit(s[i]) {
        if !overflow {
            value = value * 10 + (s[i] - '0' as u16) as i64;
            if value > i32::MAX as i64 + 1 {
                overflow = true;
            }
        }
        i += 1;
    }
    if i < s.len() {
        while i < s.len() && is_richese(s[i]) {
            i += 1;
        }
        if !trailing_zeros(&s, i) {
            return Err(ParseError::Format);
        }
    }
    let v = if negative { -value } else { value };
    if overflow || v > i32::MAX as i64 || v < i32::MIN as i64 {
        return Err(ParseError::Overflow);
    }
    Ok(v as i32)
}

/// Validates `s` against .NET's `TryParseNumber` grammar for
/// `NumberStyles.Float | NumberStyles.AllowThousands` with invariant culture
/// symbols, returning a string Rust's float parser accepts.
fn normalize_float(s: &[u16]) -> Option<String> {
    let at = |i: usize| -> u16 { if i < s.len() { s[i] } else { 0 } };
    let mut p = 0usize;
    let mut out = String::new();
    let mut has_sign = false;
    // Leading whitespace and sign. Whitespace after the sign is not allowed.
    loop {
        let ch = at(p);
        if p < s.len() && is_richese(ch) && !has_sign {
            p += 1;
            continue;
        }
        if !has_sign && (ch == '+' as u16 || ch == '-' as u16) && p < s.len() {
            has_sign = true;
            if ch == '-' as u16 {
                out.push('-');
            }
            p += 1;
            continue;
        }
        break;
    }
    let mut has_digits = false;
    let mut has_decimal = false;
    loop {
        let ch = at(p);
        if p < s.len() && is_digit(ch) {
            has_digits = true;
            out.push(ch as u8 as char);
        } else if p < s.len() && ch == '.' as u16 && !has_decimal {
            has_decimal = true;
            out.push('.');
        } else if p < s.len() && ch == ',' as u16 && has_digits && !has_decimal {
            // thousands separator: ignored
        } else {
            break;
        }
        p += 1;
    }
    if !has_digits {
        return None;
    }
    if p < s.len() && (at(p) == 'e' as u16 || at(p) == 'E' as u16) {
        let temp = p;
        p += 1;
        let mut neg_exp = false;
        if p < s.len() && at(p) == '+' as u16 {
            p += 1;
        } else if p < s.len() && at(p) == '-' as u16 {
            p += 1;
            neg_exp = true;
        }
        if p < s.len() && is_digit(at(p)) {
            let mut exp: i32 = 0;
            while p < s.len() && is_digit(at(p)) {
                exp = exp * 10 + (at(p) - '0' as u16) as i32;
                p += 1;
                if exp > 1000 {
                    exp = 9999;
                    while p < s.len() && is_digit(at(p)) {
                        p += 1;
                    }
                }
            }
            out.push('e');
            if neg_exp {
                out.push('-');
            }
            out.push_str(&exp.to_string());
        } else {
            p = temp;
        }
    }
    while p < s.len() && is_richese(at(p)) {
        p += 1;
    }
    if p < s.len() && !trailing_zeros(s, p) {
        return None;
    }
    Some(out)
}

fn special_float(s: &[u16]) -> Option<f64> {
    let mut a = 0;
    let mut b = s.len();
    while a < b && is_richese(s[a]) {
        a += 1;
    }
    while b > a && is_richese(s[b - 1]) {
        b -= 1;
    }
    let t = String::from_utf16_lossy(&s[a..b]).to_ascii_lowercase();
    match t.as_str() {
        "infinity" | "+infinity" => Some(f64::INFINITY),
        "-infinity" => Some(f64::NEG_INFINITY),
        "nan" | "+nan" | "-nan" => Some(NET_NAN_F64),
        _ => None,
    }
}

/// `double.Parse(s, CultureInfo.InvariantCulture)` (NumberStyles.Float |
/// AllowThousands). Out-of-range values become ±Infinity, as in .NET Core 3.0+.
pub fn parse_f64(s: &str) -> Result<f64, ParseError> {
    let u: Vec<u16> = s.encode_utf16().collect();
    if let Some(n) = normalize_float(&u) {
        return Ok(n.parse::<f64>().expect("normalized float must parse"));
    }
    special_float(&u).ok_or(ParseError::Format)
}

/// `float.Parse(s, CultureInfo.InvariantCulture)`; see [`parse_f64`]. Rounds
/// directly to f32 (no double rounding), as .NET Core 3.0+ does.
pub fn parse_f32(s: &str) -> Result<f32, ParseError> {
    let u: Vec<u16> = s.encode_utf16().collect();
    if let Some(n) = normalize_float(&u) {
        return Ok(n.parse::<f32>().expect("normalized float must parse"));
    }
    special_float(&u).map(|x| if x.is_nan() { NET_NAN_F32 } else { x as f32 }).ok_or(ParseError::Format)
}
