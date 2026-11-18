//! Numbers and enum names that events carry as text: force lists, id lists, deal and
//! Clairvoyance parameters.
//!
//! A refused Clairvoyance question or deal shows the error message to the player, so the
//! grammar and the messages stay fixed. They follow .NET's `int.Parse`, `float.Parse` and
//! `Enum.Parse`. Only ASCII whitespace (tab to carriage return, and space) is skipped, and trailing
//! `'\0'`s are allowed.

/// Why a text isn't a valid value: a kind (`"FormatException"`, `"OverflowException"`,
/// `"ArgumentException"`) and the message shown to players.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub kind: &'static str,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

fn format_error(s: &str) -> ParseError {
    ParseError { kind: "FormatException", message: format!("The input string '{s}' was not in a correct format.") }
}

fn overflow() -> ParseError {
    ParseError { kind: "OverflowException", message: "Value was either too large or too small for an Int32.".into() }
}

fn is_space(c: u8) -> bool {
    c == b' ' || (0x09..=0x0D).contains(&c)
}

/// Whether the rest of the input is all `'\0'`.
fn only_nuls(s: &[u8], from: usize) -> bool {
    s[from..].iter().all(|&c| c == 0)
}

enum IntError {
    Format,
    Overflow,
}

fn int_value(s: &str) -> Result<i32, IntError> {
    let s = s.as_bytes();
    let mut i = 0;
    while i < s.len() && is_space(s[i]) {
        i += 1;
    }
    let mut negative = false;
    if i < s.len() && (s[i] == b'-' || s[i] == b'+') {
        negative = s[i] == b'-';
        i += 1;
    }
    if i >= s.len() || !s[i].is_ascii_digit() {
        return Err(IntError::Format);
    }
    let mut value: i64 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        if !overflow {
            value = value * 10 + (s[i] - b'0') as i64;
            overflow = value > i32::MAX as i64 + 1;
        }
        i += 1;
    }
    while i < s.len() && is_space(s[i]) {
        i += 1;
    }
    if !only_nuls(s, i) {
        return Err(IntError::Format);
    }
    let v = if negative { -value } else { value };
    if overflow || v > i32::MAX as i64 || v < i32::MIN as i64 {
        return Err(IntError::Overflow);
    }
    Ok(v as i32)
}

/// A whole number: optional whitespace around an optional sign and digits.
pub fn int(s: &str) -> Result<i32, ParseError> {
    int_value(s).map_err(|e| match e {
        IntError::Format => format_error(s),
        IntError::Overflow => overflow(),
    })
}

/// [`int`], panicking on text that isn't a number.
#[track_caller]
pub fn int_or_panic(s: &str) -> i32 {
    int(s).unwrap_or_else(|e| panic!("{e}"))
}

/// A dial, which must be a whole or half number, in halves: `"2.5"` is 5. The text is read as
/// a 32-bit float, so `"1,5"` is 15 (a thousands separator), exponents count, and `"Infinity"`
/// is a number but not a whole or half one.
pub fn halves(s: &str) -> Result<i32, ParseError> {
    let f = float(s).ok_or_else(|| format_error(s))?;
    let halves = f * 2.0;
    // Below 2^24 every whole f32 is exact, so the cast loses nothing.
    if halves.is_finite() && halves.fract() == 0.0 && halves.abs() < 16_777_216.0 {
        Ok(halves as i32)
    } else {
        Err(ParseError { kind: "FormatException", message: format!("The input string '{s}' is not a whole or half number.") })
    }
}

/// `float.Parse` with the invariant culture.
fn float(s: &str) -> Option<f32> {
    match normalize_float(s.as_bytes()) {
        Some(n) => Some(n.parse::<f32>().expect("normalized float must parse")),
        None => special_float(s),
    }
}

/// Checks `s` against the float grammar (sign, digits with `,` as an ignored thousands
/// separator, one `.`, an exponent), returning a text Rust's float parser accepts.
fn normalize_float(s: &[u8]) -> Option<String> {
    let at = |i: usize| -> u8 { if i < s.len() { s[i] } else { 0 } };
    let mut p = 0usize;
    let mut out = String::new();
    let mut has_sign = false;
    // Leading whitespace and sign. Whitespace after the sign is not allowed.
    loop {
        let ch = at(p);
        if p < s.len() && is_space(ch) && !has_sign {
            p += 1;
            continue;
        }
        if !has_sign && (ch == b'+' || ch == b'-') && p < s.len() {
            has_sign = true;
            if ch == b'-' {
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
        if p < s.len() && ch.is_ascii_digit() {
            has_digits = true;
            out.push(ch as char);
        } else if p < s.len() && ch == b'.' && !has_decimal {
            has_decimal = true;
            out.push('.');
        } else if p < s.len() && ch == b',' && has_digits && !has_decimal {
            // thousands separator: ignored
        } else {
            break;
        }
        p += 1;
    }
    if !has_digits {
        return None;
    }
    if p < s.len() && (at(p) == b'e' || at(p) == b'E') {
        let temp = p;
        p += 1;
        let mut neg_exp = false;
        if p < s.len() && at(p) == b'+' {
            p += 1;
        } else if p < s.len() && at(p) == b'-' {
            p += 1;
            neg_exp = true;
        }
        if p < s.len() && at(p).is_ascii_digit() {
            let mut exp: i32 = 0;
            while p < s.len() && at(p).is_ascii_digit() {
                exp = exp * 10 + (at(p) - b'0') as i32;
                p += 1;
                if exp > 1000 {
                    exp = 9999;
                    while p < s.len() && at(p).is_ascii_digit() {
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
    while p < s.len() && is_space(at(p)) {
        p += 1;
    }
    if p < s.len() && !only_nuls(s, p) {
        return None;
    }
    Some(out)
}

/// `Infinity` and `NaN`, in any case, with optional whitespace and sign.
fn special_float(s: &str) -> Option<f32> {
    let t = s.trim_matches(|c: char| c.is_ascii() && is_space(c as u8)).to_ascii_lowercase();
    match t.as_str() {
        "infinity" | "+infinity" => Some(f32::INFINITY),
        "-infinity" => Some(f32::NEG_INFINITY),
        "nan" | "+nan" | "-nan" => Some(f32::NAN),
        _ => None,
    }
}

/// The value of an enum member named in `s`, where `members` lists every (name, value) pair:
///
/// - leading whitespace is skipped, and empty text is an error;
/// - text starting with a digit, `-` or `+` is first read as a number, member or not;
/// - otherwise (or if that fails) the text is split on `,`, each piece is trimmed and matched
///   case-sensitively, and the values are OR'ed together.
pub fn enum_value(s: &str, members: &[(&str, i32)]) -> Result<i32, ParseError> {
    let v = s.trim_start();
    if v.is_empty() {
        return Err(ParseError {
            kind: "ArgumentException",
            message: "Must specify valid information for parsing in the string. (Parameter 'value')".into(),
        });
    }
    let c = v.chars().next().unwrap();
    if c.is_ascii_digit() || c == '-' || c == '+' {
        match int_value(v) {
            Ok(x) => return Ok(x),
            Err(IntError::Overflow) => return Err(overflow()),
            Err(IntError::Format) => {}
        }
    }
    let mut result = 0i32;
    for piece in v.split(',') {
        let piece = piece.trim();
        match members.iter().find(|(name, _)| *name == piece) {
            Some((_, value)) => result |= value,
            None => return Err(ParseError { kind: "ArgumentException", message: format!("Requested value '{v}' was not found.") }),
        }
    }
    Ok(result)
}

/// [`enum_value`], panicking on text that names no member.
#[track_caller]
pub fn enum_value_or_panic(s: &str, members: &[(&str, i32)]) -> i32 {
    enum_value(s, members).unwrap_or_else(|e| panic!("{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ints() {
        assert_eq!(int(" -12\t"), Ok(-12));
        assert_eq!(int("+7\0\0"), Ok(7));
        assert_eq!(int("2147483647"), Ok(i32::MAX));
        assert_eq!(int("-2147483648"), Ok(i32::MIN));
        assert_eq!(int("2147483648").unwrap_err().kind, "OverflowException");
        for s in ["", " ", "1 2", "- 1", "1.0", "0x10", "\u{661}"] {
            assert_eq!(int(s), Err(format_error(s)), "{s:?}");
        }
    }

    #[test]
    fn dials() {
        assert_eq!(halves("2.5"), Ok(5));
        assert_eq!(halves(" 3 "), Ok(6));
        assert_eq!(halves("1,5"), Ok(30));
        assert_eq!(halves("5e-1"), Ok(1));
        assert_eq!(halves("-.5"), Ok(-1));
        for s in ["2.7", "Infinity", " -INFINITY", "nan", "1e9"] {
            assert_eq!(halves(s).unwrap_err().message, format!("The input string '{s}' is not a whole or half number."));
        }
        for s in ["", ",5", "- 1", "1e", "1..5", "abc"] {
            assert_eq!(halves(s), Err(format_error(s)), "{s:?}");
        }
    }

    #[test]
    fn enum_values() {
        let members = [("None", 0), ("A", 1), ("B", 2)];
        assert_eq!(enum_value(" A, B", &members), Ok(3));
        assert_eq!(enum_value("17", &members), Ok(17));
        assert_eq!(enum_value("a", &members).unwrap_err().message, "Requested value 'a' was not found.");
        assert_eq!(enum_value(" ", &members).unwrap_err().kind, "ArgumentException");
        assert_eq!(enum_value("99999999999", &members).unwrap_err().kind, "OverflowException");
    }
}
