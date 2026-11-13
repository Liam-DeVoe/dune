//! `System.DateTimeOffset` and `System.TimeSpan`, as far as the engine and its
//! JSON need them. Support module (no C# counterpart in Shared).
//!
//! Event timestamps (`GameEvent.Time`) feed game state through timers
//! (hazards.md section 13), and they are part of the savegame JSON.

use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

pub const TICKS_PER_SECOND: i64 = 10_000_000;
pub const TICKS_PER_MINUTE: i64 = 60 * TICKS_PER_SECOND;
pub const TICKS_PER_DAY: i64 = 86_400 * TICKS_PER_SECOND;
/// `DateTime.MaxValue.Ticks`
pub const MAX_TICKS: i64 = 3_155_378_975_999_999_999;

/// `DateTimeOffset`: a clock time (`DateTime.Ticks`, 100 ns since 0001-01-01)
/// plus an offset from UTC. Like C#, equality and ordering compare the UTC
/// instant, so `12:00+01:00 == 11:00+00:00`.
#[derive(Clone, Copy, Debug, Default)]
pub struct DateTimeOffset {
    /// `Ticks` (the clock time at `offset_minutes`, not UTC).
    pub ticks: i64,
    /// `Offset` in whole minutes (.NET allows -14:00 to +14:00).
    pub offset_minutes: i16,
}

/// `TimeSpan`: a signed number of ticks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TimeSpan {
    pub ticks: i64,
}

impl TimeSpan {
    pub fn from_ticks(ticks: i64) -> Self {
        TimeSpan { ticks }
    }

    /// `TotalHours`: `(double)Ticks / TicksPerHour`.
    pub fn total_hours(&self) -> f64 {
        self.ticks as f64 / (3600 * TICKS_PER_SECOND) as f64
    }

    /// `t1 + t2` (`TimeSpan.Add`): OverflowException when the sum leaves the range.
    #[track_caller]
    pub fn add(self, other: TimeSpan) -> TimeSpan {
        match self.ticks.checked_add(other.ticks) {
            Some(ticks) => TimeSpan { ticks },
            None => crate::compat::cs::throw("OverflowException", "TimeSpan overflowed because the duration is too long.".to_string()),
        }
    }
}

impl DateTimeOffset {
    pub fn new(ticks: i64, offset_minutes: i16) -> Self {
        DateTimeOffset { ticks, offset_minutes }
    }

    /// `UtcTicks`
    pub fn utc_ticks(&self) -> i64 {
        self.ticks - self.offset_minutes as i64 * TICKS_PER_MINUTE
    }

    /// The `$"{dto:yyyy-MM-ddTHH:mm:ss.FFFFFFFzzz}"` form that System.Text.Json writes:
    /// fraction digits with trailing zeros removed (and no `.` when zero), and the
    /// offset always as `+HH:MM` (never `Z`).
    pub fn to_json_string(&self) -> String {
        let mut s = self.clock_string();
        let frac = self.ticks.rem_euclid(TICKS_PER_SECOND);
        if frac != 0 {
            let digits = format!("{frac:07}");
            s.push('.');
            s.push_str(digits.trim_end_matches('0'));
        }
        s.push_str(&self.offset_string());
        s
    }

    /// `ToString("o")`: always seven fraction digits (the oracle's state dump uses this).
    pub fn to_round_trip_string(&self) -> String {
        let frac = self.ticks.rem_euclid(TICKS_PER_SECOND);
        format!("{}.{frac:07}{}", self.clock_string(), self.offset_string())
    }

    fn clock_string(&self) -> String {
        let days = self.ticks.div_euclid(TICKS_PER_DAY);
        let (y, m, d) = civil_from_days(days);
        let secs = self.ticks.rem_euclid(TICKS_PER_DAY) / TICKS_PER_SECOND;
        format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60)
    }

    fn offset_string(&self) -> String {
        let o = self.offset_minutes as i32;
        let sign = if o < 0 { '-' } else { '+' };
        format!("{sign}{:02}:{:02}", o.abs() / 60, o.abs() % 60)
    }

    /// Parses the ISO 8601 subset System.Text.Json accepts for `DateTimeOffset`:
    /// `yyyy-MM-dd[THH:mm[:ss[.fffffff...]]][Z|±HH:mm]`. Fraction digits past seven
    /// are truncated.
    ///
    /// Known difference: C# reads a value without an offset as machine-local time.
    /// This reads it as UTC. Savegames written by the server always carry an offset.
    pub fn parse_json(s: &str) -> Option<DateTimeOffset> {
        let b = s.as_bytes();
        let num = |from: usize, len: usize| -> Option<i64> {
            let part = b.get(from..from + len)?;
            if !part.iter().all(|c| c.is_ascii_digit()) {
                return None;
            }
            std::str::from_utf8(part).ok()?.parse().ok()
        };
        let year = num(0, 4)?;
        if b.get(4) != Some(&b'-') || b.get(7) != Some(&b'-') {
            return None;
        }
        let month = num(5, 2)?;
        let day = num(8, 2)?;
        if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) || year < 1 {
            return None;
        }
        let mut i = 10;
        let (mut hour, mut minute, mut second, mut frac) = (0, 0, 0, 0i64);
        if b.get(i) == Some(&b'T') {
            hour = num(i + 1, 2)?;
            if b.get(i + 3) != Some(&b':') {
                return None;
            }
            minute = num(i + 4, 2)?;
            i += 6;
            if b.get(i) == Some(&b':') {
                second = num(i + 1, 2)?;
                i += 3;
                if b.get(i) == Some(&b'.') {
                    i += 1;
                    let start = i;
                    while i < b.len() && b[i].is_ascii_digit() {
                        i += 1;
                    }
                    if i == start {
                        return None;
                    }
                    let digits = &s[start..i.min(start + 7)];
                    frac = format!("{digits:0<7}").parse().ok()?;
                }
            }
            if hour > 23 || minute > 59 || second > 59 {
                return None;
            }
        }
        let offset_minutes: i64 = match b.get(i) {
            None => 0,
            Some(b'Z') if i + 1 == b.len() => 0,
            Some(&c @ (b'+' | b'-')) if i + 6 == b.len() && b[i + 3] == b':' => {
                let (oh, om) = (num(i + 1, 2)?, num(i + 4, 2)?);
                if oh > 14 || om > 59 || oh == 14 && om != 0 {
                    return None;
                }
                let v = oh * 60 + om;
                if c == b'-' { -v } else { v }
            }
            _ => return None,
        };
        let days = days_from_civil(year, month, day);
        let ticks = days * TICKS_PER_DAY + (hour * 3600 + minute * 60 + second) * TICKS_PER_SECOND + frac;
        let result = DateTimeOffset { ticks, offset_minutes: offset_minutes as i16 };
        let utc = result.utc_ticks();
        if !(0..=MAX_TICKS).contains(&utc) {
            return None;
        }
        Some(result)
    }
}

impl PartialEq for DateTimeOffset {
    fn eq(&self, other: &Self) -> bool {
        self.utc_ticks() == other.utc_ticks()
    }
}

impl Eq for DateTimeOffset {}

impl Hash for DateTimeOffset {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.utc_ticks().hash(state)
    }
}

impl PartialOrd for DateTimeOffset {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DateTimeOffset {
    fn cmp(&self, other: &Self) -> Ordering {
        self.utc_ticks().cmp(&other.utc_ticks())
    }
}

fn is_leap(y: i64) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if is_leap(y) => 29,
        _ => 28,
    }
}

/// Days since 0001-01-01 (proleptic Gregorian).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468 + 719_162
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days - 719_162 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_format() {
        assert_eq!(DateTimeOffset::default().to_json_string(), "0001-01-01T00:00:00+00:00");
        let d = DateTimeOffset::parse_json("2024-02-29T13:45:01.1234500-05:30").unwrap();
        assert_eq!(d.to_json_string(), "2024-02-29T13:45:01.12345-05:30");
        assert_eq!(d.to_round_trip_string(), "2024-02-29T13:45:01.1234500-05:30");
        assert_eq!(DateTimeOffset::parse_json("2024-01-01").unwrap().to_json_string(), "2024-01-01T00:00:00+00:00");
        assert!(DateTimeOffset::parse_json("2023-02-29T00:00:00+00:00").is_none());
    }
}
