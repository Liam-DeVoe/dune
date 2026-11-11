//! Port of `Server/GameHub.cs`: the hub's state, constants and helpers. The hub methods are in
//! the submodules, one per C# partial class file.
//!
//! The C# hub is a class instantiated per call, with its state in static fields. Here the
//! state is one [`GameHub`] value owned by the hub thread (`runtime`), and each call gets a
//! [`CallContext`] for what C# reads from `Context`. Calls run one at a time to completion
//! (server-plan.md, "Concurrency"), so methods take `&mut self` and hold no locks.
//!
//! Every hub method returns `HubResult<R>`: `Ok` with the C# return value, or `Err` where the
//! C# method throws (the transport reports it as a failed call). Engine exceptions are panics
//! carrying a `CsException` (engine-conventions.md, "Exceptions"); `runtime` catches those
//! around each call.

pub mod authentication;
pub mod call;
pub mod chat;
pub mod game_events;
pub mod game_management;
pub mod other;
pub mod phase_control;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use treachery_engine::date_time_offset::DateTimeOffset;
use treachery_engine::enumerations::ErrorType;
use treachery_engine::logged_random::SharedRandom;

use crate::clients::{Clients, ConnectionId};
use crate::clock::Clock;
use crate::config::Config;
use crate::db::Db;
use crate::logged_in_user::{LoggedInUser, UserRef};
use crate::mail::{MailMessage, Mailer, SmtpSettings};
use crate::model::game_info::GameInfo;
use crate::model::logged_in_user_info::LoggedInUserInfo;
use crate::model::managed_game::ManagedGame;
use crate::model::result::{CallResult, VoidContents, VoidResult};
use crate::model::scheduled_game::ScheduledGame;
use crate::model::scheduled_game_info::ScheduledGameInfo;
use crate::model::user_connections::UserConnections;
use crate::timers::Timers;

/// `CleanupFrequencyHours` (GameHub.cs:6).
pub const CLEANUP_FREQUENCY_HOURS: i32 = 7;
/// `ServerStatusFrequencyMs` (GameHub.cs:7).
pub const SERVER_STATUS_FREQUENCY_MS: i32 = 6000;
/// `PersistFrequencyMinutes` (GameHub.cs:8).
pub const PERSIST_FREQUENCY_MINUTES: i32 = 30;
/// `MaximumLoginTimeDays` (GameHub.cs:9).
pub const MAXIMUM_LOGIN_TIME_DAYS: i32 = 90;
/// `ActiveGameThresholdMinutes` (GameHub.cs:10).
pub const ACTIVE_GAME_THRESHOLD_MINUTES: i32 = 120;
/// `GamePersistFrequencyMinutes` (GameHub.cs:11).
pub const GAME_PERSIST_FREQUENCY_MINUTES: i32 = 5;
/// `MaximumNumberOfGamesPerPlayer` (GameHub.cs:12).
pub const MAXIMUM_NUMBER_OF_GAMES_PER_PLAYER: i32 = 10;

/// `SmtpClient.Host` in `SendMail` (GameHub.cs:141), the default of the `SmtpHost` setting.
pub const DEFAULT_SMTP_HOST: &str = "smtp.strato.com";

/// A running game. Shared like the C# object: the games map holds it, and a call keeps using
/// it after removing it from the map (`RequestCloseGame`).
pub type ManagedGameRef = Rc<RefCell<ManagedGame>>;

/// What a hub method reads from SignalR's `Context`.
#[derive(Clone, Copy, Debug)]
pub struct CallContext {
    /// `Context.ConnectionId`. `None` for work started by a timer.
    pub connection_id: Option<ConnectionId>,
}

/// Where a C# hub method throws instead of returning.
#[derive(Debug)]
pub enum HubError {
    /// A database exception (`DbUpdateException`, `SqliteException`, ...).
    Db(sqlx::Error),
    /// A .NET exception thrown by server code (not the engine), such as the `FormatException`
    /// of `new MailAddress(...)` or `NullReferenceException` on a null argument.
    Exception { type_name: &'static str, message: String },
}

impl From<sqlx::Error> for HubError {
    fn from(e: sqlx::Error) -> Self {
        HubError::Db(e)
    }
}

impl std::fmt::Display for HubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HubError::Db(e) => write!(f, "database error: {e}"),
            HubError::Exception { type_name, message } => write!(f, "{type_name}: {message}"),
        }
    }
}

pub type HubResult<T> = Result<T, HubError>;

/// `GenerateToken()` (GameHub.cs:48), `Guid.NewGuid().ToString()` and the bots' unseeded
/// `new LoggedRandom()`: the hub's only randomness. The harness replaces them with deterministic
/// sequences (C# hooks `TestHooks.NewToken`, `TestHooks.NewGuid` and `TestHooks.NewRandom`, see
/// rust/docs/csharp-hooks.md).
pub trait IdSource {
    /// 16 characters from `[A-Za-z0-9]`, from a cryptographic RNG.
    fn generate_token(&self) -> String;
    /// A random GUID in the `D` format: `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`, lowercase.
    fn new_guid(&self) -> String;
    /// `new LoggedRandom()` for the bots. `what` names the use as the C# hook does:
    /// `"BotOrder <gameId>"` or `"Bot <gameId> <faction>"`.
    fn new_random(&self, what: &str) -> SharedRandom;
}

/// Deterministic ids for tests and the harness.
#[derive(Default)]
pub struct SequentialIds {
    next: std::cell::Cell<u64>,
    /// The last seed `new_random` gave: seeds are 1, 2, 3, ... in the order asked, as the
    /// ServerOracle's `TestHooks.NewRandom` gives them.
    seeds: std::cell::Cell<i32>,
    /// The `what` of every `new_random` call since the harness last took them.
    pub randoms: std::cell::RefCell<Vec<String>>,
}

impl IdSource for SequentialIds {
    fn generate_token(&self) -> String {
        let n = self.next.get() + 1;
        self.next.set(n);
        format!("TOKEN{n:011}")
    }

    fn new_guid(&self) -> String {
        let n = self.next.get() + 1;
        self.next.set(n);
        format!("00000000-0000-0000-0000-{n:012}")
    }

    fn new_random(&self, what: &str) -> SharedRandom {
        let seed = self.seeds.get() + 1;
        self.seeds.set(seed);
        self.randoms.borrow_mut().push(what.to_string());
        SharedRandom::from_seed(seed)
    }
}

/// The services the hub uses, injected so the harness can replace them.
pub struct Services {
    pub db: Db,
    pub config: Config,
    pub clock: Rc<dyn Clock>,
    pub timers: Rc<dyn Timers>,
    pub mailer: Rc<dyn Mailer>,
    pub ids: Rc<dyn IdSource>,
}

/// `GameHub` with its static state.
pub struct GameHub {
    pub services: Services,
    /// `Clients` and `Groups`.
    pub clients: Clients,

    // Users
    /// `UsersByUserToken` (GameHub.cs:15).
    pub users_by_user_token: BTreeMap<String, LoggedInUser>,
    /// `UsersById` (GameHub.cs:16).
    pub users_by_id: BTreeMap<i32, UserRef>,
    /// `ConnectionInfoByUserId` (GameHub.cs:17).
    pub connection_info_by_user_id: BTreeMap<i32, UserConnections>,

    // Started games
    /// `RunningGamesByGameId` (GameHub.cs:20).
    pub running_games_by_game_id: BTreeMap<String, ManagedGameRef>,

    // Scheduled games
    /// `ScheduledGamesByGameId` (GameHub.cs:23).
    pub scheduled_games_by_game_id: BTreeMap<String, ScheduledGame>,

    // Server status
    /// `RunningGames` (GameHub.cs:26).
    pub running_games: Vec<GameInfo>,
    /// `ScheduledGames` (GameHub.cs:27).
    pub scheduled_games: Vec<ScheduledGameInfo>,
    /// `RecentlySeenUsers` (GameHub.cs:28).
    pub recently_seen_users: Vec<LoggedInUserInfo>,

    // Other
    pub last_restored: DateTimeOffset,
    pub last_persisted_running_games: DateTimeOffset,
    pub last_persisted_scheduled_games: DateTimeOffset,
    pub maintenance_date: DateTimeOffset,
    pub last_cleaned_up_user_tokens: DateTimeOffset,
    pub last_cleaned_up_scheduled_games: DateTimeOffset,
    pub last_updated_server_status: DateTimeOffset,
    /// `Restoring` (GameHub_Other.cs:299).
    pub restoring: bool,

    // Phase control
    /// `PausesByGameId` (GameHub_PhaseControl.cs).
    pub pauses_by_game_id: BTreeMap<String, phase_control::Pause>,
    /// `_lastPauseId` (GameHub_PhaseControl.cs).
    pub last_pause_id: i32,

    // Bots
    /// Rust only: the `ManagedGame` each pending `PerformBotEvent` timer runs on, by the
    /// timer's id ([`crate::timers::TimerAction::PerformBotEvent`]). C#'s delayed call captures the
    /// object, which can be a game that was closed or replaced by a restore since.
    pub bot_timer_games: BTreeMap<u64, ManagedGameRef>,
    pub next_bot_timer: u64,
    /// Rust only, for `POST /dev/scene` (`dev.rs`, never on in production): games whose bots
    /// never act, so a scene stays as its fixture left it, as on the C# scene page, which has no
    /// server and no bots.
    pub scene_games: std::collections::BTreeSet<String>,
}

impl GameHub {
    pub fn new(services: Services) -> Self {
        GameHub {
            services,
            clients: Clients::default(),
            users_by_user_token: BTreeMap::new(),
            users_by_id: BTreeMap::new(),
            connection_info_by_user_id: BTreeMap::new(),
            running_games_by_game_id: BTreeMap::new(),
            scheduled_games_by_game_id: BTreeMap::new(),
            running_games: Vec::new(),
            scheduled_games: Vec::new(),
            recently_seen_users: Vec::new(),
            last_restored: DateTimeOffset::default(),
            last_persisted_running_games: DateTimeOffset::default(),
            last_persisted_scheduled_games: DateTimeOffset::default(),
            maintenance_date: DateTimeOffset::default(),
            last_cleaned_up_user_tokens: DateTimeOffset::default(),
            last_cleaned_up_scheduled_games: DateTimeOffset::default(),
            last_updated_server_status: DateTimeOffset::default(),
            restoring: false,
            pauses_by_game_id: BTreeMap::new(),
            last_pause_id: 0,
            bot_timer_games: BTreeMap::new(),
            next_bot_timer: 0,
            scene_games: std::collections::BTreeSet::new(),
        }
    }

    /// `DateTimeOffset.Now`.
    pub fn now(&self) -> DateTimeOffset {
        self.services.clock.now()
    }

    /// `GenerateToken()` (GameHub.cs:48).
    pub fn generate_token(&self) -> String {
        self.services.ids.generate_token()
    }

    /// `Error(error, errorDetails)` (GameHub.cs:58) and `Error<TResult>` (GameHub.cs:61).
    pub fn error<T>(error: ErrorType, error_details: &str) -> CallResult<T> {
        CallResult { success: false, error, error_details: error_details.to_string(), contents: None }
    }

    /// `Success()` (GameHub.cs:64).
    pub fn success() -> VoidResult {
        CallResult { success: true, error: ErrorType::None, error_details: String::new(), contents: Some(VoidContents) }
    }

    /// `Success<TResult>(contents)` (GameHub.cs:66).
    pub fn success_with<T>(contents: T) -> CallResult<T> {
        CallResult { success: true, error: ErrorType::None, error_details: String::new(), contents: Some(contents) }
    }

    /// Both `AreValid` overloads (GameHub.cs:68, :98): the token's user and the game, or the error
    /// result. The two overloads differ only in the result type (and in the `error` they leave
    /// on success, which no caller reads).
    ///
    /// The user is returned as the `UserRef` that the `LoggedInUser` holds (C# `user.User`), so
    /// `Id`, `Username` and `PlayerName` read the same object C# reads. Callers that change the
    /// `LoggedInUser` itself (`Status`, `LastSeenDateTime`) look it up again by token.
    pub fn are_valid<T>(&self, user_token: Option<&str>, game_id: Option<&str>) -> Result<(UserRef, ManagedGameRef), CallResult<T>> {
        let Some(user_token) = user_token else {
            return Err(Self::error(ErrorType::UserTokenNotFound, ""));
        };

        let Some(user) = self.users_by_user_token.get(user_token) else {
            return Err(Self::error(ErrorType::UserNotFound, ""));
        };

        let Some(game_id) = game_id else {
            return Err(Self::error(ErrorType::GameIdNotFound, ""));
        };

        match self.running_games_by_game_id.get(game_id) {
            Some(game) => Ok((user.user.clone(), game.clone())),
            None => Err(Self::error(ErrorType::GameNotFound, "")),
        }
    }

    /// `!UsersByUserToken.TryGetValue(userToken, out var user) || user.Username != Configuration["GameAdminUsername"]`,
    /// negated: the check at the top of every admin method (GameHub_Other.cs:86 and others).
    /// With no `GameAdminUsername` configured nobody is admin, since a user name is never null.
    pub fn is_admin_token(&self, user_token: &str) -> bool {
        match self.users_by_user_token.get(user_token) {
            Some(user) => self.services.config.get("GameAdminUsername") == Some(user.username().as_str()),
            None => false,
        }
    }

    /// `SendMail(mail)` (GameHub.cs:128): returns without sending when `GameEndEmailUsername` is
    /// null or empty; otherwise hands the message to the mailer with the SMTP settings. C# awaits
    /// the send and only prints a line if it fails, so handing it off changes nothing observable.
    ///
    /// The host is hard-coded in C# (`smtp.strato.com`, with a TODO to move it to configuration).
    /// Here it is the `SmtpHost` setting with that default (server-plan.md, "Coordinator answers"
    /// 7). `SmtpClient`'s default port is 25; `EnableSsl` means STARTTLS on it.
    pub fn send_mail(&self, mail: MailMessage) {
        let config = &self.services.config;
        let username = match config.get("GameEndEmailUsername") {
            Some(u) if !u.is_empty() => u.to_string(),
            _ => return,
        };

        let password = config.get("GameEndEmailPassword").unwrap_or_default().to_string();

        let settings = SmtpSettings {
            host: config.get("SmtpHost").filter(|h| !h.is_empty()).unwrap_or(DEFAULT_SMTP_HOST).to_string(),
            port: 25,
            username,
            password,
        };

        self.services.mailer.send(&settings, mail);
    }

    /// `Log(message)` (GameHub_Other.cs:26): `Console.WriteLine`.
    pub fn log(message: &str) {
        println!("{message}");
    }
}
