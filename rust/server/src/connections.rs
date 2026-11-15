//! Open connections, the connections that follow each game, and notifications to them.
//!
//! A connection is logged in once a login or a resumed session binds a session to it. Every
//! call that needs a user takes it from there (protocol.md, Connection). Ending a session logs
//! out its connections.
//!
//! A connection follows a game once it created, joined, observed or reconnected to it, and
//! stops when it closes, its user leaves or is kicked, or it is logged out. Every notification about a game goes
//! to the connections that follow it. Some notifications change what members may see; those
//! also mark the game as needing new views, which the server sends after the call
//! (`views.rs`), before its reply.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Value};
use tokio::sync::mpsc;

use super::protocol;
use super::wire::{GameChatMessage, GameInitInfo, GlobalChatMessage, PhaseControlInfo};

/// Unique per WebSocket for the life of the process.
pub type ConnectionId = u64;

/// A notification (protocol.md, Notifications), apart from `GameView`.
pub enum Notification {
    ChatMessage(GameChatMessage),
    GlobalChatMessage(GlobalChatMessage),
    SetTimer(i32),
    PhaseControl(PhaseControlInfo),
    SetSkin(String),
    /// The settings in the savegame format.
    UpdateSettings(Value),
    JoinGame { user: i32, name: String, seat: i32 },
    SetOrUnsetHost(i32),
    ObserveGame { user: i32, name: String },
    OpenOrCloseSeats(Vec<i32>),
    RemoveUser { user: i32, kick: bool },
    BotSpeed(i32),
    LoadGame(GameInitInfo),
    AssignSeats(Vec<(i32, i32)>),
    /// The connection's session ended (logout, password change, account deletion).
    SessionEnded,
}

impl Notification {
    fn encode(&self) -> String {
        use Notification::*;
        let to = |v: &dyn erased::Ser| v.value();
        let (name, args) = match self {
            ChatMessage(m) => ("ChatMessage", vec![to(m)]),
            GlobalChatMessage(m) => ("GlobalChatMessage", vec![to(m)]),
            SetTimer(v) => ("SetTimer", vec![json!(v)]),
            PhaseControl(info) => ("PhaseControl", vec![to(info)]),
            SetSkin(skin) => ("SetSkin", vec![json!(skin)]),
            UpdateSettings(settings) => ("UpdateSettings", vec![settings.clone()]),
            JoinGame { user, name, seat } => ("JoinGame", vec![json!(user), json!(name), json!(seat)]),
            SetOrUnsetHost(user) => ("SetOrUnsetHost", vec![json!(user)]),
            ObserveGame { user, name } => ("ObserveGame", vec![json!(user), json!(name)]),
            OpenOrCloseSeats(seats) => ("OpenOrCloseSeats", vec![json!(seats)]),
            RemoveUser { user, kick } => ("RemoveUser", vec![json!(user), json!(kick)]),
            BotSpeed(speed) => ("BotSpeed", vec![json!(speed)]),
            LoadGame(info) => ("LoadGame", vec![to(info)]),
            AssignSeats(seats) => ("AssignSeats", vec![json!(seats)]),
            SessionEnded => ("SessionEnded", vec![]),
        };
        protocol::notify(name, args)
    }

    /// What it means for the members' views.
    fn view_effect(&self) -> ViewEffect {
        use Notification::*;
        match self {
            LoadGame(_) => ViewEffect::Rebuild,
            JoinGame { .. } | RemoveUser { .. } | AssignSeats(_) | SetOrUnsetHost(_) | OpenOrCloseSeats(_) | SetSkin(_) | SetTimer(_) | BotSpeed(_) => ViewEffect::Update,
            ChatMessage(_) | GlobalChatMessage(_) | PhaseControl(_) | UpdateSettings(_) | ObserveGame { .. } | SessionEnded => ViewEffect::None,
        }
    }
}

mod erased {
    pub trait Ser {
        fn value(&self) -> serde_json::Value;
    }
    impl<T: serde::Serialize> Ser for T {
        fn value(&self) -> serde_json::Value {
            serde_json::to_value(self).expect("payloads serialize")
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum ViewEffect {
    None,
    Update,
    Rebuild,
}

/// The views a game's members need after the current call.
#[derive(Default, Debug)]
pub struct PendingViews {
    /// Every member gets a new view.
    pub all: bool,
    /// The game was replaced (undo, restore, dismiss, load): views start over from its history.
    pub rebuild: bool,
    /// Connections that get a view whatever `all` says, with the user each is for.
    pub callers: BTreeMap<ConnectionId, i32>,
}

#[derive(Default)]
pub struct Connections {
    sockets: BTreeMap<ConnectionId, mpsc::UnboundedSender<String>>,
    /// Game id to the connections following it, with the user each follows it as.
    groups: BTreeMap<String, BTreeMap<ConnectionId, i32>>,
    pending: BTreeMap<String, PendingViews>,
    /// The logged-in connections: the digest of their session, and its user.
    logins: BTreeMap<ConnectionId, (String, i32)>,
}

impl Connections {
    pub fn connect(&mut self, connection: ConnectionId, outgoing: mpsc::UnboundedSender<String>) {
        self.sockets.insert(connection, outgoing);
    }

    /// Forgets a connection. Dropping its queue ends the socket's writer, which closes it.
    pub fn disconnect(&mut self, connection: ConnectionId) {
        self.sockets.remove(&connection);
        self.logins.remove(&connection);
        self.unfollow_connection(connection);
    }

    /// The connection stops following every game.
    fn unfollow_connection(&mut self, connection: ConnectionId) {
        for members in self.groups.values_mut() {
            members.remove(&connection);
        }
        self.groups.retain(|_, m| !m.is_empty());
    }

    /// Binds a session to the connection. A connection that was logged in as another user stops
    /// following the games it followed as that user.
    pub fn log_in(&mut self, connection: ConnectionId, session: &str, user: i32) {
        if !self.sockets.contains_key(&connection) {
            return;
        }
        if self.logins.get(&connection).is_some_and(|(_, u)| *u != user) {
            self.unfollow_connection(connection);
        }
        self.logins.insert(connection, (session.to_string(), user));
    }

    /// The session digest and user the connection is logged in with.
    pub fn login(&self, connection: ConnectionId) -> Option<(&str, i32)> {
        self.logins.get(&connection).map(|(s, u)| (s.as_str(), *u))
    }

    /// Logs out the connections of an ended session: they stop following games and get
    /// `SessionEnded`.
    pub fn end_session(&mut self, session: &str) {
        let ended: Vec<ConnectionId> = self.logins.iter().filter(|(_, (s, _))| s == session).map(|(c, _)| *c).collect();
        for connection in ended {
            self.logins.remove(&connection);
            self.unfollow_connection(connection);
            self.send(connection, Notification::SessionEnded.encode());
        }
    }

    pub fn send(&self, connection: ConnectionId, text: String) {
        if let Some(tx) = self.sockets.get(&connection) {
            let _ = tx.send(text);
        }
    }

    /// The connection follows the game as `user`.
    pub fn follow(&mut self, game: &str, connection: ConnectionId, user: i32) {
        if self.sockets.contains_key(&connection) {
            self.groups.entry(game.to_string()).or_default().insert(connection, user);
        }
    }

    /// Every connection of `user` stops following the game.
    pub fn unfollow_user(&mut self, game: &str, user: i32) {
        if let Some(members) = self.groups.get_mut(game) {
            members.retain(|_, u| *u != user);
            if members.is_empty() {
                self.groups.remove(game);
            }
        }
    }

    /// Nobody follows the game any more (it was closed).
    pub fn drop_game(&mut self, game: &str) {
        self.groups.remove(game);
        self.pending.remove(game);
    }

    /// The connections following a game, with their users.
    pub fn members(&self, game: &str) -> BTreeMap<ConnectionId, i32> {
        self.groups.get(game).cloned().unwrap_or_default()
    }

    pub fn followed_games(&self) -> BTreeSet<String> {
        self.groups.keys().cloned().collect()
    }

    /// How many users have a connection that follows some game.
    pub fn users_in_games(&self) -> usize {
        self.groups.values().flat_map(|m| m.values()).collect::<BTreeSet<_>>().len()
    }

    /// Sends a notification to the game's followers.
    pub fn notify(&mut self, game: &str, notification: Notification) {
        let text = notification.encode();
        for connection in self.groups.get(game).map(|m| m.keys().copied().collect::<Vec<_>>()).unwrap_or_default() {
            self.send(connection, text.clone());
        }
        match notification.view_effect() {
            ViewEffect::None => {}
            ViewEffect::Update => self.game_changed(game),
            ViewEffect::Rebuild => self.game_replaced(game),
        }
    }

    /// Sends a notification to the game's followers that follow it as one of `users`.
    pub fn notify_users(&self, game: &str, users: &[i32], notification: Notification) {
        let text = notification.encode();
        for (connection, user) in self.groups.get(game).into_iter().flatten() {
            if users.contains(user) {
                self.send(*connection, text.clone());
            }
        }
    }

    /// Sends a notification to every open connection.
    pub fn notify_all(&self, notification: Notification) {
        let text = notification.encode();
        for connection in self.sockets.keys() {
            self.send(*connection, text.clone());
        }
    }

    /// The game changed: every member gets a new view.
    pub fn game_changed(&mut self, game: &str) {
        self.pending.entry(game.to_string()).or_default().all = true;
    }

    /// The game was replaced by another state of it (undo, restore, dismiss, load).
    pub fn game_replaced(&mut self, game: &str) {
        let p = self.pending.entry(game.to_string()).or_default();
        p.all = true;
        p.rebuild = true;
    }

    /// One connection gets its view of the game.
    pub fn view_for(&mut self, game: &str, connection: ConnectionId, user: i32) {
        self.pending.entry(game.to_string()).or_default().callers.insert(connection, user);
    }

    pub fn take_pending(&mut self) -> BTreeMap<String, PendingViews> {
        std::mem::take(&mut self.pending)
    }
}
