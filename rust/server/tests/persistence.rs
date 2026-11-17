//! What survives a restart.

mod common;

use std::path::Path;

use common::*;
use serde_json::{json, Value};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};


#[test]
fn games_survive_a_restart() {
    rt().block_on(async {
        let mut s = TestServer::start().await;
        let alice = s.user("alice").await;
        let bobby = s.user("bobby").await;
        let carol = s.user("carol").await;
        let game = s.fixture_game(&alice, "alliance").await;
        let (emperor, atreides) = alliance_seats();
        s.advance(std::time::Duration::from_secs(1)).await;
        s.ok(alice.conn, "assignSeats", json!({"game": game, "seats": [[alice.id, emperor]]})).await;
        s.advance(std::time::Duration::from_secs(1)).await;
        s.ok(alice.conn, "openOrCloseSeat", json!({"game": game, "seat": atreides})).await;
        s.advance(std::time::Duration::from_secs(1)).await;
        s.ok(bobby.conn, "joinGame", json!({"game": game, "passwordHash": "", "seat": atreides})).await;
        s.advance(std::time::Duration::from_secs(1)).await;
        s.ok(carol.conn, "observeGame", json!({"game": game, "passwordHash": ""})).await;
        s.advance(std::time::Duration::from_secs(1)).await;
        s.ok(alice.conn, "submitEvent", json!({"game": game, "event": offer(faction::ATREIDES)})).await;
        s.advance(std::time::Duration::from_secs(1)).await;
        s.ok(bobby.conn, "submitEvent", json!({"game": game, "event": offer(faction::EMPEROR)})).await;
        s.advance(std::time::Duration::from_secs(1)).await;
        s.ok(alice.conn, "setBotSpeed", json!({"game": game, "speed": 2})).await;
        let before = s.latest_view(alice.conn).unwrap();
        let protected = s.new_game(&bobby, "Locked", "secret").await;

        let mut s = s.restart().await;
        let c = s.connect();
        s.ok(c, "serverInfo", Value::Null).await;
        s.ok(c, "authenticate", json!({"token": alice.token})).await;
        let (reply, received) = s.call_raw(c, "reconnectGame", json!({"game": game})).await;
        let info = &reply["ok"];
        assert!(is_ok(&reply), "{reply}");
        let participation = &info["participation"];
        assert!(participation["seatedPlayers"].as_array().unwrap().contains(&json!([alice.id, emperor])));
        assert!(participation["seatedPlayers"].as_array().unwrap().contains(&json!([bobby.id, atreides])));
        assert_eq!(participation["observers"], json!([carol.id]));
        assert_eq!(participation["hosts"], json!([alice.id]));
        assert_eq!(participation["botsSpeed"], 2);
        let after = views(&received).pop().unwrap();
        assert_eq!(after["eventNumber"], before["eventNumber"]);
        assert_eq!(player(&after, "Emperor")["ally"], "Atreides");
        assert_eq!(after["me"]["faction"], "Emperor");

        let alice = User { conn: c, ..alice };
        let lobby = s.lobby(&alice, "All").await;
        let games = lobby["runningGames"].as_array().unwrap();
        assert_eq!(games.len(), 2);
        let g = games.iter().find(|g| g["gameId"] == game.as_str()).unwrap();
        assert_eq!(g["name"], "alliance");
        assert_eq!(g["creatorId"], alice.id);
        let p = games.iter().find(|g| g["gameId"] == protected.as_str()).unwrap();
        assert_eq!(p["hasPassword"], true);
        let dave = s.user("david").await;
        assert_eq!(s.err(dave.conn, "joinGame", json!({"game": protected, "passwordHash": "wrong", "seat": null})).await, "incorrectGamePassword");
        s.ok(dave.conn, "joinGame", json!({"game": protected, "passwordHash": "secret", "seat": null})).await;

        // The game goes on.
        let bobby = s.login("bobby").await;
        s.ok(bobby.conn, "reconnectGame", json!({"game": game})).await;
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
        let (emperor, atreides) = alliance_seats();
        s.ok(alice.conn, "assignSeats", json!({"game": game, "seats": [[alice.id, emperor]]})).await;
        s.ok(alice.conn, "openOrCloseSeat", json!({"game": game, "seat": atreides})).await;
        s.ok(bobby.conn, "joinGame", json!({"game": game, "passwordHash": "", "seat": atreides})).await;
        // Several changes within a few seconds, the last one just before the restart.
        for target in [faction::ATREIDES] {
            s.ok(alice.conn, "submitEvent", json!({"game": game, "event": offer(target)})).await;
        }
        let count = s.latest_view(alice.conn).unwrap()["eventNumber"].clone();
        let mut s = s.restart().await;
        let alice = s.login("alice").await;
        s.ok(alice.conn, "reconnectGame", json!({"game": game})).await;
        assert_eq!(s.latest_view(alice.conn).unwrap()["eventNumber"], count);
    });
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

/// A stored game whose savegame no longer loads, here one in the format of game version 190, is
/// left out at startup.
#[test]
fn games_that_no_longer_load_are_left_out() {
    rt().block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("treachery.db");
        let mut s = TestServer::start_on(&path).await;
        let alice = s.user("alice").await;
        let old = s.fixture_game(&alice, "alliance").await;
        let kept = s.fixture_game(&alice, "battle").await;
        s.server().shutdown().await;
        drop(s);
        execute(&path, &[("UPDATE games SET state = ? WHERE id = ?", vec![r#"{"Version":190,"_events":[]}"#.to_string(), old])]).await;

        let mut s = TestServer::start_on(&path).await;
        let alice = s.login("alice").await;
        let lobby = s.lobby(&alice, "All").await;
        let games: Vec<&str> = lobby["runningGames"].as_array().unwrap().iter().map(|g| g["gameId"].as_str().unwrap()).collect();
        assert_eq!(games, vec![kept.as_str()]);
    });
}
