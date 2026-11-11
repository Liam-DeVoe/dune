//! SignalR's `Clients`, `Groups` and `Context.ConnectionId`, without SignalR.
//!
//! The hub sends [`Callback`]s, the C# `IGameClient` calls, to a game's group or to every
//! connection. A [`ConnectionSink`] receives them for one connection. The transport's sink
//! encodes a callback to the wire format right away, on the hub thread (callbacks hold engine
//! values, which are not `Send`), and queues the text for that connection's socket. The
//! equivalence harness's sink records them.
//!
//! Group membership also records the user of each connection, so that the game-view layer
//! can send each member their own view (protocol.md, "Game view").

use std::collections::{BTreeMap, BTreeSet};

use dotnet_compat::NetDictionary;
use treachery_engine::events::GameEvent;
use treachery_engine::game_settings::GameSettings;

use crate::model::chat_message::{GameChatMessage, GlobalChatMessage};
use crate::model::game_init_info::GameInitInfo;
use crate::model::phase_control_info::PhaseControlInfo;

/// SignalR `Context.ConnectionId`: unique per WebSocket for the life of the process.
pub type ConnectionId = u64;

/// One call of `IGameClient` (Shared/Model/IGameClient.cs).
#[derive(Clone)]
pub enum Callback {
    HandleGameEvent { evt: GameEvent, new_event_number: i32 },
    HandleChatMessage(GameChatMessage),
    HandleGlobalChatMessage(GlobalChatMessage),
    HandleSetTimer(i32),
    HandlePhaseControl(PhaseControlInfo),
    HandleSetSkin(String),
    HandleUndo(i32),
    HandleRestoreRecentlyUndone,
    HandleDismissRecentlyUndone,
    HandleUpdateSettings(GameSettings),
    HandleJoinGame { user_id: i32, user_name: String, seat: i32 },
    HandleSetOrUnsetHost(i32),
    HandleObserveGame { user_id: i32, user_name: String },
    HandleOpenOrCloseSeats(Vec<i32>),
    HandleRemoveUser { user_id: i32, kick: bool },
    HandleBotSpeed(i32),
    HandleLoadGame(GameInitInfo),
    HandleAssignSeats(NetDictionary<i32, i32>),
}

/// Receives the callbacks for one connection, in order.
pub trait ConnectionSink {
    /// `game_id` and `user_id` are set for a group send: the game and the user this connection
    /// joined it as.
    fn send(&self, game_id: Option<&str>, user_id: Option<i32>, callback: &Callback);
}

/// The connections and groups of the hub. Lives on the hub thread.
#[derive(Default)]
pub struct Clients {
    connections: BTreeMap<ConnectionId, Box<dyn ConnectionSink>>,
    /// Game id to the connections in its group, with the user each joined as.
    groups: BTreeMap<String, BTreeMap<ConnectionId, i32>>,
}

impl Clients {
    pub fn connect(&mut self, id: ConnectionId, sink: Box<dyn ConnectionSink>) {
        self.connections.insert(id, sink);
    }

    /// A socket closed: SignalR removes the connection from every group.
    pub fn disconnect(&mut self, id: ConnectionId) {
        self.connections.remove(&id);
        for members in self.groups.values_mut() {
            members.remove(&id);
        }
        self.groups.retain(|_, m| !m.is_empty());
    }

    /// `Groups.AddToGroupAsync(connectionId, gameId)`, remembering the user.
    pub fn add_to_group(&mut self, connection: ConnectionId, game_id: &str, user_id: i32) {
        self.groups.entry(game_id.to_string()).or_default().insert(connection, user_id);
    }

    /// `Groups.RemoveFromGroupAsync(connectionId, gameId)`.
    pub fn remove_from_group(&mut self, connection: ConnectionId, game_id: &str) {
        if let Some(members) = self.groups.get_mut(game_id) {
            members.remove(&connection);
            if members.is_empty() {
                self.groups.remove(game_id);
            }
        }
    }

    /// The members of a game's group: connection and user.
    pub fn group_members(&self, game_id: &str) -> Vec<(ConnectionId, i32)> {
        self.groups.get(game_id).map(|m| m.iter().map(|(c, u)| (*c, *u)).collect()).unwrap_or_default()
    }

    /// `Clients.Group(gameId).HandleX(...)`.
    pub fn group(&self, game_id: &str, callback: Callback) {
        if let Some(members) = self.groups.get(game_id) {
            for (connection, user_id) in members {
                if let Some(sink) = self.connections.get(connection) {
                    sink.send(Some(game_id), Some(*user_id), &callback);
                }
            }
        }
    }

    /// `Clients.All.HandleX(...)`.
    pub fn all(&self, callback: Callback) {
        for sink in self.connections.values() {
            sink.send(None, None, &callback);
        }
    }

    pub fn connection_ids(&self) -> BTreeSet<ConnectionId> {
        self.connections.keys().copied().collect()
    }

    /// The games that have a group with at least one member. For the equivalence harness.
    pub fn group_ids(&self) -> Vec<String> {
        self.groups.iter().filter(|(_, m)| !m.is_empty()).map(|(g, _)| g.clone()).collect()
    }
}
