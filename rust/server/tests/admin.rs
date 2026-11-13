//! The admin functions, and deleting accounts.

mod common;

use common::*;
use serde_json::{json, Value};

mod error {
    pub const USER_TOKEN_NOT_FOUND: i64 = 8;
    pub const GAME_NOT_FOUND: i64 = 2;
    pub const INVALID_USER_NAME_OR_PASSWORD: i64 = 14;
    pub const NOT_ADMIN: i64 = 15;
}

#[test]
fn only_the_admin_may() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let game = s.new_game(&alice, "", "").await;
        for (method, args) in [
            ("AdminUpdateMaintenance", json!(["2026-12-01T10:00:00+00:00"])),
            ("AdminPersistState", json!([])),
            ("AdminRestoreState", json!([])),
            ("AdminCloseGame", json!([game])),
            ("AdminDownloadGame", json!([game])),
            ("AdminCancelGame", json!(["x"])),
            ("GetAdminInfo", json!([])),
        ] {
            assert_eq!(s.err(alice.conn, method, args).await, error::INVALID_USER_NAME_OR_PASSWORD, "{method}");
        }
        let bobby = s.user("bobby").await;
        assert_eq!(s.err(alice.conn, "DeleteUser", json!([bobby.id])).await, error::NOT_ADMIN);
        let anonymous = s.connect();
        assert_eq!(s.err(anonymous, "DeleteUser", json!([bobby.id])).await, error::USER_TOKEN_NOT_FOUND);
        // The game is still there.
        s.ok(alice.conn, "RequestGameState", json!([game])).await;
    });
}

#[test]
fn admin_functions() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let admin = s.user("admin").await;
        let alice = s.user("alice").await;
        let game = s.fixture_game(&alice, "alliance").await;

        let info = s.ok(admin.conn, "GetAdminInfo", json!([])).await;
        let users = info["Users"].as_array().unwrap();
        let a = users.iter().find(|u| u["Id"] == alice.id).unwrap();
        assert_eq!(a["Username"], "alice");
        assert_eq!(a["PlayerName"], "Alice Player");
        assert_eq!(a["Email"], "alice@example.com");
        assert!(a["LastLogin"].is_string());
        assert_eq!(info["GamesByGameIdCount"], 1);

        // Maintenance shows in every client's server info.
        let message = s.ok(admin.conn, "AdminUpdateMaintenance", json!(["2026-12-01T10:00:00+00:00"])).await;
        assert!(message.is_string());
        let c = s.connect();
        let server = s.ok(c, "Connect", json!([])).await;
        assert!(server["ScheduledMaintenance"].as_str().unwrap().starts_with("2026-12-01T10:00:00"), "{server}");
        assert_eq!(server["AdminName"], "admin");

        let saved = s.ok(admin.conn, "AdminDownloadGame", json!([game])).await;
        let saved: serde_json::Value = serde_json::from_str(saved.as_str().unwrap()).unwrap();
        assert!(saved["_events"].as_array().unwrap().len() > 1);
        assert_eq!(s.err(admin.conn, "AdminDownloadGame", json!(["no such game"])).await, error::GAME_NOT_FOUND);

        assert!(s.ok(admin.conn, "AdminPersistState", json!([])).await.is_string());
        assert!(s.ok(admin.conn, "AdminRestoreState", json!([])).await.is_string());
        s.ok(alice.conn, "RequestGameState", json!([game])).await;

        s.received(alice.conn);
        assert!(s.ok(admin.conn, "AdminCloseGame", json!([game])).await.is_string());
        assert_eq!(find(&s.received(alice.conn), "RemoveUser").expect("RemoveUser")["args"], json!([alice.id, true]));
        assert_eq!(s.err(alice.conn, "RequestGameState", json!([game])).await, error::GAME_NOT_FOUND);

        let status = s.ok(alice.conn, "RequestScheduleGame", json!(["2026-11-20T20:00:00+00:00", null, 6, 10, [], false])).await;
        let id = status["ScheduledGames"][0]["ScheduledGameId"].clone();
        s.ok(admin.conn, "AdminCancelGame", json!([id])).await;
        assert!(s.lobby(&alice, 2).await["ScheduledGames"].as_array().unwrap().is_empty());

        // The admin may delete anyone.
        let bobby = s.user("bobby").await;
        s.ok(admin.conn, "DeleteUser", json!([bobby.id])).await;
        let c = s.connect();
        assert_eq!(s.err(c, "RequestLogin", json!([VERSION, "bobby", "bobby-pw"])).await, error::INVALID_USER_NAME_OR_PASSWORD);
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
        s.ok(alice.conn, "RequestJoinGame", json!([other, "", -1])).await;

        s.received(bobby.conn);
        s.ok(alice.conn, "DeleteUser", json!([alice.id])).await;
        let c = s.connect();
        assert_eq!(s.err(c, "RequestLogin", json!([VERSION, "alice", "alice-pw"])).await, error::INVALID_USER_NAME_OR_PASSWORD);
        // Their sessions end, and their connections are logged out.
        assert!(find(&s.received(alice.conn), "SessionEnded").is_some());
        assert_eq!(s.err(alice.conn, "RequestGameState", json!([other])).await, error::USER_TOKEN_NOT_FOUND);
        assert_eq!(s.err(c, "RequestResumeSession", json!([alice.token])).await, error::USER_TOKEN_NOT_FOUND);
        // Games they created are closed.
        assert_eq!(s.err(bobby.conn, "RequestObserveGame", json!([own, ""])).await, error::GAME_NOT_FOUND);
        // They leave the games they joined.
        let removed: Vec<Value> = s.received(bobby.conn).into_iter().filter(|m| m["name"] == "RemoveUser").map(|m| m["args"].clone()).collect();
        assert!(removed.contains(&json!([alice.id, false])), "{removed:?}");
        let state = s.ok(bobby.conn, "RequestGameState", json!([other])).await;
        assert!(!state.to_string().contains("Alice Player"), "{state}");
        s.received(alice.conn);
        s.ok(bobby.conn, "SendChatMessage", json!([other, {"TargetUserId": -1, "SourceUserId": bobby.id, "Body": "still there?"}])).await;
        assert!(s.received(alice.conn).is_empty(), "the deleted user's connection no longer follows the game");
        // The name is free again.
        s.ok(c, "RequestCreateUser", json!(["alice", "new-pw", "", "New Alice"])).await;

        let mut s = s.restart().await;
        let c = s.connect();
        s.ok(c, "Connect", json!([])).await;
        let info = s.ok(c, "RequestLogin", json!([VERSION, "alice", "new-pw"])).await;
        assert_eq!(info["PlayerName"], "New Alice");
        let lobby = s.lobby(&User { conn: c, token: info["Token"].as_str().unwrap().into(), id: 0, name: "alice".into() }, 3).await;
        assert_eq!(lobby["RunningGames"].as_array().unwrap().len(), 1);
    });
}

#[test]
fn reloading_keeps_running_games_as_they_are() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let admin = s.user("admin").await;
        let alice = s.user("alice").await;
        let game = s.fixture_game(&alice, "alliance").await;
        s.ok(alice.conn, "RequestAssignSeats", json!([game, [[alice.id, 0]]])).await;
        s.ok(alice.conn, "SubmitEvent", json!([game, {"$type": "AllianceOffered", "Target": faction::ATREIDES, "Initiator": faction::EMPEROR}])).await;
        let before = s.latest_view(alice.conn).unwrap()["eventNumber"].clone();

        // A move made right before the reload, not saved yet, stays.
        let message = s.ok(admin.conn, "AdminRestoreState", json!([])).await;
        assert_eq!(message, "Loaded 0 saved games that were not running; 0 no longer load. 1 games are running.");
        let (_, received) = s.call_raw(alice.conn, "RequestGameState", json!([game])).await;
        assert_eq!(views(&received).pop().unwrap()["eventNumber"], before);

        let message = s.ok(admin.conn, "AdminPersistState", json!([])).await;
        assert!(message.as_str().unwrap().ends_with("of 1 running games."), "{message}");
    });
}
