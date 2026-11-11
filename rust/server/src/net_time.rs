//! The `DateTimeOffset` and `TimeSpan` arithmetic the hub uses, with .NET's rounding and range
//! checks. The engine's `date_time_offset` module has the types; the members only the server
//! needs are here as extension traits.
//!
//! Comparisons (`a > b`, `a == b`) compare UTC instants; that is the engine type's `Ord`.

use dotnet_compat::exception::throw;
use treachery_engine::date_time_offset::{DateTimeOffset, TimeSpan, MAX_TICKS, TICKS_PER_DAY, TICKS_PER_MINUTE};

const TICKS_PER_HOUR: i64 = 60 * TICKS_PER_MINUTE;
const TICKS_PER_MILLISECOND: i64 = 10_000;

pub trait DateTimeOffsetExt {
    /// `AddHours(double)`.
    fn add_hours(self, hours: f64) -> DateTimeOffset;
    /// `AddMinutes(double)`.
    fn add_minutes(self, minutes: f64) -> DateTimeOffset;
    /// `AddMilliseconds(double)`.
    fn add_milliseconds(self, milliseconds: f64) -> DateTimeOffset;
    /// `Subtract(DateTimeOffset)` and `a - b`: the difference of the UTC instants.
    fn subtract(self, other: DateTimeOffset) -> TimeSpan;
}

impl DateTimeOffsetExt for DateTimeOffset {
    fn add_hours(self, hours: f64) -> DateTimeOffset {
        add_units(self, hours, MAX_TICKS / TICKS_PER_HOUR, TICKS_PER_HOUR)
    }

    fn add_minutes(self, minutes: f64) -> DateTimeOffset {
        add_units(self, minutes, MAX_TICKS / TICKS_PER_MINUTE, TICKS_PER_MINUTE)
    }

    fn add_milliseconds(self, milliseconds: f64) -> DateTimeOffset {
        add_units(self, milliseconds, MAX_TICKS / TICKS_PER_MILLISECOND, TICKS_PER_MILLISECOND)
    }

    fn subtract(self, other: DateTimeOffset) -> TimeSpan {
        TimeSpan::from_ticks(self.utc_ticks() - other.utc_ticks())
    }
}

/// `DateTime.AddUnits` (.NET 7 and later) on the clock time, then the `DateTimeOffset`
/// constructor's check that the UTC time is in range.
fn add_units(d: DateTimeOffset, value: f64, max_unit_count: i64, ticks_per_unit: i64) -> DateTimeOffset {
    if value.abs() > max_unit_count as f64 || value.is_nan() {
        throw("ArgumentOutOfRangeException", "Value to add was out of range. (Parameter 'value')".to_string());
    }
    let integral = value.trunc();
    let fractional = value - integral;
    let delta = (integral as i64) * ticks_per_unit + (fractional * ticks_per_unit as f64) as i64;
    let ticks = d.ticks + delta;
    if !(0..=MAX_TICKS).contains(&ticks) {
        throw("ArgumentOutOfRangeException", "The added or subtracted value results in an un-representable DateTime. (Parameter 'value')".to_string());
    }
    let result = DateTimeOffset::new(ticks, d.offset_minutes);
    if !(0..=MAX_TICKS).contains(&result.utc_ticks()) {
        throw("ArgumentOutOfRangeException", "The UTC time represented when the offset is applied must be between year 0 and 10,000. (Parameter 'offset')".to_string());
    }
    result
}

pub trait TimeSpanExt {
    /// `TotalDays`: `(double)Ticks / TicksPerDay`.
    fn total_days(&self) -> f64;
    /// `TotalMinutes`: `(double)Ticks / TicksPerMinute`.
    fn total_minutes(&self) -> f64;
}

impl TimeSpanExt for TimeSpan {
    fn total_days(&self) -> f64 {
        self.ticks as f64 / TICKS_PER_DAY as f64
    }

    fn total_minutes(&self) -> f64 {
        self.ticks as f64 / TICKS_PER_MINUTE as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(ticks: i64, offset: i16) -> DateTimeOffset {
        DateTimeOffset::new(ticks, offset)
    }

    #[test]
    fn add_hours_keeps_the_offset_and_moves_the_clock_time() {
        let d = at(1_000 * TICKS_PER_DAY, 120);
        let r = d.add_hours(7.0);
        assert_eq!(r.ticks, d.ticks + 7 * TICKS_PER_HOUR);
        assert_eq!(r.offset_minutes, 120);
        assert_eq!(d.add_hours(-4.0).ticks, d.ticks - 4 * TICKS_PER_HOUR);
        assert_eq!(d.add_minutes(30.0).ticks, d.ticks + 30 * TICKS_PER_MINUTE);
        assert_eq!(d.add_milliseconds(6000.0).ticks, d.ticks + 6000 * TICKS_PER_MILLISECOND);
        // Fractions are truncated to whole ticks, as in .NET.
        assert_eq!(d.add_hours(0.5).ticks, d.ticks + TICKS_PER_HOUR / 2);
    }

    #[test]
    fn default_plus_hours_is_in_range() {
        let r = DateTimeOffset::default().add_hours(7.0);
        assert_eq!(r.ticks, 7 * TICKS_PER_HOUR);
    }

    #[test]
    fn out_of_range_throws() {
        let caught = treachery_engine::cs::catch(|| DateTimeOffset::default().add_hours(-1.0));
        assert_eq!(caught.unwrap_err().type_name, "ArgumentOutOfRangeException");
        // A clock time in range whose UTC time is not.
        let caught = treachery_engine::cs::catch(|| at(0, 60).add_minutes(1.0));
        assert_eq!(caught.unwrap_err().type_name, "ArgumentOutOfRangeException");
    }

    #[test]
    fn subtract_compares_utc_instants() {
        let a = at(10 * TICKS_PER_DAY, 0);
        let b = at(10 * TICKS_PER_DAY + 60 * TICKS_PER_MINUTE, 60); // the same instant
        assert_eq!(a.subtract(b).ticks, 0);
        let c = at(100 * TICKS_PER_DAY, 0);
        assert_eq!(c.subtract(a).total_days(), 90.0);
        assert_eq!(a.subtract(c).total_days(), -90.0);
        assert_eq!(TimeSpan::from_ticks(TICKS_PER_DAY - 1).total_days() < 1.0, true);
        assert_eq!(TimeSpan::from_ticks(90 * TICKS_PER_MINUTE).total_minutes(), 90.0);
    }
}
