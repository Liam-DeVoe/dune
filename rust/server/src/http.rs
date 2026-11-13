//! The process: the game thread that owns the [`Server`], and the HTTP server (axum) that
//! serves the WebSocket, the dev scene endpoint and the web client's files.
//!
//! Socket tasks run on a multi-threaded runtime and send what they receive to the game thread
//! as [`Command`]s; the game thread handles them one at a time. On Ctrl-C or SIGTERM the game
//! thread saves every unsaved game before the process exits.

use std::future::IntoFuture;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, Method, StatusCode, Uri};
use axum::response::Response;
use tokio::sync::{mpsc, oneshot};

use super::connections::ConnectionId;
use super::db::Db;
use super::mail::{LogMailer, Mailer, SmtpMailer};
use super::server::{Server, Services};
use super::settings::Settings;
use super::static_files::{self, StaticRoots};
use super::time::{Clock, ManualClock, SystemClock};
use super::timers::{Timer, Timers};

/// Work for the game thread.
pub enum Command {
    Connected { connection: ConnectionId, outgoing: mpsc::UnboundedSender<String> },
    Received { connection: ConnectionId, text: String },
    Disconnected { connection: ConnectionId },
    Timer(Timer),
    /// `POST /dev/scene`: the JSON to answer with, or why not.
    Scene { fixture: String, faction: String, reply: oneshot::Sender<Result<String, String>> },
    /// Save and stop.
    Shutdown(oneshot::Sender<()>),
}

/// What the HTTP handlers share. Cheap to clone.
#[derive(Clone)]
pub struct AppState {
    commands: mpsc::UnboundedSender<Command>,
    next_connection: Arc<AtomicU64>,
    roots: Arc<StaticRoots>,
    dev_scene: bool,
}

impl AppState {
    pub fn send(&self, command: Command) -> Result<(), ()> {
        self.commands.send(command).map_err(|_| ())
    }

    pub fn next_connection_id(&self) -> ConnectionId {
        self.next_connection.fetch_add(1, Ordering::Relaxed) + 1
    }
}

/// Timers that sleep on the game thread and then queue the timer.
struct TokioTimers {
    commands: mpsc::UnboundedSender<Command>,
}

impl Timers for TokioTimers {
    fn schedule(&self, delay: Duration, timer: Timer) {
        let commands = self.commands.clone();
        tokio::task::spawn_local(async move {
            tokio::time::sleep(delay).await;
            let _ = commands.send(Command::Timer(timer));
        });
    }
}

async fn game_thread(settings: Settings, commands: mpsc::UnboundedSender<Command>, mut rx: mpsc::UnboundedReceiver<Command>, ready: oneshot::Sender<Result<(), String>>) {
    let clock: Rc<dyn Clock> = match settings.dev_scene.as_ref().and_then(|d| d.clock) {
        Some(t) => Rc::new(ManualClock::new(t)),
        None => Rc::new(SystemClock),
    };
    let mailer: Rc<dyn Mailer> = match &settings.smtp {
        Some(smtp) => Rc::new(SmtpMailer { smtp: smtp.clone() }),
        None => Rc::new(LogMailer),
    };
    let db = match Db::open(&settings.database).await {
        Ok(db) => db,
        Err(e) => {
            let _ = ready.send(Err(format!("{}: {e}", settings.database.display())));
            return;
        }
    };
    let services = Services { db, settings, clock, timers: Rc::new(TokioTimers { commands }), mailer };
    let mut server = match Server::start(services).await {
        Ok(s) => s,
        Err(e) => {
            let _ = ready.send(Err(e));
            return;
        }
    };
    let _ = ready.send(Ok(()));
    while let Some(command) = rx.recv().await {
        match command {
            Command::Connected { connection, outgoing } => server.connect(connection, outgoing),
            Command::Received { connection, text } => server.receive(connection, &text).await,
            Command::Disconnected { connection } => server.disconnect(connection),
            Command::Timer(timer) => server.on_timer(timer).await,
            Command::Scene { fixture, faction, reply } => {
                let _ = reply.send(server.create_scene(&fixture, &faction).await.map(|v| v.to_string()));
            }
            Command::Shutdown(done) => {
                server.shutdown().await;
                let _ = done.send(());
                return;
            }
        }
    }
}

async fn static_handler(State(state): State<AppState>, method: Method, uri: Uri) -> Response {
    static_files::serve(&state.roots, &method, &uri).await
}

fn text_response(status: StatusCode, content_type: &str, body: String) -> Response {
    Response::builder().status(status).header(header::CONTENT_TYPE, content_type).body(Body::from(body)).expect("a text response")
}

/// `POST /dev/scene {"fixture": .., "faction": ..}`.
async fn scene_handler(State(state): State<AppState>, body: String) -> Response {
    if !state.dev_scene {
        return text_response(StatusCode::NOT_FOUND, "text/plain", "dev scenes are off".into());
    }
    let request: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => return text_response(StatusCode::BAD_REQUEST, "text/plain", e.to_string()),
    };
    let field = |k: &str| request.get(k).and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let (reply, answer) = oneshot::channel();
    if state.send(Command::Scene { fixture: field("fixture"), faction: field("faction"), reply }).is_err() {
        return text_response(StatusCode::SERVICE_UNAVAILABLE, "text/plain", "the server has stopped".into());
    }
    match answer.await {
        Ok(Ok(json)) => text_response(StatusCode::OK, "application/json", json),
        Ok(Err(e)) => text_response(StatusCode::BAD_REQUEST, "text/plain", e),
        Err(_) => text_response(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", "the server dropped the request".into()),
    }
}

async fn stop_signal() {
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("a SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}

/// Runs the server until Ctrl-C or SIGTERM.
pub fn run(settings: Settings) -> Result<(), String> {
    let (commands, rx) = mpsc::unbounded_channel();
    let (ready_tx, ready_rx) = oneshot::channel();
    let roots = StaticRoots { web: settings.web_root.clone(), shared: settings.assets_root.clone() };
    let listen = settings.listen.clone();
    let dev_scene = settings.dev_scene.clone();

    let thread_commands = commands.clone();
    let game = std::thread::Builder::new()
        .name("game".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("a runtime");
            tokio::task::LocalSet::new().block_on(&rt, game_thread(settings, thread_commands, rx, ready_tx));
        })
        .map_err(|e| e.to_string())?;

    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(|e| e.to_string())?;
    let result = rt.block_on(async {
        ready_rx.await.map_err(|_| "the game thread stopped".to_string())??;
        let state = AppState { commands: commands.clone(), next_connection: Arc::new(AtomicU64::new(0)), roots: Arc::new(roots), dev_scene: dev_scene.is_some() };
        let app = axum::Router::new()
            .route("/ws", axum::routing::get(super::socket::handler))
            .route("/dev/scene", axum::routing::post(scene_handler))
            .fallback(static_handler)
            .with_state(state);
        let mut servers = Vec::new();
        for address in &listen {
            let listener = tokio::net::TcpListener::bind(address).await.map_err(|e| format!("listening on {address}: {e}"))?;
            println!("listening on http://{address}");
            servers.push(tokio::spawn(axum::serve(listener, app.clone()).into_future()));
        }
        if let Some(scene) = &dev_scene {
            println!("dev scenes are on: POST /dev/scene loads fixtures from {}", scene.fixtures.display());
            if let Some(t) = scene.clock {
                println!("the clock stands at {}", super::time::format(t));
            }
        }
        let served = tokio::select! {
            _ = stop_signal() => Ok(()),
            r = futures::future::select_all(servers) => match r.0 {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => Err(e.to_string()),
                Err(e) => Err(e.to_string()),
            },
        };
        let (done, saved) = oneshot::channel();
        if commands.send(Command::Shutdown(done)).is_ok() {
            let _ = saved.await;
        }
        served
    });
    let _ = game.join();
    result
}
