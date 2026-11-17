//! How the harness reads, writes and compares events.
//!
//! A trace holds events as its recorder encoded them. The replay never compares that text. It
//! decodes each event into the working tree's `Event` and compares the working tree's own
//! encoding of it, so a change in how events are written doesn't show up as a difference.
//!
//! The recorder's fuzzer and `observe refit` build and change events as maps of the format of
//! version 190 (`crate::event_maps`), with the old member names the fuzzer's heuristics key on, so
//! the fuzzer makes the same games as before the format changed (fuzz.rs). [`to_trace`] converts
//! each such event to the engine's format before it is submitted and written. An event the
//! engine's format can't write stays a map in the trace, so [`decode`] reads both.

use serde_json::{Map, Value};
use treachery_engine::events::{Event, HistoryEntry};
use treachery_engine::prelude::Faction;
use crate::event_maps::{self, FromJson};

/// Whether `v` is an event map in the format of version 190 (`crate::event_maps`).
pub fn is_legacy(v: &Value) -> bool {
    v.get("$type").is_some()
}

/// Decodes an event in either format. The error text is for reports only.
pub fn decode(v: &Value) -> Result<HistoryEntry, String> {
    if is_legacy(v) {
        HistoryEntry::from_json(v).map_err(|e| format!("{e:?}"))
    } else {
        serde_json::from_value(v.clone()).map_err(|e| e.to_string())
    }
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

/// The event's type name, for reports.
pub fn type_name(v: &Value) -> &str {
    v.get("$type").or_else(|| v.get("event").and_then(|e| e.get("type"))).and_then(|t| t.as_str()).unwrap_or("?")
}

/// An entry as an object of the old format, as the fuzzer builds and changes events.
pub fn legacy_object(e: &HistoryEntry) -> Map<String, Value> {
    match serde_json::from_str(&event_maps::serialize(e)) {
        Ok(Value::Object(m)) => m,
        _ => Map::new(),
    }
}

/// An event the recorder built in the old format, as it submits and records it: decoded, and
/// written in the new format. An event that doesn't decode, or holds text the new format can't
/// write as a list, stays as it is; the replay reads it the same way.
pub fn to_trace(event: &Map<String, Value>) -> (Value, Result<HistoryEntry, String>) {
    let old = Value::Object(event.clone());
    match decode(&old) {
        Ok(e) => match serde_json::to_value(&e) {
            Ok(v) => (v, Ok(e)),
            Err(_) => (old, Ok(e)),
        },
        Err(err) => (old, Err(err)),
    }
}

/// Rewrites the events in a view's JSON (the drafts and the current permissions of action
/// options, the deals panel's buttons) as objects of the old format submitted by `by`, as the
/// fuzzer reads them. Views write them in the engine's format, without a submitter or time.
pub fn legacy_view_events(v: &mut Value, by: Option<Faction>) {
    match v {
        Value::Object(m) => {
            for (k, x) in m.iter_mut() {
                if matches!(k.as_str(), "draft" | "current" | "accept" | "decline" | "withdraw") && x.get("type").is_some() {
                    if let Ok(e) = serde_json::from_value::<Event>(x.clone()) {
                        *x = Value::Object(legacy_object(&HistoryEntry::new(by, e)));
                        continue;
                    }
                }
                legacy_view_events(x, by);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|x| legacy_view_events(x, by)),
        _ => {}
    }
}
