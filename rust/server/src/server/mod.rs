//! The server's state and its handling of calls.
//!
//! One [`Server`] owns every account, session, running game and connection, on one thread
//! (`http.rs`). It handles one message at a time, to completion: a call, a closed connection,
//! a timer. So handlers take `&mut self` and need no locks, and what players see happens in
//! one order. The engine's `Game` is not `Send`, which is the other reason for the one thread.
//!
//! Changes are written to the database as they happen, except running games: a changed game
//! is saved a few seconds later ([`SAVE_DELAY`]), so a burst of events costs one write, and on
//! shutdown.

mod accounts;
mod admin;
mod bots;
mod games;
mod lobby;
mod pauses;
mod play;
mod scene;
mod storage;

use std::collections::BTreeMap;
use std::panic::AssertUnwindSafe;
use std::rc::Rc;
use std::time::Duration;

use futures::FutureExt;
use serde_json::Value;
use tokio::sync::mpsc;
use treachery_bots::ClassicBot;
use treachery_engine::enumerations::{ErrorType, Faction, UserStatus};
use treachery_engine::game::Game;

use super::connections::{ConnectionId, Connections};
use super::db::{Db, ScheduledGameRow, UserRow};
use super::mail::Mailer;
use super::protocol::{self, Message, Request};
use super::settings::Settings;
use super::time::{Clock, Time};
use super::timers::{Timer, Timers};
use super::views::GameViews;

pub use pauses::Pause;

/// How long a session lasts.
pub const SESSION_DAYS: i64 = 90;
/// Games without activity for this long leave the lobby's default list.
pub const ACTIVE_GAME_MINUTES: i64 = 120;
/// How many running games a user may have created.
pub const MAX_GAMES_PER_USER: usize = 10;
/// How many scheduled games a user may have announced.
pub const MAX_SCHEDULED_GAMES_PER_USER: usize = 3;
/// A changed game is saved this long after its first unsaved change.
pub const SAVE_DELAY: Duration = Duration::from_secs(5);

/// What the server works with, injected so tests can replace the clock, timers and mail.
pub struct Services {
    pub db: Db,
    pub settings: Settings,
    pub clock: Rc<dyn Clock>,
    pub timers: Rc<dyn Timers>,
    pub mailer: Rc<dyn Mailer>,
}

/// Why a call did not succeed.
#[derive(Debug)]
pub enum CallError {
    /// Refused, with the error a client shows (protocol.md, Result).
    Refused(ErrorType, String),
    /// The call itself failed (a database error, a bug): an `error` reply.
    Failed(String),
}

impl std::fmt::Display for CallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallError::Refused(error, details) => write!(f, "refused ({error:?}) {details}"),
            CallError::Failed(e) => f.write_str(e),
        }
    }
}

impl From<sqlx::Error> for CallError {
    fn from(e: sqlx::Error) -> Self {
        CallError::Failed(format!("database error: {e}"))
    }
}

pub fn refuse(error: ErrorType) -> CallError {
    CallError::Refused(error, String::new())
}

/// A call's result: the `Contents` of a success.
pub type Reply = Result<Value, CallError>;

/// The empty contents of a call that returns nothing.
pub fn done() -> Reply {
    Ok(Value::Object(Default::default()))
}

pub fn contents<T: serde::Serialize>(value: &T) -> Reply {
    Ok(serde_json::to_value(value).expect("payloads serialize"))
}

/// A logged-in session.
#[derive(Clone, Debug)]
pub struct Session {
    pub user: i32,
    pub created: Time,
}

/// What the lobby shows about a user who is around.
#[derive(Clone, Debug)]
pub struct Presence {
    pub status: UserStatus,
    pub last_seen: Option<Time>,
}

/// A game in progress, with what the server keeps beside it.
pub struct RunningGame {
    pub id: String,
    pub name: String,
    pub creator: i32,
    pub password_hash: Option<String>,
    pub created: Time,
    pub last_activity: Time,
    pub game: Game,
    /// Whether the statistics mail for the game's end went out.
    pub statistics_sent: bool,
    pub last_async_mail: Option<Time>,
    async_mail_pending: bool,
    /// One bot per faction, created when that faction's bot first acts and kept across undo.
    bots: Vec<(Faction, ClassicBot)>,
    /// Counts the bot turns scheduled; only the latest runs.
    bot_turn: u64,
    pause: Pause,
    /// How many pauses the game had, to number them.
    pause_count: u32,
    /// The skin the host set, for the views' labels.
    pub skin: Option<String>,
    /// The bidding timer the host set.
    pub timer: i32,
    /// Changes when `game` is replaced by another game object (a load or an admin reload).
    pub generation: u64,
    unsaved: bool,
    save_pending: bool,
    /// A development scene (`dev_scene.rs`): its bots never act.
    pub scene: bool,
}

impl RunningGame {
    fn new(id: String, name: String, creator: i32, game: Game, now: Time) -> Self {
        RunningGame {
            id,
            name,
            creator,
            password_hash: None,
            created: now,
            last_activity: now,
            game,
            statistics_sent: false,
            last_async_mail: None,
            async_mail_pending: false,
            bots: Vec::new(),
            bot_turn: 0,
            pause: Pause::default(),
            pause_count: 0,
            skin: None,
            timer: -1,
            generation: 0,
            unsaved: false,
            save_pending: false,
            scene: false,
        }
    }
}

pub struct Server {
    services: Services,
    connections: Connections,
    views: GameViews,
    users: BTreeMap<i32, UserRow>,
    sessions: BTreeMap<String, Session>,
    presence: BTreeMap<i32, Presence>,
    games: BTreeMap<String, RunningGame>,
    scheduled: BTreeMap<String, ScheduledGameRow>,
    /// When the admin announced maintenance.
    maintenance: Option<Time>,
    next_generation: u64,
}

/// The text of a panic: an engine exception or a bug.
pub fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(e) = payload.downcast_ref::<treachery_engine::compat::cs::CsException>() {
        e.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else {
        "panic".to_string()
    }
}

impl Server {
    /// Starts on the database: loads accounts, sessions, games and scheduled games.
    pub async fn start(services: Services) -> Result<Server, String> {
        let mut server = Server {
            services,
            connections: Connections::default(),
            views: GameViews::default(),
            users: BTreeMap::new(),
            sessions: BTreeMap::new(),
            presence: BTreeMap::new(),
            games: BTreeMap::new(),
            scheduled: BTreeMap::new(),
            maintenance: None,
            next_generation: 0,
        };
        server.load().await.map_err(|e| format!("loading the database: {e}"))?;
        Ok(server)
    }

    pub fn now(&self) -> Time {
        self.services.clock.now()
    }

    pub fn settings(&self) -> &Settings {
        &self.services.settings
    }

    /// Saves every unsaved game. Call before stopping.
    pub async fn shutdown(&mut self) {
        let ids: Vec<String> = self.games.iter().filter(|(_, g)| g.unsaved).map(|(id, _)| id.clone()).collect();
        for id in ids {
            if let Err(e) = self.save_game(&id).await {
                println!("game {id}: not saved: {e}");
            }
        }
        self.services.db.close().await;
    }

    pub fn connect(&mut self, connection: ConnectionId, outgoing: mpsc::UnboundedSender<String>) {
        self.connections.connect(connection, outgoing);
    }

    pub fn disconnect(&mut self, connection: ConnectionId) {
        self.connections.disconnect(connection);
        self.views.retain(&self.connections.followed_games());
    }

    /// Handles one frame from a connection. The call's notifications go out as they happen,
    /// then the views it caused, then its reply. A malformed frame closes the connection.
    pub async fn receive(&mut self, connection: ConnectionId, text: &str) {
        let reply = match protocol::decode(text) {
            Ok(Message::Ping) => return,
            Ok(Message::BadCall { id, error }) => protocol::reply_error(id, &error),
            Ok(Message::Call { id, request }) => {
                let outcome = AssertUnwindSafe(self.handle(connection, request)).catch_unwind().await;
                self.send_views();
                match outcome {
                    Ok(Ok(contents)) => protocol::reply(id, &protocol::success(contents)),
                    Ok(Err(CallError::Refused(error, details))) => protocol::reply(id, &protocol::refusal(error, &details)),
                    Ok(Err(CallError::Failed(e))) => {
                        println!("call {id} failed: {e}");
                        protocol::reply_error(id, &e)
                    }
                    Err(panic) => {
                        let e = panic_text(panic.as_ref());
                        println!("call {id} failed: {e}");
                        protocol::reply_error(id, &e)
                    }
                }
            }
            Err(_) => {
                self.disconnect(connection);
                return;
            }
        };
        self.connections.send(connection, reply);
    }

    /// Runs a timer that came due.
    pub async fn on_timer(&mut self, timer: Timer) {
        let outcome = AssertUnwindSafe(async {
            match timer {
                Timer::BotTurn { game, turn } => {
                    if self.games.get(&game).is_some_and(|g| g.bot_turn == turn) {
                        self.bot_turn(&game).await
                    } else {
                        Ok(())
                    }
                }
                Timer::AsyncMail { game } => {
                    if let Some(g) = self.games.get_mut(&game) {
                        g.async_mail_pending = false;
                    }
                    self.send_async_mail(&game).await
                }
                Timer::PauseCountdown { game, pause } => self.countdown_ended(&game, pause).await,
                Timer::Save { game } => {
                    if let Some(g) = self.games.get_mut(&game) {
                        g.save_pending = false;
                    }
                    self.save_game(&game).await
                }
            }
        })
        .catch_unwind()
        .await;
        match outcome {
            Ok(Ok(())) => {}
            Ok(Err(CallError::Refused(_, e) | CallError::Failed(e))) => println!("timer: {e}"),
            Err(panic) => println!("timer: {}", panic_text(panic.as_ref())),
        }
        self.send_views();
    }

    fn send_views(&mut self) {
        let pending = self.connections.take_pending();
        self.views.send(pending, &self.games, &self.connections);
    }

    async fn handle(&mut self, connection: ConnectionId, request: Request) -> Reply {
        use Request::*;
        match request {
            Connect => self.server_info(),
            CreateUser { name, password_hash, email, player_name } => self.create_user(&name, &password_hash, email.as_deref(), &player_name).await,
            Login { version, name, password_hash } => self.login(version, &name, &password_hash).await,
            LoginInfo { token } => self.login_info(&token),
            PasswordReset { name_or_email } => self.password_reset(&name_or_email).await,
            SetPassword { name, reset_token, password_hash } => self.set_password(&name, &reset_token, &password_hash).await,
            UpdateUserInfo { token, password_hash, player_name, email } => {
                self.update_user_info(&token, password_hash.as_deref(), &player_name, email.as_deref()).await
            }
            SetUserStatus { token, status } => self.set_user_status(&token, status),
            Heartbeat { token, scope } => self.heartbeat(&token, scope),

            CreateGame { name, token, password_hash, savegame, skin } => {
                self.create_game(connection, name.as_deref(), &token, password_hash.as_deref(), savegame.as_deref(), skin.as_deref()).await
            }
            LoadGame { token, game, savegame, skin } => self.load_game(&token, &game, &savegame, skin.as_deref()).await,
            UpdateSettings { token, game, settings } => self.update_settings(&token, &game, settings),
            CloseGame { token, game } => self.close_game(&token, &game).await,
            GameState { token, game } => self.game_state(connection, &token, &game),
            ScheduleGame { token, starts, ruleset, players, max_turns, factions, async_play } => {
                self.schedule_game(&token, starts, ruleset, players, max_turns, factions, async_play).await
            }
            CancelScheduledGame { token, scheduled_game } => self.cancel_scheduled_game(&token, &scheduled_game).await,
            Subscribe { token, scheduled_game, subscription } => self.subscribe(&token, scheduled_game.as_deref(), subscription).await,

            JoinGame { token, game, password_hash, seat } => self.join_game(connection, &token, &game, password_hash.as_deref(), seat),
            ObserveGame { token, game, password_hash } => self.observe_game(connection, &token, &game, password_hash.as_deref()),
            ReconnectGame { token, game } => self.reconnect_game(connection, &token, &game),
            LeaveGame { token, game } => self.leave_game(&token, &game),
            SetOrUnsetHost { token, game, user } => self.set_or_unset_host(&token, &game, user),
            OpenOrCloseSeat { token, game, seat } => self.open_or_close_seat(&token, &game, seat),
            Kick { token, game, user } => self.kick(&token, &game, user),
            AssignSeats { token, game, seats } => self.assign_seats(&token, &game, seats),
            SetBotSpeed { token, game, speed } => self.set_bot_speed(&token, &game, speed),
            NudgeBots { token, game } => self.nudge_bots(&token, &game).await,

            SubmitEvent { token, game, event } => self.submit_event(&token, &game, event).await,
            Undo { token, game, until } => self.undo(&token, &game, until).await,
            RestoreUndone { token, game } => self.restore_undone(&token, &game).await,
            DismissUndone { token, game } => self.dismiss_undone(&token, &game).await,
            SetTimer { token, game, value } => self.set_timer(&token, &game, value),
            SetSkin { token, game, skin } => self.set_skin(&token, &game, skin),
            PhaseDone { token, game } => self.phase_done(&token, &game).await,
            PhaseHold { token, game } => self.phase_hold(&token, &game).await,
            PhaseRelease { token, game } => self.phase_release(&token, &game).await,
            PhaseContinue { token, game, event_number } => self.phase_continue(&token, &game, event_number).await,

            Chat { token, game, message } => self.chat(&token, &game, message),
            GlobalChat { token, message } => self.global_chat(&token, message),

            SetMaintenance { token, starts } => self.set_maintenance(&token, starts),
            SaveAll { token } => self.save_all(&token).await,
            ReloadAll { token } => self.reload_all(&token).await,
            AdminCloseGame { token, game } => self.admin_close_game(&token, &game).await,
            DownloadGame { token, game } => self.download_game(&token, &game),
            AdminCancelScheduledGame { token, scheduled_game } => self.admin_cancel_scheduled_game(&token, &scheduled_game).await,
            DeleteUser { token, user } => self.delete_user(&token, user).await,
            AdminInfo { token } => self.admin_info(&token),

            QueryDraft { token, game, event_number: _, draft } => {
                let (user, game) = self.user_in_game(&token, &game)?;
                self.views.query_draft(&self.games[&game], user, &draft)
            }
            Report { token, game, index } => {
                let (user, game) = self.user_in_game(&token, &game)?;
                self.views.report(&self.games[&game], user, index)
            }
            UndoList { token, game } => {
                let (user, game) = self.user_in_game(&token, &game)?;
                self.views.undo_list(&self.games[&game], user)
            }
            UndoPreview { token, game, gap } => {
                let (user, game) = self.user_in_game(&token, &game)?;
                self.views.undo_preview(&self.games[&game], user, gap)
            }
            SaveGame { token, game } => {
                let (user, game) = self.user_in_game(&token, &game)?;
                let g = &self.games[&game];
                if !g.game.is_host(Some(user)) && !self.is_admin(&token) {
                    return Err(refuse(ErrorType::NoHost));
                }
                Ok(Value::String(storage::savegame(&g.game)))
            }
        }
    }

    /// The user of a session, if the token is one that hasn't expired.
    fn session_user(&self, token: &str) -> Result<i32, CallError> {
        match self.sessions.get(token) {
            Some(s) if self.now() - s.created < chrono::Duration::days(SESSION_DAYS) => Ok(s.user),
            _ => Err(refuse(ErrorType::UserNotFound)),
        }
    }

    /// The session's user and a running game's id.
    fn user_in_game(&self, token: &str, game: &str) -> Result<(i32, String), CallError> {
        let user = self.session_user(token)?;
        if !self.games.contains_key(game) {
            return Err(refuse(ErrorType::GameNotFound));
        }
        Ok((user, game.to_string()))
    }

    /// Like [`Self::user_in_game`], for calls only a host of the game may make.
    fn host_of(&self, token: &str, game: &str) -> Result<(i32, String), CallError> {
        let (user, game) = self.user_in_game(token, game)?;
        if !self.games[&game].game.is_host(Some(user)) {
            return Err(refuse(ErrorType::NoHost));
        }
        Ok((user, game))
    }

    fn player_name(&self, user: i32) -> String {
        self.users.get(&user).map(|u| u.player_name.clone()).unwrap_or_default()
    }

    fn is_admin(&self, token: &str) -> bool {
        let Ok(user) = self.session_user(token) else { return false };
        match (&self.services.settings.admin, self.users.get(&user)) {
            (Some(admin), Some(u)) => *admin == u.name,
            _ => false,
        }
    }

    /// The game changed and must be saved.
    fn changed(&mut self, game: &str) {
        let Some(g) = self.games.get_mut(game) else { return };
        g.unsaved = true;
        if !g.save_pending {
            g.save_pending = true;
            self.services.timers.schedule(SAVE_DELAY, Timer::Save { game: game.to_string() });
        }
    }

    fn new_generation(&mut self) -> u64 {
        self.next_generation += 1;
        self.next_generation
    }

    /// For the test harness and the dev scene: the game with this id.
    pub fn game(&self, id: &str) -> Option<&RunningGame> {
        self.games.get(id)
    }
}
