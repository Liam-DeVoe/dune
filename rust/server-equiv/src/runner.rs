//! Drives the Rust [`GameHub`] through a scenario, the twin of `ServerOracle/Harness.cs`.
//!
//! The hub gets a [`ManualClock`], [`RecordingTimers`], a [`RecordingMailer`], [`SequentialIds`]
//! and recording connection sinks. After every step the runner writes the same transcript line
//! as the C# harness, from the same sources: the call's result, the callbacks per connection,
//! mails, timers, every table, every running game, and the hub's in-memory state. Sections are
//! always written in full; the C# transcript writes `"="` for an unchanged state section, which
//! the comparison resolves.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::rc::Rc;

use serde_json::Value;
use sqlx::{Column, Row, TypeInfo, ValueRef};
use treachery_engine::date_time_offset::DateTimeOffset;
use treachery_engine::game_state::GameState;
use treachery_engine::json::{serialize, JsonWriter, ToJson};
use treachery_server::clients::{Callback, ConnectionSink};
use treachery_server::clock::ManualClock;
use treachery_server::config::Config;
use treachery_server::db::Db;
use treachery_server::hub::call::Reply;
use treachery_server::hub::{CallContext, GameHub, HubError, SequentialIds, Services};
use treachery_server::mail::{MailMessage, MailPriority, RecordingMailer};
use treachery_server::model::result::CallResult;
use treachery_server::model::server_status::ServerStatus;
use treachery_server::timers::{RecordingTimers, TimerAction};

use crate::json_text::{escape, sha256_hex, JsonText};

/// Configuration every scenario starts with (`Harness.DefaultConfig`); a header's `config` overrides keys.
pub const DEFAULT_CONFIG: &[(&str, &str)] = &[
    ("GameEndEmailUsername", "harness"),
    ("GameEndEmailPassword", "harness"),
    ("GameEndEmailFrom", "player@treachery.online"),
    ("GameEndEmailTo", "admin@treachery.online"),
    ("GameAdminUsername", "admin"),
];

/// Strings longer than this are written as their SHA-256 in compact transcripts (`Harness.CompactLimit`).
pub const COMPACT_LIMIT: usize = 200;

/// Tables, their sort keys, and the column left out (`Harness.Tables`).
pub const TABLES: &[(&str, &str, Option<&str>)] = &[
    ("Users", "Id", None),
    ("UserTokens", "Token", None),
    ("PersistedGames", "GameId", Some("Id")),
    ("ScheduledGames", "GameId", Some("Id")),
    ("ArchivedGames", "Id", None),
];

const TICKS_PER_MS: i64 = 10_000;

/// How a step ended on the Rust side.
#[derive(Debug, Clone)]
pub enum Outcome {
    /// The step ran; here is its transcript line.
    Line(String),
    /// The hub reached a `todo!()`: not ported yet.
    Todo(String),
    /// Another panic.
    Panic(String),
    /// The step's arguments did not decode into a `HubCall`.
    BadArgs(String),
}

struct Pending {
    due: DateTimeOffset,
    seq: u64,
    action: TimerAction,
}

type Received = Rc<RefCell<BTreeMap<u64, Vec<String>>>>;

struct RecordingSink {
    connection: u64,
    received: Received,
}

impl ConnectionSink for RecordingSink {
    fn send(&self, _game_id: Option<&str>, _user_id: Option<i32>, callback: &Callback) {
        let line = callback_json(callback);
        self.received.borrow_mut().entry(self.connection).or_default().push(line);
    }
}

pub struct Runner {
    rt: tokio::runtime::Runtime,
    hub: GameHub,
    clock: Rc<ManualClock>,
    timers: Rc<RecordingTimers>,
    mailer: Rc<RecordingMailer>,
    ids: Rc<SequentialIds>,
    config: Config,
    received: Received,
    pending: Vec<Pending>,
    seq: u64,
    db_path: PathBuf,
    _dir: tempfile::TempDir,
    full: bool,
    step: usize,
    /// Results saved with `"as"`, for `${name.Path}` in later arguments (`Harness.Resolve`).
    vars: BTreeMap<String, Value>,
}

thread_local! {
    static QUIET_PANICS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Keeps expected panics (`todo!()` in unported hub methods) off stderr while the runner calls the hub.
fn install_panic_hook() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let default = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if !QUIET_PANICS.with(|q| q.get()) {
                default(info);
            }
        }));
    });
}

fn panic_outcome(payload: Box<dyn std::any::Any + Send>) -> Result<String, Outcome> {
    if let Some(e) = dotnet_compat::exception::as_cs_exception(&*payload) {
        // An engine exception escaped the hub method: the C# call fails with that exception.
        return Ok(exception_name(e.type_name));
    }
    let text = payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_else(|| "panic".to_string());
    if text.starts_with("not yet implemented") {
        Err(Outcome::Todo(text))
    } else {
        Err(Outcome::Panic(text))
    }
}

/// Exception names as compared with C# (`Harness.ExceptionName`): the short type name, with every
/// EF Core and SQLite exception counted as one kind, `(database)`.
fn exception_name(type_name: &str) -> String {
    let short = type_name.rsplit('.').next().unwrap_or(type_name);
    match short {
        "DbUpdateException" | "DbUpdateConcurrencyException" | "SqliteException" => "(database)".to_string(),
        _ => short.to_string(),
    }
}

impl Runner {
    /// A hub on a new database, with the scenario header's start time and configuration.
    pub fn new(header: &Value, full: bool) -> Result<Runner, String> {
        install_panic_hook();
        let start = header.get("start").and_then(Value::as_str).unwrap_or("2026-03-02T10:00:00+02:00");
        let start = DateTimeOffset::parse_json(start).ok_or_else(|| format!("bad start time {start}"))?;
        let full = full || header.get("full").and_then(Value::as_bool) == Some(true);

        let mut config: Vec<(String, String)> = DEFAULT_CONFIG.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        if let Some(Value::Object(overrides)) = header.get("config") {
            for (k, v) in overrides {
                config.retain(|(key, _)| key != k);
                if let Some(v) = v.as_str() {
                    config.push((k.clone(), v.to_string()));
                }
            }
        }
        let config = Config::from_pairs(config.iter().map(|(k, v)| (k.as_str(), v.as_str())));

        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        let db_path = dir.path().join("treachery.db");
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|e| e.to_string())?;
        let clock = Rc::new(ManualClock::new(start));
        let timers = Rc::new(RecordingTimers::default());
        let mailer = Rc::new(RecordingMailer::default());
        let ids = Rc::new(SequentialIds::default());
        let received: Received = Rc::default();
        let hub = Self::new_hub(&rt, &db_path, &config, &clock, &timers, &mailer, &ids)?;
        // "seed": SQL run on the new database (`Harness` does the same).
        if let Some(Value::Array(seed)) = header.get("seed") {
            for sql in seed {
                let sql = sql.as_str().ok_or("seed entries must be strings")?;
                rt.block_on(sqlx::query(sql).execute(&hub.services.db.pool)).map_err(|e| format!("seed: {e}"))?;
            }
        }
        Ok(Runner { rt, hub, clock, timers, mailer, ids, config, received, pending: Vec::new(), seq: 0, db_path, _dir: dir, full, step: 0, vars: BTreeMap::new() })
    }

    fn new_hub(
        rt: &tokio::runtime::Runtime,
        db_path: &std::path::Path,
        config: &Config,
        clock: &Rc<ManualClock>,
        timers: &Rc<RecordingTimers>,
        mailer: &Rc<RecordingMailer>,
        ids: &Rc<SequentialIds>,
    ) -> Result<GameHub, String> {
        let db = rt.block_on(Db::open(db_path.to_str().unwrap())).map_err(|e| e.to_string())?;
        Ok(GameHub::new(Services {
            db,
            config: config.clone(),
            clock: clock.clone(),
            timers: timers.clone(),
            mailer: mailer.clone(),
            ids: ids.clone(),
        }))
    }

    pub fn now(&self) -> DateTimeOffset {
        self.clock_now()
    }

    fn clock_now(&self) -> DateTimeOffset {
        use treachery_server::clock::Clock;
        self.clock.now()
    }

    /// Runs one scenario step.
    pub fn step(&mut self, step: &Value) -> Outcome {
        let op = step.get("op").and_then(Value::as_str).unwrap_or("").to_string();
        self.received.borrow_mut().clear();
        self.mailer.sent.borrow_mut().clear();
        self.ids.randoms.borrow_mut().clear();
        let mut scheduled: Vec<String> = Vec::new();
        let mut fired: Vec<String> = Vec::new();
        let step_nr = self.step;
        self.step += 1;

        let mut line = JsonText::new();
        line.obj_start().key("step").int(step_nr as i64).member("op", Some(&op));
        let mut result_json: Option<String> = None;
        // The Rust exception's message: shown in reports, never compared (C# messages are not recorded).
        let mut detail: Option<String> = None;

        let conn = || step.get("conn").and_then(Value::as_u64).unwrap_or(0);
        match op.as_str() {
            "connect" => {
                let c = conn();
                self.hub.clients.connect(c, Box::new(RecordingSink { connection: c, received: self.received.clone() }));
            }
            "disconnect" => self.hub.clients.disconnect(conn()),
            "advance" => {
                let num = |k: &str| step.get(k).and_then(Value::as_f64).unwrap_or(0.0);
                let ms = num("ms") + 1000.0 * num("seconds") + 60_000.0 * num("minutes") + 86_400_000.0 * num("days");
                let now = self.clock_now();
                let target = DateTimeOffset::new(now.ticks + (ms as i64) * TICKS_PER_MS, now.offset_minutes);
                if let Err(o) = self.advance(target, &mut scheduled, &mut fired) {
                    return o;
                }
            }
            "restart" => {
                // A new server process on the same database.
                match Self::new_hub(&self.rt, &self.db_path, &self.config, &self.clock, &self.timers, &self.mailer, &self.ids) {
                    Ok(hub) => self.hub = hub,
                    Err(e) => return Outcome::Panic(format!("restart: {e}")),
                }
                self.pending.clear();
                self.timers.scheduled.borrow_mut().clear();
            }
            "call" => {
                let method = step.get("method").and_then(Value::as_str).unwrap_or("");
                line.member("method", Some(method));
                let args = match self.resolve(step.get("args").cloned().unwrap_or(Value::Array(Vec::new()))) {
                    Ok(Value::Array(a)) => a,
                    Ok(_) => return Outcome::BadArgs("args is not an array".into()),
                    Err(e) => return Outcome::BadArgs(e),
                };
                let call = match crate::decode::decode(method, &args) {
                    Ok(c) => c,
                    Err(e) => return Outcome::BadArgs(e),
                };
                let cx = CallContext { connection_id: Some(conn()) };
                let hub = &mut self.hub;
                let rt = &self.rt;
                QUIET_PANICS.with(|q| q.set(true));
                let r = std::panic::catch_unwind(AssertUnwindSafe(|| rt.block_on(hub.invoke(&cx, call))));
                QUIET_PANICS.with(|q| q.set(false));
                let mut j = JsonText::new();
                match r {
                    Ok(Ok(reply)) => {
                        j.obj_start().member_raw("Result", &reply_json(&reply)).obj_end();
                    }
                    Ok(Err(e)) => {
                        detail = Some(e.to_string());
                        let name = match &e {
                            HubError::Db(_) => "(database)".to_string(),
                            HubError::Exception { type_name, .. } => exception_name(type_name),
                        };
                        j.obj_start().member("Exception", Some(&name)).obj_end();
                    }
                    Err(payload) => match panic_outcome(payload) {
                        Ok(name) => {
                            j.obj_start().member("Exception", Some(&name)).obj_end();
                        }
                        Err(o) => return o,
                    },
                }
                result_json = Some(j.finish());
                if let Some(name) = step.get("as").and_then(Value::as_str) {
                    let parsed: Value = serde_json::from_str(result_json.as_deref().unwrap()).unwrap_or(Value::Null);
                    let contents = parsed.get("Result").and_then(|r| r.get("Contents")).cloned().unwrap_or(Value::Null);
                    self.vars.insert(name.to_string(), contents);
                }
                self.drain_timers(&mut scheduled);
            }
            other => return Outcome::BadArgs(format!("unknown step op {other}")),
        }

        line.key("result").raw(result_json.as_deref().unwrap_or("null"));
        if let Some(d) = &detail {
            line.member("rustDetail", Some(d));
        }
        line.key("callbacks").obj_start();
        for (c, callbacks) in self.received.borrow().iter() {
            line.key(&c.to_string()).arr_start();
            for cb in callbacks {
                line.raw(cb);
            }
            line.arr_end();
        }
        line.obj_end();
        line.key("mails").arr_start();
        for m in self.mailer.sent.borrow().iter() {
            line.raw(&self.mail_json(m));
        }
        line.arr_end();
        line.key("timers").arr_start();
        for t in &scheduled {
            line.raw(t);
        }
        line.arr_end();
        line.key("fired").arr_start();
        for f in &fired {
            line.str(Some(f));
        }
        line.arr_end();
        line.key("randoms").arr_start();
        for r in self.ids.randoms.borrow().iter() {
            line.str(Some(r));
        }
        line.arr_end();

        let sections = (|| -> Result<(String, String, String), Outcome> {
            Ok((self.tables_json()?, self.games_json()?, self.hub_json()))
        })();
        let (tables, games, hub) = match sections {
            Ok(s) => s,
            Err(o) => return o,
        };
        line.member_raw("tables", &tables).member_raw("games", &games).member_raw("hub", &hub);
        line.obj_end();
        Outcome::Line(line.finish())
    }

    /// Replaces `${name.Member.0}` references in argument strings (`Harness.Resolve`).
    fn resolve(&self, v: Value) -> Result<Value, String> {
        Ok(match v {
            Value::Array(a) => Value::Array(a.into_iter().map(|x| self.resolve(x)).collect::<Result<_, _>>()?),
            Value::Object(o) => Value::Object(o.into_iter().map(|(k, x)| Ok((k, self.resolve(x)?))).collect::<Result<_, String>>()?),
            Value::String(s) if s.contains("${") => {
                if s.starts_with("${") && s.ends_with('}') && s.matches("${").count() == 1 {
                    return self.lookup(&s[2..s.len() - 1]);
                }
                let mut out = String::new();
                let mut rest = s.as_str();
                while let Some(start) = rest.find("${") {
                    out.push_str(&rest[..start]);
                    let end = rest[start..].find('}').ok_or_else(|| format!("unterminated reference in {s}"))? + start;
                    match self.lookup(&rest[start + 2..end])? {
                        Value::String(x) => out.push_str(&x),
                        other => out.push_str(&other.to_string()),
                    }
                    rest = &rest[end + 1..];
                }
                out.push_str(rest);
                Value::String(out)
            }
            other => other,
        })
    }

    fn lookup(&self, reference: &str) -> Result<Value, String> {
        let mut parts = reference.split('.');
        let name = parts.next().unwrap_or("");
        let mut node = self.vars.get(name).ok_or_else(|| format!("unknown variable {name}"))?.clone();
        for p in parts {
            node = match (p.parse::<usize>(), &node) {
                (Ok(i), Value::Array(a)) => a.get(i).cloned().unwrap_or(Value::Null),
                _ => node.get(p).cloned().unwrap_or(Value::Null),
            };
        }
        Ok(node)
    }

    /// Moves newly scheduled timers to the pending list, recording each in `scheduled`.
    fn drain_timers(&mut self, scheduled: &mut Vec<String>) {
        let now = self.clock_now();
        let new: Vec<_> = self.timers.scheduled.borrow_mut().drain(..).collect();
        for (delay, action) in new {
            let due = DateTimeOffset::new(now.ticks + delay.as_millis() as i64 * TICKS_PER_MS, now.offset_minutes);
            let mut j = JsonText::new();
            j.obj_start().member_ser("Due", &due).member("What", Some(&timer_what(&action))).obj_end();
            scheduled.push(j.finish());
            self.pending.push(Pending { due, seq: self.seq, action });
            self.seq += 1;
        }
    }

    /// Fires due timers in order (`Harness.Advance`), each at its due time, then sets the clock to `target`.
    fn advance(&mut self, target: DateTimeOffset, scheduled: &mut Vec<String>, fired: &mut Vec<String>) -> Result<(), Outcome> {
        loop {
            let next = self
                .pending
                .iter()
                .enumerate()
                .filter(|(_, p)| p.due.utc_ticks() <= target.utc_ticks())
                .min_by_key(|(_, p)| (p.due.utc_ticks(), p.seq))
                .map(|(i, _)| i);
            let Some(i) = next else { break };
            let p = self.pending.remove(i);
            if p.due.utc_ticks() > self.clock_now().utc_ticks() {
                self.clock.set(p.due);
            }
            fired.push(timer_what(&p.action));
            let hub = &mut self.hub;
            let rt = &self.rt;
            QUIET_PANICS.with(|q| q.set(true));
            let r = std::panic::catch_unwind(AssertUnwindSafe(|| rt.block_on(hub.on_timer(p.action))));
            QUIET_PANICS.with(|q| q.set(false));
            if let Err(payload) = r {
                // An exception in a delayed call goes nowhere, as in C#; anything else stops the scenario.
                panic_outcome(payload)?;
            }
            self.drain_timers(scheduled);
        }
        self.clock.set(target);
        Ok(())
    }

    fn compact(&self, s: Option<&str>) -> String {
        match s {
            Some(s) if !self.full && s.encode_utf16().count() > COMPACT_LIMIT => escape(Some(&format!("sha256:{}", sha256_hex(s.as_bytes())))),
            _ => escape(s),
        }
    }

    fn mail_json(&self, m: &MailMessage) -> String {
        let mut j = JsonText::new();
        j.obj_start().member("From", Some(&m.from.to_string())).key("To").arr_start();
        for to in &m.to {
            j.str(Some(&to.to_string()));
        }
        j.arr_end()
            .member("Subject", Some(&m.subject))
            .member("Body", Some(&m.body))
            .member_ser("IsBodyHtml", &m.is_body_html)
            .member(
                "Priority",
                Some(match m.priority {
                    MailPriority::Normal => "Normal",
                    MailPriority::Low => "Low",
                    MailPriority::High => "High",
                }),
            )
            .key("Attachments")
            .arr_start();
        for a in &m.attachments {
            j.obj_start().member("Name", Some(&a.name)).member_raw("Content", &self.compact(Some(&a.content))).obj_end();
        }
        j.arr_end().obj_end();
        j.finish()
    }

    fn tables_json(&self) -> Result<String, Outcome> {
        let mut j = JsonText::new();
        j.obj_start();
        for (table, order_by, skip) in TABLES {
            j.key(table).arr_start();
            let sql = format!("SELECT * FROM \"{table}\" ORDER BY \"{order_by}\"");
            let rows = self
                .rt
                .block_on(sqlx::query(&sql).fetch_all(&self.hub.services.db.pool))
                .map_err(|e| Outcome::Panic(format!("reading {table}: {e}")))?;
            for row in rows {
                j.obj_start();
                for (i, col) in row.columns().iter().enumerate() {
                    if Some(col.name()) == *skip {
                        continue;
                    }
                    j.key(col.name());
                    let raw = row.try_get_raw(i).map_err(|e| Outcome::Panic(e.to_string()))?;
                    if raw.is_null() {
                        j.null();
                        continue;
                    }
                    let kind = raw.type_info().name().to_string();
                    match kind.as_str() {
                        "INTEGER" => {
                            let v: i64 = row.try_get_unchecked(i).map_err(|e| Outcome::Panic(e.to_string()))?;
                            j.int(v);
                        }
                        "TEXT" => {
                            let v: String = row.try_get_unchecked(i).map_err(|e| Outcome::Panic(e.to_string()))?;
                            let c = self.compact(Some(&v));
                            j.raw(&c);
                        }
                        other => return Err(Outcome::Panic(format!("{table}.{}: unexpected SQLite value of type {other}", col.name()))),
                    }
                }
                j.obj_end();
            }
            j.arr_end();
        }
        j.obj_end();
        Ok(j.finish())
    }

    fn games_json(&self) -> Result<String, Outcome> {
        let mut j = JsonText::new();
        j.arr_start();
        for (key, mg) in &self.hub.running_games_by_game_id {
            let g = mg.borrow();
            let participation = treachery_server::json::serialize_participation(&g.game.participation.borrow());
            let state = GameState::get_state_as_string(&g.game);
            let canon = sha256_hex(&treachery_engine::canon::dump_game(&g.game));
            j.obj_start()
                .member("Key", Some(key))
                .member("GameId", Some(&g.game_id))
                .member_ser("CreationDate", &g.creation_date)
                .member_ser("LastActivity", &g.last_activity)
                .member_ser("LastPersisted", &g.last_persisted)
                .member_ser("CreatorUserId", &g.creator_user_id)
                .member("Name", g.name.as_deref())
                .member("HashedPassword", g.hashed_password.as_deref())
                .member_ser("ObserversRequirePassword", &g.observers_require_password)
                .member_ser("StatisticsSent", &g.statistics_sent)
                .member_ser("LastAsyncPlayMessageSent", &g.last_async_play_message_sent)
                .member_raw("Participation", &participation)
                .member_raw("State", &self.compact(Some(&state)))
                .member("Canon", Some(&canon))
                .key("Bots")
                .arr_start();
            for (faction, bot) in &g.bots {
                j.obj_start().member_ser("Faction", faction).member_ser("Draws", &bot.random.get_count()).obj_end();
            }
            j.arr_end().obj_end();
        }
        j.arr_end();
        Ok(j.finish())
    }

    fn hub_json(&self) -> String {
        let h = &self.hub;
        let mut j = JsonText::new();
        j.obj_start();

        j.key("UsersByUserToken").arr_start();
        for (token, u) in &h.users_by_user_token {
            let id = u.user.borrow().id;
            let same = h.users_by_id.get(&id).is_some_and(|cached| Rc::ptr_eq(cached, &u.user));
            j.obj_start()
                .member("Token", Some(token))
                .member_ser("Status", &u.status)
                .member_ser("LoggedInDateTime", &u.logged_in_date_time)
                .member_ser("LastSeenDateTime", &u.last_seen_date_time)
                .member_raw("User", &user_json(&u.user.borrow()))
                .member_ser("SameObjectAsUsersById", &same)
                .obj_end();
        }
        j.arr_end();

        j.key("UsersById").arr_start();
        for u in h.users_by_id.values() {
            j.raw(&user_json(&u.borrow()));
        }
        j.arr_end();

        j.key("ConnectionInfoByUserId").arr_start();
        for (user_id, connections) in &h.connection_info_by_user_id {
            j.obj_start().member_ser("UserId", user_id).key("Games").arr_start();
            for (game_id, info) in connections.entries() {
                j.obj_start().member("GameId", Some(game_id)).key("ConnectionId").int(info.connection_id as i64).obj_end();
            }
            j.arr_end().obj_end();
        }
        j.arr_end();

        j.key("ScheduledGamesByGameId").arr_start();
        for (key, g) in &h.scheduled_games_by_game_id {
            let mut subscribed = JsonWriter::new();
            treachery_server::json::write_int_dict(&mut subscribed, &g.subscribed_users);
            j.obj_start()
                .member("Key", Some(key))
                .member("ScheduledGameId", Some(&g.scheduled_game_id))
                .member_ser("DateTime", &g.date_time)
                .member_ser("CreatorUserId", &g.creator_user_id)
                .member("CreatorPlayerName", Some(&g.creator_player_name))
                .member_ser("NumberOfPlayers", &g.number_of_players)
                .member_ser("MaximumTurns", &g.maximum_turns)
                .member_ser("Ruleset", &g.ruleset)
                .member_ser("AsyncPlay", &g.async_play)
                .member_ser("AllowedFactionsInPlay", &g.allowed_factions_in_play)
                .member_raw("SubscribedUsers", &subscribed.finish())
                .obj_end();
        }
        j.arr_end();

        j.member_raw("RunningGames", &sorted_array(&h.running_games));
        j.member_raw("ScheduledGames", &sorted_array(&h.scheduled_games));
        j.member_raw("RecentlySeenUsers", &sorted_array(&h.recently_seen_users));

        for (name, value) in [
            ("LastRestored", h.last_restored),
            ("LastPersistedRunningGames", h.last_persisted_running_games),
            ("LastPersistedScheduledGames", h.last_persisted_scheduled_games),
            ("MaintenanceDate", h.maintenance_date),
            ("LastCleanedUpUserTokens", h.last_cleaned_up_user_tokens),
            ("LastCleanedUpScheduledGames", h.last_cleaned_up_scheduled_games),
            ("LastUpdatedServerStatus", h.last_updated_server_status),
        ] {
            j.member_ser(name, &value);
        }
        j.member_ser("Restoring", &h.restoring);

        j.key("Groups").obj_start();
        let mut groups = h.clients.group_ids();
        groups.sort();
        for game_id in groups {
            let mut members: Vec<u64> = h.clients.group_members(&game_id).into_iter().map(|(c, _)| c).collect();
            members.sort();
            j.key(&game_id).arr_start();
            for c in members {
                j.int(c as i64);
            }
            j.arr_end();
        }
        j.obj_end();

        j.key("PendingTimers").arr_start();
        let mut pending: Vec<&Pending> = self.pending.iter().collect();
        pending.sort_by_key(|p| (p.due.utc_ticks(), p.seq));
        for p in pending {
            j.obj_start().member_ser("Due", &p.due).member("What", Some(&timer_what(&p.action))).obj_end();
        }
        j.arr_end();

        j.obj_end();
        j.finish()
    }
}

/// The C# hook's name for a delayed call.
fn timer_what(action: &TimerAction) -> String {
    match action {
        TimerAction::SendAsyncPlayMessages { game_id } => format!("SendAsyncPlayMessagesIfApplicable {game_id}"),
        TimerAction::PhaseCountdown { game_id, .. } => format!("PhaseCountdown {game_id}"),
        TimerAction::PerformBotEvent { game_id, .. } => format!("PerformBotEvent {game_id}"),
    }
}

fn user_json(u: &treachery_server::db::user::User) -> String {
    let mut j = JsonText::new();
    j.obj_start()
        .member_ser("Id", &u.id)
        .member("Name", Some(&u.name))
        .member("Email", Some(&u.email))
        .member("HashedPassword", Some(&u.hashed_password))
        .member("PasswordResetToken", u.password_reset_token.as_deref())
        .member_ser("PasswordResetTokenCreated", &u.password_reset_token_created)
        .member("PlayerName", Some(&u.player_name))
        .member_ser("LastLogin", &u.last_login)
        .obj_end();
    j.finish()
}

fn sorted_array<T: ToJson>(items: &[T]) -> String {
    let mut texts: Vec<String> = items.iter().map(serialize).collect();
    texts.sort();
    let mut j = JsonText::new();
    j.arr_start();
    for t in &texts {
        j.raw(t);
    }
    j.arr_end();
    j.finish()
}

fn result_json<T: ToJson>(r: &CallResult<T>) -> String {
    serialize(r)
}

/// `Harness.ResultJson`: the result as `Utilities.Serialize` writes it, with a `ServerStatus`'s lists sorted.
pub fn reply_json(reply: &Reply) -> String {
    match reply {
        Reply::Void(r) => result_json(r),
        Reply::LoginInfo(r) => result_json(r),
        Reply::ServerStatus(r) => match &r.contents {
            Some(status) => {
                let mut j = JsonText::new();
                j.obj_start()
                    .member_ser("Success", &r.success)
                    .member_ser("Error", &r.error)
                    .member_ser("ErrorDetails", &r.error_details)
                    .member_raw("Contents", &server_status_json(status))
                    .obj_end();
                j.finish()
            }
            None => result_json(r),
        },
        Reply::GameInitInfo(r) => result_json(r),
        Reply::ServerInfo(r) => result_json(r),
        Reply::String(r) => result_json(r),
        Reply::AdminInfo(r) => result_json(r),
    }
}

fn server_status_json(s: &ServerStatus) -> String {
    let mut j = JsonText::new();
    j.obj_start()
        .member_raw("RunningGames", &sorted_array(&s.running_games))
        .member_raw("OwnGames", &sorted_array(&s.own_games))
        .member_raw("ScheduledGames", &sorted_array(&s.scheduled_games))
        .member_raw("RecentlySeenUsers", &sorted_array(&s.recently_seen_users))
        .obj_end();
    j.finish()
}

/// A callback as `RecordingClient` writes it: `["HandleX", arg, ...]`.
pub fn callback_json(callback: &Callback) -> String {
    let mut j = JsonText::new();
    j.arr_start();
    match callback {
        Callback::HandleGameEvent { evt, new_event_number } => {
            j.str(Some("HandleGameEvent")).ser(evt).ser(new_event_number);
        }
        Callback::HandleChatMessage(m) => {
            j.str(Some("HandleChatMessage")).ser(m);
        }
        Callback::HandleGlobalChatMessage(m) => {
            j.str(Some("HandleGlobalChatMessage")).ser(m);
        }
        Callback::HandleSetTimer(v) => {
            j.str(Some("HandleSetTimer")).ser(v);
        }
        Callback::HandlePhaseControl(info) => {
            j.str(Some("HandlePhaseControl")).ser(info);
        }
        Callback::HandleSetSkin(s) => {
            j.str(Some("HandleSetSkin")).ser(s);
        }
        Callback::HandleUndo(n) => {
            j.str(Some("HandleUndo")).ser(n);
        }
        Callback::HandleRestoreRecentlyUndone => {
            j.str(Some("HandleRestoreRecentlyUndone"));
        }
        Callback::HandleDismissRecentlyUndone => {
            j.str(Some("HandleDismissRecentlyUndone"));
        }
        Callback::HandleUpdateSettings(s) => {
            j.str(Some("HandleUpdateSettings")).ser(s);
        }
        Callback::HandleJoinGame { user_id, user_name, seat } => {
            j.str(Some("HandleJoinGame")).ser(user_id).ser(user_name).ser(seat);
        }
        Callback::HandleSetOrUnsetHost(id) => {
            j.str(Some("HandleSetOrUnsetHost")).ser(id);
        }
        Callback::HandleObserveGame { user_id, user_name } => {
            j.str(Some("HandleObserveGame")).ser(user_id).ser(user_name);
        }
        Callback::HandleOpenOrCloseSeats(seats) => {
            j.str(Some("HandleOpenOrCloseSeats")).ser(seats);
        }
        Callback::HandleRemoveUser { user_id, kick } => {
            j.str(Some("HandleRemoveUser")).ser(user_id).ser(kick);
        }
        Callback::HandleBotSpeed(s) => {
            j.str(Some("HandleBotSpeed")).ser(s);
        }
        Callback::HandleLoadGame(info) => {
            j.str(Some("HandleLoadGame")).ser(info);
        }
        Callback::HandleAssignSeats(assignment) => {
            let mut w = JsonWriter::new();
            treachery_server::json::write_int_dict(&mut w, assignment);
            j.str(Some("HandleAssignSeats")).raw(&w.finish());
        }
    }
    j.arr_end();
    j.finish()
}
