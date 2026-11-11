//! Unit tests for package H1: `hub/authentication.rs` (GameHub_Authentication.cs). Each test
//! states the C# behaviour it pins down. The equivalence harness compares the same paths
//! against the C# hub; these run without .NET.

use std::rc::Rc;

use sqlx::Row;
use treachery_engine::date_time_offset::{DateTimeOffset, TICKS_PER_MINUTE};
use treachery_engine::enumerations::{ErrorType, UserStatus};
use treachery_engine::game::Game;
use treachery_server::clock::{Clock, ManualClock};
use treachery_server::config::Config;
use treachery_server::db::user::User;
use treachery_server::db::{date_time, Db};
use treachery_server::hub::{GameHub, HubError, SequentialIds, Services};
use treachery_server::mail::RecordingMailer;
use treachery_server::model::login_info::LoginInfo;
use treachery_server::model::result::CallResult;
use treachery_server::timers::RecordingTimers;

struct Fixture {
    hub: GameHub,
    clock: Rc<ManualClock>,
    mailer: Rc<RecordingMailer>,
}

fn start() -> DateTimeOffset {
    date_time::parse("2026-11-08 12:00:00-05:00").unwrap()
}

async fn fixture() -> Fixture {
    let clock = Rc::new(ManualClock::new(start()));
    let mailer = Rc::new(RecordingMailer::default());
    let services = Services {
        db: Db::open_in_memory().await.unwrap(),
        // SendMail only sends with a user name configured.
        config: Config::from_pairs([("GameEndEmailUsername", "mailer")]),
        clock: clock.clone(),
        timers: Rc::new(RecordingTimers::default()),
        mailer: mailer.clone(),
        ids: Rc::new(SequentialIds::default()),
    };
    Fixture { hub: GameHub::new(services), clock, mailer }
}

impl Fixture {
    fn advance_minutes(&self, minutes: i64) {
        let now = self.clock.now();
        self.clock.set(DateTimeOffset::new(now.ticks + minutes * TICKS_PER_MINUTE, now.offset_minutes));
    }

    async fn users(&self) -> Vec<User> {
        self.hub.services.db.users_all().await.unwrap()
    }

    async fn tokens(&self) -> Vec<(String, i64, String)> {
        sqlx::query(r#"SELECT "Token", "UserId", "Created" FROM "UserTokens" ORDER BY "Token""#)
            .fetch_all(&self.hub.services.db.pool)
            .await
            .unwrap()
            .iter()
            .map(|r| (r.get(0), r.get(1), r.get(2)))
            .collect()
    }

    async fn create(&mut self, name: &str, hash: &str, email: Option<&str>, player: &str) -> CallResult<LoginInfo> {
        self.hub.request_create_user(name, hash, email, player).await.unwrap()
    }
}

fn run<F: std::future::Future<Output = ()>>(f: F) {
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(f);
}

fn error_of<T>(r: &CallResult<T>) -> ErrorType {
    assert!(!r.success);
    r.error
}

#[test]
fn create_user_validates_lengths_in_utf16_units_after_trim_and_to_lower() {
    run(async {
        let mut f = fixture().await;
        assert_eq!(error_of(&f.create("  abc ", "h", None, "player").await), ErrorType::UserNameTooShort);
        // Three UTF-16 units of name plus one astral char (two units): long enough.
        let r = f.create(" ABC😀\t", "h", None, "  Pl😀 ").await;
        assert!(r.success, "{r:?}");
        let info = r.contents.unwrap();
        assert_eq!(info.user_name, "abc😀");
        assert_eq!(info.player_name, "Pl😀");
        assert_eq!(info.email, "");
        assert_eq!(error_of(&f.create(&"x".repeat(41), "h", None, "player").await), ErrorType::UserNameTooLong);
        assert!(f.create(&"x".repeat(40), "h", None, "player").await.success);
        // The name checks come first.
        assert_eq!(error_of(&f.create("ab", "h", None, "p").await), ErrorType::UserNameTooShort);
        assert_eq!(error_of(&f.create("abcd", "h", None, "pla").await), ErrorType::PlayerNameTooShort);
        assert_eq!(error_of(&f.create("abcd", "h", None, &"p".repeat(41)).await), ErrorType::PlayerNameTooLong);
        // ToLower uses the invariant culture: U+0130 stays, and no final sigma.
        let info = f.create("İSTANBUL ΟΔΟΣ", "h", None, "player").await.contents.unwrap();
        assert_eq!(info.user_name, "İstanbul οδοσ");
    });
}

#[test]
fn create_user_rejects_an_existing_name_exactly_as_stored() {
    run(async {
        let mut f = fixture().await;
        assert!(f.create("alice", "h", None, "Alice").await.success);
        assert_eq!(error_of(&f.create(" ALICE ", "h2", None, "Alice").await), ErrorType::UserNameExists);
        // SQLite `=` is binary: an existing "alice" with padding stored would not match.
        assert!(f.create("älice", "h", None, "Alice").await.success);
        assert_eq!(error_of(&f.create("ÄLICE", "h", None, "Alice").await), ErrorType::UserNameExists);
    });
}

#[test]
fn create_user_saves_user_and_token_and_caches_both() {
    run(async {
        let mut f = fixture().await;
        let info = f.create("alice", "hash", Some("  alice@example.com "), "Alice").await.contents.unwrap();
        assert_eq!(info, LoginInfo { user_id: 1, token: "TOKEN00000000001".into(), player_name: "Alice".into(), user_name: "alice".into(), email: "alice@example.com".into() });

        let users = f.users().await;
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].last_login, start());
        assert_eq!(users[0].password_reset_token, None);
        assert_eq!(f.tokens().await, vec![("TOKEN00000000001".to_string(), 1, "2026-11-08 12:00:00-05:00".to_string())]);

        // UsersById and the LoggedInUser share one object.
        let cached = f.hub.users_by_id.get(&1).unwrap().clone();
        assert!(Rc::ptr_eq(&cached, &f.hub.users_by_user_token["TOKEN00000000001"].user));

        let sent = f.mailer.sent.borrow();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].from.to_string(), "admin@treachery.online");
        assert_eq!(sent[0].to[0].to_string(), "alice@example.com");
        assert_eq!(sent[0].subject, "Welcome to treachery.online");
        assert!(sent[0].is_body_html);
        assert_eq!(
            sent[0].body,
            "<p>Welcome to treachery.online, <strong>alice</strong>.</p>\n<p>Your player name is: <strong>Alice</strong>.</p>\n<p>If you ever need to reset your password, a reset token will be sent to this e-mail address.</p>\n<p>You can edit your player name, email address and password any time after logging in.</p>"
        );
    });
}

#[test]
fn create_user_with_an_invalid_address_throws_after_saving() {
    run(async {
        let mut f = fixture().await;
        let r = f.hub.request_create_user("alice", "h", Some("not an address"), "Alice").await;
        match r {
            Err(HubError::Exception { type_name, .. }) => assert_eq!(type_name, "FormatException"),
            other => panic!("expected FormatException, got {other:?}"),
        }
        assert_eq!(f.users().await.len(), 1);
        assert_eq!(f.tokens().await.len(), 1);
        assert_eq!(f.hub.users_by_user_token.len(), 1);
        assert!(f.mailer.sent.borrow().is_empty());
        // The user exists, so logging in works.
        assert!(f.hub.request_login(Game::LATEST_VERSION, "alice", "h").await.unwrap().success);
    });
}

#[test]
fn login_matches_trimmed_lowercased_names_and_reuses_tokens() {
    run(async {
        let mut f = fixture().await;
        f.create("alice", "h", None, "Alice").await;
        assert_eq!(error_of(&f.hub.request_login(Game::LATEST_VERSION - 1, "alice", "h").await.unwrap()), ErrorType::InvalidGameVersion);
        assert_eq!(error_of(&f.hub.request_login(Game::LATEST_VERSION, "alice", "x").await.unwrap()), ErrorType::InvalidUserNameOrPassword);

        f.advance_minutes(10);
        let info = f.hub.request_login(Game::LATEST_VERSION, "\u{a0}ALICE ", "h").await.unwrap().contents.unwrap();
        // The existing token is returned; no new row.
        assert_eq!(info.token, "TOKEN00000000001");
        assert_eq!(f.tokens().await.len(), 1);
        assert_eq!(f.users().await[0].last_login, f.clock.now());
        // The login's fresh User object is not cached: the cache still has the old LastLogin.
        assert_eq!(f.hub.users_by_id[&1].borrow().last_login, start());
    });
}

#[test]
fn login_of_an_uncached_user_creates_a_token_with_a_fresh_object() {
    run(async {
        let mut f = fixture().await;
        f.create("alice", "h", None, "Alice").await;
        f.hub.users_by_user_token.clear();
        f.advance_minutes(1);
        let info = f.hub.request_login(Game::LATEST_VERSION, "alice", "h").await.unwrap().contents.unwrap();
        assert_eq!(info.token, "TOKEN00000000002");
        let logged_in = &f.hub.users_by_user_token["TOKEN00000000002"];
        assert!(!Rc::ptr_eq(&logged_in.user, &f.hub.users_by_id[&1]));
        assert_eq!(logged_in.logged_in_date_time, f.clock.now());
        assert_eq!(logged_in.status, UserStatus::Online);
        assert_eq!(f.tokens().await.len(), 2);
    });
}

#[test]
fn password_reset_and_set_password() {
    run(async {
        let mut f = fixture().await;
        f.create("alice", "h", Some("Alice@Example.com"), "Alice").await;
        f.create("bob1", "h", Some("alice@example.com "), "Bob1").await;
        f.create("carol", "h", None, "Carol").await;
        f.mailer.sent.borrow_mut().clear();

        assert_eq!(error_of(&f.hub.request_password_reset("nobody").await.unwrap()), ErrorType::UnknownUsernameOrEmailAddress);
        assert_eq!(error_of(&f.hub.request_password_reset("carol").await.unwrap()), ErrorType::NoEmailAddressForUser);

        // By e-mail: matches both users (lower(trim()) on the column, .NET Trim().ToLower() on the argument).
        assert!(f.hub.request_password_reset(" ALICE@example.COM").await.unwrap().success);
        let token = "TOKEN00000000004";
        let users = f.users().await;
        assert_eq!(users[0].password_reset_token.as_deref(), Some(token));
        assert_eq!(users[1].password_reset_token.as_deref(), Some(token));
        assert_eq!(users[2].password_reset_token, None);
        {
            let sent = f.mailer.sent.borrow();
            assert_eq!(sent.len(), 1);
            assert_eq!(sent[0].from.to_string(), "noreply@treachery.online");
            assert_eq!(sent[0].to[0].to_string(), "Alice@Example.com");
            assert_eq!(sent[0].subject, "Password Reset");
            assert_eq!(
                sent[0].body,
                format!("<p>You have requested a password reset for <strong>users: alice, bob1</strong>.</p>\n<p>You can use this token to reset your password: <strong>{token}</strong>.</p>")
            );
        }

        // Within five minutes: too soon.
        f.advance_minutes(4);
        assert_eq!(error_of(&f.hub.request_password_reset("bob1").await.unwrap()), ErrorType::ResetRequestTooSoon);

        assert_eq!(error_of(&f.hub.request_set_password("nobody", token, "new").await.unwrap()), ErrorType::UnknownUserName);
        assert_eq!(error_of(&f.hub.request_set_password("carol", token, "new").await.unwrap()), ErrorType::InvalidResetToken);
        assert_eq!(error_of(&f.hub.request_set_password("alice", " TOKEN00000000004", "new").await.unwrap()), ErrorType::InvalidResetToken);

        let info = f.hub.request_set_password(" Alice", token, "new").await.unwrap().contents.unwrap();
        assert_eq!(info.token, "TOKEN00000000001", "the token from RequestCreateUser is still logged in");
        let alice = &f.users().await[0];
        assert_eq!(alice.hashed_password, "new");
        assert_eq!(alice.password_reset_token.as_deref(), Some(""));
        assert_eq!(alice.password_reset_token_created, DateTimeOffset::default());
        assert_eq!(error_of(&f.hub.request_set_password("alice", token, "new").await.unwrap()), ErrorType::InvalidResetToken);

        // More than 60 minutes: expired.
        f.advance_minutes(55);
        assert!(f.hub.request_set_password("bob1", token, "b").await.unwrap().success);
        f.advance_minutes(10);
        assert!(f.hub.request_password_reset("carol").await.is_ok());
        assert!(f.hub.request_password_reset("bob1").await.unwrap().success);
        f.advance_minutes(61);
        assert_eq!(error_of(&f.hub.request_set_password("bob1", "TOKEN00000000005", "b").await.unwrap()), ErrorType::ResetTokenExpired);
    });
}

#[test]
fn password_reset_with_an_invalid_stored_address_throws_after_saving_the_token() {
    run(async {
        let mut f = fixture().await;
        f.create("alice", "h", None, "Alice").await;
        sqlx::query(r#"UPDATE "Users" SET "Email" = 'a@b@c'"#).execute(&f.hub.services.db.pool).await.unwrap();
        match f.hub.request_password_reset("alice").await {
            Err(HubError::Exception { type_name, .. }) => assert_eq!(type_name, "FormatException"),
            other => panic!("expected FormatException, got {other:?}"),
        }
        assert!(f.users().await[0].password_reset_token.is_some());
    });
}

#[test]
fn update_user_info_writes_the_cached_object_back() {
    run(async {
        let mut f = fixture().await;
        let token = f.create("alice", "h", None, "Alice").await.contents.unwrap().token;
        assert_eq!(error_of(&f.hub.request_update_user_info("nope", None, "x", None).await.unwrap()), ErrorType::UserNotFound);

        // A login changes LastLogin in the database, not in the cached object.
        f.advance_minutes(5);
        f.hub.request_login(Game::LATEST_VERSION, "alice", "h").await.unwrap();
        assert_eq!(f.users().await[0].last_login, f.clock.now());

        // No length checks; an empty password keeps the old one; a null e-mail becomes "".
        let info = f.hub.request_update_user_info(&token, Some(""), " A ", None).await.unwrap().contents.unwrap();
        assert_eq!((info.player_name.as_str(), info.email.as_str()), ("A", ""));
        let row = &f.users().await[0];
        assert_eq!(row.hashed_password, "h");
        assert_eq!(row.player_name, "A");
        // Every column is written from the cached object, so LastLogin goes back.
        assert_eq!(row.last_login, start());

        f.hub.request_update_user_info(&token, Some("h2"), "Alice", Some(" a@b ")).await.unwrap();
        let row = &f.users().await[0];
        assert_eq!((row.hashed_password.as_str(), row.email.as_str()), ("h2", "a@b"));
        // No mail, even for an invalid address.
        f.hub.request_update_user_info(&token, None, "Alice", Some("not valid")).await.unwrap();
        assert!(f.mailer.sent.borrow().is_empty());
        assert_eq!(f.hub.users_by_id[&1].borrow().email, "not valid");
    });
}

#[test]
fn update_user_info_of_a_deleted_user_throws() {
    run(async {
        let mut f = fixture().await;
        let token = f.create("alice", "h", None, "Alice").await.contents.unwrap().token;
        sqlx::query(r#"DELETE FROM "Users""#).execute(&f.hub.services.db.pool).await.unwrap();
        match f.hub.request_update_user_info(&token, None, "Alice2", None).await {
            Err(HubError::Exception { type_name, .. }) => assert_eq!(type_name, "DbUpdateConcurrencyException"),
            other => panic!("expected DbUpdateConcurrencyException, got {other:?}"),
        }
        // The cached object was changed before the throw.
        assert_eq!(f.hub.users_by_user_token[&token].player_name(), "Alice2");
    });
}

#[test]
fn set_user_status_and_get_login_info_reject_unknown_tokens() {
    run(async {
        let mut f = fixture().await;
        assert_eq!(error_of(&f.hub.request_set_user_status("nope", UserStatus::Away).await.unwrap()), ErrorType::UserNotFound);
    });
}

#[test]
fn reset_time_limits_are_strict_comparisons() {
    run(async {
        let mut f = fixture().await;
        f.create("alice", "h", Some("a@b"), "Alice").await;
        assert!(f.hub.request_password_reset("alice").await.unwrap().success);
        // `TotalMinutes < 5`: exactly five minutes later is allowed again.
        f.advance_minutes(5);
        assert!(f.hub.request_password_reset("alice").await.unwrap().success);
        // `TotalMinutes > 60`: exactly sixty minutes is still valid, one tick more is not.
        f.advance_minutes(60);
        let now = f.clock.now();
        f.clock.set(DateTimeOffset::new(now.ticks + 1, now.offset_minutes));
        assert_eq!(error_of(&f.hub.request_set_password("alice", "TOKEN00000000003", "x").await.unwrap()), ErrorType::ResetTokenExpired);
        f.clock.set(now);
        assert!(f.hub.request_set_password("alice", "TOKEN00000000003", "x").await.unwrap().success);
    });
}
