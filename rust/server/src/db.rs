//! The SQLite database: accounts, sessions, login links, running games and scheduled games.
//!
//! The server keeps everything in memory while it runs and writes changes through to the
//! database, which it reads back only at startup (and on the admin's restore). The schema's
//! version is SQLite's `user_version`. [`Db::open`] creates the schema in an empty database and
//! opens a database of [`SCHEMA_VERSION`]. It refuses anything else, so the server doesn't start
//! on a database of an older schema or one it doesn't know. There are no migrations.
//!
//! Times are stored as RFC 3339 text in UTC ([`super::time::format`]). Session, login link and
//! reset tokens are stored as their SHA-256 digests (`crate::ids::digest`).

use treachery_protocol::UserId;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions, SqliteRow};
use sqlx::{Row, Sqlite};

use super::passwords;
use super::time::{self, Time};

pub const SCHEMA_VERSION: i64 = 3;

/// The schema of version [`SCHEMA_VERSION`].
const SCHEMA: &[&str] = &[
    "CREATE TABLE users (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        password_hash TEXT NOT NULL,
        email TEXT NOT NULL,
        player_name TEXT NOT NULL,
        last_login TEXT NOT NULL,
        reset_token_digest TEXT,
        reset_token_created TEXT,
        password_scheme INTEGER NOT NULL DEFAULT 0
    )",
    "CREATE INDEX users_name ON users (name)",
    "CREATE TABLE sessions (
        token_digest TEXT PRIMARY KEY,
        user_id INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
        created TEXT NOT NULL,
        last_used TEXT NOT NULL
    )",
    "CREATE TABLE login_links (
        token_digest TEXT PRIMARY KEY,
        user_id INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
        game_id TEXT NOT NULL,
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
    pub id: UserId,
    /// Trimmed and in lower case.
    pub name: String,
    /// The Argon2id hash of what the client sends (`crate::passwords`), `password_scheme` 1.
    pub password_hash: String,
    /// Empty when the user gave none.
    pub email: String,
    pub player_name: String,
    pub last_login: Time,
    /// The digest of the password reset token mailed last.
    pub reset_token_digest: Option<String>,
    pub reset_token_created: Option<Time>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionRow {
    /// The digest of the session token.
    pub token_digest: String,
    pub user_id: UserId,
    pub created: Time,
    /// When a connection last logged in with it.
    pub last_used: Time,
}

/// A single-use login link from an async-play mail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoginLinkRow {
    /// The digest of the link's token.
    pub token_digest: String,
    pub user_id: UserId,
    /// The game the link opens.
    pub game_id: String,
    pub created: Time,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameRow {
    pub id: String,
    pub name: String,
    pub creator_id: UserId,
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
    pub creator_id: UserId,
    pub creator_player_name: String,
    pub starts: Time,
    pub ruleset: Option<i32>,
    pub players: i32,
    pub max_turns: i32,
    pub factions: Vec<i32>,
    pub async_play: bool,
    /// User id and subscription type, in the order they subscribed.
    pub subscriptions: Vec<(UserId, i32)>,
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
            id: UserId(row.try_get("id")?),
            name: row.try_get("name")?,
            password_hash: row.try_get("password_hash")?,
            email: row.try_get("email")?,
            player_name: row.try_get("player_name")?,
            last_login: get_time(row, "last_login")?,
            reset_token_digest: row.try_get("reset_token_digest")?,
            reset_token_created: get_optional_time(row, "reset_token_created")?,
        })
    }
}

impl SessionRow {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(SessionRow {
            token_digest: row.try_get("token_digest")?,
            user_id: UserId(row.try_get("user_id")?),
            created: get_time(row, "created")?,
            last_used: get_time(row, "last_used")?,
        })
    }
}

impl LoginLinkRow {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(LoginLinkRow {
            token_digest: row.try_get("token_digest")?,
            user_id: UserId(row.try_get("user_id")?),
            game_id: row.try_get("game_id")?,
            created: get_time(row, "created")?,
        })
    }
}

impl GameRow {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(GameRow {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            creator_id: UserId(row.try_get("creator_id")?),
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
            creator_id: UserId(row.try_get("creator_id")?),
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
    /// Opens the database file, creating it and its schema if needed.
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
        db.create_or_check_schema().await?;
        Ok(db)
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }

    /// Creates the schema in an empty database; refuses a database of another schema version.
    async fn create_or_check_schema(&self) -> Result<(), OpenError> {
        let version: i64 = sqlx::query("PRAGMA user_version").fetch_one(&self.pool).await?.get(0);
        if version == SCHEMA_VERSION {
            return Ok(());
        }
        let refused = |what: String| OpenError::Schema(format!("{what}; this server opens only a database of version {SCHEMA_VERSION} or an empty one"));
        if version != 0 {
            return Err(refused(format!("version {version}")));
        }
        let mut tx = self.pool.begin().await?;
        let tables: Vec<String> =
            sqlx::query("SELECT name FROM sqlite_master WHERE type = 'table'").fetch_all(&mut *tx).await?.iter().map(|r| r.get(0)).collect();
        if tables.iter().any(|t| t != "sqlite_sequence") {
            return Err(refused(format!("no version, and the tables {tables:?}")));
        }
        for statement in SCHEMA {
            sqlx::query(statement).execute(&mut *tx).await?;
        }
        sqlx::query(&format!("PRAGMA user_version = {SCHEMA_VERSION}")).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn users(&self) -> Result<Vec<UserRow>, sqlx::Error> {
        let unknown: i64 = sqlx::query("SELECT COUNT(*) FROM users WHERE password_scheme != ?").bind(passwords::SCHEME).fetch_one(&self.pool).await?.get(0);
        if unknown > 0 {
            return Err(sqlx::Error::Decode(format!("{unknown} users have a password scheme other than {}", passwords::SCHEME).into()));
        }
        sqlx::query("SELECT * FROM users ORDER BY id").fetch_all(&self.pool).await?.iter().map(UserRow::from_row).collect()
    }

    /// Adds a user and returns its id; `user.id` is ignored.
    pub async fn insert_user(&self, user: &UserRow) -> Result<UserId, sqlx::Error> {
        let r = sqlx::query(
            "INSERT INTO users (name, password_hash, password_scheme, email, player_name, last_login, reset_token_digest, reset_token_created)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&user.name)
        .bind(&user.password_hash)
        .bind(passwords::SCHEME)
        .bind(&user.email)
        .bind(&user.player_name)
        .bind(time::format(user.last_login))
        .bind(&user.reset_token_digest)
        .bind(user.reset_token_created.map(time::format))
        .execute(&self.pool)
        .await?;
        Ok(UserId(r.last_insert_rowid()))
    }

    pub async fn update_user(&self, user: &UserRow) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE users SET name = ?, password_hash = ?, password_scheme = ?, email = ?, player_name = ?, last_login = ?, reset_token_digest = ?,
             reset_token_created = ? WHERE id = ?",
        )
        .bind(&user.name)
        .bind(&user.password_hash)
        .bind(passwords::SCHEME)
        .bind(&user.email)
        .bind(&user.player_name)
        .bind(time::format(user.last_login))
        .bind(&user.reset_token_digest)
        .bind(user.reset_token_created.map(time::format))
        .bind(user.id.0)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Deletes a user, their sessions and their login links.
    pub async fn delete_user(&self, id: UserId) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM users WHERE id = ?").bind(id.0).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn sessions(&self) -> Result<Vec<SessionRow>, sqlx::Error> {
        sqlx::query("SELECT * FROM sessions").fetch_all(&self.pool).await?.iter().map(SessionRow::from_row).collect()
    }

    /// Inserts or replaces the session with this digest.
    pub async fn save_session(&self, session: &SessionRow) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT OR REPLACE INTO sessions (token_digest, user_id, created, last_used) VALUES (?, ?, ?, ?)")
            .bind(&session.token_digest)
            .bind(session.user_id.0)
            .bind(time::format(session.created))
            .bind(time::format(session.last_used))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_session(&self, token_digest: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM sessions WHERE token_digest = ?").bind(token_digest).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn login_links(&self) -> Result<Vec<LoginLinkRow>, sqlx::Error> {
        sqlx::query("SELECT * FROM login_links").fetch_all(&self.pool).await?.iter().map(LoginLinkRow::from_row).collect()
    }

    pub async fn insert_login_link(&self, link: &LoginLinkRow) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO login_links (token_digest, user_id, game_id, created) VALUES (?, ?, ?, ?)")
            .bind(&link.token_digest)
            .bind(link.user_id.0)
            .bind(&link.game_id)
            .bind(time::format(link.created))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_login_link(&self, token_digest: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM login_links WHERE token_digest = ?").bind(token_digest).execute(&self.pool).await?;
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
        .bind(game.creator_id.0)
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
    .bind(game.creator_id.0)
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
            id: UserId(0),
            name: "alice".into(),
            password_hash: "h".into(),
            email: "".into(),
            player_name: "Alice".into(),
            last_login: t("2026-11-08T12:00:00Z"),
            reset_token_digest: None,
            reset_token_created: None,
        };
        user.id = db.insert_user(&user).await.unwrap();
        user.reset_token_digest = Some("T".into());
        user.reset_token_created = Some(t("2026-11-08T12:30:00.5Z"));
        db.update_user(&user).await.unwrap();
        assert_eq!(db.users().await.unwrap(), vec![user.clone()]);

        let session = SessionRow { token_digest: "abc".into(), user_id: user.id, created: t("2026-11-01T00:00:00Z"), last_used: t("2026-11-01T00:00:00Z") };
        db.save_session(&session).await.unwrap();
        db.save_session(&SessionRow { token_digest: "new".into(), ..session.clone() }).await.unwrap();
        let used = SessionRow { last_used: t("2026-11-07T00:00:00Z"), ..session.clone() };
        db.save_session(&used).await.unwrap();
        db.delete_session("new").await.unwrap();
        assert_eq!(db.sessions().await.unwrap(), vec![used]);

        let link = LoginLinkRow { token_digest: "l".into(), user_id: user.id, game_id: "g".into(), created: t("2026-11-01T00:00:00Z") };
        db.insert_login_link(&link).await.unwrap();
        db.insert_login_link(&LoginLinkRow { token_digest: "m".into(), ..link.clone() }).await.unwrap();
        db.delete_login_link("m").await.unwrap();
        assert_eq!(db.login_links().await.unwrap(), vec![link]);

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
            subscriptions: vec![(user.id, 50), (UserId(3), 30)],
        };
        db.save_scheduled_game(&scheduled).await.unwrap();
        assert_eq!(db.scheduled_games().await.unwrap(), vec![scheduled]);

        // Deleting a user deletes their sessions and login links.
        db.delete_user(user.id).await.unwrap();
        assert!(db.users().await.unwrap().is_empty() && db.sessions().await.unwrap().is_empty() && db.login_links().await.unwrap().is_empty());
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

    /// A database of an older schema (or a newer one) is refused, not migrated.
    #[tokio::test(flavor = "current_thread")]
    async fn refuses_other_schema_versions() {
        let dir = tempfile::tempdir().unwrap();
        for version in [1, 2, SCHEMA_VERSION + 1] {
            let path = dir.path().join(format!("v{version}.db"));
            {
                let pool = SqlitePoolOptions::new().connect_with(SqliteConnectOptions::new().filename(&path).create_if_missing(true)).await.unwrap();
                sqlx::query(&format!("PRAGMA user_version = {version}")).execute(&pool).await.unwrap();
                pool.close().await;
            }
            match Db::open(&path).await {
                Err(OpenError::Schema(m)) => assert!(m.starts_with(&format!("version {version};")), "{m}"),
                Err(e) => panic!("{e}"),
                Ok(_) => panic!("version {version} opened"),
            }
        }
    }
}
