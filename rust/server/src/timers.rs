//! `Task.Delay(ms).ContinueWith(...)` in the hub, as data.
//!
//! The C# hub schedules work for later in two places: `SendAsyncPlayMessagesIfApplicable`
//! re-runs itself when the mail interval has not passed yet (GameHub_GameEvents.cs:238),
//! `ScheduleBotEvent` runs a bot turn after a delay (GameHub_GameEvents.cs:325), and the phase
//! control runs out a pause's countdown. A delayed action here is a [`TimerAction`] handed to [`Timers`]. The real
//! implementation (`runtime`) sleeps and then queues the action on the hub thread, where it
//! runs like any other hub call. The equivalence harness keeps them in a list and fires the
//! due ones when the scenario advances the virtual clock.

use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimerAction {
    /// `SendAsyncPlayMessagesIfApplicable(gameId)` (GameHub_GameEvents.cs:239).
    SendAsyncPlayMessages { game_id: String },
    /// The countdown of a pause runs out (GameHub_PhaseControl.cs, `"PhaseCountdown " + gameId`).
    PhaseCountdown { game_id: String, pause_id: i32 },
    /// `PerformBotEvent(managedGame)` (GameHub_GameEvents.cs:325). C# captures the `ManagedGame`
    /// object, not its id, so the hub keeps that object under `id` until the call runs
    /// (`GameHub::bot_timer_games`): the action itself must stay `Send`.
    PerformBotEvent { game_id: String, id: u64 },
}

pub trait Timers {
    /// Runs `action` on the hub thread after `delay`. Fire and forget, like `_ = Task.Delay(...)`.
    fn schedule(&self, delay: Duration, action: TimerAction);
}

/// Records scheduled actions. For tests and the harness.
#[derive(Default)]
pub struct RecordingTimers {
    pub scheduled: std::cell::RefCell<Vec<(Duration, TimerAction)>>,
}

impl Timers for RecordingTimers {
    fn schedule(&self, delay: Duration, action: TimerAction) {
        self.scheduled.borrow_mut().push((delay, action));
    }
}
