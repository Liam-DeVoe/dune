//! The hub thread and the axum app.
//!
//! All hub state lives on one thread (a current-thread tokio runtime with a `LocalSet`), and the
//! hub runs one call at a time to completion: see server-plan.md, "Concurrency". The axum
//! runtime owns the sockets and talks to the hub thread through [`HubHandle`].

use std::collections::BTreeMap;
use std::panic::AssertUnwindSafe;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::{FromRef, State};
use axum::http::{header, Method, StatusCode, Uri};
use axum::response::Response;
use futures::FutureExt;
use std::future::IntoFuture;
use tokio::sync::{mpsc, oneshot};

use crate::clients::{Callback, ConnectionId, ConnectionSink};
use crate::config::Config;
use crate::db::Db;
use crate::hub::call::{HubCall, Reply};
use crate::hub::{CallContext, GameHub, IdSource, Services};
use crate::mail::SmtpMailer;
use crate::timers::{TimerAction, Timers};
use crate::transport::static_files::{self, StaticRoots};
use crate::transport::{self, ClientMessage};
use crate::views::{GameViews, PendingViewsRef};

/// Work for the hub thread.
pub enum HubCommand {
    Connected { connection: ConnectionId, outgoing: mpsc::UnboundedSender<String> },
    Received { connection: ConnectionId, text: String },
    Disconnected { connection: ConnectionId },
    Timer(TimerAction),
    /// `POST /dev/scene` (`dev.rs`). Answers with the JSON to return, or an error text.
    DevScene { fixture: String, faction: String, reply: oneshot::Sender<Result<String, String>> },
}

/// The sending side of the hub thread's queue. Cheap to clone, `Send`.
#[derive(Clone)]
pub struct HubHandle {
    tx: mpsc::UnboundedSender<HubCommand>,
    next_connection: Arc<AtomicU64>,
}

impl HubHandle {
    pub fn send(&self, command: HubCommand) -> Result<(), ()> {
        self.tx.send(command).map_err(|_| ())
    }

    pub fn next_connection_id(&self) -> ConnectionId {
        self.next_connection.fetch_add(1, Ordering::Relaxed) + 1
    }
}

/// Timers that sleep on the hub thread and then queue the action.
struct TokioTimers {
    handle: HubHandle,
}

impl Timers for TokioTimers {
    fn schedule(&self, delay: Duration, action: TimerAction) {
        let handle = self.handle.clone();
        tokio::task::spawn_local(async move {
            tokio::time::sleep(delay).await;
            let _ = handle.send(HubCommand::Timer(action));
        });
    }
}

/// `GenerateToken()` and `Guid.NewGuid()` from the operating system's CSPRNG, which is what
/// .NET's `RandomNumberGenerator` and `Guid.NewGuid` use.
pub struct RandomIds;

/// `GenerateToken`'s `TokenChars` (GameHub.cs:47).
const TOKEN_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut b = [0u8; N];
    getrandom::fill(&mut b).expect("the operating system's random number generator failed");
    b
}

/// `RandomNumberGenerator.GetInt32(toExclusive)`: uniform in `[0, n)`, by drawing the bits
/// that cover `n - 1` and rejecting values out of range, as .NET does.
fn random_below(n: u32) -> u32 {
    assert!(n > 0);
    let mask = u32::MAX >> (n - 1).leading_zeros().min(31);
    let mask = if n == 1 { 0 } else { mask };
    loop {
        let r = u32::from_le_bytes(random_bytes::<4>()) & mask;
        if r < n {
            return r;
        }
    }
}

impl IdSource for RandomIds {
    /// GameHub.cs:48: 16 characters, each `TokenChars[RandomNumberGenerator.GetInt32(62)]`.
    fn generate_token(&self) -> String {
        (0..16).map(|_| TOKEN_CHARS[random_below(TOKEN_CHARS.len() as u32) as usize] as char).collect()
    }

    /// `Guid.NewGuid().ToString()`: a version 4 GUID in the `D` format, lowercase.
    fn new_guid(&self) -> String {
        let mut b = random_bytes::<16>();
        b[6] = (b[6] & 0x0f) | 0x40;
        b[8] = (b[8] & 0x3f) | 0x80;
        let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
        format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
    }

    /// `new LoggedRandom()`: .NET seeds an unseeded `Random` from the OS, so any seed will do.
    /// A non-negative seed from the OS's generator.
    fn new_random(&self, _what: &str) -> treachery_engine::logged_random::SharedRandom {
        let seed = (u32::from_le_bytes(random_bytes::<4>()) >> 1) as i32;
        treachery_engine::logged_random::SharedRandom::from_seed(seed)
    }
}

/// A socket's sink: encodes callbacks on the hub thread and queues the text. Callbacks to a
/// game group are also noted for the game-view layer, which sends `GameView`s after the call.
pub struct SocketSink {
    pub outgoing: mpsc::UnboundedSender<String>,
    pub pending: PendingViewsRef,
}

impl ConnectionSink for SocketSink {
    fn send(&self, game_id: Option<&str>, _user_id: Option<i32>, callback: &Callback) {
        if let Some(game_id) = game_id {
            self.pending.borrow_mut().note(game_id, callback);
        }
        if let Some(text) = transport::encode_callback(callback) {
            let _ = self.outgoing.send(text);
        }
    }
}

/// The text of a panic that escaped a hub call: a C# exception, or a Rust bug.
fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(e) = payload.downcast_ref::<dotnet_compat::exception::CsException>() {
        e.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else {
        "panic".to_string()
    }
}

/// The token of a call that answers with a `GameInitInfo` (create, join, observe, reconnect,
/// game state): after it succeeds, the caller gets their view of that game (protocol.md, When
/// views are sent).
fn game_init_caller(call: &HubCall) -> Option<String> {
    match call {
        HubCall::RequestCreateGame { user_token, .. }
        | HubCall::RequestJoinGame { user_token, .. }
        | HubCall::RequestObserveGame { user_token, .. }
        | HubCall::RequestReconnectGame { user_token, .. }
        | HubCall::RequestGameState { user_token, .. } => Some(user_token.clone()),
        _ => None,
    }
}

/// Each connection's queue of outgoing text.
pub type Outgoing = BTreeMap<ConnectionId, mpsc::UnboundedSender<String>>;

/// Opens a connection: the hub sends it callbacks, the views layer its views.
pub fn connect(hub: &mut GameHub, views: &GameViews, outgoing: &mut Outgoing, connection: ConnectionId, tx: mpsc::UnboundedSender<String>) {
    hub.clients.connect(connection, Box::new(SocketSink { outgoing: tx.clone(), pending: views.pending.clone() }));
    outgoing.insert(connection, tx);
}

/// Handles one text frame of a connection: a call, a view call or a ping. The call's
/// notifications are queued as the hub sends them, then the views it caused, then the reply.
/// A malformed message closes the connection.
pub async fn receive(hub: &mut GameHub, views: &mut GameViews, outgoing: &mut Outgoing, connection: ConnectionId, text: &str) {
    let reply = match transport::decode(text) {
        Ok(ClientMessage::Ping) => None,
        Ok(ClientMessage::BadCall { id, error }) => Some(transport::encode_call_error(id, &error)),
        Ok(ClientMessage::View { id, call }) => Some(transport::encode_reply_json(id, &views.call(hub, call))),
        Ok(ClientMessage::Call { id, call }) => {
            let caller_token = game_init_caller(&call);
            let cx = CallContext { connection_id: Some(connection) };
            let text = match AssertUnwindSafe(hub.invoke(&cx, call)).catch_unwind().await {
                Ok(Ok(reply)) => {
                    if let (Some(token), Reply::GameInitInfo(r)) = (caller_token, &reply) {
                        let game_id = r.contents.as_ref().filter(|_| r.success).and_then(|c| c.game_id.clone());
                        if let (Some(game_id), Some(user)) = (game_id, hub.users_by_user_token.get(&token)) {
                            views.pending.borrow_mut().caller(&game_id, connection, user.id());
                        }
                    }
                    transport::encode_reply(id, &reply)
                }
                Ok(Err(e)) => transport::encode_call_error(id, &e.to_string()),
                Err(panic) => transport::encode_call_error(id, &panic_text(panic.as_ref())),
            };
            flush_views(views, hub, outgoing);
            Some(text)
        }
        Err(_) => {
            // Dropping the connection's queue ends the socket's writer, which closes it.
            hub.clients.disconnect(connection);
            outgoing.remove(&connection);
            None
        }
    };
    if let (Some(text), Some(tx)) = (reply, outgoing.get(&connection)) {
        let _ = tx.send(text);
    }
}

/// Sends the views the last call or timer caused.
pub fn flush_views(views: &mut GameViews, hub: &GameHub, outgoing: &Outgoing) {
    views.flush(hub, &mut |connection, text| {
        if let Some(tx) = outgoing.get(&connection) {
            let _ = tx.send(text);
        }
    });
}

async fn hub_loop(
    config: Config,
    handle: HubHandle,
    mut rx: mpsc::UnboundedReceiver<HubCommand>,
    ready: oneshot::Sender<Result<(), String>>,
) -> Result<(), String> {
    // `run` has checked `DevSceneClock` already.
    let clock = crate::dev::clock(&config)?;
    let opened = match config.database_path() {
        None => Err("ConnectionStrings:TreacheryDatabase has no Data Source".to_string()),
        Some(path) => Db::open(&path).await.map_err(|e| format!("{path}: {e}")),
    };
    let db = match opened {
        Ok(db) => {
            let _ = ready.send(Ok(()));
            db
        }
        Err(e) => {
            let _ = ready.send(Err(e.clone()));
            return Err(e);
        }
    };
    let services = Services {
        db,
        config,
        clock,
        timers: Rc::new(TokioTimers { handle: handle.clone() }),
        mailer: Rc::new(SmtpMailer),
        ids: Rc::new(RandomIds),
    };
    let mut hub = GameHub::new(services);
    let mut views = GameViews::new();
    let mut outgoing: Outgoing = BTreeMap::new();

    while let Some(command) = rx.recv().await {
        match command {
            HubCommand::Connected { connection, outgoing: tx } => connect(&mut hub, &views, &mut outgoing, connection, tx),
            HubCommand::Disconnected { connection } => {
                hub.clients.disconnect(connection);
                outgoing.remove(&connection);
            }
            HubCommand::Timer(action) => {
                let _ = AssertUnwindSafe(hub.on_timer(action)).catch_unwind().await;
                flush_views(&mut views, &hub, &outgoing);
            }
            HubCommand::DevScene { fixture, faction, reply } => {
                let ghost = handle.next_connection_id();
                let result = match AssertUnwindSafe(crate::dev::create_scene(&mut hub, ghost, &fixture, &faction)).catch_unwind().await {
                    Ok(r) => r.map(|g| g.to_json()),
                    Err(panic) => Err(panic_text(panic.as_ref())),
                };
                flush_views(&mut views, &hub, &outgoing);
                let _ = reply.send(result);
            }
            HubCommand::Received { connection, text } => receive(&mut hub, &mut views, &mut outgoing, connection, &text).await,
        }
    }
    Ok(())
}

/// What the HTTP handlers share.
#[derive(Clone)]
pub struct AppState {
    pub hub: HubHandle,
    pub roots: Arc<StaticRoots>,
    pub dev_scene: bool,
}

impl FromRef<AppState> for HubHandle {
    fn from_ref(state: &AppState) -> Self {
        state.hub.clone()
    }
}

async fn static_handler(State(state): State<AppState>, method: Method, uri: Uri) -> Response {
    static_files::serve(&state.roots, &method, &uri).await
}

fn text_response(status: StatusCode, content_type: &str, body: String) -> Response {
    Response::builder().status(status).header(header::CONTENT_TYPE, content_type).body(Body::from(body)).expect("a text response")
}

/// `POST /dev/scene {"fixture":..,"faction":..}` (`dev.rs`).
async fn dev_scene_handler(State(state): State<AppState>, body: String) -> Response {
    if !state.dev_scene {
        return text_response(StatusCode::NOT_FOUND, "text/plain", "DevScene is off".into());
    }
    let request: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => return text_response(StatusCode::BAD_REQUEST, "text/plain", e.to_string()),
    };
    let field = |k: &str| request.get(k).and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let (reply, answer) = oneshot::channel();
    if state.hub.send(HubCommand::DevScene { fixture: field("fixture"), faction: field("faction"), reply }).is_err() {
        return text_response(StatusCode::SERVICE_UNAVAILABLE, "text/plain", "the hub has stopped".into());
    }
    match answer.await {
        Ok(Ok(json)) => text_response(StatusCode::OK, "application/json", json),
        Ok(Err(e)) => text_response(StatusCode::BAD_REQUEST, "text/plain", e),
        Err(_) => text_response(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", "the hub dropped the request".into()),
    }
}

/// The socket address of an ASP.NET Core `urls` entry (`http://localhost:5000`,
/// `http://*:80`, `http://+:8080`, `http://0.0.0.0:5000`).
pub fn listen_address(url: &str) -> Result<String, String> {
    let rest = url.strip_prefix("http://").ok_or_else(|| format!("{url}: only http:// is served; TLS belongs to the reverse proxy"))?;
    let host_port = rest.trim_end_matches('/');
    let (host, port) = host_port.rsplit_once(':').ok_or_else(|| format!("{url}: no port"))?;
    let host = match host {
        "*" | "+" | "" => "0.0.0.0",
        "localhost" => "127.0.0.1",
        h => h,
    };
    Ok(format!("{host}:{port}"))
}

/// Starts the hub thread and serves HTTP until the process is stopped.
pub fn run(config: Config) -> Result<(), String> {
    let (tx, rx) = mpsc::unbounded_channel();
    let handle = HubHandle { tx, next_connection: Arc::new(AtomicU64::new(0)) };
    let urls = config.urls();
    let roots = StaticRoots::from_config(&config);
    let dev_scene = crate::dev::enabled(&config);
    let fixed_time = crate::dev::fixed_time(&config)?;
    let fixtures = crate::dev::fixtures_dir(&config);

    let hub_handle = handle.clone();
    let (ready_tx, ready_rx) = oneshot::channel();
    std::thread::Builder::new()
        .name("hub".into())
        .spawn(move || -> Result<(), String> {
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|e| e.to_string())?;
            let local = tokio::task::LocalSet::new();
            let r = local.block_on(&rt, hub_loop(config, hub_handle, rx, ready_tx));
            if let Err(e) = &r {
                eprintln!("hub: {e}");
            }
            r
        })
        .map_err(|e| e.to_string())?;

    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(|e| e.to_string())?;
    rt.block_on(async move {
        ready_rx.await.map_err(|_| "the hub thread stopped".to_string())??;
        let state = AppState { hub: handle, roots: Arc::new(roots), dev_scene };
        let app = axum::Router::new()
            .route("/ws", axum::routing::get(transport::ws::handler))
            .route("/dev/scene", axum::routing::post(dev_scene_handler))
            .fallback(static_handler)
            .with_state(state);
        let mut servers = Vec::new();
        for url in &urls {
            let address = listen_address(url)?;
            let listener = tokio::net::TcpListener::bind(&address).await.map_err(|e| format!("bind {address}: {e}"))?;
            println!("Now listening on: {url}");
            servers.push(tokio::spawn(axum::serve(listener, app.clone()).into_future()));
        }
        if dev_scene {
            println!("DevScene is on: POST /dev/scene loads fixtures from {}", fixtures.display());
            if let Some(t) = fixed_time {
                println!("DevSceneClock: the server's clock stands at {}", t.to_round_trip_string());
            }
        }
        tokio::select! {
            _ = tokio::signal::ctrl_c() => Ok(()),
            r = futures::future::select_all(servers) => match r.0 {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => Err(e.to_string()),
                Err(e) => Err(e.to_string()),
            },
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_and_guids() {
        let ids = RandomIds;
        let t = ids.generate_token();
        assert_eq!(t.len(), 16);
        assert!(t.bytes().all(|c| TOKEN_CHARS.contains(&c)));
        assert_ne!(t, ids.generate_token());
        let g = ids.new_guid();
        assert_eq!(g.len(), 36);
        assert_eq!(&g[14..15], "4");
        assert!("89ab".contains(&g[19..20]));
        assert!(g.bytes().all(|c| c == b'-' || c.is_ascii_digit() || (b'a'..=b'f').contains(&c)));
    }

    #[test]
    fn token_characters_are_uniform() {
        // 62 does not divide 2^k: a plain modulo would favour the first characters. With
        // rejection every character has probability 1/62; check the counts stay near 1/62.
        let mut counts = [0u32; 62];
        for _ in 0..62_000 {
            counts[random_below(62) as usize] += 1;
        }
        assert!(counts.iter().all(|&c| (800..1200).contains(&c)), "{counts:?}");
        assert_eq!(random_below(1), 0);
    }

    #[test]
    fn listen_addresses() {
        assert_eq!(listen_address("http://localhost:5000").unwrap(), "127.0.0.1:5000");
        assert_eq!(listen_address("http://*:80/").unwrap(), "0.0.0.0:80");
        assert!(listen_address("https://localhost:5001").is_err());
    }
}
