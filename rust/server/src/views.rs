//! The game-view layer on the server: one [`ViewReplica`] per game that has connected members,
//! `GameView` notifications for them, and the view calls of protocol.md (`QueryDraft`,
//! `RequestReport`, `RequestUndoList`, `RequestUndoPreview`, `RequestSaveGame`).
//!
//! The hub never sees any of this. Its callbacks reach each connection's sink; the transport's
//! sink sends the ordinary ones as notifications and records in [`PendingViews`] which games
//! need new views, and why. After each hub call (and each timer), the runtime calls
//! [`GameViews::flush`], which brings each such game's replica up to date and sends the views,
//! after the call's other notifications and before its reply.
//!
//! The replica follows the server's game by executing the events of its history that it has
//! not executed yet, as `Client.HandleGameEvent` did. It never calls the server's game except to
//! read `History`, `Version` and `Participation` (server-plan.md, Transport: the view layer must
//! not run engine queries on the authoritative game). It is rebuilt from the history after an
//! undo, a restore, a dismiss or a load, as `Client.HandleUndo` and `HandleLoadGame` did, and
//! whenever it can't be the same game any more: another `ManagedGame` object (admin restore),
//! another `Participation` object, more events than the server's game.
//!
//! When a game's group has no member left, its replica is dropped; the next view rebuilds it.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::panic::AssertUnwindSafe;
use std::rc::{Rc, Weak};

use treachery_engine::enumerations::ErrorType;
use treachery_engine::game::Game;
use treachery_engine::json;
use treachery_view::build::Viewer;
use treachery_view::replica::ViewReplica;

use crate::clients::{Callback, ConnectionId};
use crate::hub::{GameHub, ManagedGameRef};
use crate::model::managed_game::ManagedGame;
use crate::transport::{self, ViewCall};

/// What the callbacks of one hub call asked of one game's views.
#[derive(Default, Debug)]
struct PendingGame {
    /// Every member of the group gets a view.
    all: bool,
    /// The replica must be rebuilt from the history (undo, restore, dismiss, load).
    rebuild: bool,
    timer: Option<i32>,
    skin: Option<String>,
    /// Connections that get a view even if `all` is false: the caller of a join, observe,
    /// reconnect or game-state request, with the user they called as.
    callers: BTreeMap<ConnectionId, i32>,
}

/// Filled by the sinks during a hub call, emptied by [`GameViews::flush`].
#[derive(Default, Debug)]
pub struct PendingViews {
    games: BTreeMap<String, PendingGame>,
}

pub type PendingViewsRef = Rc<RefCell<PendingViews>>;

impl PendingViews {
    /// Records what a group callback means for the game's views (protocol.md, When views are
    /// sent). Called once per member, so every effect is idempotent.
    pub fn note(&mut self, game_id: &str, callback: &Callback) {
        use Callback::*;
        fn entry<'a>(s: &'a mut PendingViews, game_id: &str) -> &'a mut PendingGame {
            s.games.entry(game_id.to_string()).or_default()
        }
        match callback {
            HandleGameEvent { .. } => entry(self, game_id).all = true,
            HandleUndo(_) | HandleRestoreRecentlyUndone | HandleDismissRecentlyUndone | HandleLoadGame(_) => {
                let e = entry(self, game_id);
                e.all = true;
                e.rebuild = true;
            }
            HandleJoinGame { .. } | HandleRemoveUser { .. } | HandleAssignSeats(_) | HandleSetOrUnsetHost(_) | HandleOpenOrCloseSeats(_) => {
                entry(self, game_id).all = true
            }
            HandleSetSkin(skin) => {
                let e = entry(self, game_id);
                e.all = true;
                e.skin = Some(skin.clone());
            }
            // The bidding timer is part of the view (`timing.timer`).
            HandleSetTimer(value) => {
                let e = entry(self, game_id);
                e.all = true;
                e.timer = Some(*value);
            }
            HandleChatMessage(_) | HandleGlobalChatMessage(_) | HandlePhaseControl(_) | HandleUpdateSettings(_) | HandleObserveGame { .. } | HandleBotSpeed(_) => {}
        }
    }

    /// A view for one connection: the caller of a join, observe, reconnect or game-state call.
    pub fn caller(&mut self, game_id: &str, connection: ConnectionId, user_id: i32) {
        self.games.entry(game_id.to_string()).or_default().callers.insert(connection, user_id);
    }

    fn take(&mut self) -> BTreeMap<String, PendingGame> {
        std::mem::take(&mut self.games)
    }
}

/// One game's replica and the server state the C# client kept beside its game.
struct Entry {
    /// The `ManagedGame` the replica follows. Another object under the same id (admin restore)
    /// means a rebuild.
    game: Weak<RefCell<ManagedGame>>,
    replica: Option<ViewReplica>,
    /// `Client.Timer`, set by `SetTimer`.
    timer: i32,
    /// The skin set by `SetSkin`, kept across rebuilds.
    skin: Option<String>,
}

impl Entry {
    fn new(game: &ManagedGameRef) -> Self {
        Entry { game: Rc::downgrade(game), replica: None, timer: -1, skin: None }
    }

    /// Brings the replica up to date with the server's game: rebuilds it when it can't be
    /// continued, then executes the events it lacks.
    fn sync(&mut self, managed: &ManagedGameRef) -> Result<&mut ViewReplica, String> {
        if !self.game.upgrade().is_some_and(|g| Rc::ptr_eq(&g, managed)) {
            self.game = Rc::downgrade(managed);
            self.replica = None;
        }
        let mg = managed.borrow();
        let game: &Game = &mg.game;
        if let Some(r) = &self.replica {
            if !r.game.participation.ptr_eq(&game.participation) || r.game.version != game.version || r.game.event_count() > game.event_count() {
                self.replica = None;
            }
        }
        let replica = match &mut self.replica {
            Some(r) => r,
            slot @ None => {
                let mut r = ViewReplica::new(game.version, &game.participation);
                r.timer = self.timer;
                if let Some(skin) = &self.skin {
                    // A skin that doesn't load leaves the default, as in the C# client.
                    let _ = r.set_skin(skin);
                }
                slot.insert(r)
            }
        };
        replica.timer = self.timer;
        let done = replica.game.event_count() as usize;
        for (nr, e) in game.history.iter().enumerate().skip(done) {
            // The event as the C# client received it in `HandleGameEvent`: serialized, then
            // executed on its own copy.
            replica.apply(&json::serialize(e)).map_err(|m| format!("event {nr} ({}): {m}", e.event_type().name()))?;
        }
        Ok(replica)
    }
}

/// The replicas of every game that has members.
#[derive(Default)]
pub struct GameViews {
    entries: BTreeMap<String, Entry>,
    pub pending: PendingViewsRef,
}

fn caught<T>(f: impl FnOnce() -> T) -> Result<T, String> {
    std::panic::catch_unwind(AssertUnwindSafe(f)).map_err(|p| {
        if let Some(e) = p.downcast_ref::<dotnet_compat::exception::CsException>() {
            e.to_string()
        } else if let Some(s) = p.downcast_ref::<String>() {
            s.clone()
        } else if let Some(s) = p.downcast_ref::<&str>() {
            (*s).to_string()
        } else {
            "panic".to_string()
        }
    })
}

impl GameViews {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sends the views the last hub call asked for. `send` queues text on a connection.
    pub fn flush(&mut self, hub: &GameHub, send: &mut dyn FnMut(ConnectionId, String)) {
        let pending = self.pending.borrow_mut().take();
        for (game_id, p) in pending {
            let Some(managed) = hub.running_games_by_game_id.get(&game_id) else {
                self.entries.remove(&game_id);
                continue;
            };
            let entry = self.entries.entry(game_id.clone()).or_insert_with(|| Entry::new(managed));
            if p.rebuild {
                entry.replica = None;
            }
            if let Some(t) = p.timer {
                entry.timer = t;
            }
            if let Some(skin) = p.skin {
                if let Some(r) = &mut entry.replica {
                    let _ = r.set_skin(&skin);
                }
                entry.skin = Some(skin);
            }

            let mut targets: BTreeMap<ConnectionId, i32> = if p.all { hub.clients.group_members(&game_id).into_iter().collect() } else { BTreeMap::new() };
            targets.extend(p.callers);
            if targets.is_empty() {
                continue;
            }

            let built = caught(|| -> Result<Vec<(ConnectionId, String)>, String> {
                let replica = entry.sync(managed)?;
                let mut by_user: BTreeMap<i32, String> = BTreeMap::new();
                let mut out = Vec::new();
                for (connection, user_id) in targets {
                    let text = by_user
                        .entry(user_id)
                        .or_insert_with(|| {
                            let view = replica.view(Viewer { user_id });
                            transport::encode_notify("GameView", &[serde_json::to_string(&view).expect("views serialize")])
                        })
                        .clone();
                    out.push((connection, text));
                }
                Ok(out)
            });
            match built {
                Ok(Ok(out)) => {
                    for (connection, text) in out {
                        send(connection, text);
                    }
                }
                Ok(Err(e)) | Err(e) => {
                    // The replica could not follow the server's game: a bug in the view layer or
                    // the engine. Drop it so the next view starts over, and say so in the log.
                    eprintln!("game {game_id}: no views: {e}");
                    entry.replica = None;
                }
            }
        }
        self.retain_active(hub);
    }

    /// Drops the replicas of games that are gone or have no connected member.
    fn retain_active(&mut self, hub: &GameHub) {
        let groups: BTreeSet<String> = hub.clients.group_ids().into_iter().collect();
        self.entries.retain(|id, _| groups.contains(id) && hub.running_games_by_game_id.contains_key(id));
    }

    /// Answers a view call with the `Result` JSON (protocol.md, Game view, Calls).
    pub fn call(&mut self, hub: &GameHub, call: ViewCall) -> String {
        let (user_token, game_id) = match &call {
            ViewCall::QueryDraft { user_token, game_id, .. }
            | ViewCall::RequestReport { user_token, game_id, .. }
            | ViewCall::RequestUndoList { user_token, game_id }
            | ViewCall::RequestUndoPreview { user_token, game_id, .. }
            | ViewCall::RequestSaveGame { user_token, game_id } => (user_token.clone(), game_id.clone()),
        };
        let (user, managed) = match hub.are_valid::<()>(Some(&user_token), Some(&game_id)) {
            Ok(x) => x,
            Err(e) => return result_json(false, e.error, &e.error_details, None),
        };
        let viewer = Viewer { user_id: user.borrow().id };
        if let ViewCall::RequestSaveGame { .. } = call {
            // MenuComponent's Save game (`CanSave`: hosts and the admin), from the server's game
            // rather than a copy the client kept, as `AdminDownloadGame` does.
            if !managed.borrow().game.is_host(Some(viewer.user_id)) && !hub.is_admin_token(&user_token) {
                return result_json(false, ErrorType::NoHost, "", None);
            }
            let state = json::serialize(&treachery_engine::game_state::GameState::get_state_as_string(&managed.borrow().game));
            return result_json(true, ErrorType::None, "", Some(&state));
        }
        let entry = self.entries.entry(game_id.clone()).or_insert_with(|| Entry::new(&managed));
        let answer = caught(|| -> Result<Result<String, (ErrorType, String)>, String> {
            let replica = entry.sync(&managed)?;
            Ok(match call {
                ViewCall::QueryDraft { draft, .. } => {
                    if !replica.has_view_for(viewer) {
                        // The replica was rebuilt since this user's last view: build it again to
                        // know their actions.
                        replica.view(viewer);
                    }
                    match replica.query(viewer, &draft) {
                        Ok(r) => Ok(serde_json::to_string(&r).expect("draft results serialize")),
                        Err(e) => Err((ErrorType::InvalidGameEvent, format!("{e:?}"))),
                    }
                }
                ViewCall::RequestReport { index, .. } => match usize::try_from(index).ok().and_then(|i| replica.report(viewer, i)) {
                    Some(lines) => Ok(serde_json::to_string(&lines).expect("reports serialize")),
                    None => Err((ErrorType::InvalidGameEvent, format!("There is no report {index}"))),
                },
                ViewCall::RequestUndoList { .. } => {
                    if !replica.game.is_host(Some(viewer.user_id)) {
                        Err((ErrorType::NoHost, String::new()))
                    } else {
                        Ok(serde_json::to_string(&replica.undo_list()).expect("undo lists serialize"))
                    }
                }
                ViewCall::RequestUndoPreview { gap, .. } => {
                    if !replica.game.is_host(Some(viewer.user_id)) {
                        Err((ErrorType::NoHost, String::new()))
                    } else {
                        match replica.preview(viewer, gap) {
                            Ok(view) => Ok(serde_json::to_string(&view).expect("views serialize")),
                            Err(e) => Err((ErrorType::InvalidGameEvent, e)),
                        }
                    }
                }
                ViewCall::RequestSaveGame { .. } => unreachable!("answered above"),
            })
        });
        match answer {
            Ok(Ok(Ok(contents))) => result_json(true, ErrorType::None, "", Some(&contents)),
            Ok(Ok(Err((error, details)))) => result_json(false, error, &details, None),
            Ok(Err(e)) | Err(e) => {
                entry.replica = None;
                result_json(false, ErrorType::InvalidGameEvent, &e, None)
            }
        }
    }
}

/// A `Result` whose `Contents` is already JSON.
fn result_json(success: bool, error: ErrorType, details: &str, contents: Option<&str>) -> String {
    let mut w = treachery_engine::json::JsonWriter::new();
    w.object_value();
    w.member("Success", &success);
    w.member("Error", &error);
    w.member("ErrorDetails", details);
    w.end_object();
    let head = w.finish();
    format!("{},\"Contents\":{}}}", &head[..head.len() - 1], contents.unwrap_or("null"))
}
