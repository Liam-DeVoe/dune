//! How events are written: serde, in the format of rewrite-plan.md, "Wire format".
//!
//! An event is an object tagged with its type, its fields in camelCase:
//! `{"type":"StormDialled","amount":3}`. An event that can be passed (it has a `passed` field)
//! holds its fields in `play`, and a pass is `{"type":"Move","play":null}`, so a pass carries no
//! other fields. A field that is missing takes its default, and an unknown field doesn't decode.
//!
//! Some fields are kept as text inside the engine. They are written as lists:
//! [`force_list`] (`"12:3|1,14:2|0"` as `[{"location":12,"forces":3,"specialForces":1},..]`),
//! [`id_list`] (`"3;17"` as `[3,17]`), [`name_list`] and [`faction_list`].

use serde::de::{DeserializeOwned, Error as _};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

use crate::prelude::*;

/// An event that can't be passed, as written: its type, then its fields.
#[derive(Serialize)]
pub(crate) struct Tagged<'a, T> {
    #[serde(rename = "type")]
    pub type_: &'static str,
    #[serde(flatten)]
    pub event: &'a T,
}

/// An event that can be passed, as written: its type, and its fields unless it is a pass.
#[derive(Serialize)]
pub(crate) struct Passable<'a, T> {
    #[serde(rename = "type")]
    pub type_: &'static str,
    pub play: Option<&'a T>,
}

/// The fields of a non-passable event: the object without `type`.
pub(crate) fn fields<T: DeserializeOwned>(map: Map<String, Value>) -> Result<T, serde_json::Error> {
    serde_json::from_value(Value::Object(map))
}

/// The `play` of a passable event: `None` for a pass. `play` must be there, and nothing else.
pub(crate) fn play<T: DeserializeOwned>(mut map: Map<String, Value>) -> Result<Option<T>, serde_json::Error> {
    let play = map.remove("play").ok_or_else(|| serde_json::Error::missing_field("play"))?;
    if let Some(k) = map.keys().next() {
        return Err(serde_json::Error::unknown_field(k, &["type", "play"]));
    }
    match play {
        Value::Null => Ok(None),
        v => serde_json::from_value(v).map(Some),
    }
}

/// Reads an event: `type`, then the rest as the event of that type.
pub(crate) fn split_type<'de, D: Deserializer<'de>>(d: D) -> Result<(EventType, Map<String, Value>), D::Error> {
    let mut map = Map::<String, Value>::deserialize(d)?;
    let t = map.remove("type").ok_or_else(|| D::Error::missing_field("type"))?;
    let name = t.as_str().ok_or_else(|| D::Error::custom(format!("event type {t} is not a string")))?;
    let t = EventType::from_name(name).ok_or_else(|| D::Error::custom(format!("unknown event type {name:?}")))?;
    Ok((t, map))
}

/// One location of a force list: forces and special forces there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ForceEntry {
    /// The location's id. An id that names no location is kept, and the event leaves it out.
    pub location: i32,
    pub forces: u16,
    pub special_forces: u16,
}

/// `#[serde(with)]` for a force list kept as `location:forces|special,...`.
pub mod force_list {
    use super::*;

    /// The entries of `s`, or None when it isn't a well-formed list.
    pub fn parse(s: &str) -> Option<Vec<ForceEntry>> {
        if s.is_empty() {
            return Some(Vec::new());
        }
        s.split(',')
            .map(|entry| {
                let (location, amounts) = entry.split_once(':')?;
                let (forces, special) = amounts.split_once('|')?;
                Some(ForceEntry { location: canonical(location)?, forces: canonical(forces)?, special_forces: canonical(special)? })
            })
            .collect()
    }

    pub fn format(entries: &[ForceEntry]) -> String {
        entries.iter().map(|e| format!("{}:{}|{}", e.location, e.forces, e.special_forces)).collect::<Vec<_>>().join(",")
    }

    pub fn serialize<S: Serializer>(v: &Option<String>, s: S) -> Result<S::Ok, S::Error> {
        match v {
            None => s.serialize_none(),
            Some(text) => parse(text).ok_or_else(|| serde::ser::Error::custom(format!("not a force list: {text:?}")))?.serialize(s),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
        Ok(Option::<Vec<ForceEntry>>::deserialize(d)?.map(|e| format(&e)))
    }
}

/// A number written the way `format!` writes it, so that reading and writing it again gives
/// the same text.
fn canonical<T: std::str::FromStr + ToString>(s: &str) -> Option<T> {
    let n: T = s.parse().ok()?;
    (n.to_string() == s).then_some(n)
}

/// `#[serde(with)]` for ids kept as `3;17` (`Event::id_string_to_objects`).
pub mod id_list {
    use super::*;

    pub fn serialize<S: Serializer>(v: &Option<String>, s: S) -> Result<S::Ok, S::Error> {
        match v.as_deref() {
            None => s.serialize_none(),
            Some("") => Vec::<i32>::new().serialize(s),
            Some(text) => text
                .split(Event::ID_STRING_SEPARATOR)
                .map(canonical::<i32>)
                .collect::<Option<Vec<i32>>>()
                .ok_or_else(|| serde::ser::Error::custom(format!("not a list of ids: {text:?}")))?
                .serialize(s),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
        Ok(Option::<Vec<i32>>::deserialize(d)?.map(|ids| ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(&Event::ID_STRING_SEPARATOR.to_string())))
    }
}

/// `#[serde(with)]` for names kept as `Alice>Bob` (`EstablishPlayers::players`).
pub mod name_list {
    use super::*;

    pub fn serialize<S: Serializer>(v: &Option<String>, s: S) -> Result<S::Ok, S::Error> {
        match v.as_deref() {
            None => s.serialize_none(),
            Some("") => Vec::<&str>::new().serialize(s),
            Some(text) => text.split('>').collect::<Vec<_>>().serialize(s),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
        let names = Option::<Vec<String>>::deserialize(d)?;
        if let Some(n) = names.as_ref().and_then(|names| names.iter().find(|n| n.contains('>'))) {
            return Err(D::Error::custom(format!("a player name with '>': {n:?}")));
        }
        Ok(names.map(|n| n.join(">")))
    }
}

/// `#[serde(with)]` for factions kept as `Atreides,Harkonnen` (`EstablishPlayers::factions_in_play`),
/// written as the factions that text names.
pub mod faction_list {
    use super::*;

    pub fn serialize<S: Serializer>(v: &Option<String>, s: S) -> Result<S::Ok, S::Error> {
        match v {
            None => s.serialize_none(),
            Some(_) => EstablishPlayers { factions_in_play: v.clone(), ..Default::default() }.factions_in_play().serialize(s),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
        Ok(Option::<Vec<Faction>>::deserialize(d)?.map(|f| f.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(",")))
    }
}

/// Static objects are written as their id. An id that names no object of the type doesn't
/// decode. In TypeScript they are numbers.
macro_rules! handle_wire {
    ($($t:ty => $name:literal, $find:expr;)*) => {$(
        impl<'de> Deserialize<'de> for $t {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let id = i32::deserialize(d)?;
                let find: fn(i32) -> Option<$t> = $find;
                find(id).ok_or_else(|| D::Error::custom(format!("{id} is not a valid {}", $name)))
            }
        }

        impl ts_rs::TS for $t {
            type WithoutGenerics = Self;
            type OptionInnerType = Self;

            fn name(_: &ts_rs::Config) -> String {
                "number".to_string()
            }

            fn inline(cfg: &ts_rs::Config) -> String {
                <Self as ts_rs::TS>::name(cfg)
            }
        }
    )*};
}

handle_wire! {
    Location => "Location", Location::from_id;
    Territory => "Territory", Territory::from_id;
    Leader => "Leader", |id| LeaderManager::leader_lookup().find(id);
    Hero => "Hero", |id| LeaderManager::hero_lookup().find(id);
    TreacheryCard => "TreacheryCard", |id| TreacheryCardManager::lookup().find(id);
}

/// A time as RFC 3339 text: `2026-11-13T04:14:35.622405Z`, or with its offset when it isn't UTC.
impl Serialize for DateTimeOffset {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let text = self.to_json_string();
        match text.strip_suffix("+00:00") {
            Some(utc) => s.serialize_str(&format!("{utc}Z")),
            None => s.serialize_str(&text),
        }
    }
}

impl<'de> Deserialize<'de> for DateTimeOffset {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        DateTimeOffset::parse_json(&text).ok_or_else(|| D::Error::custom(format!("not a time: {text:?}")))
    }
}

impl ts_rs::TS for DateTimeOffset {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn name(_: &ts_rs::Config) -> String {
        "string".to_string()
    }

    fn inline(cfg: &ts_rs::Config) -> String {
        <Self as ts_rs::TS>::name(cfg)
    }
}

impl ts_rs::TS for Turn {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn name(_: &ts_rs::Config) -> String {
        "number".to_string()
    }

    fn inline(cfg: &ts_rs::Config) -> String {
        <Self as ts_rs::TS>::name(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn force_lists_read_back_as_written() {
        for s in ["", "12:3|1", "12:3|1,14:2|0", "-1:0|0"] {
            assert_eq!(force_list::parse(s).map(|e| force_list::format(&e)).as_deref(), Some(s));
        }
        for s in ["12", "12:3", "12:+3|1", "12:03|1", "12:70000|0", "12:3|1,"] {
            assert_eq!(force_list::parse(s), None, "{s}");
        }
    }
}
