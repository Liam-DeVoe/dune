//! How the replay reads and compares events.
//!
//! A trace holds events as the reference encoded them. The replay never compares that text. It
//! decodes each event into the working tree's `Event` and compares the working tree's own
//! encoding of it, so a change in how events are written doesn't show up as a difference.
//!
//! The reference and the working tree use the same event format today, so both go through the
//! engine's reader. When the format changes (wave 4b of rewrite-plan.md), the reference keeps
//! recording the old one: [`from_reference`] is then the one place that reads it, through
//! `legacy_events.rs`, while [`from_working`] and [`encode`] switch to the new format.

use serde_json::Value;
use treachery_engine::events::Event;
use treachery_engine::json::{self, FromJson};

/// Decodes an event as the reference recorded it. The error text is for reports only.
pub fn from_reference(v: &Value) -> Result<Event, String> {
    Event::from_json(v).map_err(|e| format!("{e:?}"))
}

/// Decodes an event in the working tree's own format.
pub fn from_working(v: &Value) -> Result<Event, String> {
    Event::from_json(v).map_err(|e| format!("{e:?}"))
}

/// The working tree's encoding of an event.
pub fn encode(e: &Event) -> Value {
    let text = json::serialize(e);
    serde_json::from_str(&text).unwrap_or(Value::String(text))
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

/// The event's type name, for reports: `$type` in the current format, `type` in the planned one.
pub fn type_name(v: &Value) -> &str {
    v.get("$type").or_else(|| v.get("type")).and_then(|t| t.as_str()).unwrap_or("?")
}
