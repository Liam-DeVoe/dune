//! Game views: what each player and observer sees of a game, sent as `gameView` notifications,
//! and the view calls (`queryDraft`, `report`, `undoList`, `undoPreview`).
//!
//! Views are built from the running game itself: the engine answers their questions from
//! `&Game`, so building a view can't change the game. What views keep between builds (the skin
//! and the faction sheets) is a [`ViewCache`] per followed game. It starts over, with the
//! game's skin, after an undo, a restore, a dismiss or a load, when the running game was
//! replaced by another object (`RunningGame::generation`), and after a view failed to build.

use treachery_protocol::UserId;
use std::collections::{BTreeMap, BTreeSet};
use std::panic::AssertUnwindSafe;

use treachery_view::cache::{UndoEntry, ViewCache};
use treachery_view::schema::{DraftResult, GameView, ReportLines};

use super::connections::{ConnectionId, Connections, PendingViews};
use super::protocol::{self, ErrorCode, Notification};
use super::server::{panic_text, refuse, Answer, CallError, RunningGame};

struct Entry {
    generation: u64,
    cache: Option<ViewCache>,
}

impl Entry {
    /// The cache for the running game, new if the game was replaced.
    fn cache(&mut self, g: &RunningGame) -> &mut ViewCache {
        if self.generation != g.generation {
            self.generation = g.generation;
            self.cache = None;
        }
        self.cache.get_or_insert_with(|| {
            let mut cache = ViewCache::default();
            if let Some(skin) = &g.skin {
                // A skin that doesn't load leaves the default.
                let _ = cache.set_skin(skin);
            }
            cache
        })
    }
}

/// The view caches of the games that connections follow.
#[derive(Default)]
pub struct GameViews {
    entries: BTreeMap<String, Entry>,
}

fn caught<T>(f: impl FnOnce() -> T) -> Result<T, String> {
    std::panic::catch_unwind(AssertUnwindSafe(f)).map_err(|p| panic_text(p.as_ref()))
}

impl GameViews {
    fn entry(&mut self, g: &RunningGame) -> &mut Entry {
        self.entries.entry(g.id.clone()).or_insert_with(|| Entry { generation: g.generation, cache: None })
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
                entry.cache = None;
            }
            let mut targets: BTreeMap<ConnectionId, UserId> = if p.all { connections.members(&id) } else { BTreeMap::new() };
            targets.extend(p.callers);
            if targets.is_empty() {
                continue;
            }
            let seating = g.participation.seating();
            let built = caught(|| -> Vec<(ConnectionId, String)> {
                let cache = entry.cache(g);
                let mut by_user: BTreeMap<UserId, String> = BTreeMap::new();
                targets
                    .into_iter()
                    .map(|(connection, user_id)| {
                        let text = by_user
                            .entry(user_id)
                            .or_insert_with(|| protocol::notify(Notification::GameView(Box::new(cache.view(&g.game, g.participation.viewer(user_id), &seating, g.timer)))))
                            .clone();
                        (connection, text)
                    })
                    .collect()
            });
            match built {
                Ok(out) => out.into_iter().for_each(|(connection, text)| connections.send(connection, text)),
                Err(e) => {
                    // A bug in the view layer or the engine: start over next time.
                    println!("game {id}: no views: {e}");
                    entry.cache = None;
                }
            }
        }
        self.retain(&connections.followed_games());
    }

    /// Drops the caches of games nobody follows.
    pub fn retain(&mut self, followed: &BTreeSet<String>) {
        self.entries.retain(|id, _| followed.contains(id));
    }

    fn with_cache<T>(&mut self, g: &RunningGame, f: impl FnOnce(&mut ViewCache) -> Answer<T>) -> Answer<T> {
        let entry = self.entry(g);
        match caught(|| f(entry.cache(g))) {
            Ok(answer) => answer,
            Err(e) => {
                entry.cache = None;
                Err(CallError::Refused(ErrorCode::InvalidGameEvent, e))
            }
        }
    }

    /// The options and validation of a draft of one of the user's actions.
    pub fn query_draft(&mut self, g: &RunningGame, user_id: UserId, draft: &str) -> Answer<DraftResult> {
        let viewer = g.participation.viewer(user_id);
        let seating = g.participation.seating();
        self.with_cache(g, |cache| match cache.query(&g.game, viewer, &seating, draft) {
            Ok(r) => Ok(r),
            Err(e) => Err(CallError::Refused(ErrorCode::InvalidGameEvent, format!("{e:?}"))),
        })
    }

    /// The lines of a report, as the user may see them.
    pub fn report(&mut self, g: &RunningGame, user_id: UserId, index: i32) -> Answer<ReportLines> {
        self.with_cache(g, |cache| match usize::try_from(index).ok().and_then(|i| cache.report(&g.game, g.participation.viewer(user_id), i)) {
            Some(lines) => Ok(lines),
            None => Err(CallError::Refused(ErrorCode::InvalidGameEvent, format!("There is no report {index}"))),
        })
    }

    /// The events a host can undo to.
    pub fn undo_list(&mut self, g: &RunningGame, user_id: UserId) -> Answer<Vec<UndoEntry>> {
        if !g.participation.is_host(user_id) {
            return Err(refuse(ErrorCode::NoHost));
        }
        self.with_cache(g, |cache| Ok(cache.undo_list(&g.game)))
    }

    /// The host's view just before event `gap`, without actions.
    pub fn undo_preview(&mut self, g: &RunningGame, user_id: UserId, gap: i32) -> Answer<GameView> {
        if !g.participation.is_host(user_id) {
            return Err(refuse(ErrorCode::NoHost));
        }
        let seating = g.participation.seating();
        self.with_cache(g, |cache| match cache.preview(&g.game, g.participation.viewer(user_id), &seating, gap) {
            Ok(view) => Ok(view),
            Err(e) => Err(CallError::Refused(ErrorCode::InvalidGameEvent, e)),
        })
    }
}
