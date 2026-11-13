//! Accounts and sessions: creating users, logging in, resuming a session on a new connection,
//! login links, logging out, password reset by mail, changing details.
//!
//! A session belongs to the connections that logged in with it (`Connections::log_in`). Its
//! token goes to the client once, in the `LoginInfo` of the call that started it, and the
//! server keeps only its digest. Logging out, a new password and deleting the account end
//! sessions, which logs out their connections.

use treachery_engine::enumerations::{ErrorType, GameListScope, UserStatus};
use treachery_engine::game::Game;

use super::{contents, done, refuse, CallError, Presence, Reply, Server, Session, LOGIN_LINK_HOURS, MAX_SESSIONS_PER_USER};
use crate::connections::ConnectionId;
use crate::db::{LoginLinkRow, SessionRow, UserRow};
use crate::ids;
use crate::mail::{is_valid_address, Mail};
use crate::passwords;
use crate::wire::{LinkLoginInfo, LoginInfo};

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

/// `token` is the session's token when the call started the session, and `None` otherwise.
fn login_info(user: &UserRow, token: Option<String>) -> LoginInfo {
    LoginInfo { user_id: user.id, token, player_name: user.player_name.clone(), user_name: user.name.clone(), email: user.email.clone() }
}

impl Server {
    fn find_user(&self, name: &str) -> Option<&UserRow> {
        let name = normalize(name);
        self.users.values().find(|u| normalize(&u.name) == name)
    }

    /// Starts a session for the user and returns its token. Expired sessions end first, and
    /// the user's least recently used sessions beyond [`MAX_SESSIONS_PER_USER`].
    pub(super) async fn create_session(&mut self, user: i32) -> Result<String, CallError> {
        self.drop_expired_sessions().await?;
        let mut own: Vec<(String, crate::time::Time)> = self.sessions.iter().filter(|(_, s)| s.user == user).map(|(d, s)| (d.clone(), s.last_used)).collect();
        own.sort_by_key(|(_, used)| *used);
        let surplus = (own.len() + 1).saturating_sub(MAX_SESSIONS_PER_USER);
        for (digest, _) in own.into_iter().take(surplus) {
            self.end_session(&digest).await?;
        }

        let token = ids::new_token();
        let now = self.now();
        let row = SessionRow { token_digest: ids::digest(&token), user_id: user, created: now, last_used: now };
        self.services.db.save_session(&row).await?;
        self.sessions.insert(row.token_digest, Session { user, created: now, last_used: now });
        self.presence.entry(user).or_insert(Presence { status: UserStatus::Online, last_seen: None });
        Ok(token)
    }

    /// Starts a session for the user, logs the connection in with it, and returns its token.
    async fn start_session(&mut self, connection: ConnectionId, user: i32) -> Result<String, CallError> {
        let token = self.create_session(user).await?;
        self.connections.log_in(connection, &ids::digest(&token), user);
        Ok(token)
    }

    /// Ends a session: forgets it and logs out its connections.
    pub(super) async fn end_session(&mut self, digest: &str) -> Result<(), CallError> {
        self.sessions.remove(digest);
        self.services.db.delete_session(digest).await?;
        self.connections.end_session(digest);
        Ok(())
    }

    /// Ends every session of the user but `keep`.
    pub(super) async fn end_sessions_of(&mut self, user: i32, keep: Option<&str>) -> Result<(), CallError> {
        let ended: Vec<String> = self.sessions.iter().filter(|(d, s)| s.user == user && Some(d.as_str()) != keep).map(|(d, _)| d.clone()).collect();
        for digest in ended {
            self.end_session(&digest).await?;
        }
        Ok(())
    }

    /// Ends the sessions that expired, and forgets the login links that did.
    pub(super) async fn drop_expired_sessions(&mut self) -> Result<(), CallError> {
        let now = self.now();
        let expired: Vec<String> = self.sessions.iter().filter(|(_, s)| s.expired(now)).map(|(d, _)| d.clone()).collect();
        for digest in expired {
            self.end_session(&digest).await?;
        }
        let stale: Vec<String> = self.login_links.iter().filter(|(_, l)| link_expired(l, now)).map(|(d, _)| d.clone()).collect();
        for digest in stale {
            self.login_links.remove(&digest);
            self.services.db.delete_login_link(&digest).await?;
        }
        Ok(())
    }

    /// A single-use link that logs `user` in and opens `game`, for an async-play mail. Returns
    /// its token.
    pub(super) async fn new_login_link(&mut self, user: i32, game: &str) -> Result<String, CallError> {
        let token = ids::new_token();
        let row = LoginLinkRow { token_digest: ids::digest(&token), user_id: user, game_id: game.to_string(), created: self.now() };
        self.services.db.insert_login_link(&row).await?;
        self.login_links.insert(row.token_digest.clone(), row);
        Ok(token)
    }

    fn mail(&self, from: &str, to: &str, subject: &str, body: String) {
        if !is_valid_address(to) {
            println!("mail '{subject}' not sent: '{to}' is not an address");
            return;
        }
        self.services.mailer.send(Mail { from: from.into(), to: to.trim().into(), subject: subject.into(), body, attachments: vec![], low_priority: false });
    }

    pub(super) async fn create_user(&mut self, connection: ConnectionId, name: &str, password_hash: &str, email: Option<&str>, player_name: &str) -> Reply {
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
            password_hash: passwords::hash(password_hash),
            email: email.clone(),
            player_name: player_name.clone(),
            last_login: self.now(),
            reset_token_digest: None,
            reset_token_created: None,
        };
        user.id = self.services.db.insert_user(&user).await?;
        self.users.insert(user.id, user.clone());
        let token = self.start_session(connection, user.id).await?;

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
        contents(&login_info(&user, Some(token)))
    }

    pub(super) async fn login(&mut self, connection: ConnectionId, version: i32, name: &str, password_hash: &str) -> Reply {
        if version != Game::LATEST_VERSION {
            return Err(refuse(ErrorType::InvalidGameVersion));
        }
        let Some(user) = self.find_user(name).filter(|u| passwords::verify(password_hash, &u.password_hash)) else {
            return Err(refuse(ErrorType::InvalidUserNameOrPassword));
        };
        let mut user = user.clone();
        user.last_login = self.now();
        self.services.db.update_user(&user).await?;
        self.users.insert(user.id, user.clone());
        let token = self.start_session(connection, user.id).await?;
        contents(&login_info(&user, Some(token)))
    }

    /// A new connection logs in with the token of a session it started earlier.
    pub(super) async fn resume_session(&mut self, connection: ConnectionId, token: &str) -> Reply {
        let digest = ids::digest(token);
        let now = self.now();
        let session = match self.sessions.get_mut(&digest) {
            Some(s) if !s.expired(now) => {
                s.last_used = now;
                s.clone()
            }
            Some(_) => {
                self.end_session(&digest).await?;
                return Err(refuse(ErrorType::UserTokenNotFound));
            }
            None => return Err(refuse(ErrorType::UserTokenNotFound)),
        };
        self.services.db.save_session(&SessionRow { token_digest: digest.clone(), user_id: session.user, created: session.created, last_used: now }).await?;
        self.connections.log_in(connection, &digest, session.user);
        contents(&login_info(&self.users[&session.user], None))
    }

    /// The link of an async-play mail: works once, and starts a new session.
    pub(super) async fn link_login(&mut self, connection: ConnectionId, token: &str) -> Reply {
        let digest = ids::digest(token);
        let Some(link) = self.login_links.remove(&digest) else {
            return Err(refuse(ErrorType::UserTokenNotFound));
        };
        self.services.db.delete_login_link(&digest).await?;
        if link_expired(&link, self.now()) || !self.users.contains_key(&link.user_id) {
            return Err(refuse(ErrorType::UserTokenNotFound));
        }
        let token = self.start_session(connection, link.user_id).await?;
        let game = self.games.get(&link.game_id).filter(|g| g.game.is_participant(link.user_id)).map(|g| g.id.clone());
        contents(&LinkLoginInfo { login_info: login_info(&self.users[&link.user_id], Some(token)), game_id: game })
    }

    /// Ends the connection's session, which logs out every connection that uses it.
    pub(super) async fn logout(&mut self, connection: ConnectionId) -> Reply {
        if let Some((digest, _)) = self.connections.login(connection).map(|(d, u)| (d.to_string(), u)) {
            self.end_session(&digest).await?;
        }
        done()
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

        let token = ids::new_reset_token();
        for mut user in users.iter().cloned() {
            user.reset_token_digest = Some(ids::digest(&token));
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

    /// A new password with a mailed reset token. Every earlier session of the user ends.
    pub(super) async fn set_password(&mut self, connection: ConnectionId, name: &str, reset_token: &str, password_hash: &str) -> Reply {
        let Some(user) = self.find_user(name) else {
            return Err(refuse(ErrorType::UnknownUserName));
        };
        let mut user = user.clone();
        if !user.reset_token_digest.as_deref().is_some_and(|d| *d == ids::digest(reset_token.trim())) {
            return Err(refuse(ErrorType::InvalidResetToken));
        }
        if user.reset_token_created.is_none_or(|t| self.now() - t > chrono::Duration::minutes(RESET_TOKEN_MINUTES)) {
            return Err(refuse(ErrorType::ResetTokenExpired));
        }
        user.password_hash = passwords::hash(password_hash);
        user.reset_token_digest = None;
        user.reset_token_created = None;
        self.services.db.update_user(&user).await?;
        self.users.insert(user.id, user.clone());
        self.end_sessions_of(user.id, None).await?;
        let token = self.start_session(connection, user.id).await?;
        contents(&login_info(&user, Some(token)))
    }

    /// An empty password keeps the old one. A new one ends the user's other sessions.
    pub(super) async fn update_user_info(&mut self, connection: ConnectionId, id: i32, password_hash: Option<&str>, player_name: &str, email: Option<&str>) -> Reply {
        let mut user = self.users[&id].clone();
        user.player_name = player_name.trim().to_string();
        user.email = email.unwrap_or_default().trim().to_string();
        let new_password = password_hash.filter(|h| !h.is_empty());
        if let Some(hash) = new_password {
            user.password_hash = passwords::hash(hash);
        }
        self.services.db.update_user(&user).await?;
        self.users.insert(id, user.clone());
        if new_password.is_some() {
            let current = self.connections.login(connection).map(|(d, _)| d.to_string());
            self.end_sessions_of(id, current.as_deref()).await?;
        }
        contents(&login_info(&user, None))
    }

    pub(super) fn set_user_status(&mut self, user: i32, status: UserStatus) -> Reply {
        self.presence.entry(user).or_insert(Presence { status, last_seen: None }).status = status;
        contents(&self.lobby_status(GameListScope::Active, user))
    }
}

fn link_expired(link: &LoginLinkRow, now: crate::time::Time) -> bool {
    now - link.created >= chrono::Duration::hours(LOGIN_LINK_HOURS)
}
