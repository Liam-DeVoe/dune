//! Accounts and sessions: creating users, logging in, password reset by mail, changing details.

use treachery_engine::enumerations::{ErrorType, GameListScope, UserStatus};
use treachery_engine::game::Game;

use super::{contents, done, refuse, CallError, Presence, Reply, Server, Session};
use crate::db::{SessionRow, UserRow};
use crate::ids;
use crate::mail::{is_valid_address, Mail};
use crate::wire::LoginInfo;

/// User and player names must be longer than this.
const MIN_NAME: usize = 4;
const MAX_NAME: usize = 40;
/// A user may ask for a password reset this often.
const RESET_INTERVAL_MINUTES: i64 = 5;
/// A reset token is valid this long.
const RESET_TOKEN_MINUTES: i64 = 60;

/// User names are compared trimmed and without case.
fn normalize(name: &str) -> String {
    name.trim().to_lowercase()
}

fn login_info(user: &UserRow, token: &str) -> LoginInfo {
    LoginInfo { user_id: user.id, token: token.to_string(), player_name: user.player_name.clone(), user_name: user.name.clone(), email: user.email.clone() }
}

impl Server {
    fn find_user(&self, name: &str) -> Option<&UserRow> {
        let name = normalize(name);
        self.users.values().find(|u| normalize(&u.name) == name)
    }

    /// Starts a session for the user and returns its token.
    pub(super) async fn new_session(&mut self, user: i32) -> Result<String, CallError> {
        let token = ids::new_token();
        let created = self.now();
        self.services.db.insert_session(&SessionRow { token: token.clone(), user_id: user, created }).await?;
        self.sessions.insert(token.clone(), Session { user, created });
        self.presence.entry(user).or_insert(Presence { status: UserStatus::Online, last_seen: None });
        Ok(token)
    }

    fn mail(&self, from: &str, to: &str, subject: &str, body: String) {
        if !is_valid_address(to) {
            println!("mail '{subject}' not sent: '{to}' is not an address");
            return;
        }
        self.services.mailer.send(Mail { from: from.into(), to: to.trim().into(), subject: subject.into(), body, attachments: vec![], low_priority: false });
    }

    pub(super) async fn create_user(&mut self, name: &str, password_hash: &str, email: Option<&str>, player_name: &str) -> Reply {
        let name = normalize(name);
        let player_name = player_name.trim().to_string();
        let email = email.unwrap_or_default().trim().to_string();
        if name.chars().count() < MIN_NAME {
            return Err(refuse(ErrorType::UserNameTooShort));
        }
        if name.chars().count() > MAX_NAME {
            return Err(refuse(ErrorType::UserNameTooLong));
        }
        if player_name.chars().count() < MIN_NAME {
            return Err(refuse(ErrorType::PlayerNameTooShort));
        }
        if player_name.chars().count() > MAX_NAME {
            return Err(refuse(ErrorType::PlayerNameTooLong));
        }
        if self.find_user(&name).is_some() {
            return Err(refuse(ErrorType::UserNameExists));
        }

        let mut user = UserRow {
            id: 0,
            name: name.clone(),
            password_hash: password_hash.to_string(),
            email: email.clone(),
            player_name: player_name.clone(),
            last_login: self.now(),
            reset_token: None,
            reset_token_created: None,
        };
        user.id = self.services.db.insert_user(&user).await?;
        self.users.insert(user.id, user.clone());
        let token = self.new_session(user.id).await?;

        if !email.is_empty() {
            self.mail(
                "admin@treachery.online",
                &email,
                "Welcome to treachery.online",
                format!(
                    "<p>Welcome to treachery.online, <strong>{name}</strong>.</p>\n\
                     <p>Your player name is: <strong>{player_name}</strong>.</p>\n\
                     <p>If you ever need to reset your password, a reset token will be sent to this e-mail address.</p>\n\
                     <p>You can edit your player name, email address and password any time after logging in.</p>"
                ),
            );
        }
        contents(&login_info(&user, &token))
    }

    pub(super) async fn login(&mut self, version: i32, name: &str, password_hash: &str) -> Reply {
        if version != Game::LATEST_VERSION {
            return Err(refuse(ErrorType::InvalidGameVersion));
        }
        let Some(user) = self.find_user(name).filter(|u| u.password_hash == password_hash) else {
            return Err(refuse(ErrorType::InvalidUserNameOrPassword));
        };
        let mut user = user.clone();
        user.last_login = self.now();
        self.services.db.update_user(&user).await?;
        self.users.insert(user.id, user.clone());
        let token = self.new_session(user.id).await?;
        contents(&login_info(&user, &token))
    }

    pub(super) fn login_info(&self, token: &str) -> Reply {
        let user = self.session_user(token)?;
        contents(&login_info(&self.users[&user], token))
    }

    pub(super) async fn password_reset(&mut self, name_or_email: &str) -> Reply {
        let wanted = normalize(name_or_email);
        let users: Vec<UserRow> = self.users.values().filter(|u| normalize(&u.email) == wanted || normalize(&u.name) == wanted).cloned().collect();
        let Some(first) = users.first() else {
            return Err(refuse(ErrorType::UnknownUsernameOrEmailAddress));
        };
        let address = first.email.clone();
        if address.trim().is_empty() {
            return Err(refuse(ErrorType::NoEmailAddressForUser));
        }
        let now = self.now();
        if users.iter().any(|u| u.reset_token_created.is_some_and(|t| now - t < chrono::Duration::minutes(RESET_INTERVAL_MINUTES))) {
            return Err(refuse(ErrorType::ResetRequestTooSoon));
        }

        let token = ids::new_token();
        for mut user in users.iter().cloned() {
            user.reset_token = Some(token.clone());
            user.reset_token_created = Some(now);
            self.services.db.update_user(&user).await?;
            self.users.insert(user.id, user);
        }
        let names = if users.len() == 1 {
            format!("user: {}", users[0].name)
        } else {
            format!("users: {}", users.iter().map(|u| u.name.as_str()).collect::<Vec<_>>().join(", "))
        };
        self.mail(
            "noreply@treachery.online",
            &address,
            "Password Reset",
            format!(
                "<p>You have requested a password reset for <strong>{names}</strong>.</p>\n\
                 <p>You can use this token to reset your password: <strong>{token}</strong>.</p>"
            ),
        );
        done()
    }

    pub(super) async fn set_password(&mut self, name: &str, reset_token: &str, password_hash: &str) -> Reply {
        let Some(user) = self.find_user(name) else {
            return Err(refuse(ErrorType::UnknownUserName));
        };
        let mut user = user.clone();
        if !user.reset_token.as_deref().is_some_and(|t| !t.is_empty() && t.trim() == reset_token.trim()) {
            return Err(refuse(ErrorType::InvalidResetToken));
        }
        if user.reset_token_created.is_none_or(|t| self.now() - t > chrono::Duration::minutes(RESET_TOKEN_MINUTES)) {
            return Err(refuse(ErrorType::ResetTokenExpired));
        }
        user.password_hash = password_hash.to_string();
        user.reset_token = None;
        user.reset_token_created = None;
        self.services.db.update_user(&user).await?;
        self.users.insert(user.id, user.clone());
        let token = self.new_session(user.id).await?;
        contents(&login_info(&user, &token))
    }

    /// An empty password keeps the old one.
    pub(super) async fn update_user_info(&mut self, token: &str, password_hash: Option<&str>, player_name: &str, email: Option<&str>) -> Reply {
        let id = self.session_user(token)?;
        let mut user = self.users[&id].clone();
        user.player_name = player_name.trim().to_string();
        user.email = email.unwrap_or_default().trim().to_string();
        if let Some(hash) = password_hash.filter(|h| !h.is_empty()) {
            user.password_hash = hash.to_string();
        }
        self.services.db.update_user(&user).await?;
        self.users.insert(id, user.clone());
        contents(&login_info(&user, token))
    }

    pub(super) fn set_user_status(&mut self, token: &str, status: UserStatus) -> Reply {
        let user = self.session_user(token)?;
        self.presence.entry(user).or_insert(Presence { status, last_seen: None }).status = status;
        contents(&self.lobby_status(GameListScope::Active, user))
    }

    /// Drops the sessions that expired, here and in the database.
    pub(super) async fn drop_expired_sessions(&mut self) -> Result<(), CallError> {
        let cutoff = self.now() - chrono::Duration::days(super::SESSION_DAYS);
        self.sessions.retain(|_, s| s.created >= cutoff);
        self.services.db.delete_sessions_before(cutoff).await?;
        Ok(())
    }
}
