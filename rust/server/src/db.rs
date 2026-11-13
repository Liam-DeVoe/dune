//! The SQLite database: accounts, sessions, running games and scheduled games.
//!
//! The server keeps everything in memory while it runs and writes changes through to the
//! database, which it reads back only at startup (and on the admin's restore). The schema's
//! version is SQLite's `user_version`; [`Db::open`] brings any older database up to date before
//! the server starts, in one transaction:
//!
//! - version 0 and empty: creates the schema.
//! - version 0 with the tables of the old C# server (Entity Framework's `__EFMigrationsHistory`
//!   among them): copies accounts, sessions, games and scheduled games into the new tables and
//!   drops the old ones. See [`migrate_from_csharp`].
//! - version 1: the current schema.
//!
//! Times are stored as RFC 3339 text in UTC ([`super::time::format`]).

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions, SqliteRow};
use sqlx::{Row, Sqlite, Transaction};

use super::time::{self, Time};

pub const SCHEMA_VERSION: i64 = 1;

const SCHEMA: &[&str] = &[
    "CREATE TABLE users (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        password_hash TEXT NOT NULL,
        email TEXT NOT NULL,
        player_name TEXT NOT NULL,
        last_login TEXT NOT NULL,
        reset_token TEXT,
        reset_token_created TEXT
    )",
    "CREATE INDEX users_name ON users (name)",
    "CREATE TABLE sessions (
        token TEXT PRIMARY KEY,
        user_id INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
        created TEXT NOT NULL
    )",
    "CREATE TABLE games (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        creator_id INTEGER NOT NULL,
        password_hash TEXT,
        created TEXT NOT NULL,
        last_activity TEXT NOT NULL,
        statistics_sent INTEGER NOT NULL,
        last_async_mail TEXT,
        participation TEXT NOT NULL,
        state TEXT NOT NULL
    )",
    "CREATE TABLE scheduled_games (
        id TEXT PRIMARY KEY,
        creator_id INTEGER NOT NULL,
        creator_player_name TEXT NOT NULL,
        starts TEXT NOT NULL,
        ruleset INTEGER,
        players INTEGER NOT NULL,
        max_turns INTEGER NOT NULL,
        factions TEXT NOT NULL,
        async_play INTEGER NOT NULL,
        subscriptions TEXT NOT NULL
    )",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserRow {
    pub id: i32,
    /// Trimmed and in lower case.
    pub name: String,
    /// The hash the client sends (protocol.md, Server and accounts).
    pub password_hash: String,
    /// Empty when the user gave none.
    pub email: String,
    pub player_name: String,
    pub last_login: Time,
    pub reset_token: Option<String>,
    pub reset_token_created: Option<Time>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionRow {
    pub token: String,
    pub user_id: i32,
    pub created: Time,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameRow {
    pub id: String,
    pub name: String,
    pub creator_id: i32,
    pub password_hash: Option<String>,
    pub created: Time,
    pub last_activity: Time,
    pub statistics_sent: bool,
    pub last_async_mail: Option<Time>,
    /// `crate::participation` JSON.
    pub participation: String,
    /// The savegame.
    pub state: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledGameRow {
    pub id: String,
    pub creator_id: i32,
    pub creator_player_name: String,
    pub starts: Time,
    pub ruleset: Option<i32>,
    pub players: i32,
    pub max_turns: i32,
    pub factions: Vec<i32>,
    pub async_play: bool,
    /// User id and subscription type, in the order they subscribed.
    pub subscriptions: Vec<(i32, i32)>,
}

#[derive(Clone)]
pub struct Db {
    pool: SqlitePool,
}

fn get_time(row: &SqliteRow, column: &str) -> Result<Time, sqlx::Error> {
    let text: String = row.try_get(column)?;
    time::parse(&text).ok_or_else(|| sqlx::Error::Decode(format!("{column}: not a time: {text:?}").into()))
}

fn get_optional_time(row: &SqliteRow, column: &str) -> Result<Option<Time>, sqlx::Error> {
    let text: Option<String> = row.try_get(column)?;
    text.map(|t| time::parse(&t).ok_or_else(|| sqlx::Error::Decode(format!("{column}: not a time: {t:?}").into()))).transpose()
}

fn json<T: serde::de::DeserializeOwned>(row: &SqliteRow, column: &str) -> Result<T, sqlx::Error> {
    let text: String = row.try_get(column)?;
    serde_json::from_str(&text).map_err(|e| sqlx::Error::Decode(format!("{column}: {e}").into()))
}

impl UserRow {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(UserRow {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            password_hash: row.try_get("password_hash")?,
            email: row.try_get("email")?,
            player_name: row.try_get("player_name")?,
            last_login: get_time(row, "last_login")?,
            reset_token: row.try_get("reset_token")?,
            reset_token_created: get_optional_time(row, "reset_token_created")?,
        })
    }
}

impl GameRow {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(GameRow {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            creator_id: row.try_get("creator_id")?,
            password_hash: row.try_get("password_hash")?,
            created: get_time(row, "created")?,
            last_activity: get_time(row, "last_activity")?,
            statistics_sent: row.try_get("statistics_sent")?,
            last_async_mail: get_optional_time(row, "last_async_mail")?,
            participation: row.try_get("participation")?,
            state: row.try_get("state")?,
        })
    }
}

impl ScheduledGameRow {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(ScheduledGameRow {
            id: row.try_get("id")?,
            creator_id: row.try_get("creator_id")?,
            creator_player_name: row.try_get("creator_player_name")?,
            starts: get_time(row, "starts")?,
            ruleset: row.try_get("ruleset")?,
            players: row.try_get("players")?,
            max_turns: row.try_get("max_turns")?,
            factions: json(row, "factions")?,
            async_play: row.try_get("async_play")?,
            subscriptions: json(row, "subscriptions")?,
        })
    }
}

#[derive(Debug)]
pub enum OpenError {
    Sqlx(sqlx::Error),
    /// A schema this server doesn't know.
    Schema(String),
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenError::Sqlx(e) => write!(f, "database error: {e}"),
            OpenError::Schema(s) => write!(f, "database schema: {s}"),
        }
    }
}

impl From<sqlx::Error> for OpenError {
    fn from(e: sqlx::Error) -> Self {
        OpenError::Sqlx(e)
    }
}

impl Db {
    /// Opens the database file, creating it if needed, and brings its schema up to date.
    pub async fn open(path: &std::path::Path) -> Result<Db, OpenError> {
        let options = SqliteConnectOptions::new().filename(path).create_if_missing(true).foreign_keys(true);
        Self::open_with(options).await
    }

    /// A database in memory, for tests.
    pub async fn open_in_memory() -> Result<Db, OpenError> {
        Self::open_with(SqliteConnectOptions::new().in_memory(true).foreign_keys(true)).await
    }

    async fn open_with(options: SqliteConnectOptions) -> Result<Db, OpenError> {
        // One connection: the server writes from one thread, and an in-memory database exists
        // only on its connection.
        let pool = SqlitePoolOptions::new().max_connections(1).connect_with(options).await?;
        let db = Db { pool };
        db.migrate().await?;
        Ok(db)
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }

    async fn migrate(&self) -> Result<(), OpenError> {
        let version: i64 = sqlx::query("PRAGMA user_version").fetch_one(&self.pool).await?.get(0);
        match version {
            SCHEMA_VERSION => return Ok(()),
            0 => {}
            v => return Err(OpenError::Schema(format!("version {v} is newer than this server's ({SCHEMA_VERSION})"))),
        }
        let tables: Vec<String> =
            sqlx::query("SELECT name FROM sqlite_master WHERE type = 'table'").fetch_all(&self.pool).await?.iter().map(|r| r.get(0)).collect();
        let mut tx = self.pool.begin().await?;
        if tables.iter().any(|t| t == "__EFMigrationsHistory") {
            migrate_from_csharp(&mut tx).await?;
        } else if tables.iter().any(|t| t != "sqlite_sequence") {
            return Err(OpenError::Schema(format!("unknown tables {tables:?}")));
        } else {
            create_schema(&mut tx).await?;
        }
        sqlx::query(&format!("PRAGMA user_version = {SCHEMA_VERSION}")).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn users(&self) -> Result<Vec<UserRow>, sqlx::Error> {
        sqlx::query("SELECT * FROM users ORDER BY id").fetch_all(&self.pool).await?.iter().map(UserRow::from_row).collect()
    }

    /// Adds a user and returns its id; `user.id` is ignored.
    pub async fn insert_user(&self, user: &UserRow) -> Result<i32, sqlx::Error> {
        let r = sqlx::query(
            "INSERT INTO users (name, password_hash, email, player_name, last_login, reset_token, reset_token_created) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&user.name)
        .bind(&user.password_hash)
        .bind(&user.email)
        .bind(&user.player_name)
        .bind(time::format(user.last_login))
        .bind(&user.reset_token)
        .bind(user.reset_token_created.map(time::format))
        .execute(&self.pool)
        .await?;
        Ok(r.last_insert_rowid() as i32)
    }

    pub async fn update_user(&self, user: &UserRow) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE users SET name = ?, password_hash = ?, email = ?, player_name = ?, last_login = ?, reset_token = ?, reset_token_created = ? WHERE id = ?",
        )
        .bind(&user.name)
        .bind(&user.password_hash)
        .bind(&user.email)
        .bind(&user.player_name)
        .bind(time::format(user.last_login))
        .bind(&user.reset_token)
        .bind(user.reset_token_created.map(time::format))
        .bind(user.id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Deletes a user and their sessions.
    pub async fn delete_user(&self, id: i32) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM users WHERE id = ?").bind(id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn sessions(&self) -> Result<Vec<SessionRow>, sqlx::Error> {
        sqlx::query("SELECT * FROM sessions")
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(|r| Ok(SessionRow { token: r.try_get("token")?, user_id: r.try_get("user_id")?, created: get_time(r, "created")? }))
            .collect()
    }

    pub async fn insert_session(&self, session: &SessionRow) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO sessions (token, user_id, created) VALUES (?, ?, ?)")
            .bind(&session.token)
            .bind(session.user_id)
            .bind(time::format(session.created))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Deletes the sessions created before `cutoff`.
    pub async fn delete_sessions_before(&self, cutoff: Time) -> Result<(), sqlx::Error> {
        // Times are stored in one format, so they compare as text.
        sqlx::query("DELETE FROM sessions WHERE created < ?").bind(time::format(cutoff)).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn games(&self) -> Result<Vec<GameRow>, sqlx::Error> {
        sqlx::query("SELECT * FROM games").fetch_all(&self.pool).await?.iter().map(GameRow::from_row).collect()
    }

    /// Inserts or replaces the game with this id.
    pub async fn save_game(&self, game: &GameRow) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT OR REPLACE INTO games (id, name, creator_id, password_hash, created, last_activity, statistics_sent, last_async_mail, participation, state)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&game.id)
        .bind(&game.name)
        .bind(game.creator_id)
        .bind(&game.password_hash)
        .bind(time::format(game.created))
        .bind(time::format(game.last_activity))
        .bind(game.statistics_sent)
        .bind(game.last_async_mail.map(time::format))
        .bind(&game.participation)
        .bind(&game.state)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_game(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM games WHERE id = ?").bind(id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn scheduled_games(&self) -> Result<Vec<ScheduledGameRow>, sqlx::Error> {
        sqlx::query("SELECT * FROM scheduled_games ORDER BY starts").fetch_all(&self.pool).await?.iter().map(ScheduledGameRow::from_row).collect()
    }

    pub async fn save_scheduled_game(&self, game: &ScheduledGameRow) -> Result<(), sqlx::Error> {
        save_scheduled_game(&self.pool, game).await
    }

    pub async fn delete_scheduled_game(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM scheduled_games WHERE id = ?").bind(id).execute(&self.pool).await?;
        Ok(())
    }
}

async fn save_scheduled_game<'e>(executor: impl sqlx::Executor<'e, Database = Sqlite>, game: &ScheduledGameRow) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR REPLACE INTO scheduled_games (id, creator_id, creator_player_name, starts, ruleset, players, max_turns, factions, async_play, subscriptions)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&game.id)
    .bind(game.creator_id)
    .bind(&game.creator_player_name)
    .bind(time::format(game.starts))
    .bind(game.ruleset)
    .bind(game.players)
    .bind(game.max_turns)
    .bind(serde_json::to_string(&game.factions).expect("numbers serialize"))
    .bind(game.async_play)
    .bind(serde_json::to_string(&game.subscriptions).expect("numbers serialize"))
    .execute(executor)
    .await?;
    Ok(())
}

async fn create_schema(tx: &mut Transaction<'_, Sqlite>) -> Result<(), sqlx::Error> {
    for statement in SCHEMA {
        sqlx::query(statement).execute(&mut **tx).await?;
    }
    Ok(())
}

/// The C# server wrote `0001-01-01 00:00:00+00:00` for "never".
fn csharp_time(text: Option<String>) -> Option<Time> {
    let t = time::parse(&text?)?;
    (t.timestamp() > 0).then_some(t)
}

/// Converts a database of the old C# server (Entity Framework's schema after its last
/// migration, `UserTokensAdded`) to the current schema. Accounts and sessions are copied as
/// they are. Games are copied with their savegame and seats; games whose savegame no longer
/// loads are copied too; the server leaves them out with a log line. The tables
/// the C# server used only for itself (`ArchivedGames`, which it never wrote, and Entity
/// Framework's bookkeeping) are dropped.
pub async fn migrate_from_csharp(tx: &mut Transaction<'_, Sqlite>) -> Result<(), OpenError> {
    let history: Vec<String> = sqlx::query(r#"SELECT "MigrationId" FROM "__EFMigrationsHistory" ORDER BY "MigrationId""#)
        .fetch_all(&mut **tx)
        .await?
        .iter()
        .map(|r| r.get(0))
        .collect();
    if history.last().map(String::as_str) != Some("20260921083521_UserTokensAdded") {
        return Err(OpenError::Schema(format!("a C# database at migration {:?}; only the last one (UserTokensAdded) is converted", history.last())));
    }
    for old in ["Users", "UserTokens", "PersistedGames", "ScheduledGames"] {
        sqlx::query(&format!(r#"ALTER TABLE "{old}" RENAME TO "csharp_{old}""#)).execute(&mut **tx).await?;
    }
    create_schema(tx).await?;

    let users = sqlx::query(r#"SELECT * FROM "csharp_Users" ORDER BY "Id""#).fetch_all(&mut **tx).await?;
    for r in &users {
        let reset_token: Option<String> = r.try_get("PasswordResetToken")?;
        sqlx::query(
            "INSERT INTO users (id, name, password_hash, email, player_name, last_login, reset_token, reset_token_created) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(r.try_get::<i64, _>("Id")?)
        .bind(r.try_get::<String, _>("Name")?)
        .bind(r.try_get::<String, _>("HashedPassword")?)
        .bind(r.try_get::<String, _>("Email")?)
        .bind(r.try_get::<String, _>("PlayerName")?)
        .bind(time::format(csharp_time(r.try_get("LastLogin")?).unwrap_or_default()))
        .bind(reset_token.filter(|t| !t.is_empty()))
        .bind(csharp_time(r.try_get("PasswordResetTokenCreated")?).map(time::format))
        .execute(&mut **tx)
        .await?;
    }

    for r in sqlx::query(r#"SELECT * FROM "csharp_UserTokens" WHERE "UserId" IN (SELECT "Id" FROM "csharp_Users")"#).fetch_all(&mut **tx).await? {
        sqlx::query("INSERT INTO sessions (token, user_id, created) VALUES (?, ?, ?)")
            .bind(r.try_get::<String, _>("Token")?)
            .bind(r.try_get::<i64, _>("UserId")?)
            .bind(time::format(csharp_time(r.try_get("Created")?).unwrap_or_default()))
            .execute(&mut **tx)
            .await?;
    }

    // A game id could appear more than once; the row written last wins.
    for r in sqlx::query(r#"SELECT * FROM "csharp_PersistedGames" WHERE "GameId" IS NOT NULL ORDER BY "Id""#).fetch_all(&mut **tx).await? {
        let created = csharp_time(r.try_get("CreationDate")?).unwrap_or_default();
        let password: Option<String> = r.try_get("HashedPassword")?;
        let game = GameRow {
            id: r.try_get("GameId")?,
            name: r.try_get::<Option<String>, _>("GameName")?.unwrap_or_default(),
            creator_id: r.try_get("CreatorUserId")?,
            password_hash: password.filter(|p| !p.is_empty()),
            created,
            last_activity: csharp_time(r.try_get("LastAction")?).unwrap_or(created),
            statistics_sent: r.try_get("StatisticsSent")?,
            last_async_mail: csharp_time(r.try_get("LastAsyncPlayMessageSent")?),
            participation: r.try_get("GameParticipation")?,
            state: r.try_get("GameState")?,
        };
        sqlx::query(
            "INSERT OR REPLACE INTO games (id, name, creator_id, password_hash, created, last_activity, statistics_sent, last_async_mail, participation, state)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&game.id)
        .bind(&game.name)
        .bind(game.creator_id)
        .bind(&game.password_hash)
        .bind(time::format(game.created))
        .bind(time::format(game.last_activity))
        .bind(game.statistics_sent)
        .bind(game.last_async_mail.map(time::format))
        .bind(&game.participation)
        .bind(&game.state)
        .execute(&mut **tx)
        .await?;
    }

    for r in sqlx::query(r#"SELECT * FROM "csharp_ScheduledGames""#).fetch_all(&mut **tx).await? {
        // `{"<userId>": <subscription>, ...}`, in subscription order.
        let text: String = r.try_get("SubscribedUsers")?;
        let subscriptions: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(&text).map_err(|e| OpenError::Schema(format!("ScheduledGames.SubscribedUsers {text:?}: {e}")))?;
        let factions: String = r.try_get("AllowedFactionsInPlay")?;
        let game = ScheduledGameRow {
            id: r.try_get("GameId")?,
            creator_id: r.try_get("CreatorUserId")?,
            creator_player_name: r.try_get("CreatorPlayerName")?,
            starts: csharp_time(r.try_get("DateTime")?).unwrap_or_default(),
            ruleset: r.try_get("Ruleset")?,
            players: r.try_get("NumberOfPlayers")?,
            max_turns: r.try_get("MaximumTurns")?,
            factions: serde_json::from_str(&factions).map_err(|e| OpenError::Schema(format!("ScheduledGames.AllowedFactionsInPlay: {e}")))?,
            async_play: r.try_get("AsyncPlay")?,
            subscriptions: subscriptions
                .iter()
                .filter_map(|(user, kind)| Some((user.parse().ok()?, kind.as_i64()? as i32)))
                .collect(),
        };
        save_scheduled_game(&mut **tx, &game).await?;
    }

    for old in ["csharp_Users", "csharp_UserTokens", "csharp_PersistedGames", "csharp_ScheduledGames", "ArchivedGames", "__EFMigrationsHistory", "__EFMigrationsLock"] {
        sqlx::query(&format!(r#"DROP TABLE IF EXISTS "{old}""#)).execute(&mut **tx).await?;
    }
    println!("converted the C# server's database: {} accounts", users.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> Time {
        time::parse(s).unwrap()
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rows_round_trip() {
        let db = Db::open_in_memory().await.unwrap();
        let mut user = UserRow {
            id: 0,
            name: "alice".into(),
            password_hash: "h".into(),
            email: "".into(),
            player_name: "Alice".into(),
            last_login: t("2026-11-08T12:00:00Z"),
            reset_token: None,
            reset_token_created: None,
        };
        user.id = db.insert_user(&user).await.unwrap();
        user.reset_token = Some("T".into());
        user.reset_token_created = Some(t("2026-11-08T12:30:00.5Z"));
        db.update_user(&user).await.unwrap();
        assert_eq!(db.users().await.unwrap(), vec![user.clone()]);

        let session = SessionRow { token: "abc".into(), user_id: user.id, created: t("2026-11-01T00:00:00Z") };
        db.insert_session(&session).await.unwrap();
        db.insert_session(&SessionRow { token: "new".into(), created: t("2026-11-07T00:00:00Z"), ..session.clone() }).await.unwrap();
        db.delete_sessions_before(t("2026-11-05T00:00:00Z")).await.unwrap();
        assert_eq!(db.sessions().await.unwrap().len(), 1);

        let game = GameRow {
            id: "g".into(),
            name: "Game".into(),
            creator_id: user.id,
            password_hash: None,
            created: t("2026-11-08T12:00:00Z"),
            last_activity: t("2026-11-08T12:00:01Z"),
            statistics_sent: false,
            last_async_mail: None,
            participation: "{}".into(),
            state: "{}".into(),
        };
        db.save_game(&game).await.unwrap();
        db.save_game(&GameRow { name: "Renamed".into(), ..game.clone() }).await.unwrap();
        assert_eq!(db.games().await.unwrap()[0].name, "Renamed");
        db.delete_game("g").await.unwrap();
        assert!(db.games().await.unwrap().is_empty());

        let scheduled = ScheduledGameRow {
            id: "s".into(),
            creator_id: user.id,
            creator_player_name: "Alice".into(),
            starts: t("2026-11-20T19:00:00Z"),
            ruleset: None,
            players: 6,
            max_turns: 10,
            factions: vec![20, 10],
            async_play: true,
            subscriptions: vec![(user.id, 50), (3, 30)],
        };
        db.save_scheduled_game(&scheduled).await.unwrap();
        assert_eq!(db.scheduled_games().await.unwrap(), vec![scheduled]);

        // Deleting a user deletes their sessions.
        db.delete_user(user.id).await.unwrap();
        assert!(db.users().await.unwrap().is_empty() && db.sessions().await.unwrap().is_empty());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn refuses_unknown_schemas() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.db");
        {
            let pool = SqlitePoolOptions::new().connect_with(SqliteConnectOptions::new().filename(&path).create_if_missing(true)).await.unwrap();
            sqlx::query("CREATE TABLE other (x)").execute(&pool).await.unwrap();
            pool.close().await;
        }
        assert!(matches!(Db::open(&path).await, Err(OpenError::Schema(_))));
        let path = dir.path().join("new.db");
        Db::open(&path).await.unwrap().close().await;
        Db::open(&path).await.unwrap().close().await;
    }
}
