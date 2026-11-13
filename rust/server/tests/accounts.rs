//! Accounts: creating users, logging in, sessions, password reset by mail, changing details.

mod common;

use common::*;
use serde_json::json;

/// `ErrorType` values (protocol.md, Result).
mod error {
    pub const USER_NOT_FOUND: i64 = 1;
    pub const USER_NAME_TOO_SHORT: i64 = 5;
    pub const USER_NAME_TOO_LONG: i64 = 6;
    pub const USER_NAME_EXISTS: i64 = 7;
    pub const PLAYER_NAME_TOO_SHORT: i64 = 10;
    pub const PLAYER_NAME_TOO_LONG: i64 = 11;
    pub const INVALID_GAME_VERSION: i64 = 13;
    pub const INVALID_USER_NAME_OR_PASSWORD: i64 = 14;
    pub const UNKNOWN_USERNAME_OR_EMAIL_ADDRESS: i64 = 16;
    pub const NO_EMAIL_ADDRESS_FOR_USER: i64 = 17;
    pub const RESET_REQUEST_TOO_SOON: i64 = 18;
    pub const UNKNOWN_USER_NAME: i64 = 19;
    pub const INVALID_RESET_TOKEN: i64 = 20;
    pub const RESET_TOKEN_EXPIRED: i64 = 21;
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
        s.ok(c, "Connect", json!([])).await;

        let create = |name: &str, player: &str| json!([name, "hash", "a@example.com", player]);
        assert_eq!(s.err(c, "RequestCreateUser", create(" abc ", "Player")).await, error::USER_NAME_TOO_SHORT);
        assert_eq!(s.err(c, "RequestCreateUser", create(&"x".repeat(41), "Player")).await, error::USER_NAME_TOO_LONG);
        assert_eq!(s.err(c, "RequestCreateUser", create("alice", " Bo ")).await, error::PLAYER_NAME_TOO_SHORT);
        assert_eq!(s.err(c, "RequestCreateUser", create("alice", &"y".repeat(41))).await, error::PLAYER_NAME_TOO_LONG);
        assert!(s.mails().is_empty());

        // The user name is stored trimmed and in lower case; the player name trimmed.
        let info = s.ok(c, "RequestCreateUser", json!(["  Alice ", "hash", " alice@example.com ", " Alice Player "])).await;
        assert_eq!(info["UserName"], "alice");
        assert_eq!(info["PlayerName"], "Alice Player");
        assert_eq!(info["Email"], "alice@example.com");
        assert!(info["UserId"].as_i64().unwrap() > 0);
        assert!(!info["Token"].as_str().unwrap().is_empty());

        let mails = s.mails();
        assert_eq!(mails.len(), 1);
        assert_eq!(mails[0].to, vec!["alice@example.com"]);
        assert_eq!(mails[0].subject, "Welcome to treachery.online");
        assert!(mails[0].body.contains("<strong>alice</strong>") && mails[0].body.contains("<strong>Alice Player</strong>"), "{}", mails[0].body);

        assert_eq!(s.err(c, "RequestCreateUser", create("ALICE", "Someone")).await, error::USER_NAME_EXISTS);

        // Without an address there is no mail.
        s.ok(c, "RequestCreateUser", json!(["bobby", "hash", null, "Bobby Player"])).await;
        s.ok(c, "RequestCreateUser", json!(["carol", "hash", "", "Carol Player"])).await;
        assert!(s.mails().is_empty());
    });
}

#[test]
fn login_and_sessions() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let c = s.connect();
        s.ok(c, "Connect", json!([])).await;

        assert_eq!(s.err(c, "RequestLogin", json!([VERSION - 1, "alice", "alice-pw"])).await, error::INVALID_GAME_VERSION);
        assert_eq!(s.err(c, "RequestLogin", json!([VERSION, "alice", "wrong"])).await, error::INVALID_USER_NAME_OR_PASSWORD);
        assert_eq!(s.err(c, "RequestLogin", json!([VERSION, "nobody", "alice-pw"])).await, error::INVALID_USER_NAME_OR_PASSWORD);

        // The name is matched without case and surrounding spaces.
        let info = s.ok(c, "RequestLogin", json!([VERSION, " ALICE ", "alice-pw"])).await;
        assert_eq!(info["UserId"], alice.id);
        assert_eq!(info["UserName"], "alice");
        assert_eq!(info["PlayerName"], "Alice Player");
        assert_eq!(info["Email"], "alice@example.com");
        let token = info["Token"].as_str().unwrap().to_string();

        // A token resumes the session, from any connection.
        let other = s.connect();
        let resumed = s.ok(other, "GetLoginInfo", json!([token])).await;
        assert_eq!(resumed["UserId"], alice.id);
        assert_eq!(resumed["Token"], token.as_str());
        assert_eq!(s.err(other, "GetLoginInfo", json!(["no such token"])).await, error::USER_NOT_FOUND);

        // Sessions survive a restart.
        let mut s = s.restart().await;
        let c = s.connect();
        s.ok(c, "Connect", json!([])).await;
        assert_eq!(s.ok(c, "GetLoginInfo", json!([token])).await["UserId"], alice.id);
        assert_eq!(s.ok(c, "GetLoginInfo", json!([alice.token])).await["UserId"], alice.id);
        s.ok(c, "RequestLogin", json!([VERSION, "alice", "alice-pw"])).await;
    });
}

#[test]
fn sessions_expire_after_90_days() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        s.advance_minutes(60 * 24 * 89).await;
        assert_eq!(s.ok(alice.conn, "GetLoginInfo", json!([alice.token])).await["UserId"], alice.id);
        s.advance_minutes(60 * 24 + 1).await;
        // Expired sessions are dropped by the housekeeping that heartbeats run.
        let bob = s.user("bobby").await;
        s.call(bob.conn, "RequestHeartbeat", json!([bob.token, 2])).await;
        assert_eq!(s.err(alice.conn, "GetLoginInfo", json!([alice.token])).await, error::USER_NOT_FOUND);
        // and stay gone after a restart.
        let mut s = s.restart().await;
        let c = s.connect();
        s.ok(c, "Connect", json!([])).await;
        assert_eq!(s.err(c, "GetLoginInfo", json!([alice.token])).await, error::USER_NOT_FOUND);
    });
}

#[test]
fn password_reset_by_mail() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        s.user("alice").await;
        let c = s.connect();
        s.ok(c, "Connect", json!([])).await;
        s.ok(c, "RequestCreateUser", json!(["nomail", "hash", "", "No Mail"])).await;

        assert_eq!(s.err(c, "RequestPasswordReset", json!(["nobody"])).await, error::UNKNOWN_USERNAME_OR_EMAIL_ADDRESS);
        assert_eq!(s.err(c, "RequestPasswordReset", json!(["nomail"])).await, error::NO_EMAIL_ADDRESS_FOR_USER);

        // By address, without case.
        s.ok(c, "RequestPasswordReset", json!([" Alice@Example.com"])).await;
        let mails = s.mails();
        assert_eq!(mails.len(), 1);
        assert_eq!(mails[0].to, vec!["alice@example.com"]);
        assert_eq!(mails[0].subject, "Password Reset");
        assert!(mails[0].body.contains("<strong>user: alice</strong>"), "{}", mails[0].body);
        let token = reset_token(&mails[0].body);

        // One request per five minutes.
        s.advance_minutes(4).await;
        assert_eq!(s.err(c, "RequestPasswordReset", json!(["alice"])).await, error::RESET_REQUEST_TOO_SOON);
        assert!(s.mails().is_empty());

        assert_eq!(s.err(c, "RequestSetPassword", json!(["nobody", token, "new-pw"])).await, error::UNKNOWN_USER_NAME);
        assert_eq!(s.err(c, "RequestSetPassword", json!(["alice", "WRONGTOKEN", "new-pw"])).await, error::INVALID_RESET_TOKEN);
        let info = s.ok(c, "RequestSetPassword", json!([" Alice", token, "new-pw"])).await;
        assert_eq!(info["UserName"], "alice");

        // The token works once, and the new password replaces the old one.
        assert_eq!(s.err(c, "RequestSetPassword", json!(["alice", token, "other-pw"])).await, error::INVALID_RESET_TOKEN);
        assert_eq!(s.err(c, "RequestLogin", json!([VERSION, "alice", "alice-pw"])).await, error::INVALID_USER_NAME_OR_PASSWORD);
        s.ok(c, "RequestLogin", json!([VERSION, "alice", "new-pw"])).await;
    });
}

#[test]
fn reset_tokens_expire_after_an_hour() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        s.user("alice").await;
        let c = s.connect();
        s.ok(c, "RequestPasswordReset", json!(["alice"])).await;
        let token = reset_token(&s.mails()[0].body);
        s.advance_minutes(61).await;
        assert_eq!(s.err(c, "RequestSetPassword", json!(["alice", token, "new-pw"])).await, error::RESET_TOKEN_EXPIRED);

        // A new request is allowed and gives a new token.
        s.ok(c, "RequestPasswordReset", json!(["alice"])).await;
        let token = reset_token(&s.mails()[0].body);
        s.advance_minutes(59).await;
        s.ok(c, "RequestSetPassword", json!(["alice", token, "new-pw"])).await;
    });
}

#[test]
fn users_change_their_details() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let c = alice.conn;

        assert_eq!(s.err(c, "RequestUpdateUserInfo", json!(["no such token", "", "Name", "a@b.c"])).await, error::USER_NOT_FOUND);

        // An empty password keeps the old one.
        let info = s.ok(c, "RequestUpdateUserInfo", json!([alice.token, "", " Lady Alice ", " lady@example.com "])).await;
        assert_eq!(info["PlayerName"], "Lady Alice");
        assert_eq!(info["Email"], "lady@example.com");
        assert_eq!(info["Token"], alice.token.as_str());
        s.ok(c, "RequestLogin", json!([VERSION, "alice", "alice-pw"])).await;
        assert_eq!(s.ok(c, "GetLoginInfo", json!([alice.token])).await["PlayerName"], "Lady Alice");

        let info = s.ok(c, "RequestUpdateUserInfo", json!([alice.token, "changed-pw", "Lady Alice", null])).await;
        assert_eq!(info["Email"], "");
        assert_eq!(s.err(c, "RequestLogin", json!([VERSION, "alice", "alice-pw"])).await, error::INVALID_USER_NAME_OR_PASSWORD);
        let info = s.ok(c, "RequestLogin", json!([VERSION, "alice", "changed-pw"])).await;
        assert_eq!(info["PlayerName"], "Lady Alice");

        // The new player name is what others see.
        let game = s.new_game(&alice, "", "").await;
        let bob = s.user("bobby").await;
        let lobby = s.lobby(&bob, 3).await;
        let entry = lobby["RunningGames"].as_array().unwrap().iter().find(|g| g["GameId"] == game.as_str()).unwrap().clone();
        assert_eq!(entry["Name"], "Lady Alice's Game");

        // and it survives a restart.
        let mut s = s.restart().await;
        let c = s.connect();
        s.ok(c, "Connect", json!([])).await;
        let info = s.ok(c, "RequestLogin", json!([VERSION, "alice", "changed-pw"])).await;
        assert_eq!(info["PlayerName"], "Lady Alice");
    });
}

#[test]
fn user_status_shows_in_the_lobby() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let bob = s.user("bobby").await;
        assert_eq!(s.err(alice.conn, "RequestSetUserStatus", json!(["no such token", 2])).await, error::USER_NOT_FOUND);
        s.ok(alice.conn, "RequestHeartbeat", json!([alice.token, 2])).await;
        let status = s.ok(alice.conn, "RequestSetUserStatus", json!([alice.token, 20])).await;
        assert!(status["RunningGames"].is_array());

        let lobby = s.lobby(&bob, 2).await;
        let seen = lobby["RecentlySeenUsers"].as_array().unwrap();
        let alice_seen = seen.iter().find(|u| u["Id"] == alice.id).unwrap_or_else(|| panic!("{seen:?}"));
        assert_eq!(alice_seen["Status"], 20);
        assert_eq!(alice_seen["Name"], "Alice Player");
    });
}
