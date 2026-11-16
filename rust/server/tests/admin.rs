//! The admin functions, and deleting accounts.

mod common;

use common::*;
use serde_json::{json, Value};

mod error {
    pub const NOT_LOGGED_IN: &str = "notLoggedIn";
    pub const GAME_NOT_FOUND: &str = "gameNotFound";
    pub const INVALID_USER_NAME_OR_PASSWORD: &str = "invalidUserNameOrPassword";
    pub const NOT_ADMIN: &str = "notAdmin";
}

#[test]
fn only_the_admin_may() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let game = s.new_game(&alice, "", "").await;
        for (method, args) in [
            ("adminSetMaintenance", json!({"starts": "2026-12-01T10:00:00+00:00"})),
            ("adminSaveAll", Value::Null),
            ("adminReloadAll", Value::Null),
            ("adminCloseGame", json!({"game": game})),
            ("adminDownloadGame", json!({"game": game})),
            ("adminCancelScheduledGame", json!({"scheduledGame": "x"})),
            ("adminInfo", Value::Null),
        ] {
            assert_eq!(s.err(alice.conn, method, args).await, error::INVALID_USER_NAME_OR_PASSWORD, "{method}");
        }
        let bobby = s.user("bobby").await;
        assert_eq!(s.err(alice.conn, "deleteUser", json!({"user": bobby.id})).await, error::NOT_ADMIN);
        let anonymous = s.connect();
        assert_eq!(s.err(anonymous, "deleteUser", json!({"user": bobby.id})).await, error::NOT_LOGGED_IN);
        // The game is still there.
        s.ok(alice.conn, "gameState", json!({"game": game})).await;
    });
}

#[test]
fn admin_functions() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let admin = s.user("admin").await;
        let alice = s.user("alice").await;
        let game = s.fixture_game(&alice, "alliance").await;

        let info = s.ok(admin.conn, "adminInfo", Value::Null).await;
        let users = info["users"].as_array().unwrap();
        let a = users.iter().find(|u| u["userId"] == alice.id).unwrap();
        assert_eq!(a["userName"], "alice");
        assert_eq!(a["playerName"], "Alice Player");
        assert_eq!(a["email"], "alice@example.com");
        assert!(a["lastLogin"].is_string());
        assert_eq!(info["runningGames"], 1);

        // Maintenance shows in every client's server info.
        let message = s.ok(admin.conn, "adminSetMaintenance", json!({"starts": "2026-12-01T10:00:00+00:00"})).await;
        assert!(message.is_string());
        let c = s.connect();
        let server = s.ok(c, "serverInfo", Value::Null).await;
        assert!(server["scheduledMaintenance"].as_str().unwrap().starts_with("2026-12-01T10:00:00"), "{server}");
        assert_eq!(server["adminName"], "admin");

        let saved = s.ok(admin.conn, "adminDownloadGame", json!({"game": game})).await;
        let saved: serde_json::Value = serde_json::from_str(saved.as_str().unwrap()).unwrap();
        assert!(saved["_events"].as_array().unwrap().len() > 1);
        assert_eq!(s.err(admin.conn, "adminDownloadGame", json!({"game": "no such game"})).await, error::GAME_NOT_FOUND);

        assert!(s.ok(admin.conn, "adminSaveAll", Value::Null).await.is_string());
        assert!(s.ok(admin.conn, "adminReloadAll", Value::Null).await.is_string());
        s.ok(alice.conn, "gameState", json!({"game": game})).await;

        s.received(alice.conn);
        assert!(s.ok(admin.conn, "adminCloseGame", json!({"game": game})).await.is_string());
        assert_eq!(find(&s.received(alice.conn), "removeUser").expect("removeUser"), &json!({"user": alice.id, "kick": true}));
        assert_eq!(s.err(alice.conn, "gameState", json!({"game": game})).await, error::GAME_NOT_FOUND);

        let status = s.ok(alice.conn, "scheduleGame", json!({"starts": "2026-11-20T20:00:00+00:00", "ruleset": null, "players": 6, "maxTurns": 10, "factions": [], "asyncPlay": false})).await;
        let id = status["scheduledGames"][0]["scheduledGameId"].clone();
        s.ok(admin.conn, "adminCancelScheduledGame", json!({"scheduledGame": id})).await;
        assert!(s.lobby(&alice, "Active").await["scheduledGames"].as_array().unwrap().is_empty());

        // The admin may delete anyone.
        let bobby = s.user("bobby").await;
        s.ok(admin.conn, "deleteUser", json!({"user": bobby.id})).await;
        let c = s.connect();
        assert_eq!(s.err(c, "login", json!({"version": VERSION, "userName": "bobby", "passwordHash": "bobby-pw"})).await, error::INVALID_USER_NAME_OR_PASSWORD);
    });
}

#[test]
fn users_delete_themselves() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let bobby = s.user("bobby").await;
        let own = s.new_game(&alice, "", "").await;
        let other = s.new_game(&bobby, "", "").await;
        s.ok(alice.conn, "joinGame", json!({"game": other, "passwordHash": "", "seat": null})).await;

        s.received(bobby.conn);
        s.ok(alice.conn, "deleteUser", json!({"user": alice.id})).await;
        let c = s.connect();
        assert_eq!(s.err(c, "login", json!({"version": VERSION, "userName": "alice", "passwordHash": "alice-pw"})).await, error::INVALID_USER_NAME_OR_PASSWORD);
        // Their sessions end, and their connections are logged out.
        assert!(find(&s.received(alice.conn), "sessionEnded").is_some());
        assert_eq!(s.err(alice.conn, "gameState", json!({"game": other})).await, error::NOT_LOGGED_IN);
        assert_eq!(s.err(c, "authenticate", json!({"token": alice.token})).await, error::NOT_LOGGED_IN);
        // Games they created are closed.
        assert_eq!(s.err(bobby.conn, "observeGame", json!({"game": own, "passwordHash": ""})).await, error::GAME_NOT_FOUND);
        // They leave the games they joined.
        let removed: Vec<Value> = s.received(bobby.conn).into_iter().filter(|m| m["notify"] == "removeUser").map(|m| m["data"].clone()).collect();
        assert!(removed.contains(&json!({"user": alice.id, "kick": false})), "{removed:?}");
        let state = s.ok(bobby.conn, "gameState", json!({"game": other})).await;
        assert!(!state.to_string().contains("Alice Player"), "{state}");
        s.received(alice.conn);
        s.ok(bobby.conn, "chat", json!({"game": other, "to": null, "body": "still there?"})).await;
        assert!(s.received(alice.conn).is_empty(), "the deleted user's connection no longer follows the game");
        // The name is free again.
        s.ok(c, "createUser", json!({"userName": "alice", "passwordHash": "new-pw", "email": "", "playerName": "New Alice"})).await;

        let mut s = s.restart().await;
        let c = s.connect();
        s.ok(c, "serverInfo", Value::Null).await;
        let info = s.ok(c, "login", json!({"version": VERSION, "userName": "alice", "passwordHash": "new-pw"})).await;
        assert_eq!(info["account"]["playerName"], "New Alice");
        let lobby = s.lobby(&User { conn: c, token: info["token"].as_str().unwrap().into(), id: 0, name: "alice".into() }, "All").await;
        assert_eq!(lobby["runningGames"].as_array().unwrap().len(), 1);
    });
}

#[test]
fn reloading_keeps_running_games_as_they_are() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let admin = s.user("admin").await;
        let alice = s.user("alice").await;
        let game = s.fixture_game(&alice, "alliance").await;
        s.ok(alice.conn, "assignSeats", json!({"game": game, "seats": [[alice.id, alliance_seats().0]]})).await;
        s.ok(alice.conn, "submitEvent", json!({"game": game, "event": {"$type": "AllianceOffered", "Target": faction::ATREIDES, "Initiator": faction::EMPEROR}})).await;
        let before = s.latest_view(alice.conn).unwrap()["eventNumber"].clone();

        // A move made right before the reload, not saved yet, stays.
        let message = s.ok(admin.conn, "adminReloadAll", Value::Null).await;
        assert_eq!(message, "Loaded 0 saved games that were not running; 0 no longer load. 1 games are running.");
        let (_, received) = s.call_raw(alice.conn, "gameState", json!({"game": game})).await;
        assert_eq!(views(&received).pop().unwrap()["eventNumber"], before);

        let message = s.ok(admin.conn, "adminSaveAll", Value::Null).await;
        assert!(message.as_str().unwrap().ends_with("of 1 running games."), "{message}");
    });
}
