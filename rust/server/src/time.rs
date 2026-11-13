//! The server's clock and its time values.
//!
//! Server times are UTC instants ([`Time`]). The engine stamps events with its own
//! `DateTimeOffset`; [`to_event_time`] and [`from_event_time`] convert between the two.

use std::cell::Cell;

use chrono::{DateTime, NaiveDateTime, SecondsFormat, Utc};
use treachery_engine::date_time_offset::DateTimeOffset;

pub type Time = DateTime<Utc>;

pub trait Clock {
    fn now(&self) -> Time;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Time {
        Utc::now()
    }
}

/// A clock that moves only when told to: tests, and dev scenes with a fixed time.
pub struct ManualClock(Cell<Time>);

impl ManualClock {
    pub fn new(start: Time) -> Self {
        ManualClock(Cell::new(start))
    }

    pub fn set(&self, now: Time) {
        self.0.set(now);
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Time {
        self.0.get()
    }
}

/// Ticks (100 ns) from 0001-01-01 to 1970-01-01: the scale of the engine's times.
const UNIX_EPOCH_TICKS: i64 = 621_355_968_000_000_000;

/// A time as the engine stamps events: UTC, offset zero.
pub fn to_event_time(t: Time) -> DateTimeOffset {
    DateTimeOffset::new(UNIX_EPOCH_TICKS + t.timestamp_micros() * 10, 0)
}

pub fn from_event_time(t: &DateTimeOffset) -> Time {
    DateTime::from_timestamp_micros((t.utc_ticks() - UNIX_EPOCH_TICKS) / 10).unwrap_or_default()
}

/// RFC 3339 in UTC with microseconds: how times are stored and sent.
pub fn format(t: Time) -> String {
    t.to_rfc3339_opts(SecondsFormat::Micros, true)
}

/// Reads a time a client or the database gives: RFC 3339 (`T` or a space between date and time,
/// any fraction), or a date and time without an offset, which is taken as UTC.
pub fn parse(s: &str) -> Option<Time> {
    let s = s.trim();
    if let Ok(t) = DateTime::parse_from_rfc3339(s) {
        return Some(t.with_timezone(&Utc));
    }
    for format in ["%Y-%m-%d %H:%M:%S%.f%:z", "%Y-%m-%dT%H:%M:%S%.f%:z", "%Y-%m-%d %H:%M:%S%.f%#z", "%Y-%m-%dT%H:%M:%S%.f%#z"] {
        if let Ok(t) = DateTime::parse_from_str(s, format) {
            return Some(t.with_timezone(&Utc));
        }
    }
    for format in ["%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%d"] {
        if let Ok(t) = NaiveDateTime::parse_from_str(s, format) {
            return Some(t.and_utc());
        }
    }
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok().map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc())
}

/// Serializes a [`Time`] as [`format`] does (`#[serde(with = "crate::time::serde_time")]`).
pub mod serde_time {
    use serde::Serializer;

    pub fn serialize<S: Serializer>(t: &super::Time, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&super::format(*t))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_what_clients_and_old_databases_hold() {
        let t = parse("2026-11-20T19:00:00Z").unwrap();
        assert_eq!(parse("2026-11-20T20:00:00+01:00"), Some(t));
        assert_eq!(parse("2026-11-20T19:00:00.000Z"), Some(t));
        assert_eq!(parse("2026-11-20 20:00:00+01:00"), Some(t));
        assert_eq!(parse("2026-11-20 20:00:00.0000000+01:00"), Some(t));
        assert_eq!(parse("2026-11-20 19:00:00"), Some(t));
        assert_eq!(parse("2026-11-20T19:00:00"), Some(t));
        assert!(parse("0001-01-01 00:00:00+00:00").is_some());
        assert!(parse("soon").is_none());
        assert_eq!(format(t), "2026-11-20T19:00:00.000000Z");
    }

    #[test]
    fn event_times_round_trip() {
        let t = parse("2026-11-20T19:00:00.123456Z").unwrap();
        assert_eq!(from_event_time(&to_event_time(t)), t);
        assert_eq!(to_event_time(t).to_json_string(), "2026-11-20T19:00:00.123456+00:00");
    }
}
