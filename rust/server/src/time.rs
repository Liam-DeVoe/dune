//! The server's clock and its time values.
//!
//! Server times are UTC instants ([`Time`]). The engine stamps events with its own
//! `DateTimeOffset`; [`to_event_time`] and [`from_event_time`] convert between the two.

use std::cell::Cell;

use chrono::{DateTime, Utc};
use treachery_engine::compat::date_time_offset::DateTimeOffset;

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

pub use treachery_protocol::time::{format, parse, serde_time};

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
