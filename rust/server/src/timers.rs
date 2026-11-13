//! Work the server does later: bot turns, async-play mail, the countdown of a pause, saving a
//! game. The server hands a [`Timer`] to [`Timers`]; when it comes due, the game thread runs it
//! like any other message.

use std::cell::RefCell;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Timer {
    /// The bots of a game act. Only the turn scheduled last for the game runs.
    BotTurn { game: String, turn: u64 },
    /// Mail the players of an async-play game what happened since the last mail.
    AsyncMail { game: String },
    /// The countdown of a pause between phases runs out.
    PauseCountdown { game: String, pause: u32 },
    /// Save a game that changed.
    Save { game: String },
}

pub trait Timers {
    fn schedule(&self, delay: Duration, timer: Timer);
}

/// Keeps what was scheduled, for tests to run.
#[derive(Default)]
pub struct RecordingTimers {
    pub scheduled: RefCell<Vec<(Duration, Timer)>>,
}

impl Timers for RecordingTimers {
    fn schedule(&self, delay: Duration, timer: Timer) {
        self.scheduled.borrow_mut().push((delay, timer));
    }
}
