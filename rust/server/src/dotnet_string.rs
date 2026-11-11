//! The .NET `string` operations the hub applies to user input, as the C# server runs them: ICU
//! globalization (no `InvariantGlobalization`) and the invariant culture (the systemd unit sets
//! no `LANG`). Checked against .NET for every code point in `tests/auth_dotnet.rs`.

/// `s.Length`: UTF-16 code units.
pub fn length(s: &str) -> usize {
    s.encode_utf16().count()
}

/// `s.Trim()`: removes leading and trailing `char.IsWhiteSpace` characters. .NET's white-space
/// set is exactly Unicode's White_Space property, which `char::is_whitespace` implements.
pub fn trim(s: &str) -> &str {
    s.trim_matches(char::is_whitespace)
}

/// `string.IsNullOrWhiteSpace(s)` for a non-null string.
pub fn is_white_space(s: &str) -> bool {
    s.chars().all(char::is_whitespace)
}

/// `s.ToLower()` in the invariant culture with ICU: each code point's simple lowercase mapping,
/// never changing the length, and no context rules (no final sigma). .NET leaves U+0130 (LATIN
/// CAPITAL LETTER I WITH DOT ABOVE) unchanged in the invariant culture. Rust's
/// `char::to_lowercase` is the full mapping, which differs from the simple one only for U+0130.
pub fn to_lower(s: &str) -> String {
    s.chars().map(to_lower_char).collect()
}

/// [`to_lower`] for one code point.
pub fn to_lower_char(c: char) -> char {
    if c == '\u{130}' {
        return c;
    }
    let mut lower = c.to_lowercase();
    match (lower.next(), lower.next()) {
        (Some(l), None) => l,
        _ => c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn examples() {
        assert_eq!(length("a😀"), 3);
        assert_eq!(trim("\u{85}\u{a0} a b\u{3000}"), "a b");
        assert_eq!(trim("\u{200b}a"), "\u{200b}a");
        assert_eq!(to_lower("ΟΔΟΣ"), "οδοσ");
        assert_eq!(to_lower("İSTANBUL"), "İstanbul");
        assert_eq!(to_lower("ẞǅ"), "ßǆ");
        assert!(is_white_space(""));
        assert!(is_white_space(" \t\u{2028}"));
    }
}
