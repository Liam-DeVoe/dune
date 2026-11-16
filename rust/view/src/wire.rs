//! Events in views: the drafts of action forms and the deals panel's buttons, which the client
//! sends back as they are. They are written in the engine's format (`{"type":..}`), without the
//! submitter or a time: the server fills those in.
//!
//! The harness replays traces of a reference that still writes views with events in the format
//! of version 190 (rewrite-plan.md, wave 4b). While it does, `observe` sets a [`LegacyWire`] for
//! the replay's thread, and views write their events and version as that reference did. Delete
//! this once the shared reference records version 191.

use std::cell::Cell;

use serde_json::Value;
use treachery_engine::prelude::*;

/// How views wrote events before version 191.
#[derive(Clone, Copy)]
pub struct LegacyWire {
    /// The old JSON of a history entry: the event with `Initiator` and `Time`.
    pub encode: fn(&HistoryEntry) -> Value,
    /// The game version views show.
    pub version: i32,
}

thread_local! {
    static LEGACY: Cell<Option<LegacyWire>> = const { Cell::new(None) };
}

/// Views built on this thread write events the old way (`Some`), or the engine's way (`None`).
pub fn set_legacy(wire: Option<LegacyWire>) {
    LEGACY.with(|l| l.set(wire));
}

/// The draft or button for `event`, as the viewer `by` would submit it.
pub fn event_json(by: Option<Faction>, event: &Event) -> Value {
    match LEGACY.with(Cell::get) {
        Some(l) => (l.encode)(&HistoryEntry::new(by, event.clone())),
        None => serde_json::to_value(event).expect("events serialize"),
    }
}

/// The button that submits `entry`'s event again (withdrawing a deal offer).
pub fn entry_json(entry: &HistoryEntry) -> Value {
    match LEGACY.with(Cell::get) {
        Some(l) => (l.encode)(entry),
        None => serde_json::to_value(&entry.event).expect("events serialize"),
    }
}

/// The game version the view shows.
pub fn shown_version(game: &Game) -> i32 {
    LEGACY.with(Cell::get).map_or(game.rules.version, |l| l.version)
}
