//! Game views: what each player and observer sees of a game, sent as `gameView` notifications,
//! and the view calls (`queryDraft`, `report`, `undoList`, `undoPreview`).
//!
//! Views are built from a replica of each followed game (`treachery_view::replica`), never
//! from the running game itself: building a view asks the engine questions that can change a
//! game's state, and the running game must change only through events. The replica follows the
//! running game by executing the events of its history it hasn't executed yet. It starts over
//! from the history after an undo, a restore, a dismiss or a load, when the running game was
//! replaced by another object (`RunningGame::generation`), and when it can't be the same game
//! any more.

use treachery_protocol::UserId;
use std::collections::{BTreeMap, BTreeSet};
use std::panic::AssertUnwindSafe;

use treachery_engine::json;
use treachery_view::replica::{UndoEntry, ViewReplica};
use treachery_view::schema::{DraftResult, GameView, ReportLines};

use super::connections::{ConnectionId, Connections, PendingViews};
use super::protocol::{self, ErrorCode, Notification};
use super::server::{panic_text, refuse, Answer, CallError, RunningGame};

struct Entry {
    generation: u64,
    replica: Option<ViewReplica>,
}

impl Entry {
    /// Brings the replica up to date with the running game.
    fn sync(&mut self, g: &RunningGame) -> Result<&mut ViewReplica, String> {
        if self.generation != g.generation {
            self.generation = g.generation;
            self.replica = None;
        }
        let game = &g.game;
        if let Some(r) = &self.replica {
            if r.game.rules.version != game.rules.version || r.game.event_count() > game.event_count() {
                self.replica = None;
            }
        }
        let replica = self.replica.get_or_insert_with(|| {
            let mut r = ViewReplica::new(game.rules.version);
            if let Some(skin) = &g.skin {
                // A skin that doesn't load leaves the default.
                let _ = r.set_skin(skin);
            }
            r
        });
        replica.timer = g.timer;
        // Seats change outside the events, so the replica gets them before it executes events
        // that may ask which players are bots.
        replica.seating = g.participation.seating();
        replica.game.human_seats = game.human_seats.clone();
        let done = replica.game.event_count() as usize;
        for (nr, e) in game.history.entries.iter().enumerate().skip(done) {
            replica.apply(&json::serialize(e)).map_err(|m| format!("event {nr} ({}): {m}", e.event_type().name()))?;
        }
        Ok(replica)
    }
}

/// The replicas of the games that connections follow.
#[derive(Default)]
pub struct GameViews {
    entries: BTreeMap<String, Entry>,
}

fn caught<T>(f: impl FnOnce() -> T) -> Result<T, String> {
    std::panic::catch_unwind(AssertUnwindSafe(f)).map_err(|p| panic_text(p.as_ref()))
}

impl GameViews {
    fn entry(&mut self, g: &RunningGame) -> &mut Entry {
        self.entries.entry(g.id.clone()).or_insert_with(|| Entry { generation: g.generation, replica: None })
    }

    /// Sends the views a call or timer made necessary.
    pub fn send(&mut self, pending: BTreeMap<String, PendingViews>, games: &BTreeMap<String, RunningGame>, connections: &Connections) {
        for (id, p) in pending {
            let Some(g) = games.get(&id) else {
                self.entries.remove(&id);
                continue;
            };
            let entry = self.entry(g);
            if p.rebuild {
                entry.replica = None;
            }
            let mut targets: BTreeMap<ConnectionId, UserId> = if p.all { connections.members(&id) } else { BTreeMap::new() };
            targets.extend(p.callers);
            if targets.is_empty() {
                continue;
            }
            let built = caught(|| -> Result<Vec<(ConnectionId, String)>, String> {
                let replica = entry.sync(g)?;
                let mut by_user: BTreeMap<UserId, String> = BTreeMap::new();
                Ok(targets
                    .into_iter()
                    .map(|(connection, user_id)| {
                        let text = by_user
                            .entry(user_id)
                            .or_insert_with(|| protocol::notify(Notification::GameView(Box::new(replica.view(g.participation.viewer(user_id))))))
                            .clone();
                        (connection, text)
                    })
                    .collect())
            });
            match built.and_then(|r| r) {
                Ok(out) => out.into_iter().for_each(|(connection, text)| connections.send(connection, text)),
                Err(e) => {
                    // A bug in the view layer or the engine: start over next time.
                    println!("game {id}: no views: {e}");
                    entry.replica = None;
                }
            }
        }
        self.retain(&connections.followed_games());
    }

    /// Drops the replicas of games nobody follows.
    pub fn retain(&mut self, followed: &BTreeSet<String>) {
        self.entries.retain(|id, _| followed.contains(id));
    }

    fn with_replica<T>(&mut self, g: &RunningGame, f: impl FnOnce(&mut ViewReplica) -> Answer<T>) -> Answer<T> {
        let entry = self.entry(g);
        let answer = caught(|| entry.sync(g).map_err(|e| CallError::Refused(ErrorCode::InvalidGameEvent, e)).and_then(f));
        match answer {
            Ok(Ok(answer)) => Ok(answer),
            Ok(Err(e)) => {
                if matches!(e, CallError::Refused(ErrorCode::InvalidGameEvent, _)) {
                    entry.replica = None;
                }
                Err(e)
            }
            Err(e) => {
                entry.replica = None;
                Err(CallError::Refused(ErrorCode::InvalidGameEvent, e))
            }
        }
    }

    /// The options and validation of a draft of one of the user's actions.
    pub fn query_draft(&mut self, g: &RunningGame, user_id: UserId, draft: &str) -> Answer<DraftResult> {
        let viewer = g.participation.viewer(user_id);
        self.with_replica(g, |replica| {
            if !replica.has_view_for(viewer) {
                // The replica was rebuilt since this user's last view: build it again to know
                // their actions.
                replica.view(viewer);
            }
            match replica.query(viewer, draft) {
                Ok(r) => Ok(r),
                Err(e) => Err(CallError::Refused(ErrorCode::InvalidGameEvent, format!("{e:?}"))),
            }
        })
    }

    /// The lines of a report, as the user may see them.
    pub fn report(&mut self, g: &RunningGame, user_id: UserId, index: i32) -> Answer<ReportLines> {
        self.with_replica(g, |replica| match usize::try_from(index).ok().and_then(|i| replica.report(g.participation.viewer(user_id), i)) {
            Some(lines) => Ok(lines),
            None => Err(CallError::Refused(ErrorCode::InvalidGameEvent, format!("There is no report {index}"))),
        })
    }

    /// The events a host can undo to.
    pub fn undo_list(&mut self, g: &RunningGame, user_id: UserId) -> Answer<Vec<UndoEntry>> {
        if !g.participation.is_host(user_id) {
            return Err(refuse(ErrorCode::NoHost));
        }
        self.with_replica(g, |replica| Ok(replica.undo_list()))
    }

    /// The host's view just before event `gap`, without actions.
    pub fn undo_preview(&mut self, g: &RunningGame, user_id: UserId, gap: i32) -> Answer<GameView> {
        if !g.participation.is_host(user_id) {
            return Err(refuse(ErrorCode::NoHost));
        }
        self.with_replica(g, |replica| match replica.preview(g.participation.viewer(user_id), gap) {
            Ok(view) => Ok(view),
            Err(e) => Err(CallError::Refused(ErrorCode::InvalidGameEvent, e)),
        })
    }
}
