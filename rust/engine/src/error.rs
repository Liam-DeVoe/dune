//! The engine's error type.

use std::fmt;

/// An event that passed validation but couldn't be carried out.
///
/// Validation doesn't rule out every event the rules can't handle, so some accepted events fail
/// partway through executing. They are not added to the history, and `Game::play` rolls back
/// what they changed before failing. Players see the failure as a "Game Error"
/// (`SubmitError::Failed`).
#[derive(Debug, Clone, PartialEq)]
pub struct GameError {
    message: String,
    /// The error as a message, for errors that name a game object, so that the player sees the
    /// object's name (`SubmitError::message`). `message` is then its plain text.
    expressed: Option<Box<crate::messages::message::Message>>,
}

impl GameError {
    pub fn new(message: impl Into<String>) -> GameError {
        GameError { message: message.into(), expressed: None }
    }

    /// An error that names game objects, such as a leader or a territory.
    pub fn express(m: crate::messages::message::Message) -> GameError {
        GameError { message: m.plain_text(), expressed: Some(Box::new(m)) }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn expressed(&self) -> Option<&crate::messages::message::Message> {
        self.expressed.as_deref()
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
    /// How many `catch_panic` calls this thread is inside. The panic hook is the process's, so it
    /// asks the panicking thread whether to stay quiet. This isn't game state: no game reads it.
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
