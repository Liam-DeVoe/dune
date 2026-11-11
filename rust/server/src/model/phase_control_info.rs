//! Port of `Model/PhaseControlInfo.cs`: what the current pause between phases is waiting for.
//! Sent to everyone in the game; every field is public.

use treachery_engine::date_time_offset::DateTimeOffset;
use treachery_engine::json::{JsonWriter, ToJson};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PhaseControlInfo {
    /// False when the host alone continues the current phase, as before.
    pub active: bool,
    /// Seats of the players who may still act or pass here.
    pub waiting_for: Vec<i32>,
    /// Whether this pause runs a countdown that any player can hold.
    pub countdown: bool,
    /// When the countdown runs out, while it is running and nobody holds it.
    pub countdown_ends: Option<DateTimeOffset>,
    /// Seats of the players holding the countdown.
    pub holding: Vec<i32>,
}

impl ToJson for PhaseControlInfo {
    fn write_json(&self, w: &mut JsonWriter) {
        w.object_value();
        w.member("Active", &self.active);
        w.member("WaitingFor", &self.waiting_for);
        w.member("Countdown", &self.countdown);
        w.member("CountdownEnds", &self.countdown_ends);
        w.member("Holding", &self.holding);
        w.end_object();
    }
}
