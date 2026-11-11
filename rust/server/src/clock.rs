//! `DateTimeOffset.Now` and `DateTime.Now`, behind a trait so the equivalence harness can
//! run the C# and Rust servers on the same virtual clock (see the plan, "Equivalence").
//!
//! The C# server calls `DateTimeOffset.Now` directly; the harness replaces it through the
//! `TestHooks.Now` hook (rust/docs/csharp-hooks.md). Every C# `DateTimeOffset.Now`,
//! `DateTime.Now` and `new ConnectionInfo()` in the server becomes `clock.now()`.

use std::cell::Cell;

use treachery_engine::date_time_offset::DateTimeOffset;

pub trait Clock {
    /// `DateTimeOffset.Now`: local time with the machine's current UTC offset.
    fn now(&self) -> DateTimeOffset;
}

/// .NET ticks (100 ns) from 0001-01-01 to 1970-01-01.
pub const UNIX_EPOCH_TICKS: i64 = 621_355_968_000_000_000;

/// The real clock, with the local offset as .NET computes it.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTimeOffset {
        let now = chrono::Local::now();
        let offset_seconds = now.offset().local_minus_utc();
        let utc_ticks = UNIX_EPOCH_TICKS + now.timestamp() * 10_000_000 + i64::from(now.timestamp_subsec_nanos() / 100);
        let offset_minutes = (offset_seconds / 60) as i16;
        DateTimeOffset::new(utc_ticks + i64::from(offset_minutes) * 600_000_000, offset_minutes)
    }
}

/// A clock that only moves when told to. Used by tests and the equivalence harness.
pub struct ManualClock {
    now: Cell<DateTimeOffset>,
}

impl ManualClock {
    pub fn new(start: DateTimeOffset) -> Self {
        ManualClock { now: Cell::new(start) }
    }

    pub fn set(&self, now: DateTimeOffset) {
        self.now.set(now);
    }
}

impl Clock for ManualClock {
    fn now(&self) -> DateTimeOffset {
        self.now.get()
    }
}
