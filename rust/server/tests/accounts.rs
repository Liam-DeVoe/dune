//! Accounts: creating users, logging in, sessions, password reset by mail, changing details.

mod common;

use common::*;
use serde_json::{json, Value};

/// `ErrorType` values (protocol.md, Result).
mod error {
    pub const NOT_LOGGED_IN: &str = "notLoggedIn";
    pub const NO_HOST: &str = "noHost";
    pub const USER_NAME_TOO_SHORT: &str = "userNameTooShort";
    pub const USER_NAME_TOO_LONG: &str = "userNameTooLong";
    pub const USER_NAME_EXISTS: &str = "userNameExists";
    pub const PLAYER_NAME_TOO_SHORT: &str = "playerNameTooShort";
    pub const PLAYER_NAME_TOO_LONG: &str = "playerNameTooLong";
    pub const INVALID_GAME_VERSION: &str = "invalidGameVersion";
    pub const INVALID_USER_NAME_OR_PASSWORD: &str = "invalidUserNameOrPassword";
    pub const UNKNOWN_USERNAME_OR_EMAIL_ADDRESS: &str = "unknownUsernameOrEmailAddress";
    pub const NO_EMAIL_ADDRESS_FOR_USER: &str = "noEmailAddressForUser";
    pub const RESET_REQUEST_TOO_SOON: &str = "resetRequestTooSoon";
    pub const UNKNOWN_USER_NAME: &str = "unknownUserName";
    pub const INVALID_RESET_TOKEN: &str = "invalidResetToken";
    pub const RESET_TOKEN_EXPIRED: &str = "resetTokenExpired";
}

/// The reset token in a password reset mail.
fn reset_token(body: &str) -> String {
    let marker = "reset your password: <strong>";
    let start = body.find(marker).unwrap_or_else(|| panic!("no token in {body}")) + marker.len();
    body[start..start + body[start..].find("</strong>").unwrap()].to_string()
}

#[test]
fn create_user_checks_names_and_welcomes_by_mail() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let c = s.connect();
        s.ok(c, "serverInfo", Value::Null).await;

        let create = |name: &str, player: &str| json!({"userName": name, "passwordHash": "hash", "email": "a@example.com", "playerName": player});
        assert_eq!(s.err(c, "createUser", create(" abc ", "Player")).await, error::USER_NAME_TOO_SHORT);
        assert_eq!(s.err(c, "createUser", create(&"x".repeat(41), "Player")).await, error::USER_NAME_TOO_LONG);
        assert_eq!(s.err(c, "createUser", create("alice", " Bo ")).await, error::PLAYER_NAME_TOO_SHORT);
        assert_eq!(s.err(c, "createUser", create("alice", &"y".repeat(41))).await, error::PLAYER_NAME_TOO_LONG);
        assert!(s.mails().is_empty());

        // The user name is stored trimmed and in lower case; the player name trimmed.
        let session = s.ok(c, "createUser", json!({"userName": "  Alice ", "passwordHash": "hash", "email": " alice@example.com ", "playerName": " Alice Player "})).await;
        let info = &session["account"];
        assert_eq!(info["userName"], "alice");
        assert_eq!(info["playerName"], "Alice Player");
        assert_eq!(info["email"], "alice@example.com");
        assert!(info["userId"].as_i64().unwrap() > 0);
        assert!(!session["token"].as_str().unwrap().is_empty());

        let mails = s.mails();
        assert_eq!(mails.len(), 1);
        assert_eq!(mails[0].to, vec!["alice@example.com"]);
        assert_eq!(mails[0].subject, "Welcome to treachery.online");
        assert!(mails[0].body.contains("<strong>alice</strong>") && mails[0].body.contains("<strong>Alice Player</strong>"), "{}", mails[0].body);

        assert_eq!(s.err(c, "createUser", create("ALICE", "Someone")).await, error::USER_NAME_EXISTS);

        // Without an address there is no mail.
        s.ok(c, "createUser", json!({"userName": "bobby", "passwordHash": "hash", "email": null, "playerName": "Bobby Player"})).await;
        s.ok(c, "createUser", json!({"userName": "carol", "passwordHash": "hash", "email": "", "playerName": "Carol Player"})).await;
        assert!(s.mails().is_empty());
    });
}

#[test]
fn login_and_sessions() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let c = s.connect();
        s.ok(c, "serverInfo", Value::Null).await;

        assert_eq!(s.err(c, "login", json!({"version": VERSION - 1, "userName": "alice", "passwordHash": "alice-pw"})).await, error::INVALID_GAME_VERSION);
        assert_eq!(s.err(c, "login", json!({"version": VERSION, "userName": "alice", "passwordHash": "wrong"})).await, error::INVALID_USER_NAME_OR_PASSWORD);
        assert_eq!(s.err(c, "login", json!({"version": VERSION, "userName": "nobody", "passwordHash": "alice-pw"})).await, error::INVALID_USER_NAME_OR_PASSWORD);
        // Without a login the connection can't make calls that need a user.
        assert_eq!(s.err(c, "heartbeat", json!({"scope": "Active"})).await, error::NOT_LOGGED_IN);

        // The name is matched without case and surrounding spaces.
        let session = s.ok(c, "login", json!({"version": VERSION, "userName": " ALICE ", "passwordHash": "alice-pw"})).await;
        let info = &session["account"];
        assert_eq!(info["userId"], alice.id);
        assert_eq!(info["userName"], "alice");
        assert_eq!(info["playerName"], "Alice Player");
        assert_eq!(info["email"], "alice@example.com");
        let token = session["token"].as_str().unwrap().to_string();
        assert_eq!(token.len(), 64, "32 random bytes");
        assert_ne!(token, alice.token, "each login starts its own session");
        // The login logged the connection in.
        s.ok(c, "heartbeat", json!({"scope": "Active"})).await;

        // A token authenticates another connection with the session; the reply carries no token.
        let other = s.connect();
        assert_eq!(s.err(other, "authenticate", json!({"token": "no such token"})).await, error::NOT_LOGGED_IN);
        assert_eq!(s.err(other, "heartbeat", json!({"scope": "Active"})).await, error::NOT_LOGGED_IN);
        let resumed = s.ok(other, "authenticate", json!({"token": token})).await;
        assert_eq!(resumed["userId"], alice.id);
        assert!(resumed.get("token").is_none());
        s.ok(other, "heartbeat", json!({"scope": "Active"})).await;

        // Sessions survive a restart; the database holds only their digests.
        let mut s = s.restart().await;
        let c = s.connect();
        s.ok(c, "serverInfo", Value::Null).await;
        assert_eq!(s.ok(c, "authenticate", json!({"token": token})).await["userId"], alice.id);
        assert_eq!(s.ok(c, "authenticate", json!({"token": alice.token})).await["userId"], alice.id);
        s.ok(c, "login", json!({"version": VERSION, "userName": "alice", "passwordHash": "alice-pw"})).await;
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect_with(sqlx::sqlite::SqliteConnectOptions::new().filename(s.db_path()))
            .await
            .unwrap();
        let stored: Vec<String> = sqlx::query_scalar("SELECT token_digest FROM sessions").fetch_all(&pool).await.unwrap();
        assert!(!stored.is_empty() && !stored.contains(&token) && !stored.contains(&alice.token), "{stored:?}");
        let password: String = sqlx::query_scalar("SELECT password_hash FROM users").fetch_one(&pool).await.unwrap();
        assert!(password.starts_with("$argon2id$"), "{password}");
        pool.close().await;
    });
}

#[test]
fn logging_in_as_another_user_leaves_the_first_users_games() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let game = s.new_game(&alice, "", "").await;
        assert_eq!(s.err(alice.conn, "login", json!({"version": VERSION, "userName": "bobby", "passwordHash": "x"})).await, error::INVALID_USER_NAME_OR_PASSWORD);
        s.ok(alice.conn, "createUser", json!({"userName": "bobby", "passwordHash": "bobby-pw", "email": "", "playerName": "Bobby Player"})).await;
        s.received(alice.conn);
        // The connection is Bobby's now: it doesn't follow Alice's game, and acts as Bobby.
        let other = s.login("alice").await;
        s.ok(other.conn, "setTimer", json!({"game": game, "value": 5})).await;
        assert!(s.notifications(alice.conn).is_empty());
        assert_eq!(s.err(alice.conn, "setTimer", json!({"game": game, "value": 5})).await, error::NO_HOST);
    });
}

#[test]
fn sessions_expire() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        // 30 days without use end a session.
        s.advance_minutes(60 * 24 * 29).await;
        let resumed = s.resume(&alice.token).await;
        s.ok(resumed.conn, "heartbeat", json!({"scope": "Active"})).await;
        s.advance_minutes(60 * 24 * 29).await;
        s.ok(resumed.conn, "heartbeat", json!({"scope": "Active"})).await;
        s.resume(&alice.token).await;
        s.advance_minutes(60 * 24 * 30).await;
        assert_eq!(s.err(resumed.conn, "heartbeat", json!({"scope": "Active"})).await, error::NOT_LOGGED_IN);
        let c = s.connect();
        assert_eq!(s.err(c, "authenticate", json!({"token": alice.token})).await, error::NOT_LOGGED_IN);

        // 90 days end it however often it is used.
        let alice = s.login("alice").await;
        for _ in 0..4 {
            s.advance_minutes(60 * 24 * 20).await;
            s.resume(&alice.token).await;
        }
        s.advance_minutes(60 * 24 * 10).await;
        let c = s.connect();
        assert_eq!(s.err(c, "authenticate", json!({"token": alice.token})).await, error::NOT_LOGGED_IN);
        // and stay gone after a restart.
        let mut s = s.restart().await;
        let c = s.connect();
        assert_eq!(s.err(c, "authenticate", json!({"token": alice.token})).await, error::NOT_LOGGED_IN);
    });
}

#[test]
fn logging_out_ends_the_session_everywhere() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let game = s.new_game(&alice, "", "").await;
        let tab = s.resume(&alice.token).await;
        s.ok(tab.conn, "reconnectGame", json!({"game": game})).await;
        let phone = s.login("alice").await;
        s.received(tab.conn);

        let (_, before) = s.call_raw(alice.conn, "logout", Value::Null).await;
        assert!(find(&before, "sessionEnded").is_some());
        // Every connection of the session is logged out, and stops following the game.
        assert!(find(&s.received(tab.conn), "sessionEnded").is_some());
        assert_eq!(s.err(alice.conn, "heartbeat", json!({"scope": "Active"})).await, error::NOT_LOGGED_IN);
        assert_eq!(s.err(tab.conn, "setTimer", json!({"game": game, "value": 5})).await, error::NOT_LOGGED_IN);
        let c = s.connect();
        assert_eq!(s.err(c, "authenticate", json!({"token": alice.token})).await, error::NOT_LOGGED_IN);
        // Another session goes on.
        s.ok(phone.conn, "setTimer", json!({"game": game, "value": 5})).await;
        assert!(s.notifications(tab.conn).is_empty());
        let mut s = s.restart().await;
        let c = s.connect();
        assert_eq!(s.err(c, "authenticate", json!({"token": alice.token})).await, error::NOT_LOGGED_IN);
        s.ok(c, "authenticate", json!({"token": phone.token})).await;
    });
}

#[test]
fn a_user_keeps_twenty_sessions() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let first = s.user("alice").await;
        let mut sessions = vec![first];
        for _ in 0..20 {
            s.advance_minutes(1).await;
            sessions.push(s.login("alice").await);
        }
        // The least recently used one ended.
        assert_eq!(s.err(sessions[0].conn, "heartbeat", json!({"scope": "Active"})).await, error::NOT_LOGGED_IN);
        for u in &sessions[1..] {
            s.ok(u.conn, "heartbeat", json!({"scope": "Active"})).await;
        }
    });
}

#[test]
fn password_reset_by_mail() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice_token = s.user("alice").await.token;
        let c = s.connect();
        s.ok(c, "serverInfo", Value::Null).await;
        s.ok(c, "createUser", json!({"userName": "nomail", "passwordHash": "hash", "email": "", "playerName": "No Mail"})).await;

        assert_eq!(s.err(c, "requestPasswordReset", json!({"userNameOrEmail": "nobody"})).await, error::UNKNOWN_USERNAME_OR_EMAIL_ADDRESS);
        assert_eq!(s.err(c, "requestPasswordReset", json!({"userNameOrEmail": "nomail"})).await, error::NO_EMAIL_ADDRESS_FOR_USER);

        // By address, without case.
        s.ok(c, "requestPasswordReset", json!({"userNameOrEmail": " Alice@Example.com"})).await;
        let mails = s.mails();
        assert_eq!(mails.len(), 1);
        assert_eq!(mails[0].to, vec!["alice@example.com"]);
        assert_eq!(mails[0].subject, "Password Reset");
        assert!(mails[0].body.contains("<strong>user: alice</strong>"), "{}", mails[0].body);
        let token = reset_token(&mails[0].body);

        // One request per five minutes.
        s.advance_minutes(4).await;
        assert_eq!(s.err(c, "requestPasswordReset", json!({"userNameOrEmail": "alice"})).await, error::RESET_REQUEST_TOO_SOON);
        assert!(s.mails().is_empty());

        assert_eq!(s.err(c, "setPassword", json!({"userName": "nobody", "resetToken": token, "passwordHash": "new-pw"})).await, error::UNKNOWN_USER_NAME);
        assert_eq!(s.err(c, "setPassword", json!({"userName": "alice", "resetToken": "WRONGTOKEN", "passwordHash": "new-pw"})).await, error::INVALID_RESET_TOKEN);
        let before = s.resume(&alice_token).await;
        let info = s.ok(c, "setPassword", json!({"userName": " Alice", "resetToken": token, "passwordHash": "new-pw"})).await;
        assert_eq!(info["account"]["userName"], "alice");
        // The reset ends every earlier session and starts a new one on this connection.
        assert!(find(&s.received(before.conn), "sessionEnded").is_some());
        assert_eq!(s.err(before.conn, "heartbeat", json!({"scope": "Active"})).await, error::NOT_LOGGED_IN);
        let d = s.connect();
        assert_eq!(s.err(d, "authenticate", json!({"token": alice_token})).await, error::NOT_LOGGED_IN);
        s.ok(c, "heartbeat", json!({"scope": "Active"})).await;
        s.ok(d, "authenticate", json!({"token": info["token"]})).await;

        // The token works once, and the new password replaces the old one.
        assert_eq!(s.err(c, "setPassword", json!({"userName": "alice", "resetToken": token, "passwordHash": "other-pw"})).await, error::INVALID_RESET_TOKEN);
        assert_eq!(s.err(c, "login", json!({"version": VERSION, "userName": "alice", "passwordHash": "alice-pw"})).await, error::INVALID_USER_NAME_OR_PASSWORD);
        s.ok(c, "login", json!({"version": VERSION, "userName": "alice", "passwordHash": "new-pw"})).await;
    });
}

#[test]
fn reset_tokens_expire_after_an_hour() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        s.user("alice").await;
        let c = s.connect();
        s.ok(c, "requestPasswordReset", json!({"userNameOrEmail": "alice"})).await;
        let token = reset_token(&s.mails()[0].body);
        s.advance_minutes(61).await;
        assert_eq!(s.err(c, "setPassword", json!({"userName": "alice", "resetToken": token, "passwordHash": "new-pw"})).await, error::RESET_TOKEN_EXPIRED);

        // A new request is allowed and gives a new token.
        s.ok(c, "requestPasswordReset", json!({"userNameOrEmail": "alice"})).await;
        let token = reset_token(&s.mails()[0].body);
        s.advance_minutes(59).await;
        s.ok(c, "setPassword", json!({"userName": "alice", "resetToken": token, "passwordHash": "new-pw"})).await;
    });
}

#[test]
fn users_change_their_details() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let c = alice.conn;
        let anonymous = s.connect();
        assert_eq!(s.err(anonymous, "updateUserInfo", json!({"passwordHash": "", "playerName": "Name", "email": "a@b.c"})).await, error::NOT_LOGGED_IN);

        // An empty password keeps the old one, and the sessions.
        let other = s.login("alice").await;
        let info = s.ok(c, "updateUserInfo", json!({"passwordHash": "", "playerName": " Lady Alice ", "email": " lady@example.com "})).await;
        assert_eq!(info["playerName"], "Lady Alice");
        assert_eq!(info["email"], "lady@example.com");
        assert!(info.get("token").is_none(), "no new session");
        s.ok(other.conn, "heartbeat", json!({"scope": "Active"})).await;
        let check = s.connect();
        s.ok(check, "login", json!({"version": VERSION, "userName": "alice", "passwordHash": "alice-pw"})).await;
        let peek = s.connect();
        assert_eq!(s.ok(peek, "authenticate", json!({"token": alice.token})).await["playerName"], "Lady Alice");

        // A new password ends every other session; this one goes on.
        let info = s.ok(c, "updateUserInfo", json!({"passwordHash": "changed-pw", "playerName": "Lady Alice", "email": null})).await;
        assert_eq!(info["email"], "");
        assert!(find(&s.received(other.conn), "sessionEnded").is_some());
        assert_eq!(s.err(other.conn, "heartbeat", json!({"scope": "Active"})).await, error::NOT_LOGGED_IN);
        assert_eq!(s.err(check, "heartbeat", json!({"scope": "Active"})).await, error::NOT_LOGGED_IN, "logged in with the old password");
        s.ok(c, "heartbeat", json!({"scope": "Active"})).await;
        let again = s.connect();
        s.ok(again, "authenticate", json!({"token": alice.token})).await;
        assert_eq!(s.err(again, "authenticate", json!({"token": other.token})).await, error::NOT_LOGGED_IN);
        assert_eq!(s.err(again, "login", json!({"version": VERSION, "userName": "alice", "passwordHash": "alice-pw"})).await, error::INVALID_USER_NAME_OR_PASSWORD);
        let info = s.ok(again, "login", json!({"version": VERSION, "userName": "alice", "passwordHash": "changed-pw"})).await;
        assert_eq!(info["account"]["playerName"], "Lady Alice");

        // The new player name is what others see.
        let alice = User { conn: again, ..alice };
        let game = s.new_game(&alice, "", "").await;
        let bob = s.user("bobby").await;
        let lobby = s.lobby(&bob, "All").await;
        let entry = lobby["runningGames"].as_array().unwrap().iter().find(|g| g["gameId"] == game.as_str()).unwrap().clone();
        assert_eq!(entry["name"], "Lady Alice's Game");

        // and it survives a restart.
        let mut s = s.restart().await;
        let c = s.connect();
        s.ok(c, "serverInfo", Value::Null).await;
        let info = s.ok(c, "login", json!({"version": VERSION, "userName": "alice", "passwordHash": "changed-pw"})).await;
        assert_eq!(info["account"]["playerName"], "Lady Alice");
    });
}

#[test]
fn user_status_shows_in_the_lobby() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let bob = s.user("bobby").await;
        let anonymous = s.connect();
        assert_eq!(s.err(anonymous, "setUserStatus", json!({"status": "Away"})).await, error::NOT_LOGGED_IN);
        s.ok(alice.conn, "heartbeat", json!({"scope": "Active"})).await;
        let status = s.ok(alice.conn, "setUserStatus", json!({"status": "Away"})).await;
        assert!(status["runningGames"].is_array());

        let lobby = s.lobby(&bob, "Active").await;
        let seen = lobby["recentlySeenUsers"].as_array().unwrap();
        let alice_seen = seen.iter().find(|u| u["userId"] == alice.id).unwrap_or_else(|| panic!("{seen:?}"));
        assert_eq!(alice_seen["status"], "Away");
        assert_eq!(alice_seen["name"], "Alice Player");
    });
}
