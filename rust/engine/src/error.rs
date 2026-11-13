//! The engine's error type.

use std::fmt;

/// An event that passed validation but couldn't be carried out.
///
/// Validation doesn't rule out every event the rules can't handle, so some accepted events fail
/// partway through executing. What they changed before failing stays changed, and they are not
/// added to the history. Players see the failure as a "Game Error" (see `GameEventMethods::execute`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameError {
    message: String,
}

impl GameError {
    pub fn new(message: impl Into<String>) -> GameError {
        GameError { message: message.into() }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for GameError {}

/// The result of executing (part of) an event.
pub type GameResult<T = ()> = Result<T, GameError>;

/// `return Err(GameError::new(format!(...)))`.
#[macro_export]
macro_rules! game_error {
    ($($arg:tt)+) => {
        return ::std::result::Result::Err($crate::error::GameError::new(format!($($arg)+)))
    };
}

/// A value that must be present for the event to go on: `None` fails the event with `what`.
pub trait Required<T> {
    fn required(self, what: &str) -> GameResult<T>;
}

impl<T> Required<T> for Option<T> {
    fn required(self, what: &str) -> GameResult<T> {
        self.ok_or_else(|| GameError::new(format!("{what} is missing")))
    }
}

/// The text of a panic payload.
pub fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else {
        "panic".to_string()
    }
}

thread_local! {
    static QUIET: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// For tests of code that must panic: runs `f` and returns the panic message, without printing
/// the panic.
pub fn catch_panic<R>(f: impl FnOnce() -> R) -> Result<R, String> {
    static HOOK: std::sync::Once = std::sync::Once::new();
    HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if QUIET.with(|q| q.get()) == 0 {
                previous(info)
            }
        }));
    });
    QUIET.with(|q| q.set(q.get() + 1));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    QUIET.with(|q| q.set(q.get() - 1));
    result.map_err(|payload| panic_message(payload.as_ref()))
}
