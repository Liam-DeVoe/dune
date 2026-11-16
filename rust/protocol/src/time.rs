//! Times on the wire: RFC 3339 strings. The server writes UTC with microseconds
//! (`2026-09-18T04:14:35.622405Z`) and reads any offset. A time without an offset is UTC.

use chrono::{DateTime, NaiveDateTime, SecondsFormat, Utc};

pub type Time = DateTime<Utc>;

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

/// `#[serde(with = "crate::time::serde_time")]`.
pub mod serde_time {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(t: &super::Time, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&super::format(*t))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<super::Time, D::Error> {
        let s = String::deserialize(d)?;
        super::parse(&s).ok_or_else(|| serde::de::Error::custom(format!("not a time: {s}")))
    }
}

/// `#[serde(with = "crate::time::optional_time")]`, for `Option<Time>`.
pub mod optional_time {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(t: &Option<super::Time>, s: S) -> Result<S::Ok, S::Error> {
        match t {
            Some(t) => s.serialize_str(&super::format(*t)),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<super::Time>, D::Error> {
        match Option::<String>::deserialize(d)? {
            Some(s) => super::parse(&s).map(Some).ok_or_else(|| serde::de::Error::custom(format!("not a time: {s}"))),
            None => Ok(None),
        }
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
}
