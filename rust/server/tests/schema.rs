//! The Rust server and the C# server must be able to use each other's database files.
//!
//! - `tests/golden/ef_schema.json` is `sqlite_master` of a database the C# server created
//!   (`Database.Migrate()` on an empty file). A database created by `Db::open` must have the
//!   same `sqlite_master` entries, byte for byte, and the same migration history, so the C#
//!   server finds nothing to migrate in it. Only the order of the entries may differ: in the C#
//!   database `sqlite_sequence` comes before `Users`, because a later migration rebuilt `Users`.
//! - `tests/fixtures/csharp_fresh.db` is that C# database. `Db::open` must accept it.
//! - With `TREACHERY_DB=<path>`, a copy of a real database (such as `Server/treachery.db`) is
//!   opened and every row of every table is read and decoded, including the JSON columns and
//!   the savegames. The file itself is never modified.
//! - Rows written by other tools (sqlite3, scripts), not by EF: every date column holds what
//!   SQLite's own date functions write, and the server must read it as the C# server does.

use std::path::{Path, PathBuf};

use sqlx::Row;
use treachery_server::db::{schema, Db};

fn golden() -> serde_json::Value {
    let text = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/ef_schema.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// `sqlite_master`, sorted by type and name.
async fn sqlite_master(db: &Db) -> serde_json::Value {
    let mut entries: Vec<serde_json::Value> = sqlx::query("SELECT type, name, tbl_name, sql FROM sqlite_master ORDER BY rowid")
        .fetch_all(&db.pool)
        .await
        .unwrap()
        .iter()
        .map(|r| {
            serde_json::json!({
                "type": r.get::<String, _>(0),
                "name": r.get::<String, _>(1),
                "tbl_name": r.get::<String, _>(2),
                "sql": r.get::<Option<String>, _>(3),
            })
        })
        .collect();
    entries.sort_by_key(|e| (e["type"].to_string(), e["name"].to_string()));
    serde_json::Value::Array(entries)
}

fn golden_sqlite_master() -> serde_json::Value {
    let mut entries = golden()["sqlite_master"].as_array().unwrap().clone();
    entries.sort_by_key(|e| (e["type"].to_string(), e["name"].to_string()));
    serde_json::Value::Array(entries)
}

fn copy_to_temp(source: &Path) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("copy.db");
    std::fs::copy(source, &target).unwrap();
    (dir, target)
}

#[tokio::test]
async fn new_database_has_the_csharp_schema() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new.db");
    let db = Db::open(path.to_str().unwrap()).await.unwrap();
    let golden = golden();
    assert_eq!(sqlite_master(&db).await, golden_sqlite_master());

    let history: Vec<(String, String)> = sqlx::query(r#"SELECT "MigrationId", "ProductVersion" FROM "__EFMigrationsHistory" ORDER BY rowid"#)
        .fetch_all(&db.pool)
        .await
        .unwrap()
        .iter()
        .map(|r| (r.get(0), r.get(1)))
        .collect();
    let expected: Vec<(String, String)> = golden["migrations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| (m.as_str().unwrap().to_string(), schema::PRODUCT_VERSION.to_string()))
        .collect();
    assert_eq!(history, expected);
    assert_eq!(golden["product_versions"], serde_json::json!([schema::PRODUCT_VERSION]));

    // Reopening an existing Rust database is accepted too.
    db.pool.close().await;
    Db::open(path.to_str().unwrap()).await.unwrap();

    // To check by hand that the C# server opens a database made here, point
    // SERVER_SCHEMA_OUT at a file and start the C# server on it.
    if let Some(out) = std::env::var_os("SERVER_SCHEMA_OUT") {
        std::fs::copy(&path, out).unwrap();
    }
}

#[tokio::test]
async fn opens_a_database_the_csharp_server_created() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/csharp_fresh.db");
    let (_dir, path) = copy_to_temp(&fixture);
    let db = Db::open(path.to_str().unwrap()).await.unwrap();
    assert_eq!(sqlite_master(&db).await, golden_sqlite_master());
    assert!(db.users_all().await.unwrap().is_empty());
}

#[tokio::test]
async fn refuses_an_unknown_migration_history() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.db");
    let db = Db::open(path.to_str().unwrap()).await.unwrap();
    sqlx::query(r#"DELETE FROM "__EFMigrationsHistory" WHERE "MigrationId" = ?"#)
        .bind(schema::MIGRATIONS.last().unwrap())
        .execute(&db.pool)
        .await
        .unwrap();
    db.pool.close().await;
    assert!(Db::open(path.to_str().unwrap()).await.is_err());
}

#[tokio::test]
async fn rows_round_trip() {
    use treachery_engine::date_time_offset::DateTimeOffset;
    use treachery_server::db::persisted_user_token::PersistedUserToken;
    use treachery_server::db::user::User;

    let db = Db::open_in_memory().await.unwrap();
    let user = User {
        id: 0,
        name: "alice".into(),
        email: "alice@example.com".into(),
        hashed_password: "hash".into(),
        password_reset_token: None,
        password_reset_token_created: DateTimeOffset::default(),
        player_name: "Alice".into(),
        last_login: treachery_server::db::date_time::parse("2026-09-19 19:51:27.837212-04:00").unwrap(),
    };
    let id = db.users_insert(&user).await.unwrap();
    assert_eq!(id, 1);
    assert_eq!(db.users_all().await.unwrap(), vec![User { id, ..user.clone() }]);
    let stored: String = sqlx::query(r#"SELECT "LastLogin" FROM "Users""#).fetch_one(&db.pool).await.unwrap().get(0);
    assert_eq!(stored, "2026-09-19 19:51:27.837212-04:00");

    let token = PersistedUserToken { token: "ABCDEFGHIJKLMNOP".into(), user_id: id, created: user.last_login };
    db.user_tokens_insert(&token).await.unwrap();
    assert_eq!(db.user_tokens_all().await.unwrap(), vec![token]);
}

/// Every `DateTimeOffset` column, filled by SQLite's date functions as an admin or script would
/// (`deploy/provision.sh` once wrote `datetime('now')`). EF reads text without an offset as UTC
/// (`tests/db_dates.rs` has the .NET comparison). Each value must read as the instant SQLite
/// itself sees in the text, at offset +00:00.
#[tokio::test]
async fn reads_rows_written_by_other_tools() {
    use treachery_server::clock::UNIX_EPOCH_TICKS;

    const DATES: &[&str] = &[
        "datetime('now')",
        "datetime('now', 'subsec')",
        "date('now')",
        "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        "strftime('%Y-%m-%dT%H:%M:%S', 'now')",
        "datetime('now') || '+00:00'",
        "datetime('now', '-1 day')",
    ];
    let db = Db::open_in_memory().await.unwrap();
    for (i, date) in DATES.iter().enumerate() {
        let sql = [
            format!(
                r#"INSERT INTO "Users" ("Name", "Email", "HashedPassword", "PasswordResetToken", "PasswordResetTokenCreated", "PlayerName", "LastLogin") VALUES ('admin{i}', '', 'hash', '', {date}, 'Admin', {date})"#
            ),
            format!(r#"INSERT INTO "UserTokens" ("Token", "UserId", "Created") VALUES ('token{i}', 1, {date})"#),
            format!(
                r#"INSERT INTO "PersistedGames" ("CreationDate", "CreatorUserId", "GameId", "GameParticipation", "GameState", "LastAction", "LastAsyncPlayMessageSent", "ObserversRequirePassword", "StatisticsSent") VALUES ({date}, 1, 'game{i}', '{{}}', '', {date}, {date}, 0, 0)"#
            ),
            format!(
                r#"INSERT INTO "ScheduledGames" ("AllowedFactionsInPlay", "AsyncPlay", "CreatorPlayerName", "CreatorUserId", "DateTime", "GameId", "MaximumTurns", "NumberOfPlayers", "Ruleset", "SubscribedUsers") VALUES ('[]', 0, 'Admin', 1, {date}, 'game{i}', 10, 6, NULL, '{{}}')"#
            ),
        ];
        for statement in sql {
            sqlx::query(&statement).execute(&db.pool).await.unwrap_or_else(|e| panic!("{statement}: {e}"));
        }
    }
    // What SQLite reads in each text: milliseconds since 1970 (UTC), by row.
    async fn sqlite_millis(db: &Db, table: &str, column: &str) -> Vec<i64> {
        let sql = format!(r#"SELECT CAST(round(unixepoch("{column}", 'subsec') * 1000) AS INTEGER) FROM "{table}" ORDER BY rowid"#);
        sqlx::query(&sql).fetch_all(&db.pool).await.unwrap().iter().map(|r| r.get(0)).collect()
    }
    fn check(table: &str, column: &str, read: Vec<treachery_engine::date_time_offset::DateTimeOffset>, want: Vec<i64>) {
        assert_eq!(read.len(), DATES.len(), "{table}.{column}");
        for ((v, ms), date) in read.iter().zip(want).zip(DATES) {
            assert_eq!(v.offset_minutes, 0, "{table}.{column} = {date}");
            assert_eq!((v.utc_ticks() - UNIX_EPOCH_TICKS) / 10_000, ms, "{table}.{column} = {date}");
        }
    }

    let users = db.users_all().await.unwrap();
    check("Users", "LastLogin", users.iter().map(|u| u.last_login).collect(), sqlite_millis(&db, "Users", "LastLogin").await);
    check(
        "Users",
        "PasswordResetTokenCreated",
        users.iter().map(|u| u.password_reset_token_created).collect(),
        sqlite_millis(&db, "Users", "PasswordResetTokenCreated").await,
    );
    // The login query itself, which is where the bug showed.
    assert!(db.users_first_for_login("admin0", "hash").await.unwrap().is_some());
    let tokens = db.user_tokens_all().await.unwrap();
    check("UserTokens", "Created", tokens.iter().map(|t| t.created).collect(), sqlite_millis(&db, "UserTokens", "Created").await);
    let games = db.persisted_games_all().await.unwrap();
    for (column, read) in [
        ("CreationDate", games.iter().map(|g| g.creation_date).collect::<Vec<_>>()),
        ("LastAction", games.iter().map(|g| g.last_action).collect()),
        ("LastAsyncPlayMessageSent", games.iter().map(|g| g.last_async_play_message_sent).collect()),
    ] {
        check("PersistedGames", column, read, sqlite_millis(&db, "PersistedGames", column).await);
    }
    let scheduled = db.scheduled_games_all().await.unwrap();
    check("ScheduledGames", "DateTime", scheduled.iter().map(|g| g.date_time).collect(), sqlite_millis(&db, "ScheduledGames", "DateTime").await);
}

/// Reads every row of a real database. Skipped unless `TREACHERY_DB` is set.
#[tokio::test]
async fn reads_a_real_database() {
    let Some(source) = std::env::var_os("TREACHERY_DB") else {
        return;
    };
    let (_dir, path) = copy_to_temp(Path::new(&source));
    let db = Db::open(path.to_str().unwrap()).await.unwrap();

    let users = db.users_all().await.unwrap();
    let tokens = db.user_tokens_all().await.unwrap();
    let scheduled = db.scheduled_games_all().await.unwrap();
    let archived = db.archived_games_all().await.unwrap();
    let games = db.persisted_games_all().await.unwrap();
    for g in &scheduled {
        treachery_server::json::read_int_dict::<treachery_engine::enumerations::SubscriptionType>(&serde_json::from_str(&g.subscribed_users).unwrap())
            .unwrap();
    }
    let mut old_savegames = 0;
    for g in &games {
        let participation = treachery_server::json::deserialize_participation(&g.game_participation).unwrap();
        // Writing it back gives the stored text: same member order and formatting as .NET.
        assert_eq!(treachery_server::json::serialize_participation(&participation), g.game_participation);
        // A savegame from before an event class was renamed (Blue* to BeneGesserit*, for
        // example `BluePrediction`) no longer loads, in C# either: `GameState.Load` throws
        // "JsonException: Read unrecognized type discriminator id 'BluePrediction'. Path:
        // $._events[2] | ..." and the C# server logs "Unable to restore game". Old savegames
        // need not load, so that one failure is expected. engine/tests/event_json.rs checks
        // that Rust throws .NET's message.
        match treachery_engine::cs::catch(|| treachery_engine::game_state::GameState::load(&g.game_state)) {
            Ok(state) => assert!(state.version >= 100, "game {:?}: version {}", g.game_id, state.version),
            Err(e) => {
                assert!(
                    e.type_name == "JsonException" && e.message.starts_with("Read unrecognized type discriminator id '"),
                    "game {:?}: {e}",
                    g.game_id
                );
                old_savegames += 1;
            }
        }
    }
    eprintln!(
        "read {} users, {} tokens, {} scheduled, {} archived, {} persisted games ({} from before an event rename)",
        users.len(),
        tokens.len(),
        scheduled.len(),
        archived.len(),
        games.len(),
        old_savegames
    );
}
