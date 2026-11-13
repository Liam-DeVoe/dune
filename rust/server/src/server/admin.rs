//! What the admin (the user named in `TREACHERY_ADMIN`) may do, and deleting accounts.

use treachery_engine::enumerations::ErrorType;

use super::{contents, refuse, CallError, Reply, Server};
use crate::time::Time;
use crate::wire::{AdminInfo, UserInfo};

fn message(text: String) -> Reply {
    Ok(serde_json::Value::String(text))
}

impl Server {
    fn check_admin(&self, token: &str) -> Result<(), CallError> {
        if self.is_admin(token) {
            Ok(())
        } else {
            Err(refuse(ErrorType::InvalidUserNameOrPassword))
        }
    }

    pub(super) fn set_maintenance(&mut self, token: &str, starts: Time) -> Reply {
        self.check_admin(token)?;
        self.maintenance = Some(starts);
        message("Maintenance window updated".into())
    }

    /// Saves every game with unsaved changes now.
    pub(super) async fn save_all(&mut self, token: &str) -> Reply {
        self.check_admin(token)?;
        let saved = self.save_unsaved().await?;
        message(format!("Saved {saved} of {} running games.", self.games.len()))
    }

    /// Loads the saved games that aren't running (games added to the database by hand, for
    /// example). Running games are kept as they are.
    pub(super) async fn reload_all(&mut self, token: &str) -> Reply {
        self.check_admin(token)?;
        let (loaded, dropped) = self.load_games(true).await?;
        message(format!("Loaded {loaded} saved games that were not running; {dropped} no longer load. {} games are running.", self.games.len()))
    }

    pub(super) async fn admin_close_game(&mut self, token: &str, game: &str) -> Reply {
        self.check_admin(token)?;
        self.end_game(game).await?;
        message("Game removed".into())
    }

    pub(super) fn download_game(&self, token: &str, game: &str) -> Reply {
        self.check_admin(token)?;
        let g = self.games.get(game).ok_or(refuse(ErrorType::GameNotFound))?;
        Ok(serde_json::Value::String(super::storage::savegame(&g.game)))
    }

    pub(super) async fn admin_cancel_scheduled_game(&mut self, token: &str, id: &str) -> Reply {
        self.check_admin(token)?;
        self.scheduled.remove(id);
        self.services.db.delete_scheduled_game(id).await?;
        message("Scheduled game cancelled".into())
    }

    /// Users may delete themselves; the admin anyone. The games they created close, they leave
    /// the others, and all their sessions end.
    pub(super) async fn delete_user(&mut self, token: &str, user: i32) -> Reply {
        let caller = self.session_user(token).map_err(|_| refuse(ErrorType::InvalidUserNameOrPassword))?;
        if caller != user && !self.is_admin(token) {
            return Err(refuse(ErrorType::NotAdmin));
        }
        let games: Vec<String> = self.games.values().filter(|g| g.creator == user).map(|g| g.id.clone()).collect();
        for game in games {
            self.end_game(&game).await?;
        }
        let joined: Vec<String> = self.games.values().filter(|g| g.game.is_participant(user)).map(|g| g.id.clone()).collect();
        for game in joined {
            self.remove_from_game(&game, user);
        }
        self.services.db.delete_user(user).await?;
        self.users.remove(&user);
        self.sessions.retain(|_, s| s.user != user);
        self.presence.remove(&user);
        message("User deleted".into())
    }

    pub(super) fn admin_info(&self, token: &str) -> Reply {
        self.check_admin(token)?;
        contents(&AdminInfo {
            users: self
                .users
                .values()
                .map(|u| UserInfo { id: u.id, username: u.name.clone(), email: u.email.clone(), last_login: u.last_login, player_name: u.player_name.clone() })
                .collect(),
            users_by_user_token_count: self.sessions.len() as i32,
            connection_info_by_user_id_count: self.connections.users_in_games() as i32,
            games_by_game_id_count: self.games.len() as i32,
        })
    }
}
