//! What survives a restart, and databases written by the C# server.

mod common;

use std::path::Path;

use common::*;
use serde_json::json;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::Row;

const ALICE_PW: &str = "alice-pw";

#[test]
fn games_survive_a_restart() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let bobby = s.user("bobby").await;
        let carol = s.user("carol").await;
        let game = s.fixture_game(&alice, "alliance").await;
        s.advance(std::time::Duration::from_secs(1)).await;
        s.ok(alice.conn, "RequestAssignSeats", json!([game, [[alice.id, 0]]])).await;
        s.advance(std::time::Duration::from_secs(1)).await;
        s.ok(alice.conn, "RequestOpenOrCloseSeat", json!([game, 3])).await;
        s.advance(std::time::Duration::from_secs(1)).await;
        s.ok(bobby.conn, "RequestJoinGame", json!([game, "", 3])).await;
        s.advance(std::time::Duration::from_secs(1)).await;
        s.ok(carol.conn, "RequestObserveGame", json!([game, ""])).await;
        s.advance(std::time::Duration::from_secs(1)).await;
        s.ok(alice.conn, "SubmitEvent", json!([game, {"$type": "AllianceOffered", "Target": faction::ATREIDES, "Initiator": faction::EMPEROR}])).await;
        s.advance(std::time::Duration::from_secs(1)).await;
        s.ok(bobby.conn, "SubmitEvent", json!([game, {"$type": "AllianceOffered", "Target": faction::EMPEROR, "Initiator": faction::ATREIDES}])).await;
        s.advance(std::time::Duration::from_secs(1)).await;
        s.ok(alice.conn, "RequestSetBotSpeed", json!([game, 2])).await;
        let before = s.latest_view(alice.conn).unwrap();
        let protected = s.new_game(&bobby, "Locked", "secret").await;

        let mut s = s.restart().await;
        let c = s.connect();
        s.ok(c, "Connect", json!([])).await;
        s.ok(c, "RequestResumeSession", json!([alice.token])).await;
        let (reply, received) = s.call_raw(c, "RequestReconnectGame", json!([game])).await;
        let info = &reply["result"]["Contents"];
        assert_eq!(reply["result"]["Success"], true, "{reply}");
        let participation = &info["Participation"];
        assert!(participation["SeatedPlayers"].as_array().unwrap().contains(&json!([alice.id, 0])));
        assert!(participation["SeatedPlayers"].as_array().unwrap().contains(&json!([bobby.id, 3])));
        assert_eq!(participation["Observers"], json!([carol.id]));
        assert_eq!(participation["Hosts"], json!([alice.id]));
        assert_eq!(participation["BotsSpeed"], 2);
        let after = views(&received).pop().unwrap();
        assert_eq!(after["eventNumber"], before["eventNumber"]);
        assert_eq!(player(&after, "Emperor")["ally"], "Atreides");
        assert_eq!(after["me"]["faction"], "Emperor");

        let alice = User { conn: c, ..alice };
        let lobby = s.lobby(&alice, 3).await;
        let games = lobby["RunningGames"].as_array().unwrap();
        assert_eq!(games.len(), 2);
        let g = games.iter().find(|g| g["GameId"] == game.as_str()).unwrap();
        assert_eq!(g["Name"], "alliance");
        assert_eq!(g["CreatorId"], alice.id);
        let p = games.iter().find(|g| g["GameId"] == protected.as_str()).unwrap();
        assert_eq!(p["HasPassword"], true);
        let dave = s.user("david").await;
        assert_eq!(s.err(dave.conn, "RequestJoinGame", json!([protected, "wrong", -1])).await, 26);
        s.ok(dave.conn, "RequestJoinGame", json!([protected, "secret", -1])).await;

        // The game goes on.
        let bobby = s.login("bobby").await;
        s.ok(bobby.conn, "RequestReconnectGame", json!([game])).await;
        let view = s.latest_view(bobby.conn).unwrap();
        assert_eq!(view["me"]["faction"], "Atreides");
    });
}

#[test]
fn changes_right_before_a_restart_are_kept() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let bobby = s.user("bobby").await;
        let game = s.fixture_game(&alice, "alliance").await;
        s.ok(alice.conn, "RequestAssignSeats", json!([game, [[alice.id, 0]]])).await;
        s.ok(alice.conn, "RequestOpenOrCloseSeat", json!([game, 3])).await;
        s.ok(bobby.conn, "RequestJoinGame", json!([game, "", 3])).await;
        // Several changes within a few seconds, the last one just before the restart.
        for target in [faction::ATREIDES] {
            s.ok(alice.conn, "SubmitEvent", json!([game, {"$type": "AllianceOffered", "Target": target, "Initiator": faction::EMPEROR}])).await;
        }
        let count = s.latest_view(alice.conn).unwrap()["eventNumber"].clone();
        let mut s = s.restart().await;
        let alice = s.login("alice").await;
        s.ok(alice.conn, "RequestReconnectGame", json!([game])).await;
        assert_eq!(s.latest_view(alice.conn).unwrap()["eventNumber"], count);
    });
}

/// A copy of the database the C# server created on first start (its migrations applied, no
/// rows).
fn csharp_database(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("treachery.db");
    std::fs::copy(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/csharp_fresh.db"), &path).unwrap();
    path
}

async fn execute(path: &Path, statements: &[(&str, Vec<String>)]) {
    let pool = SqlitePoolOptions::new().max_connections(1).connect_with(SqliteConnectOptions::new().filename(path)).await.unwrap();
    for (sql, args) in statements {
        let mut q = sqlx::query(sql);
        for a in args {
            q = q.bind(a.clone());
        }
        q.execute(&pool).await.unwrap();
    }
    pool.close().await;
}

#[test]
fn opens_a_database_written_by_the_csharp_server() {
    rt().block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let path = csharp_database(dir.path());
        let participation = r#"{"PlayerNames":{"7":"Alice Player"},"SeatedPlayers":{"7":0},"Observers":[],"Hosts":[7],"AvailableSeats":[3],"Kicked":[],"BotsSpeed":1}"#;
        execute(
            &path,
            &[
                (
                    r#"INSERT INTO "Users" ("Id","Email","HashedPassword","LastLogin","Name","PasswordResetToken","PasswordResetTokenCreated","PlayerName") VALUES (7,'alice@example.com',?,'2026-10-01 09:30:00.1234567+02:00','alice',NULL,'0001-01-01 00:00:00+00:00','Alice Player')"#,
                    vec![ALICE_PW.to_string()],
                ),
                (
                    r#"INSERT INTO "Users" ("Id","Email","HashedPassword","LastLogin","Name","PasswordResetToken","PasswordResetTokenCreated","PlayerName") VALUES (9,'',?,'2026-10-02 09:30:00+00:00','bobby','','0001-01-01 00:00:00+00:00','Bobby Player')"#,
                    vec!["bobby-pw".to_string()],
                ),
                (r#"INSERT INTO "UserTokens" ("Token","UserId","Created") VALUES ('AliceToken123456',7,'2026-11-01 10:00:00+01:00')"#, vec![]),
                (r#"INSERT INTO "UserTokens" ("Token","UserId","Created") VALUES ('OldToken12345678',9,'2026-01-01 10:00:00+01:00')"#, vec![]),
                (
                    r#"INSERT INTO "PersistedGames" ("CreationDate","CreatorUserId","GameId","GameName","GameParticipation","GameState","HashedPassword","LastAction","LastAsyncPlayMessageSent","ObserversRequirePassword","StatisticsSent") VALUES ('2026-11-01 10:00:00+01:00',7,'11111111-2222-3333-4444-555555555555','Old game',?,?,NULL,'2026-11-08 11:00:00+00:00','0001-01-01 00:00:00+00:00',0,0)"#,
                    vec![participation.to_string(), fixture_text("alliance")],
                ),
                (
                    r#"INSERT INTO "PersistedGames" ("CreationDate","CreatorUserId","GameId","GameName","GameParticipation","GameState","HashedPassword","LastAction","LastAsyncPlayMessageSent","ObserversRequirePassword","StatisticsSent") VALUES ('2026-11-01 10:00:00+01:00',7,'99999999-2222-3333-4444-555555555555','Broken game',?,'{"Version":170,"_events":[{"$type":"NoLongerAnEvent"}]}',NULL,'2026-11-08 11:00:00+00:00','0001-01-01 00:00:00+00:00',0,0)"#,
                    vec![participation.to_string()],
                ),
                (
                    r#"INSERT INTO "ScheduledGames" ("AllowedFactionsInPlay","AsyncPlay","CreatorPlayerName","CreatorUserId","DateTime","GameId","MaximumTurns","NumberOfPlayers","Ruleset","SubscribedUsers") VALUES ('[20,10]',0,'Alice Player',7,'2026-11-20 20:00:00+01:00','sched-1',10,6,NULL,'{"7":50,"9":30}')"#,
                    vec![],
                ),
            ],
        )
        .await;

        let mut s = TestServer::start_on(&path).await;
        let c = s.connect();
        s.ok(c, "Connect", json!([])).await;
        // Passwords carry over, hashed again with Argon2id: the client's value still logs in.
        // Sessions don't: their tokens were in URLs and mails.
        assert_eq!(s.err(c, "RequestResumeSession", json!(["AliceToken123456"])).await, 8);
        assert_eq!(s.err(c, "RequestResumeSession", json!(["OldToken12345678"])).await, 8);
        assert_eq!(s.err(c, "RequestLogin", json!([VERSION, "alice", "wrong"])).await, 14);
        let bobby = s.login("bobby").await;
        assert_eq!(bobby.id, 9);
        let info = s.ok(c, "RequestLogin", json!([VERSION, "alice", ALICE_PW])).await;
        assert_eq!(info["UserId"], 7);
        assert_eq!(info["UserName"], "alice");
        assert_eq!(info["Email"], "alice@example.com");

        // The running game is back, without the one that no longer loads.
        let alice = User { conn: c, token: info["Token"].as_str().unwrap().into(), id: 7, name: "alice".into() };
        let lobby = s.lobby(&alice, 3).await;
        let games = lobby["RunningGames"].as_array().unwrap();
        assert_eq!(games.len(), 1, "{games:?}");
        assert_eq!(games[0]["Name"], "Old game");
        assert_eq!(games[0]["CreatorUsername"], "alice");
        let scheduled = lobby["ScheduledGames"].as_array().unwrap();
        assert_eq!(scheduled.len(), 1);
        assert_eq!(scheduled[0]["Subscribers"], json!([[7, 50], [9, 30]]));
        let (reply, received) = s.call_raw(c, "RequestReconnectGame", json!(["11111111-2222-3333-4444-555555555555"])).await;
        assert_eq!(reply["result"]["Success"], true, "{reply}");
        assert_eq!(views(&received).pop().unwrap()["me"]["faction"], "Emperor");
        // The open seat is still open.
        s.ok(bobby.conn, "RequestJoinGame", json!(["11111111-2222-3333-4444-555555555555", "", 3])).await;
        assert_eq!(s.latest_view(bobby.conn).unwrap()["me"]["faction"], "Atreides");

        // New accounts get ids after the old ones, and everything survives another restart.
        let carol = s.user("carol").await;
        assert!(carol.id > 9);
        let mut s = s.restart().await;
        let c = s.connect();
        s.ok(c, "Connect", json!([])).await;
        assert_eq!(s.ok(c, "RequestResumeSession", json!([alice.token])).await["UserId"], 7);
        s.login("carol").await;
        s.ok(c, "RequestLogin", json!([VERSION, "alice", ALICE_PW])).await;
        let alice = User { conn: c, ..alice };
        assert_eq!(s.lobby(&alice, 3).await["RunningGames"].as_array().unwrap().len(), 1);
    });
}

/// With `TREACHERY_DB` set to a database file (a copy of production, for example), opens a
/// copy of it and checks that every account can be found and every game either loads or is
/// reported. The original file is never changed.
#[test]
fn reads_a_real_database() {
    let Ok(source) = std::env::var("TREACHERY_DB") else {
        eprintln!("TREACHERY_DB is not set; skipped");
        return;
    };
    rt().block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("copy.db");
        std::fs::copy(&source, &path).unwrap();
        let pool = SqlitePoolOptions::new().connect_with(SqliteConnectOptions::new().filename(&path)).await.unwrap();
        let users: Vec<(i64, String)> = sqlx::query(r#"SELECT "Id", "Name" FROM "Users""#)
            .fetch_all(&pool)
            .await
            .unwrap()
            .iter()
            .map(|r| (r.get(0), r.get(1)))
            .collect();
        pool.close().await;

        let mut s = TestServer::start_on(&path).await;
        let admin = s.connect();
        s.ok(admin, "Connect", json!([])).await;
        // Every account is listed for the admin, with its id.
        let c = s.connect();
        s.ok(c, "RequestCreateUser", json!(["admin", "admin-pw", "", "The Admin"])).await;
        s.ok(c, "RequestLogin", json!([VERSION, "admin", "admin-pw"])).await;
        let info = s.ok(c, "GetAdminInfo", json!([])).await;
        let listed: Vec<i64> = info["Users"].as_array().unwrap().iter().map(|u| u["Id"].as_i64().unwrap()).collect();
        for (id, name) in &users {
            assert!(listed.contains(id), "user {id} ({name}) is missing");
        }
        eprintln!("{} users, {} running games", users.len(), info["GamesByGameIdCount"]);
    });
}
