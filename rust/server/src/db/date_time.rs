//! How EF Core's SQLite provider stores `DateTimeOffset`: TEXT in the format
//! `yyyy-MM-dd HH:mm:ss.FFFFFFFzzz` (`SqliteDateTimeOffsetTypeMapping`). The fraction drops
//! trailing zeros and disappears when it is zero; the offset is always `+hh:mm`:
//!
//! ```text
//! 2026-09-19 19:51:27.837212-04:00
//! 0001-01-01 00:00:00+00:00
//! ```
//!
//! Reading is more lenient, because a row may come from sqlite3 or a script rather than EF
//! (`deploy/provision.sh` once wrote `datetime('now')`, which has no offset). EF reads the
//! column with Microsoft.Data.Sqlite's `GetDateTimeOffset`, which behaves exactly like
//! `DateTimeOffset.Parse(text, CultureInfo.InvariantCulture, DateTimeStyles.AssumeUniversal)`.
//! So text without an offset is UTC, whatever the machine's time zone. A plain
//! `DateTimeOffset.Parse` would use the local offset instead, but that is not what EF does.
//! The H1 probe checks both claims (`dotnet-probe` mode `dates`, `tests/db_dates.rs`).
//!
//! [`parse`] accepts the ISO 8601 shapes that tools write, and gives .NET's value for each:
//!
//! ```text
//! ws* yyyy-MM-dd [ ('T' | 't' | ws+) HH:mm [:ss [.f+]] ] ws* [ 'Z' | 'z' | (+|-)HH[[:]mm] ] ws*
//! ```
//!
//! where `ws` is space, tab, CR or LF. Digits beyond the seventh in the fraction are rounded
//! as .NET does (they can carry into the next second). Anything else is refused. That includes
//! forms .NET's very lenient parser also accepts, such as month names, `09/17/2026`, AM/PM and
//! one-digit hours; `tests/db_dates.rs` lists the ones we know of. No tool we use writes them.

use treachery_engine::date_time_offset::DateTimeOffset;

const TICKS_PER_SECOND: i64 = 10_000_000;
const TICKS_PER_DAY: i64 = 86_400 * TICKS_PER_SECOND;

/// Days from 0001-01-01 to the given proleptic Gregorian date.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    // Howard Hinnant's algorithm, shifted from 1970-01-01 to 0001-01-01.
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468 + 719_162
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days - 719_162 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// The column text EF writes for `value`.
pub fn format(value: DateTimeOffset) -> String {
    let ticks = value.ticks;
    let (y, mo, d) = civil_from_days(ticks.div_euclid(TICKS_PER_DAY));
    let in_day = ticks.rem_euclid(TICKS_PER_DAY);
    let secs = in_day / TICKS_PER_SECOND;
    let frac = in_day % TICKS_PER_SECOND;
    let mut s = format!("{y:04}-{mo:02}-{d:02} {:02}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60);
    if frac != 0 {
        let digits = format!("{frac:07}");
        s.push('.');
        s.push_str(digits.trim_end_matches('0'));
    }
    let off = value.offset_minutes as i32;
    let sign = if off < 0 { '-' } else { '+' };
    s.push_str(&format!("{sign}{:02}:{:02}", off.abs() / 60, off.abs() % 60));
    s
}

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        2 if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

struct Cursor<'a> {
    b: &'a [u8],
    i: usize,
}

impl Cursor<'_> {
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn eat(&mut self, c: u8) -> Option<()> {
        (self.peek() == Some(c)).then(|| self.i += 1)
    }

    fn skip_ws(&mut self) -> bool {
        let start = self.i;
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.i += 1;
        }
        self.i > start
    }

    /// Exactly `n` ASCII digits.
    fn digits(&mut self, n: usize) -> Option<i64> {
        let part = self.b.get(self.i..self.i + n)?;
        if !part.iter().all(u8::is_ascii_digit) {
            return None;
        }
        self.i += n;
        Some(part.iter().fold(0, |v, d| v * 10 + i64::from(d - b'0')))
    }
}

/// Reads a column as EF does (see the module comment for the accepted shapes).
pub fn parse(text: &str) -> Option<DateTimeOffset> {
    use treachery_engine::date_time_offset::{MAX_TICKS, TICKS_PER_MINUTE};

    let mut c = Cursor { b: text.as_bytes(), i: 0 };
    c.skip_ws();
    let y = c.digits(4)?;
    c.eat(b'-')?;
    let mo = c.digits(2)?;
    c.eat(b'-')?;
    let d = c.digits(2)?;
    if y < 1 || !(1..=12).contains(&mo) || d < 1 || d > days_in_month(y, mo) {
        return None;
    }
    let mut ticks = days_from_civil(y, mo, d) * TICKS_PER_DAY;

    let has_time = if matches!(c.peek(), Some(b'T' | b't')) {
        c.i += 1;
        true
    } else {
        c.skip_ws() && c.peek().is_some_and(|b| b.is_ascii_digit())
    };
    if has_time {
        let h = c.digits(2)?;
        c.eat(b':')?;
        let mi = c.digits(2)?;
        let mut se = 0;
        let mut fraction = 0.0f64;
        if c.eat(b':').is_some() {
            se = c.digits(2)?;
            if c.eat(b'.').is_some() {
                // C# parity: DateTimeParse.ParseFraction sums digit * 0.1^k in doubles, and the
                // caller adds Math.Round(fraction * TicksPerSecond) ticks (round half to even).
                let mut base = 0.1f64;
                let start = c.i;
                while let Some(digit @ b'0'..=b'9') = c.peek() {
                    fraction += f64::from(digit - b'0') * base;
                    base *= 0.1;
                    c.i += 1;
                }
                if c.i == start {
                    return None;
                }
            }
        }
        if h > 23 || mi > 59 || se > 59 {
            return None;
        }
        ticks += (h * 3600 + mi * 60 + se) * TICKS_PER_SECOND + (fraction * TICKS_PER_SECOND as f64).round_ties_even() as i64;
        if ticks > MAX_TICKS {
            return None;
        }
    }

    c.skip_ws();
    let offset_minutes = match c.peek() {
        Some(b'Z' | b'z') => {
            c.i += 1;
            0
        }
        Some(sign @ (b'+' | b'-')) => {
            c.i += 1;
            let oh = c.digits(2)?;
            let om = if c.eat(b':').is_some() { c.digits(2)? } else { c.digits(2).unwrap_or(0) };
            if om > 59 || oh * 60 + om > 14 * 60 {
                return None;
            }
            if sign == b'-' { -(oh * 60 + om) } else { oh * 60 + om }
        }
        _ => 0,
    };
    c.skip_ws();
    if c.i != c.b.len() {
        return None;
    }
    let utc = ticks - offset_minutes * TICKS_PER_MINUTE;
    if !(0..=MAX_TICKS).contains(&utc) {
        return None;
    }
    Some(DateTimeOffset::new(ticks, offset_minutes as i16))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_ef_samples() {
        // Values copied from a database the C# server wrote.
        for s in [
            "2026-09-19 19:51:27.837212-04:00",
            "0001-01-01 00:00:00+00:00",
            "2026-11-03 14:21:57.543305-08:00",
            "2024-02-29 23:59:59.9999999+14:00",
        ] {
            let v = parse(s).unwrap_or_else(|| panic!("parse {s}"));
            assert_eq!(format(v), s);
        }
    }

    #[test]
    fn reads_other_tools_like_ef() {
        // (text, Ticks, Offset in minutes) as EF read them in the H1 probe under TZ=America/New_York.
        // The full comparison is tests/db_dates.rs.
        for (s, ticks, offset) in [
            ("2026-09-17 19:30:14", 639_252_702_140_000_000, 0),
            ("2026-09-17", 639_252_000_000_000_000, 0),
            ("2026-09-17T19:30:14.123Z", 639_252_702_141_230_000, 0),
            ("2026-09-17T19:30:14.123456+00:00", 639_252_702_141_234_560, 0),
            ("2026-09-17 19:30:14.99999999", 639_252_702_150_000_000, 0),
            ("2026-09-17 19:30:14+0530", 639_252_702_140_000_000, 330),
        ] {
            let v = parse(s).unwrap_or_else(|| panic!("parse {s}"));
            assert_eq!((v.ticks, v.offset_minutes), (ticks, offset), "{s}");
        }
        for s in ["0001-01-01 00:00:00+01:00", "2026-09-17 19:30:14+14:01", "2026-02-29 00:00:00", "2026-09-17T"] {
            assert_eq!(parse(s), None, "{s}");
        }
    }

    #[test]
    fn default_is_year_one() {
        assert_eq!(format(DateTimeOffset::default()), "0001-01-01 00:00:00+00:00");
    }

    #[test]
    fn agrees_with_engine_json_format() {
        // The engine's System.Text.Json writer is checked against .NET; use it as the reference
        // for the calendar arithmetic.
        let v = parse("2026-09-19 19:51:27.837212-04:00").unwrap();
        assert_eq!(v.to_json_string(), "2026-09-19T19:51:27.837212-04:00");
    }
}
