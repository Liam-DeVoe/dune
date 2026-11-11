//! .NET text helpers that `Skin` calls: `WebUtility.HtmlEncode` and the regular expression of
//! `Skin.WithSpiceIcons`. No C# counterpart file.

/// `System.Net.WebUtility.HtmlEncode(text)`: `<`, `>`, `&`, `"` and `'` become entities, and so
/// does every character from U+00A0 to U+00FF and every character outside the BMP (a
/// surrogate pair in .NET), as `&#N;`.
pub fn html_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            '&' => out.push_str("&amp;"),
            c if ('\u{a0}'..='\u{ff}').contains(&c) || c as u32 >= 0x10000 => {
                out.push_str(&format!("&#{};", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

/// A character of .NET's `\w`: letters, marks, digits and connector punctuation. The skin
/// texts this runs on are ASCII apart from a few accented letters, which `char::is_alphanumeric`
/// classifies the same way.
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '\u{200c}' || c == '\u{200d}'
}

/// .NET's `\d`: a Unicode decimal digit. The skin texts only contain ASCII digits.
fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

/// `\b` between `chars[i - 1]` and `chars[i]`.
fn is_boundary(chars: &[char], i: usize) -> bool {
    let before = i > 0 && is_word_char(chars[i - 1]);
    let after = i < chars.len() && is_word_char(chars[i]);
    before != after
}

/// Case-insensitive match of the ASCII-or-not word `word` at `chars[i..]`, comparing simple
/// lowercase mappings as `RegexOptions.IgnoreCase` does under the invariant culture.
fn matches_at(chars: &[char], i: usize, word: &str) -> Option<usize> {
    let mut j = i;
    for w in word.chars() {
        let c = *chars.get(j)?;
        if lower(c) != lower(w) {
            return None;
        }
        j += 1;
    }
    Some(j)
}

fn lower(c: char) -> char {
    let mut l = c.to_lowercase();
    match (l.next(), l.next()) {
        (Some(x), None) => x,
        _ => c,
    }
}

/// `Regex.Replace(text, $@"\b(\d+) (?:spice|{name})\b(?! (?:cards?|blows?|decks?)\b)", m =>
/// replace(m.Groups[1].Value), RegexOptions.IgnoreCase)`, where `name` is
/// `Regex.Escape(Describe(Concept.Resource))` and so matches the label literally.
pub fn replace_spice_amounts(text: &str, name: &str, mut replace: impl FnMut(&str) -> String) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if let Some(end) = spice_match_at(&chars, i, name) {
            let digits: String = chars[i..].iter().take_while(|c| is_digit(**c)).collect();
            out.push_str(&replace(&digits));
            i = end;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// The end of a match starting at `i`, if the pattern matches there.
fn spice_match_at(chars: &[char], i: usize, name: &str) -> Option<usize> {
    if !is_boundary(chars, i) {
        return None;
    }
    let mut j = i;
    while j < chars.len() && is_digit(chars[j]) {
        j += 1;
    }
    // `\d+` must take at least one digit, and backtracking to fewer digits can't help: the next
    // character would be a digit, not the space.
    if j == i || chars.get(j) != Some(&' ') {
        return None;
    }
    j += 1;
    // `(?:spice|{name})`: the first alternative that lets the rest match.
    for word in ["spice", name] {
        let Some(k) = matches_at(chars, j, word) else { continue };
        if !is_boundary(chars, k) {
            continue;
        }
        if lookahead_unit(chars, k) {
            continue;
        }
        return Some(k);
    }
    None
}

/// ` (?:cards?|blows?|decks?)\b` at `chars[k..]`.
fn lookahead_unit(chars: &[char], k: usize) -> bool {
    if chars.get(k) != Some(&' ') {
        return false;
    }
    let start = k + 1;
    for word in ["card", "blow", "deck"] {
        let Some(end) = matches_at(chars, start, word) else { continue };
        // `s?` is greedy: try with the `s`, then without.
        if let Some(with_s) = matches_at(chars, end, "s") {
            if is_boundary(chars, with_s) {
                return true;
            }
        }
        if is_boundary(chars, end) {
            return true;
        }
    }
    false
}
