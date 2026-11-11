//! Port of `Server/GameHub_Authentication.cs`. Package H1.
//!
//! EF Core details that matter here:
//!
//! - A `User` loaded through a context is tracked; `SaveChangesAsync` writes only the columns
//!   that changed ([`User::changed_columns`]). `db.Users.Update(user)` writes all of them.
//! - Both throw `DbUpdateConcurrencyException` when the row is gone (a user who deleted
//!   themselves with `DeleteUser` keeps their token, see GameHub_Other.cs:470).
//! - `new MailAddress(...)` runs after the database writes and outside `SendMail`'s try/catch,
//!   so an address .NET rejects fails the call with the user or reset token already saved.

use std::cell::RefCell;
use std::rc::Rc;

use treachery_engine::date_time_offset::DateTimeOffset;
use treachery_engine::enumerations::{ErrorType, GameListScope, UserStatus};
use treachery_engine::game::Game;

use super::{GameHub, HubError, HubResult};
use crate::db::persisted_user_token::PersistedUserToken;
use crate::db::user::User;
use crate::dotnet_string;
use crate::logged_in_user::{LoggedInUser, UserRef};
use crate::mail::{MailAddress, MailMessage, MailPriority};
use crate::model::login_info::LoginInfo;
use crate::model::result::{CallResult, VoidResult};
use crate::model::server_status::ServerStatus;
use crate::net_time::{DateTimeOffsetExt, TimeSpanExt};

/// `(a - b).TotalMinutes` for `DateTimeOffset`s.
fn total_minutes_between(a: DateTimeOffset, b: DateTimeOffset) -> f64 {
    a.subtract(b).total_minutes()
}

/// What EF throws when an UPDATE matches no row.
fn db_update_concurrency_exception() -> HubError {
    HubError::Exception {
        type_name: "DbUpdateConcurrencyException",
        message: "The database operation was expected to affect 1 row(s), but actually affected 0 row(s); data may have been modified or deleted since entities were loaded. See https://go.microsoft.com/fwlink/?LinkId=527962 for information on understanding and handling optimistic concurrency exceptions.".to_string(),
    }
}

/// `new MailAddress(text)`, throwing as C# does.
fn mail_address(text: &str) -> HubResult<MailAddress> {
    Ok(MailAddress::parse(text)?)
}

/// `new LoginInfo { UserId = user.Id, Token = userToken, PlayerName = user.PlayerName, UserName = user.Name, Email = user.Email }`.
fn login_info(user: &User, token: &str) -> LoginInfo {
    LoginInfo { user_id: user.id, token: token.to_string(), player_name: user.player_name.clone(), user_name: user.name.clone(), email: user.email.clone() }
}

impl GameHub {
    /// `SaveChangesAsync` for one tracked user loaded as `original` and changed to `user`.
    async fn save_tracked_user(&self, user: &User, original: &User) -> HubResult<()> {
        if self.services.db.users_update_columns(user, &user.changed_columns(original)).await? {
            Ok(())
        } else {
            Err(db_update_concurrency_exception())
        }
    }

    /// C#: `public async Task<Result<LoginInfo>> RequestCreateUser(string userName, string hashedPassword, string email, string playerName)` (GameHub_Authentication.cs:5)
    pub async fn request_create_user(&mut self, user_name: &str, hashed_password: &str, email: Option<&str>, player_name: &str) -> HubResult<CallResult<LoginInfo>> {
        let cleaned_username = dotnet_string::to_lower(dotnet_string::trim(user_name));

        if dotnet_string::length(&cleaned_username) <= 3 {
            return Ok(Self::error(ErrorType::UserNameTooShort, ""));
        }

        if dotnet_string::length(&cleaned_username) > 40 {
            return Ok(Self::error(ErrorType::UserNameTooLong, ""));
        }

        let trimmed_player_name = dotnet_string::trim(player_name).to_string();
        let trimmed_email = email.map(|e| dotnet_string::trim(e).to_string()).unwrap_or_default();

        if dotnet_string::length(&trimmed_player_name) <= 3 {
            return Ok(Self::error(ErrorType::PlayerNameTooShort, ""));
        }

        if dotnet_string::length(&trimmed_player_name) > 40 {
            return Ok(Self::error(ErrorType::PlayerNameTooLong, ""));
        }

        let db = self.services.db.clone();

        if db.users_any_with_name(&cleaned_username).await? {
            return Ok(Self::error(ErrorType::UserNameExists, ""));
        }

        let mut added = User {
            name: cleaned_username.clone(),
            hashed_password: hashed_password.to_string(),
            email: trimmed_email.clone(),
            player_name: trimmed_player_name.clone(),
            last_login: self.now(),
            ..User::default()
        };
        added.id = db.users_insert(&added).await?;

        // The context still tracks the added entity, so EF's identity resolution returns that
        // object when the row found is the one just added.
        let user = match db.users_first_by_name_and_password(&cleaned_username, hashed_password).await? {
            None => return Ok(Self::error(ErrorType::UserCreationFailed, "")),
            Some(row) if row.id == added.id => added,
            Some(row) => row,
        };
        let user: UserRef = Rc::new(RefCell::new(user));
        let user_id = user.borrow().id;

        // UsersById.TryAdd
        self.users_by_id.entry(user_id).or_insert_with(|| user.clone());

        let user_token = self.login_and_create_token(&user).await?;

        if trimmed_email.is_empty() {
            return Ok(Self::success_with(login_info(&user.borrow(), &user_token)));
        }

        // A C# raw string literal: the lines of the source file joined by its line ends (LF in
        // the deployed checkout), with the closing delimiter's indentation removed.
        let mut mail_message = MailMessage {
            from: mail_address("admin@treachery.online")?,
            to: Vec::new(),
            subject: "Welcome to treachery.online".to_string(),
            is_body_html: true,
            body: format!(
                "<p>Welcome to treachery.online, <strong>{cleaned_username}</strong>.</p>\n\
                 <p>Your player name is: <strong>{trimmed_player_name}</strong>.</p>\n\
                 <p>If you ever need to reset your password, a reset token will be sent to this e-mail address.</p>\n\
                 <p>You can edit your player name, email address and password any time after logging in.</p>"
            ),
            priority: MailPriority::Normal,
            attachments: Vec::new(),
        };

        mail_message.to.push(mail_address(&trimmed_email)?);
        self.send_mail(mail_message);

        Ok(Self::success_with(login_info(&user.borrow(), &user_token)))
    }

    /// C#: `private static async Task<string> LoginAndCreateToken(TreacheryContext db, User user)` (GameHub_Authentication.cs:76)
    ///
    /// Returns the user's existing token if there is one; that `LoggedInUser` keeps the `User`
    /// object it was created with. C# takes the first match in `ConcurrentDictionary` order,
    /// which is unspecified; this takes the smallest token (the harness compares as a set).
    pub async fn login_and_create_token(&mut self, user: &UserRef) -> HubResult<String> {
        let user_id = user.borrow().id;
        if let Some((token, _)) = self.users_by_user_token.iter().find(|(_, v)| v.id() == user_id) {
            return Ok(token.clone());
        }

        let user_token = self.generate_token();
        let logged_in_user = LoggedInUser::new(user.clone(), None, self.now());
        let created = logged_in_user.logged_in_date_time;
        // ConcurrentDictionary.TryAdd
        self.users_by_user_token.entry(user_token.clone()).or_insert(logged_in_user);

        // Saved so the token is still valid after a restart. See RestoreUserTokens.
        self.services.db.user_tokens_insert(&PersistedUserToken { token: user_token.clone(), user_id, created }).await?;
        Ok(user_token)
    }

    /// C#: `public async Task<Result<LoginInfo>> RequestLogin(int version, string userName, string hashedPassword)` (GameHub_Authentication.cs:92)
    pub async fn request_login(&mut self, version: i32, user_name: &str, hashed_password: &str) -> HubResult<CallResult<LoginInfo>> {
        if Game::LATEST_VERSION != version {
            return Ok(Self::error(ErrorType::InvalidGameVersion, ""));
        }

        let Some(mut user) = self.services.db.users_first_for_login(user_name, hashed_password).await? else {
            return Ok(Self::error(ErrorType::InvalidUserNameOrPassword, ""));
        };

        let original = user.clone();
        user.last_login = self.now();
        self.save_tracked_user(&user, &original).await?;

        // A fresh object, not the one in UsersById.
        let user: UserRef = Rc::new(RefCell::new(user));
        let user_token = self.login_and_create_token(&user).await?;

        Ok(Self::success_with(login_info(&user.borrow(), &user_token)))
    }

    /// C#: `public async Task<VoidResult> RequestPasswordReset(string usernameOrEmail)` (GameHub_Authentication.cs:113)
    pub async fn request_password_reset(&mut self, username_or_email: &str) -> HubResult<VoidResult> {
        let mut users = self.services.db.users_where_email_or_name(username_or_email).await?;

        if users.is_empty() {
            return Ok(Self::error(ErrorType::UnknownUsernameOrEmailAddress, ""));
        }

        let mail = users[0].email.clone();

        if dotnet_string::is_white_space(&mail) {
            return Ok(Self::error(ErrorType::NoEmailAddressForUser, ""));
        }

        if users.iter().any(|u| total_minutes_between(self.now(), u.password_reset_token_created) < 5.0) {
            return Ok(Self::error(ErrorType::ResetRequestTooSoon, ""));
        }

        let token = self.generate_token();
        let now = self.now();

        let originals = users.clone();
        for user in users.iter_mut() {
            user.password_reset_token = Some(token.clone());
            user.password_reset_token_created = now;
        }

        // One SaveChangesAsync for all of them.
        for (user, original) in users.iter().zip(&originals) {
            self.save_tracked_user(user, original).await?;
        }

        let users_message = if users.len() == 1 {
            format!("user: {}", users[0].name)
        } else {
            format!("users: {}", users.iter().map(|u| u.name.as_str()).collect::<Vec<_>>().join(", "))
        };

        let mut mail_message = MailMessage {
            from: mail_address("noreply@treachery.online")?,
            to: Vec::new(),
            subject: "Password Reset".to_string(),
            is_body_html: true,
            body: format!(
                "<p>You have requested a password reset for <strong>{users_message}</strong>.</p>\n\
                 <p>You can use this token to reset your password: <strong>{token}</strong>.</p>"
            ),
            priority: MailPriority::Normal,
            attachments: Vec::new(),
        };

        mail_message.to.push(mail_address(&mail)?);
        self.send_mail(mail_message);

        Ok(Self::success())
    }

    /// C#: `public async Task<Result<LoginInfo>> RequestSetPassword(string userName, string passwordResetToken, string newHashedPassword)` (GameHub_Authentication.cs:165)
    pub async fn request_set_password(&mut self, user_name: &str, password_reset_token: &str, new_hashed_password: &str) -> HubResult<CallResult<LoginInfo>> {
        let Some(mut user) = self.services.db.users_first_by_name(user_name).await? else {
            return Ok(Self::error(ErrorType::UnknownUserName, ""));
        };

        match user.password_reset_token.as_deref() {
            Some(t) if !t.is_empty() && dotnet_string::trim(t) == password_reset_token => {}
            _ => return Ok(Self::error(ErrorType::InvalidResetToken, "")),
        }

        // DateTime.Now converts to a DateTimeOffset at the local offset: the same instant.
        if total_minutes_between(self.now(), user.password_reset_token_created) > 60.0 {
            return Ok(Self::error(ErrorType::ResetTokenExpired, ""));
        }

        let original = user.clone();
        user.hashed_password = new_hashed_password.to_string();
        user.password_reset_token = Some(String::new());
        user.password_reset_token_created = DateTimeOffset::default();

        self.save_tracked_user(&user, &original).await?;

        let user: UserRef = Rc::new(RefCell::new(user));
        let user_token = self.login_and_create_token(&user).await?;
        Ok(Self::success_with(login_info(&user.borrow(), &user_token)))
    }

    /// C#: `public async Task<Result<LoginInfo>> GetLoginInfo(string userToken)` (GameHub_Authentication.cs:191)
    pub async fn get_login_info(&mut self, user_token: &str) -> HubResult<CallResult<LoginInfo>> {
        self.restore_games_if_server_just_started().await?;

        let Some(user) = self.users_by_user_token.get(user_token) else {
            return Ok(Self::error(ErrorType::UserNotFound, ""));
        };

        Ok(Self::success_with(LoginInfo {
            user_id: user.id(),
            token: user_token.to_string(),
            player_name: user.player_name(),
            user_name: user.username(),
            email: user.email(),
        }))
    }

    /// C#: `public async Task<Result<LoginInfo>> RequestUpdateUserInfo(string userToken, string hashedPassword, string playerName, string email)` (GameHub_Authentication.cs:201)
    ///
    /// Changes the `User` object the `LoggedInUser` holds (shared with `UsersById` or not) and
    /// writes all of its columns, stale ones included. No length checks on the player name.
    pub async fn request_update_user_info(&mut self, user_token: &str, hashed_password: Option<&str>, player_name: &str, email: Option<&str>) -> HubResult<CallResult<LoginInfo>> {
        let Some(logged_in_user) = self.users_by_user_token.get(user_token) else {
            return Ok(Self::error(ErrorType::UserNotFound, ""));
        };

        let user = logged_in_user.user.clone();
        {
            let mut u = user.borrow_mut();
            u.player_name = dotnet_string::trim(player_name).to_string();
            u.email = email.map(|e| dotnet_string::trim(e).to_string()).unwrap_or_default();
            if let Some(h) = hashed_password.filter(|h| !h.is_empty()) {
                u.hashed_password = h.to_string();
            }
        }

        let snapshot = user.borrow().clone();
        if !self.services.db.users_update_all(&snapshot).await? {
            return Err(db_update_concurrency_exception());
        }

        self.users_by_id.insert(snapshot.id, user.clone());

        Ok(Self::success_with(login_info(&user.borrow(), user_token)))
    }

    /// C#: `public async Task<Result<ServerStatus>> RequestSetUserStatus(string userToken, UserStatus status)` (GameHub_Authentication.cs:221)
    pub async fn request_set_user_status(&mut self, user_token: &str, status: UserStatus) -> HubResult<CallResult<ServerStatus>> {
        let Some(user) = self.users_by_user_token.get_mut(user_token) else {
            return Ok(Self::error(ErrorType::UserNotFound, ""));
        };

        user.status = status;
        let user_id = user.id();
        self.update_server_status_if_needed(true);

        Ok(Self::success_with(self.filtered_server_status(GameListScope::Active, user_id)))
    }
}
