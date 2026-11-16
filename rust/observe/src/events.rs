//! How the harness reads, writes and compares events.
//!
//! A trace holds events as its recorder encoded them. The replay never compares that text. It
//! decodes each event into the working tree's `Event` and compares the working tree's own
//! encoding of it, so a change in how events are written doesn't show up as a difference.
//!
//! Version 191 changed the format (rewrite-plan.md, wave 4b): an entry is
//! `{"by":..,"at":..,"event":{"type":..}}`, where version 190 wrote `{"$type":..,"Initiator":..,
//! "Time":..}`. [`decode`] reads both, so traces of a reference that records version 190 still
//! replay. Replaying such a trace, the working tree also writes the events in its views the old
//! way and shows version 190 ([`legacy_views`]), because the reference hashed views like that.
//!
//! The recorder's fuzzer still builds events as maps of the old format, with the old member names
//! its heuristics key on, so it makes the same games as before (fuzz.rs). [`to_trace`] converts
//! each to the new format before it is submitted and written.

use serde_json::{Map, Value};
use treachery_engine::events::HistoryEntry;
use treachery_engine::game::Game;
use treachery_legacy::{self as legacy, FromJson};

/// Whether `v` is an event in the format of version 190.
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
    match serde_json::from_str(&legacy::serialize(e)) {
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

/// An event of a trace or a session (either format) as an object of the old format, for the
/// fuzzer. None when it doesn't decode.
pub fn to_legacy_object(v: &Value) -> Option<Map<String, Value>> {
    if let Value::Object(m) = v {
        if is_legacy(v) {
            return Some(m.clone());
        }
    }
    decode(v).ok().map(|e| legacy_object(&e))
}

/// The view of events and the version as a reference of version 190 wrote them.
fn legacy_wire() -> treachery_view::wire::LegacyWire {
    treachery_view::wire::LegacyWire {
        encode: |e| serde_json::from_str(&legacy::serialize(e)).expect("the old writer writes valid JSON"),
        version: legacy::LAST_LEGACY_VERSION,
    }
}

/// Views built on this thread write events in the format of version 190 (`true`) or in the
/// engine's (`false`). See the top of this file.
pub fn legacy_views(on: bool) {
    treachery_view::wire::set_legacy(on.then(legacy_wire));
}

/// Runs `f` with views that write events in the old format, as the fuzzer reads them.
pub fn with_legacy_views<R>(f: impl FnOnce() -> R) -> R {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            legacy_views(self.0);
        }
    }
    let _restore = Restore(LEGACY_REPLAY.with(|l| l.get()));
    legacy_views(true);
    f()
}

thread_local! {
    /// Whether this thread replays a trace of version 190, whose views stay in the old format.
    static LEGACY_REPLAY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The game version the working tree plays a trace of `version` at: version 190 is played as
/// the version after it, which has the same rules, and its views are written as the reference
/// wrote them.
pub fn replay_version(version: i32) -> i32 {
    let legacy = version == legacy::LAST_LEGACY_VERSION;
    LEGACY_REPLAY.with(|l| l.set(legacy));
    legacy_views(legacy);
    if legacy { Game::LATEST_VERSION } else { version }
}
