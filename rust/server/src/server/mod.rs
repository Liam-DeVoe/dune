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

use treachery_protocol::UserId;
use std::collections::BTreeMap;
use std::panic::AssertUnwindSafe;
use std::rc::Rc;
use std::time::Duration;

use futures::FutureExt;
use serde_json::Value;
use tokio::sync::mpsc;
use treachery_bots::ClassicBot;
use treachery_engine::enumerations::Faction;
use treachery_engine::game::Game;

use super::connections::{ConnectionId, Connections};
use super::db::{Db, LoginLinkRow, ScheduledGameRow, UserRow};
use super::mail::Mailer;
use super::participation::Participation;
use super::protocol::{self, replies, Call, Decoded, ErrorCode};
use super::settings::Settings;
use super::time::{Clock, Time};
use super::timers::{Timer, Timers};
use super::views::GameViews;

pub use pauses::Pause;

/// A session ends this long after it was created,
pub const SESSION_DAYS: i64 = 90;
/// or this long after a connection last logged in with it, whichever comes first.
pub const SESSION_IDLE_DAYS: i64 = 30;
/// A user keeps at most this many sessions; a new one ends the least recently used.
pub const MAX_SESSIONS_PER_USER: usize = 20;
/// The login link of an async-play mail works once, within this time.
pub const LOGIN_LINK_HOURS: i64 = 24;
/// Games without activity for this long leave the lobby's default list.
pub const ACTIVE_GAME_MINUTES: i64 = 120;
/// The fastest bot speed a host may set (protocol.md, Bots).
pub const MAX_BOT_SPEED: i32 = 4;
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
    /// Draws the seed of each new game (`ids::new_seed` in production).
    pub seeds: Rc<dyn Fn() -> i32>,
}

/// Why a call did not succeed.
#[derive(Debug)]
pub enum CallError {
    /// Refused, with the code and a text for the user (protocol.md, Errors). An empty text
    /// gives the code's usual one.
    Refused(ErrorCode, String),
    /// The call itself failed (a database error, a bug): a `serverError`.
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

pub fn refuse(error: ErrorCode) -> CallError {
    CallError::Refused(error, String::new())
}

/// A call's outcome, with the reply type of the call (`protocol::replies`).
pub type Answer<T> = Result<T, CallError>;

/// A call's reply, encoded.
pub type Reply = Answer<Value>;

/// Encodes an answer. The type parameter names the call's reply type, so every reply is of
/// the type the protocol declares for its call: `reply::<replies::JoinGame>(...)`.
fn reply<T: serde::Serialize>(answer: Answer<T>) -> Reply {
    answer.map(|v| protocol::to_value(&v))
}

/// A session, kept by the digest of its token (`ids::digest`).
#[derive(Clone, Debug)]
pub struct Session {
    pub user: UserId,
    pub created: Time,
    pub last_used: Time,
}

impl Session {
    fn expired(&self, now: Time) -> bool {
        now - self.created >= chrono::Duration::days(SESSION_DAYS) || now - self.last_used >= chrono::Duration::days(SESSION_IDLE_DAYS)
    }
}

/// What the lobby shows about a user who is around.
#[derive(Clone, Debug)]
pub struct Presence {
    pub status: treachery_protocol::UserStatus,
    pub last_seen: Option<Time>,
}

/// A game in progress, with what the server keeps beside it.
pub struct RunningGame {
    pub id: String,
    pub name: String,
    pub creator: UserId,
    pub password_hash: Option<String>,
    pub created: Time,
    pub last_activity: Time,
    pub game: Game,
    /// Who plays which seat, observes or hosts. It tells `game` which seats users play.
    pub participation: Participation,
    /// Whether the statistics mail for the game's end went out.
    pub statistics_sent: bool,
    pub last_async_mail: Option<Time>,
    async_mail_pending: bool,
    /// One bot per faction, created when that faction's bot first acts and kept across undo.
    /// Keyed by the faction, `None` for a player who has none yet.
    bots: Vec<(Option<Faction>, ClassicBot)>,
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
    fn new(id: String, name: String, creator: UserId, game: Game, participation: Participation, now: Time) -> Self {
        RunningGame {
            id,
            name,
            creator,
            password_hash: None,
            created: now,
            last_activity: now,
            game,
            participation,
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
    users: BTreeMap<UserId, UserRow>,
    /// By the digest of the session token.
    sessions: BTreeMap<String, Session>,
    /// The login links of async-play mails, by the digest of their token.
    login_links: BTreeMap<String, LoginLinkRow>,
    presence: BTreeMap<UserId, Presence>,
    games: BTreeMap<String, RunningGame>,
    scheduled: BTreeMap<String, ScheduledGameRow>,
    /// When the admin announced maintenance.
    maintenance: Option<Time>,
    next_generation: u64,
}

/// The text of a panic (a bug in the engine or the server).
pub fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    treachery_engine::error::panic_message(payload)
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
            login_links: BTreeMap::new(),
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
            Ok(Decoded::BadCall { id, error }) => protocol::err(id, ErrorCode::BadCall, &error),
            Ok(Decoded::Call { id, call }) => {
                let name = call.name();
                let outcome = AssertUnwindSafe(self.handle(connection, call)).catch_unwind().await;
                self.send_views();
                match outcome {
                    Ok(Ok(value)) => protocol::ok(id, value),
                    Ok(Err(CallError::Refused(code, message))) => protocol::err(id, code, &message),
                    Ok(Err(CallError::Failed(e))) => {
                        println!("call {id} ({name}) failed: {e}");
                        protocol::err(id, ErrorCode::ServerError, &e)
                    }
                    Err(panic) => {
                        let e = panic_text(panic.as_ref());
                        println!("call {id} ({name}) failed: {e}");
                        protocol::err(id, ErrorCode::ServerError, &e)
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

    async fn handle(&mut self, connection: ConnectionId, call: Call) -> Reply {
        use replies as r;
        use Call::*;
        // The calls that work without a session.
        let call = match call {
            GetServerInfo => return reply::<r::GetServerInfo>(Ok(self.server_info())),
            CreateUser { user_name, password_hash, email, player_name } => {
                return reply::<r::CreateUser>(self.create_user(connection, &user_name, &password_hash, email.as_deref(), &player_name).await)
            }
            Login { version, user_name, password_hash } => return reply::<r::Login>(self.login(connection, version, &user_name, &password_hash).await),
            Authenticate { token } => return reply::<r::Authenticate>(self.authenticate(connection, &token).await),
            LinkLogin { token } => return reply::<r::LinkLogin>(self.link_login(connection, &token).await),
            RequestPasswordReset { user_name_or_email } => return reply::<r::RequestPasswordReset>(self.password_reset(&user_name_or_email).await),
            SetPassword { user_name, reset_token, password_hash } => {
                return reply::<r::SetPassword>(self.set_password(connection, &user_name, &reset_token, &password_hash).await)
            }
            call => call,
        };
        let user = self.caller(connection).await?;
        match call {
            GetServerInfo | CreateUser { .. } | Login { .. } | Authenticate { .. } | LinkLogin { .. } | RequestPasswordReset { .. } | SetPassword { .. } => {
                unreachable!("handled above")
            }
            Logout => reply::<r::Logout>(self.logout(connection).await),
            UpdateUserInfo { password_hash, player_name, email } => {
                reply::<r::UpdateUserInfo>(self.update_user_info(connection, user, password_hash.as_deref(), &player_name, email.as_deref()).await)
            }
            SetUserStatus { status } => reply::<r::SetUserStatus>(Ok(self.set_user_status(user, status))),
            Heartbeat { scope } => reply::<r::Heartbeat>(Ok(self.heartbeat(user, scope))),

            CreateGame { name, password_hash, savegame, skin } => reply::<r::CreateGame>(
                self.create_game(connection, user, name.as_deref(), password_hash.as_deref(), savegame.as_deref(), skin.as_deref()).await,
            ),
            LoadGame { game, savegame, skin } => reply::<r::LoadGame>(self.load_game(user, &game, &savegame, skin.as_deref()).await),
            UpdateSettings { game, settings } => reply::<r::UpdateSettings>(self.update_settings(user, &game, &settings)),
            CloseGame { game } => reply::<r::CloseGame>(self.close_game(user, &game).await),
            GameState { game } => reply::<r::GameState>(self.game_state(connection, user, &game)),
            ScheduleGame { starts, ruleset, players, max_turns, factions, async_play } => {
                reply::<r::ScheduleGame>(self.schedule_game(user, starts, ruleset.as_deref(), players, max_turns, &factions, async_play).await)
            }
            CancelScheduledGame { scheduled_game } => reply::<r::CancelScheduledGame>(self.cancel_scheduled_game(user, &scheduled_game).await),
            Subscribe { scheduled_game, subscription } => reply::<r::Subscribe>(self.subscribe(user, &scheduled_game, subscription).await),

            JoinGame { game, password_hash, seat } => {
                reply::<r::JoinGame>(self.join_game(connection, user, &game, password_hash.as_deref(), seat))
            }
            ObserveGame { game, password_hash } => reply::<r::ObserveGame>(self.observe_game(connection, user, &game, password_hash.as_deref())),
            ReconnectGame { game } => reply::<r::ReconnectGame>(self.reconnect_game(connection, user, &game)),
            LeaveGame { game } => reply::<r::LeaveGame>(self.leave_game(user, &game)),
            SetOrUnsetHost { game, user: target } => reply::<r::SetOrUnsetHost>(self.set_or_unset_host(user, &game, target)),
            OpenOrCloseSeat { game, seat } => reply::<r::OpenOrCloseSeat>(self.open_or_close_seat(user, &game, seat)),
            Kick { game, user: target } => reply::<r::Kick>(self.kick(user, &game, target)),
            AssignSeats { game, seats } => reply::<r::AssignSeats>(self.assign_seats(user, &game, seats)),
            SetBotSpeed { game, speed } => reply::<r::SetBotSpeed>(self.set_bot_speed(user, &game, speed)),
            NudgeBots { game } => reply::<r::NudgeBots>(self.nudge_bots(user, &game).await),

            SubmitEvent { game, event } => reply::<r::SubmitEvent>(self.submit_event(user, &game, event).await),
            Undo { game, until } => reply::<r::Undo>(self.undo(user, &game, until).await),
            RestoreUndone { game } => reply::<r::RestoreUndone>(self.restore_undone(user, &game).await),
            DismissUndone { game } => reply::<r::DismissUndone>(self.dismiss_undone(user, &game).await),
            SetTimer { game, value } => reply::<r::SetTimer>(self.set_timer(user, &game, value)),
            SetSkin { game, skin } => reply::<r::SetSkin>(self.set_skin(user, &game, skin)),
            PhaseDone { game } => reply::<r::PhaseDone>(self.phase_done(user, &game).await),
            PhaseHold { game } => reply::<r::PhaseHold>(self.phase_hold(user, &game).await),
            PhaseRelease { game } => reply::<r::PhaseRelease>(self.phase_release(user, &game).await),
            PhaseContinue { game, event_number } => reply::<r::PhaseContinue>(self.phase_continue(user, &game, event_number).await),

            Chat { game, to, body } => reply::<r::Chat>(self.chat(user, &game, to, body)),
            GlobalChat { body } => reply::<r::GlobalChat>(Ok(self.global_chat(user, body))),

            AdminSetMaintenance { starts } => reply::<r::AdminSetMaintenance>(self.set_maintenance(user, starts)),
            AdminSaveAll => reply::<r::AdminSaveAll>(self.save_all(user).await),
            AdminReloadAll => reply::<r::AdminReloadAll>(self.reload_all(user).await),
            AdminCloseGame { game } => reply::<r::AdminCloseGame>(self.admin_close_game(user, &game).await),
            AdminDownloadGame { game } => reply::<r::AdminDownloadGame>(self.download_game(user, &game)),
            AdminCancelScheduledGame { scheduled_game } => {
                reply::<r::AdminCancelScheduledGame>(self.admin_cancel_scheduled_game(user, &scheduled_game).await)
            }
            DeleteUser { user: target } => reply::<r::DeleteUser>(self.delete_user(user, target).await),
            GetAdminInfo => reply::<r::GetAdminInfo>(self.admin_info(user)),

            QueryDraft { game, event_number: _, draft } => {
                let game = self.player_of(user, &game)?;
                reply::<r::QueryDraft>(self.views.query_draft(&self.games[&game], user, &Value::Object(draft).to_string()))
            }
            Report { game, index } => {
                let game = self.participant_of(user, &game)?;
                reply::<r::Report>(self.views.report(&self.games[&game], user, index))
            }
            UndoList { game } => {
                let game = self.host_of(user, &game)?;
                reply::<r::UndoList>(self.views.undo_list(&self.games[&game], user))
            }
            UndoPreview { game, gap } => {
                let game = self.host_of(user, &game)?;
                reply::<r::UndoPreview>(self.views.undo_preview(&self.games[&game], user, gap))
            }
            SaveGame { game } => {
                let game = if self.is_admin(user) { self.running(&game)? } else { self.host_of(user, &game)? };
                reply::<r::SaveGame>(Ok(storage::savegame(&self.games[&game].game)))
            }
        }
    }

    /// The user the connection is logged in as. A session that expired since ends here.
    async fn caller(&mut self, connection: ConnectionId) -> Result<UserId, CallError> {
        let Some((digest, user)) = self.connections.login(connection).map(|(d, u)| (d.to_string(), u)) else {
            return Err(refuse(ErrorCode::NotLoggedIn));
        };
        match self.sessions.get(&digest) {
            Some(s) if !s.expired(self.now()) => Ok(user),
            _ => {
                self.end_session(&digest).await?;
                Err(refuse(ErrorCode::NotLoggedIn))
            }
        }
    }

    /// A running game's id.
    fn running(&self, game: &str) -> Result<String, CallError> {
        if !self.games.contains_key(game) {
            return Err(refuse(ErrorCode::GameNotFound));
        }
        Ok(game.to_string())
    }

    /// A running game the user takes part in, as a player or an observer.
    fn participant_of(&self, user: UserId, game: &str) -> Result<String, CallError> {
        let game = self.running(game)?;
        if !self.games[&game].participation.is_participant(user) {
            return Err(refuse(ErrorCode::UserNotInGame));
        }
        Ok(game)
    }

    /// A running game the user plays in. Observers may watch and nothing more.
    fn player_of(&self, user: UserId, game: &str) -> Result<String, CallError> {
        let game = self.participant_of(user, game)?;
        if !self.games[&game].participation.is_player(user) {
            return Err(CallError::Refused(ErrorCode::UserNotInGame, "Observers can't do this".into()));
        }
        Ok(game)
    }

    /// A running game the user hosts.
    fn host_of(&self, user: UserId, game: &str) -> Result<String, CallError> {
        let game = self.running(game)?;
        if !self.games[&game].participation.is_host(user) {
            return Err(refuse(ErrorCode::NoHost));
        }
        Ok(game)
    }

    fn player_name(&self, user: UserId) -> String {
        self.users.get(&user).map(|u| u.player_name.clone()).unwrap_or_default()
    }

    fn is_admin(&self, user: UserId) -> bool {
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
