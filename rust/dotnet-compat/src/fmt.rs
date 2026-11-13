//! Number formatting with .NET's `ToString()` under the invariant culture.
//! Same functions as `math::format_f32`/`math::format_f64`.

/// `float.ToString()` (invariant culture): shortest round-trip digits,
/// `"1E+09"`/`"1E-05"` style exponents, `"Infinity"`, `"NaN"`, `"-0"`.
pub fn f32_to_string(x: f32) -> String {
    crate::math::format_f32(x)
}

/// `double.ToString()` (invariant culture); see [`f32_to_string`].
pub fn f64_to_string(x: f64) -> String {
    crate::math::format_f64(x)
}

/// Why [`string_format`] failed. .NET throws `FormatException` in every case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatError {
    /// A `}` with no matching `{` (`Format_UnexpectedClosingBrace`).
    UnexpectedClosingBrace,
    /// A digit was expected for the index or the alignment (`Format_ExpectedAsciiDigit`).
    ExpectedAsciiDigit,
    /// The format item is not closed, or has a stray character (`Format_UnclosedFormatItem`).
    UnclosedFormatItem,
    /// The index is not below the number of arguments (`Format_IndexOutOfRange`).
    IndexOutOfRange,
}

/// `string.Format(format, args)` where every argument is a string (or null),
/// as .NET 10's `StringBuilder.AppendFormat` parses it.
///
/// - `{{` and `}}` are literal braces.
/// - A format item is `{index[,alignment][:formatString]}`. Spaces may follow the
///   index and surround the alignment. The format string is ignored, because a
///   `string` argument is not `IFormattable`, but it may not contain `{`.
/// - The alignment pads with spaces to that many UTF-16 code units: on the left,
///   or on the right when it is negative.
/// - An index or alignment of 1,000,000 or more is an error.
///
/// `None` arguments print as nothing. Any error is a [`FormatError`]; .NET throws
/// `FormatException` with no partial output.
pub fn string_format(format: &str, args: &[Option<&str>]) -> Result<String, FormatError> {
    const INDEX_LIMIT: usize = 1_000_000;
    const WIDTH_LIMIT: usize = 1_000_000;
    let chars: Vec<char> = format.chars().collect();
    let mut out = String::with_capacity(format.len());
    let mut pos = 0usize;
    // MoveNext: the next character, or UnclosedFormatItem at the end.
    let move_next = |pos: &mut usize| -> Result<char, FormatError> {
        *pos += 1;
        chars.get(*pos).copied().ok_or(FormatError::UnclosedFormatItem)
    };
    loop {
        // Find the next brace, copying the text before it.
        let mut ch;
        loop {
            match chars[pos..].iter().position(|&c| c == '{' || c == '}') {
                None => {
                    out.extend(&chars[pos..]);
                    return Ok(out);
                }
                Some(k) => {
                    out.extend(&chars[pos..pos + k]);
                    pos += k;
                }
            }
            let brace = chars[pos];
            ch = move_next(&mut pos)?;
            if brace == ch {
                out.push(ch);
                pos += 1;
                continue;
            }
            if brace != '{' {
                return Err(FormatError::UnexpectedClosingBrace);
            }
            break;
        }

        // Just after the opening brace.
        let mut width = 0usize;
        let mut left_justify = false;
        let mut index = match ch.to_digit(10) {
            Some(d) if ch.is_ascii_digit() => d as usize,
            _ => return Err(FormatError::ExpectedAsciiDigit),
        };
        ch = move_next(&mut pos)?;
        if ch != '}' {
            while ch.is_ascii_digit() && index < INDEX_LIMIT {
                index = index * 10 + (ch as usize - '0' as usize);
                ch = move_next(&mut pos)?;
            }
            while ch == ' ' {
                ch = move_next(&mut pos)?;
            }
            if ch == ',' {
                loop {
                    ch = move_next(&mut pos)?;
                    if ch != ' ' {
                        break;
                    }
                }
                if ch == '-' {
                    left_justify = true;
                    ch = move_next(&mut pos)?;
                }
                if !ch.is_ascii_digit() {
                    return Err(FormatError::ExpectedAsciiDigit);
                }
                width = ch as usize - '0' as usize;
                ch = move_next(&mut pos)?;
                while ch.is_ascii_digit() && width < WIDTH_LIMIT {
                    width = width * 10 + (ch as usize - '0' as usize);
                    ch = move_next(&mut pos)?;
                }
                while ch == ' ' {
                    ch = move_next(&mut pos)?;
                }
            }
            if ch != '}' {
                if ch != ':' {
                    return Err(FormatError::UnclosedFormatItem);
                }
                loop {
                    ch = move_next(&mut pos)?;
                    if ch == '}' {
                        break;
                    }
                    if ch == '{' {
                        return Err(FormatError::UnclosedFormatItem);
                    }
                }
            }
        }

        // `pos` is at the closing brace.
        pos += 1;
        if index >= args.len() {
            return Err(FormatError::IndexOutOfRange);
        }
        let s = args[index].unwrap_or("");
        let len = s.encode_utf16().count();
        let pad = width.saturating_sub(len);
        if !left_justify {
            out.extend(std::iter::repeat_n(' ', pad));
        }
        out.push_str(s);
        if left_justify {
            out.extend(std::iter::repeat_n(' ', pad));
        }
    }
}
