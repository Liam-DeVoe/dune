//! Port of `Server/Data`: the EF Core `TreacheryContext` and its entities, on sqlx/SQLite with
//! the same schema (see [`schema`]).
//!
//! Each entity module has the row struct and the queries the hub runs on it. The queries
//! mirror the SQL EF generates for the C# LINQ (capture it with `LogTo` in the harness when in
//! doubt): same filters, same SQLite functions (`trim`, `lower`), no `ORDER BY` where EF has
//! none. Value formats:
//!
//! | C# type | Column | Format |
//! |---|---|---|
//! | `DateTimeOffset` | TEXT | [`date_time`] |
//! | `bool` | INTEGER | 0 / 1 |
//! | enum, `Ruleset?` | INTEGER | the C# value, NULL for null |
//! | `List<Faction>` | TEXT | EF primitive collection: a JSON array of numbers, `[1,2]` |
//! | strings | TEXT | as is |
//!
//! `PersistedGames.GameParticipation` and `ScheduledGames.SubscribedUsers` hold JSON written
//! by `Utilities.Serialize` (see [`crate::json`]); `GameState` holds `GameState.GetStateAsString`.

pub mod archived_game;
pub mod date_time;
pub mod persisted_game;
pub mod persisted_scheduled_game;
pub mod persisted_user_token;
pub mod schema;
pub mod user;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;

/// `TreacheryContext`. Cheap to clone; every hub method uses the same pool.
#[derive(Clone)]
pub struct Db {
    pub pool: SqlitePool,
}

#[derive(Debug)]
pub enum OpenError {
    Sqlx(sqlx::Error),
    /// The database has tables but a migration history this server does not know.
    UnsupportedSchema(String),
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenError::Sqlx(e) => write!(f, "database error: {e}"),
            OpenError::UnsupportedSchema(s) => write!(f, "unsupported database schema: {s}"),
        }
    }
}

impl From<sqlx::Error> for OpenError {
    fn from(e: sqlx::Error) -> Self {
        OpenError::Sqlx(e)
    }
}

impl Db {
    /// Opens (creating if needed) the SQLite file and brings an empty database to the C#
    /// schema. `Startup.Configure` runs `Database.Migrate()` at the same point.
    pub async fn open(path: &str) -> Result<Db, OpenError> {
        let options = SqliteConnectOptions::new().filename(path).create_if_missing(true);
        let pool = SqlitePoolOptions::new().max_connections(1).connect_with(options).await?;
        let db = Db { pool };
        db.ensure_schema().await?;
        Ok(db)
    }

    /// An in-memory database with the C# schema, for tests.
    pub async fn open_in_memory() -> Result<Db, OpenError> {
        let options = SqliteConnectOptions::new().in_memory(true);
        let pool = SqlitePoolOptions::new().max_connections(1).connect_with(options).await?;
        let db = Db { pool };
        db.ensure_schema().await?;
        Ok(db)
    }

    /// See [`schema`]: create in an empty database, accept the latest history, refuse others.
    pub async fn ensure_schema(&self) -> Result<(), OpenError> {
        let tables: i64 = sqlx::query("SELECT count(*) FROM sqlite_master WHERE type = 'table'")
            .fetch_one(&self.pool)
            .await?
            .get(0);
        if tables == 0 {
            let mut tx = self.pool.begin().await?;
            for statement in schema::SCHEMA {
                sqlx::query(statement).execute(&mut *tx).await?;
            }
            for id in schema::MIGRATIONS {
                sqlx::query(r#"INSERT INTO "__EFMigrationsHistory" ("MigrationId", "ProductVersion") VALUES (?, ?)"#)
                    .bind(id)
                    .bind(schema::PRODUCT_VERSION)
                    .execute(&mut *tx)
                    .await?;
            }
            tx.commit().await?;
            return Ok(());
        }
        let history: Vec<String> = match sqlx::query(r#"SELECT "MigrationId" FROM "__EFMigrationsHistory" ORDER BY "MigrationId""#)
            .fetch_all(&self.pool)
            .await
        {
            Ok(rows) => rows.iter().map(|r| r.get(0)).collect(),
            Err(e) => return Err(OpenError::UnsupportedSchema(format!("no migration history ({e})"))),
        };
        if history.iter().map(String::as_str).ne(schema::MIGRATIONS.iter().copied()) {
            return Err(OpenError::UnsupportedSchema(format!(
                "migration history ends with {:?}, expected {:?}; open it with the C# server once to migrate",
                history.last(),
                schema::MIGRATIONS.last()
            )));
        }
        Ok(())
    }
}

/// Reads a `DateTimeOffset` column.
pub(crate) fn get_date_time(row: &sqlx::sqlite::SqliteRow, column: &str) -> Result<treachery_engine::date_time_offset::DateTimeOffset, sqlx::Error> {
    let text: String = row.try_get(column)?;
    date_time::parse(&text).ok_or_else(|| sqlx::Error::Decode(format!("column {column}: not a DateTimeOffset: {text:?}").into()))
}
