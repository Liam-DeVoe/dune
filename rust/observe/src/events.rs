//! How the harness reads, writes and compares events.
//!
//! A trace holds events as its recorder encoded them. The replay never compares that text. It
//! decodes each event into the working tree's `Event` and compares the working tree's own
//! encoding of it, so a change in how events are written doesn't show up as a difference.
//!
//! The recorder's fuzzer and `observe refit` build and change events as flat maps ([`flatten`]):
//! the engine's JSON of the event with its submitter (`by`), time (`at`) and `type` next to its
//! fields, and a pass event's `play` replaced by a `passed` flag and the fields themselves. The
//! fuzzer changes fields one by one, and with `passed` a bool like any other it can turn a pass
//! into an action and back. [`to_trace`] turns such a map back into a history entry before it is
//! submitted and written.

use std::collections::HashSet;
use std::sync::OnceLock;

use serde_json::{Map, Value};
use treachery_engine::events::{Event, EventType, HistoryEntry};
use treachery_engine::prelude::Faction;

/// Decodes an event. The error text is for reports only.
pub fn decode(v: &Value) -> Result<HistoryEntry, String> {
    serde_json::from_value(v.clone()).map_err(|e| e.to_string())
}

/// Decodes an event as the reference recorded it.
pub fn from_reference(v: &Value) -> Result<HistoryEntry, String> {
    decode(v)
}

/// Decodes an event in the working tree's own format.
pub fn from_working(v: &Value) -> Result<HistoryEntry, String> {
    decode(v)
}

/// The working tree's encoding of an event.
pub fn encode(e: &HistoryEntry) -> Value {
    serde_json::to_value(e).unwrap_or_else(|err| Value::String(format!("unwritable: {err}")))
}

/// A reference event in the form the replay compares: decoded, then encoded by the working tree.
pub fn normalize_reference(v: &Value) -> Result<Value, String> {
    from_reference(v).map(|e| encode(&e))
}

/// The same for an event the working tree encoded itself. It is decoded and encoded again, so a
/// field the reader ignores counts on neither side.
pub fn normalize_working(v: &Value) -> Result<Value, String> {
    from_working(v).map(|e| encode(&e))
}

/// The event's type name, for reports. Traces recorded before game version 191, and events a
/// recorder of that time couldn't write in the new format, have it in `$type`.
pub fn type_name(v: &Value) -> &str {
    v.get("$type").or_else(|| v.get("event").and_then(|e| e.get("type"))).and_then(|t| t.as_str()).unwrap_or("?")
}

/// Whether events of type `t` can be passed, which the JSON writes as `"play": null`.
pub fn passable(t: EventType) -> bool {
    static PASSABLE: OnceLock<HashSet<EventType>> = OnceLock::new();
    PASSABLE
        .get_or_init(|| EventType::ALL.iter().copied().filter(|t| serde_json::to_value(t.default_event()).is_ok_and(|v| v.get("play").is_some())).collect())
        .contains(&t)
}

/// The fields of event type `t` with their defaults, as `play` holds them for a passable type.
fn default_fields(t: EventType) -> Map<String, Value> {
    match serde_json::to_value(t.default_event()) {
        Ok(Value::Object(mut m)) => {
            m.remove("type");
            match m.remove("play") {
                Some(Value::Object(play)) => play,
                _ => m,
            }
        }
        _ => Map::new(),
    }
}

/// The flat map of an event in the engine's JSON (`{"type":..}`), submitted by `by` (a faction
/// name or null) at `at`, if given. None if `event` names no event type.
pub fn flatten_event(by: Value, at: Option<Value>, event: &Value) -> Option<Map<String, Value>> {
    let Value::Object(event) = event else { return None };
    let t = EventType::from_name(event.get("type")?.as_str()?)?;
    let mut m = Map::new();
    m.insert("by".into(), by);
    if let Some(at) = at {
        m.insert("at".into(), at);
    }
    m.insert("type".into(), Value::from(t.name()));
    if passable(t) {
        let (passed, fields) = match event.get("play") {
            Some(Value::Object(play)) => (false, play.clone()),
            _ => (true, default_fields(t)),
        };
        m.insert("passed".into(), Value::Bool(passed));
        m.extend(fields);
    } else {
        m.extend(event.iter().filter(|(k, _)| *k != "type").map(|(k, v)| (k.clone(), v.clone())));
    }
    Some(m)
}

/// The flat map of a history entry. None if the entry can't be written (a force list or card id
/// text that isn't a well-formed list).
pub fn flatten(e: &HistoryEntry) -> Option<Map<String, Value>> {
    let Value::Object(mut v) = serde_json::to_value(e).ok()? else { return None };
    let event = v.remove("event")?;
    flatten_event(v.remove("by").unwrap_or(Value::Null), v.remove("at"), &event)
}

/// The flat map of event `e` submitted by `by`, as the client's forms and the deals panel send it.
pub fn flatten_new(by: Option<Faction>, e: Event) -> Option<Map<String, Value>> {
    flatten(&HistoryEntry::new(by, e))
}

/// The history entry JSON of a flat map. A map that isn't flat (an entry of a file that didn't
/// decode) is returned as it is.
pub fn unflatten(m: &Map<String, Value>) -> Value {
    if m.contains_key("event") || !m.contains_key("type") {
        return Value::Object(m.clone());
    }
    let type_name = m["type"].as_str().unwrap_or("");
    let mut fields: Map<String, Value> = m.iter().filter(|(k, _)| !matches!(k.as_str(), "by" | "at" | "type" | "passed")).map(|(k, v)| (k.clone(), v.clone())).collect();
    let mut event = Map::new();
    event.insert("type".into(), Value::from(type_name));
    if EventType::from_name(type_name).is_some_and(passable) {
        let passed = m.get("passed").and_then(|p| p.as_bool()).unwrap_or(false);
        event.insert("play".into(), if passed { Value::Null } else { Value::Object(fields) });
    } else {
        event.append(&mut fields);
    }
    let at = match m.get("at") {
        Some(at) if !at.is_null() => at.clone(),
        _ => default_time(),
    };
    serde_json::json!({ "by": m.get("by").cloned().unwrap_or(Value::Null), "at": at, "event": event })
}

fn default_time() -> Value {
    static AT: OnceLock<Value> = OnceLock::new();
    AT.get_or_init(|| serde_json::to_value(HistoryEntry::new(None, Event::from(treachery_engine::events::EndPhase::default())).at).unwrap_or(Value::Null)).clone()
}

/// A flat map as the recorder submits and records it: the history entry JSON, written again
/// after decoding, and the decoded entry. An event that doesn't decode is recorded as it is.
pub fn to_trace(event: &Map<String, Value>) -> (Value, Result<HistoryEntry, String>) {
    let v = unflatten(event);
    match decode(&v) {
        Ok(e) => match serde_json::to_value(&e) {
            Ok(written) => (written, Ok(e)),
            Err(_) => (v, Ok(e)),
        },
        Err(err) => (v, Err(err)),
    }
}
