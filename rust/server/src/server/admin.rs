//! What the admin (the user named in `TREACHERY_ADMIN`) may do, and deleting accounts.

use treachery_protocol::UserId;
use super::{refuse, Answer, CallError, Server};
use crate::protocol::ErrorCode;
use crate::time::Time;
use treachery_protocol::{AdminInfo, AdminUser};

/// The text the admin page shows.
fn message(text: String) -> Answer<String> {
    Ok(text)
}

impl Server {
    fn check_admin(&self, user: UserId) -> Result<(), CallError> {
        if self.is_admin(user) {
            Ok(())
        } else {
            Err(refuse(ErrorCode::InvalidUserNameOrPassword))
        }
    }

    pub(super) fn set_maintenance(&mut self, user: UserId, starts: Time) -> Answer<String> {
        self.check_admin(user)?;
        self.maintenance = Some(starts);
        message("Maintenance window updated".into())
    }

    /// Saves every game with unsaved changes now.
    pub(super) async fn save_all(&mut self, user: UserId) -> Answer<String> {
        self.check_admin(user)?;
        let saved = self.save_unsaved().await?;
        message(format!("Saved {saved} of {} running games.", self.games.len()))
    }

    /// Loads the saved games that aren't running (games added to the database by hand, for
    /// example). Running games are kept as they are.
    pub(super) async fn reload_all(&mut self, user: UserId) -> Answer<String> {
        self.check_admin(user)?;
        let (loaded, dropped) = self.load_games(true).await?;
        message(format!("Loaded {loaded} saved games that were not running; {dropped} no longer load. {} games are running.", self.games.len()))
    }

    pub(super) async fn admin_close_game(&mut self, user: UserId, game: &str) -> Answer<String> {
        self.check_admin(user)?;
        self.end_game(game).await?;
        message("Game removed".into())
    }

    pub(super) fn download_game(&self, user: UserId, game: &str) -> Answer<String> {
        self.check_admin(user)?;
        let g = self.games.get(game).ok_or(refuse(ErrorCode::GameNotFound))?;
        Ok(super::storage::savegame(&g.game))
    }

    pub(super) async fn admin_cancel_scheduled_game(&mut self, user: UserId, id: &str) -> Answer<String> {
        self.check_admin(user)?;
        self.scheduled.remove(id);
        self.services.db.delete_scheduled_game(id).await?;
        message("Scheduled game cancelled".into())
    }

    /// Users may delete themselves; the admin anyone. The games they created close, they leave
    /// the others, and all their sessions end, which logs out their connections.
    pub(super) async fn delete_user(&mut self, caller: UserId, user: UserId) -> Answer<String> {
        if caller != user && !self.is_admin(caller) {
            return Err(refuse(ErrorCode::NotAdmin));
        }
        let games: Vec<String> = self.games.values().filter(|g| g.creator == user).map(|g| g.id.clone()).collect();
        for game in games {
            self.end_game(&game).await?;
        }
        let joined: Vec<String> = self.games.values().filter(|g| g.participation.is_participant(user)).map(|g| g.id.clone()).collect();
        for game in joined {
            self.remove_from_game(&game, user);
        }
        self.end_sessions_of(user, None).await?;
        self.services.db.delete_user(user).await?;
        self.users.remove(&user);
        self.login_links.retain(|_, l| l.user_id != user);
        self.presence.remove(&user);
        message("User deleted".into())
    }

    pub(super) fn admin_info(&self, user: UserId) -> Answer<AdminInfo> {
        self.check_admin(user)?;
        Ok(AdminInfo {
            users: self
                .users
                .values()
                .map(|u| AdminUser { user_id: u.id, user_name: u.name.clone(), email: u.email.clone(), last_login: u.last_login, player_name: u.player_name.clone() })
                .collect(),
            sessions: self.sessions.len() as i32,
            users_in_games: self.connections.users_in_games() as i32,
            running_games: self.games.len() as i32,
        })
    }
}
