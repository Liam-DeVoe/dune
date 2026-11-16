//! Why a call failed: the `err` of a reply.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The `err` of a reply: a code for the client to act on, and a text to show the user.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CallError {
    pub code: ErrorCode,
    /// For the user. The validation message for `invalidGameEvent`, otherwise what went wrong in
    /// a sentence.
    pub message: String,
}

impl CallError {
    /// An error with the code's usual text.
    pub fn new(code: ErrorCode) -> Self {
        CallError { code, message: code.default_message().to_string() }
    }

    /// An error with a text of its own, or the code's usual text when `message` is empty.
    pub fn with_message(code: ErrorCode, message: impl Into<String>) -> Self {
        let message = message.into();
        if message.is_empty() {
            Self::new(code)
        } else {
            CallError { code, message }
        }
    }
}

/// Why a call failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    /// The call isn't one of [`crate::Call`], or its arguments don't decode.
    BadCall,
    /// The server failed while handling the call (a database error or a bug).
    ServerError,
    /// The connection isn't authenticated, or its session ended. The client logs in again.
    NotLoggedIn,

    // Accounts
    InvalidGameVersion,
    InvalidUserNameOrPassword,
    UserNameTooShort,
    UserNameTooLong,
    PlayerNameTooShort,
    PlayerNameTooLong,
    UserNameExists,
    UnknownUserName,
    UnknownUsernameOrEmailAddress,
    NoEmailAddressForUser,
    ResetRequestTooSoon,
    InvalidResetToken,
    ResetTokenExpired,
    NotAdmin,

    // Games
    GameNotFound,
    ScheduledGameNotFound,
    TooManyGames,
    TooManyScheduledGames,
    IncorrectGamePassword,
    SeatNotAvailableNotOpen,
    SeatNotAvailableKicked,
    AlreadyPlayer,
    /// The caller isn't in the game, or only observes it and the call is for players. Also a
    /// target user who isn't in the game.
    UserNotInGame,
    NoHost,
    NoCreator,
    CannotRemoveLastHost,
    /// An event or draft the game refuses, or an argument that doesn't fit the game (a seat it
    /// doesn't have, a report it doesn't have, a savegame that doesn't load). The message says
    /// why.
    InvalidGameEvent,
}

impl ErrorCode {
    /// What the client shows when the server gives no text of its own (the C# client's
    /// `Skin.Describe(ErrorType)`).
    pub fn default_message(self) -> &'static str {
        use ErrorCode::*;
        match self {
            BadCall => "The server didn't understand the request",
            ServerError => "Something went wrong on the server",
            NotLoggedIn => "You are not logged in",
            InvalidGameVersion => "Invalid game version",
            InvalidUserNameOrPassword => "Invalid user name or password",
            UserNameTooShort => "Username must be more than 3 characters",
            UserNameTooLong => "Username must be 40 characters or less",
            PlayerNameTooShort => "Player name must be more than 3 characters",
            PlayerNameTooLong => "Player name must be 40 characters or less",
            UserNameExists => "This username already exists",
            UnknownUserName => "Unknown user name",
            UnknownUsernameOrEmailAddress => "Unknown user name or email address",
            NoEmailAddressForUser => "This account has no e-mail address, so its password cannot be reset",
            ResetRequestTooSoon => "Please wait at least 5 minutes before requesting another password reset",
            InvalidResetToken => "Invalid password reset token",
            ResetTokenExpired => "Your password reset token has expired",
            NotAdmin => "You are not an administrator",
            GameNotFound | ScheduledGameNotFound => "Game not found",
            TooManyGames => "You have too many active games, delete a game first",
            TooManyScheduledGames => "You have scheduled too many games, delete a game first",
            IncorrectGamePassword => "Incorrect password",
            SeatNotAvailableNotOpen => "Seat is not available",
            SeatNotAvailableKicked => "You were removed from this game",
            AlreadyPlayer => "You are already a player in this game",
            UserNotInGame => "User not found in game",
            NoHost => "You are not a host",
            NoCreator => "You are not the creator of this game",
            CannotRemoveLastHost => "You cannot remove the only remaining host from the game",
            InvalidGameEvent => "Invalid game action",
        }
    }
}
