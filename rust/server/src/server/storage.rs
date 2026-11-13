//! Loading the server's state from the database, and saving games.

use treachery_engine::cs::Shared;
use treachery_engine::game::Game;
use treachery_engine::game_state::GameState;

use super::games::load_savegame;
use super::{CallError, Presence, RunningGame, Server, Session};
use crate::db::GameRow;
use crate::participation;
use treachery_engine::enumerations::UserStatus;

/// A game as a savegame.
pub fn savegame(game: &Game) -> String {
    GameState::get_state_as_string(game)
}

/// A running game from its saved row, or why it no longer loads (an event renamed since it
/// was saved, for example).
fn restore(row: &GameRow) -> Result<RunningGame, String> {
    let participation = participation::from_json(&row.participation).map_err(|e| format!("participation: {e}"))?;
    let game = load_savegame(&row.state, &Shared::new(participation), true)?;
    let mut g = RunningGame::new(row.id.clone(), row.name.clone(), row.creator_id, game, row.created);
    g.password_hash = row.password_hash.clone();
    g.last_activity = row.last_activity;
    g.statistics_sent = row.statistics_sent;
    g.last_async_mail = row.last_async_mail;
    Ok(g)
}

impl Server {
    /// Reads everything from the database. Expired sessions are deleted. Games that no longer
    /// load are left out with a log line, and stay in the database.
    pub(super) async fn load(&mut self) -> Result<(), CallError> {
        self.users = self.services.db.users().await?.into_iter().map(|u| (u.id, u)).collect();
        self.drop_expired_sessions().await?;
        for s in self.services.db.sessions().await? {
            if self.users.contains_key(&s.user_id) {
                self.sessions.insert(s.token, Session { user: s.user_id, created: s.created });
                self.presence.entry(s.user_id).or_insert(Presence { status: UserStatus::Online, last_seen: None });
            }
        }
        self.drop_expired_sessions().await?;
        let (loaded, dropped) = self.load_games(false).await?;
        self.scheduled = self.services.db.scheduled_games().await?.into_iter().map(|g| (g.id.clone(), g)).collect();
        println!("loaded {} users, {} sessions, {loaded} games ({dropped} no longer load), {} scheduled games", self.users.len(), self.sessions.len(), self.scheduled.len());
        Ok(())
    }

    /// Loads the saved games that aren't running. Returns how many loaded and how many no
    /// longer load.
    pub(super) async fn load_games(&mut self, keep_running: bool) -> Result<(usize, usize), CallError> {
        let (mut loaded, mut dropped) = (0, 0);
        for row in self.services.db.games().await? {
            if keep_running && self.games.contains_key(&row.id) {
                continue;
            }
            match restore(&row) {
                Ok(mut g) => {
                    g.generation = self.new_generation();
                    self.games.insert(row.id.clone(), g);
                    loaded += 1;
                }
                Err(reason) => {
                    println!("game {} ({}) no longer loads: {reason}", row.id, row.name);
                    dropped += 1;
                }
            }
        }
        Ok((loaded, dropped))
    }

    pub(super) async fn save_game(&mut self, id: &str) -> Result<(), CallError> {
        let Some(g) = self.games.get_mut(id) else { return Ok(()) };
        let row = GameRow {
            id: g.id.clone(),
            name: g.name.clone(),
            creator_id: g.creator,
            password_hash: g.password_hash.clone(),
            created: g.created,
            last_activity: g.last_activity,
            statistics_sent: g.statistics_sent,
            last_async_mail: g.last_async_mail,
            participation: participation::to_json(&g.game.participation.borrow()),
            state: savegame(&g.game),
        };
        g.unsaved = false;
        self.services.db.save_game(&row).await?;
        Ok(())
    }

    /// Saves every game that has unsaved changes. Returns how many.
    pub(super) async fn save_unsaved(&mut self) -> Result<usize, CallError> {
        let ids: Vec<String> = self.games.iter().filter(|(_, g)| g.unsaved).map(|(id, _)| id.clone()).collect();
        for id in &ids {
            self.save_game(id).await?;
        }
        Ok(ids.len())
    }
}
