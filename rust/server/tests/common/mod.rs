//! A server driven through the WebSocket protocol (rust/docs/protocol.md), without sockets.
//!
//! Tests send the JSON a client sends and read the JSON a client receives. The clock, the
//! timers and the mail are fakes: [`TestServer::advance`] moves the clock and runs whatever
//! came due, and [`TestServer::mails`] returns what the server sent. [`TestServer::restart`]
//! shuts the server down and starts a new one on the same database file.
//!
//! Only this file knows the server's internals; the tests use nothing but the protocol.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::sync::mpsc;
use treachery_server::db::Db;
use treachery_server::mail::RecordingMailer;
use treachery_server::server::{Server, Services};
use treachery_server::settings::Settings;
use treachery_server::time::{self, Clock, ManualClock, Time};
use treachery_server::timers::{RecordingTimers, Timer};

/// The engine version a client logs in with.
pub const VERSION: i32 = treachery_engine::game::Game::LATEST_VERSION;

/// A mail the server sent.
#[derive(Clone, Debug)]
pub struct Mail {
    pub from: String,
    pub to: Vec<String>,
    pub subject: String,
    pub body: String,
    pub attachments: Vec<String>,
}

/// A connection.
pub type Conn = u64;

pub struct TestServer {
    server: Server,
    queues: BTreeMap<Conn, mpsc::UnboundedReceiver<String>>,
    /// Messages read from a queue that a test hasn't taken yet.
    inbox: BTreeMap<Conn, Vec<Value>>,
    clock: Rc<ManualClock>,
    timers: Rc<RecordingTimers>,
    mailer: Rc<RecordingMailer>,
    /// Timers not run yet, with the time they come due, in the order they were set.
    pending: Vec<(Time, Timer)>,
    next_connection: Conn,
    next_call: u64,
    db_path: PathBuf,
    settings: Vec<(String, String)>,
    _dir: Option<Rc<tempfile::TempDir>>,
}

/// The settings every test server has: an admin, and the statistics mail on.
pub fn default_settings() -> Vec<(String, String)> {
    [("TREACHERY_ADMIN", "admin"), ("TREACHERY_STATS_FROM", "games@example.com"), ("TREACHERY_STATS_TO", "stats@example.com")]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn start_time() -> Time {
    time::parse("2026-11-08T12:00:00Z").unwrap()
}

impl TestServer {
    /// A server on a new, empty database.
    pub async fn start() -> TestServer {
        Self::start_with(&[]).await
    }

    /// A server on a new database, with extra settings.
    pub async fn start_with(settings: &[(&str, &str)]) -> TestServer {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("treachery.db");
        let mut all = default_settings();
        all.extend(settings.iter().map(|(k, v)| (k.to_string(), v.to_string())));
        Self::open(path, all, Some(Rc::new(dir)), start_time()).await
    }

    /// A server on an existing database file.
    pub async fn start_on(path: &Path) -> TestServer {
        Self::open(path.to_path_buf(), default_settings(), None, start_time()).await
    }

    async fn open(db_path: PathBuf, settings: Vec<(String, String)>, dir: Option<Rc<tempfile::TempDir>>, now: Time) -> TestServer {
        let clock = Rc::new(ManualClock::new(now));
        let timers = Rc::new(RecordingTimers::default());
        let mailer = Rc::new(RecordingMailer::default());
        let pairs: Vec<(&str, &str)> = settings.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let services = Services {
            db: Db::open(&db_path).await.unwrap_or_else(|e| panic!("{e}")),
            settings: Settings::from_pairs(&pairs).unwrap(),
            clock: clock.clone(),
            timers: timers.clone(),
            mailer: mailer.clone(),
        };
        TestServer {
            server: Server::start(services).await.unwrap(),
            queues: BTreeMap::new(),
            inbox: BTreeMap::new(),
            clock,
            timers,
            mailer,
            pending: Vec::new(),
            next_connection: 0,
            next_call: 0,
            db_path,
            settings,
            _dir: dir,
        }
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn server(&mut self) -> &mut Server {
        &mut self.server
    }

    /// Stops this server as the service manager would, then starts a new one on the same
    /// database. Connections, timers and everything else in memory are gone.
    pub async fn restart(self) -> TestServer {
        let TestServer { mut server, db_path, settings, _dir, clock, .. } = self;
        server.shutdown().await;
        let now = clock.now();
        Self::open(db_path, settings, _dir, now).await
    }

    /// Opens a connection, as a new WebSocket.
    pub fn connect(&mut self) -> Conn {
        self.next_connection += 1;
        let connection = self.next_connection;
        let (tx, rx) = mpsc::unbounded_channel();
        self.server.connect(connection, tx);
        self.queues.insert(connection, rx);
        connection
    }

    /// Closes a connection.
    pub fn disconnect(&mut self, connection: Conn) {
        self.server.disconnect(connection);
        self.queues.remove(&connection);
        self.inbox.remove(&connection);
    }

    /// Whether the server still has the connection open.
    pub fn is_open(&mut self, connection: Conn) -> bool {
        match self.queues.get_mut(&connection) {
            Some(rx) => !rx.is_closed() || !rx.is_empty(),
            None => false,
        }
    }

    /// Sends a raw text frame.
    pub async fn send_text(&mut self, connection: Conn, text: &str) {
        self.server.receive(connection, text).await;
        self.collect_timers();
    }

    /// Everything the connection received since the last time it was read.
    pub fn received(&mut self, connection: Conn) -> Vec<Value> {
        let mut out = self.inbox.remove(&connection).unwrap_or_default();
        if let Some(rx) = self.queues.get_mut(&connection) {
            while let Ok(text) = rx.try_recv() {
                out.push(serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}: {text}")));
            }
        }
        out
    }

    /// The notifications the connection received since it was last read (replies are dropped).
    pub fn notifications(&mut self, connection: Conn) -> Vec<Value> {
        self.received(connection).into_iter().filter(|m| m["type"] == "notify").collect()
    }

    /// Sends a call and returns the reply message, and every message before it that the test
    /// had not read yet.
    pub async fn call_raw(&mut self, connection: Conn, method: &str, args: Value) -> (Value, Vec<Value>) {
        let earlier = self.received(connection);
        self.next_call += 1;
        let id = self.next_call;
        let text = json!({"type": "call", "id": id, "method": method, "args": args}).to_string();
        self.send_text(connection, &text).await;
        let mut received = self.received(connection);
        let reply = received.pop().unwrap_or_else(|| panic!("{method}: no reply"));
        assert_eq!(reply["type"], "reply", "{method}: the reply comes last: {received:?} {reply}");
        assert_eq!(reply["id"], id);
        let mut before = earlier;
        before.extend(received);
        (reply, before)
    }

    /// Sends a call and returns its `Result` object. Panics if the call failed as a call.
    pub async fn call(&mut self, connection: Conn, method: &str, args: Value) -> Value {
        let (reply, before) = self.call_raw(connection, method, args).await;
        assert!(reply.get("error").is_none(), "{method}: {reply}");
        self.requeue(connection, before);
        reply["result"].clone()
    }

    /// A call that must succeed: its `Contents`.
    pub async fn ok(&mut self, connection: Conn, method: &str, args: Value) -> Value {
        let result = self.call(connection, method, args).await;
        assert_eq!(result["Success"], true, "{method}: {result}");
        result["Contents"].clone()
    }

    /// A call that must fail with an application error: its `Error` code.
    pub async fn err(&mut self, connection: Conn, method: &str, args: Value) -> i64 {
        let result = self.call(connection, method, args).await;
        assert_eq!(result["Success"], false, "{method}: {result}");
        result["Error"].as_i64().unwrap()
    }

    /// Puts messages back so the next `received` returns them first.
    fn requeue(&mut self, connection: Conn, messages: Vec<Value>) {
        let rest = self.received(connection);
        self.inbox.insert(connection, messages.into_iter().chain(rest).collect());
    }

    fn collect_timers(&mut self) {
        let now = self.clock.now();
        for (delay, timer) in self.timers.scheduled.borrow_mut().drain(..) {
            self.pending.push((now + chrono::Duration::from_std(delay).unwrap(), timer));
        }
    }

    /// Moves the clock forward, running every timer that comes due on the way, each at its
    /// time.
    pub async fn advance(&mut self, by: Duration) {
        let target = self.clock.now() + chrono::Duration::from_std(by).unwrap();
        loop {
            self.collect_timers();
            let next = self.pending.iter().enumerate().filter(|(_, (due, _))| *due <= target).min_by_key(|(i, (due, _))| (*due, *i)).map(|(i, _)| i);
            let Some(i) = next else { break };
            let (due, timer) = self.pending.remove(i);
            self.clock.set(due.max(self.clock.now()));
            self.server.on_timer(timer).await;
        }
        self.clock.set(target);
    }

    pub async fn advance_minutes(&mut self, minutes: u64) {
        self.advance(Duration::from_secs(60 * minutes)).await;
    }

    /// Whether a timer is waiting to run.
    pub fn has_pending_timers(&mut self) -> bool {
        self.collect_timers();
        !self.pending.is_empty()
    }

    /// The mails sent since the last call.
    pub fn mails(&mut self) -> Vec<Mail> {
        self.mailer
            .sent
            .borrow_mut()
            .drain(..)
            .map(|m| Mail { from: m.from, to: vec![m.to], subject: m.subject, body: m.body, attachments: m.attachments.into_iter().map(|(name, _)| name).collect() })
            .collect()
    }
}

/// A logged-in user: their connection, token and id.
#[derive(Clone, Debug)]
pub struct User {
    pub conn: Conn,
    pub token: String,
    pub id: i64,
    pub name: String,
}

impl TestServer {
    /// Opens a connection, calls `Connect`, and creates the account `name` (password
    /// `<name>-pw`, player name `<Name> Player`, mail `<name>@example.com`).
    pub async fn user(&mut self, name: &str) -> User {
        let conn = self.connect();
        self.ok(conn, "Connect", json!([])).await;
        let player_name = format!("{} Player", capitalize(name));
        let info = self.ok(conn, "RequestCreateUser", json!([name, format!("{name}-pw"), format!("{name}@example.com"), player_name])).await;
        self.mails();
        User { conn, token: info["Token"].as_str().unwrap().to_string(), id: info["UserId"].as_i64().unwrap(), name: name.to_string() }
    }

    /// Opens a connection and logs in as an existing user.
    pub async fn login(&mut self, name: &str) -> User {
        let conn = self.connect();
        self.ok(conn, "Connect", json!([])).await;
        let info = self.ok(conn, "RequestLogin", json!([VERSION, name, format!("{name}-pw")])).await;
        User { conn, token: info["Token"].as_str().unwrap().to_string(), id: info["UserId"].as_i64().unwrap(), name: name.to_string() }
    }

    /// `user` creates a new game (no savegame) and returns its id.
    pub async fn new_game(&mut self, user: &User, name: &str, password: &str) -> String {
        let info = self.ok(user.conn, "RequestCreateGame", json!([name, user.token, password, "", ""])).await;
        info["GameId"].as_str().unwrap().to_string()
    }

    /// `user` creates a game from a fixture in scripts/fixtures.
    pub async fn fixture_game(&mut self, user: &User, fixture: &str) -> String {
        let info = self.ok(user.conn, "RequestCreateGame", json!([fixture, user.token, "", fixture_text(fixture), ""])).await;
        info["GameId"].as_str().unwrap().to_string()
    }

    /// The lobby as `user` sees it, after the lobby's refresh interval has passed.
    pub async fn lobby(&mut self, user: &User, scope: i64) -> Value {
        self.advance(Duration::from_secs(7)).await;
        self.ok(user.conn, "RequestHeartbeat", json!([user.token, scope])).await
    }

    /// The latest game view the connection received, reading everything it received.
    pub fn latest_view(&mut self, conn: Conn) -> Option<Value> {
        views(&self.received(conn)).last().cloned()
    }

    pub async fn submit(&mut self, user: &User, game: &str, event: Value) -> Value {
        self.call(user.conn, "SubmitEvent", json!([user.token, game, event])).await
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// The game views among received messages, in order.
pub fn views(messages: &[Value]) -> Vec<Value> {
    messages.iter().filter(|m| m["type"] == "notify" && m["name"] == "GameView").map(|m| m["args"][0].clone()).collect()
}

/// The names of the notifications among received messages, in order.
pub fn names(messages: &[Value]) -> Vec<String> {
    messages.iter().filter(|m| m["type"] == "notify").map(|m| m["name"].as_str().unwrap().to_string()).collect()
}

/// The notification `name` among received messages, if any.
pub fn find<'a>(messages: &'a [Value], name: &str) -> Option<&'a Value> {
    messages.iter().find(|m| m["type"] == "notify" && m["name"] == name)
}

pub fn fixture_text(name: &str) -> String {
    std::fs::read_to_string(format!("{}/../../scripts/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

/// Faction numbers (engine `Faction`).
pub mod faction {
    pub const ATREIDES: i64 = 20;
    pub const HARKONNEN: i64 = 10;
    pub const EMPEROR: i64 = 40;
}

/// An `EstablishPlayers` event that starts a game for `players` players with the given rules.
pub fn establish_players(players: i64, rules: &[i64], async_play: bool, interval_minutes: i64) -> Value {
    json!({
        "$type": "EstablishPlayers",
        "Settings": {
            "NumberOfPlayers": players,
            "MaximumTurns": 3,
            "InitialRules": rules,
            "AllowedFactionsInPlay": [20, 10, 40, 30, 50, 60],
            "AutoOpenEmptySeats": true,
            "AsyncPlay": async_play,
            "AsyncPlayMessageIntervalMinutes": interval_minutes
        },
        "Seed": 12345,
        "MaximumNumberOfPlayers": players,
        "MaximumTurns": 3,
        "ApplicableRules": [],
        "_players": "",
        "_factionsInPlay": "",
        "Initiator": 0
    })
}

/// Rule numbers (engine `Rule`).
pub mod rule {
    pub const FILL_WITH_BOTS: i64 = 998;
}

/// The player of `faction` in a view.
pub fn player<'a>(view: &'a Value, faction: &str) -> &'a Value {
    view["players"].as_array().unwrap().iter().find(|p| p["faction"] == faction).unwrap_or_else(|| panic!("no {faction} in the view"))
}

/// The event types of the actions in a view.
pub fn actions(view: &Value) -> Vec<String> {
    view["actions"].as_array().unwrap().iter().map(|a| a["eventType"].as_str().unwrap().to_string()).collect()
}

pub fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap()
}
