//! Port of `Server/Data/PersistedGame.cs` and the hub's queries on `context.PersistedGames`.

use sqlx::sqlite::SqliteRow;
use sqlx::Row;
use treachery_engine::date_time_offset::DateTimeOffset;

use super::{date_time, get_date_time, Db};

/// One `PersistedGames` row: a running game saved so it survives a restart.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PersistedGame {
    pub id: i32,
    pub game_name: Option<String>,
    pub game_id: Option<String>,
    pub creation_date: DateTimeOffset,
    pub creator_user_id: i32,
    /// `GameState.GetStateAsString(game)`.
    pub game_state: String,
    /// `Utilities.Serialize(game.Participation)`.
    pub game_participation: String,
    pub hashed_password: Option<String>,
    pub observers_require_password: bool,
    pub statistics_sent: bool,
    pub last_async_play_message_sent: DateTimeOffset,
    pub last_action: DateTimeOffset,
}

impl PersistedGame {
    pub fn from_row(row: &SqliteRow) -> Result<PersistedGame, sqlx::Error> {
        Ok(PersistedGame {
            id: row.try_get("Id")?,
            game_name: row.try_get("GameName")?,
            game_id: row.try_get("GameId")?,
            creation_date: get_date_time(row, "CreationDate")?,
            creator_user_id: row.try_get("CreatorUserId")?,
            game_state: row.try_get("GameState")?,
            game_participation: row.try_get("GameParticipation")?,
            hashed_password: row.try_get("HashedPassword")?,
            observers_require_password: row.try_get("ObserversRequirePassword")?,
            statistics_sent: row.try_get("StatisticsSent")?,
            last_async_play_message_sent: get_date_time(row, "LastAsyncPlayMessageSent")?,
            last_action: get_date_time(row, "LastAction")?,
        })
    }
}

impl Db {
    /// `context.PersistedGames.AsNoTracking()` (GameHub_Other.cs:341) and
    /// `context.PersistedGames.ToListAsync()` (GameHub_Other.cs:170).
    pub async fn persisted_games_all(&self) -> Result<Vec<PersistedGame>, sqlx::Error> {
        sqlx::query(r#"SELECT * FROM "PersistedGames""#).fetch_all(&self.pool).await?.iter().map(PersistedGame::from_row).collect()
    }

    /// `context.PersistedGames.FirstOrDefaultAsync(g => g.GameId == game.GameId)`
    /// (GameHub_Other.cs:130, :233, :255).
    /// EF: `SELECT ... FROM "PersistedGames" AS "p" WHERE "p"."GameId" = @gameId LIMIT 1`.
    pub async fn persisted_games_first_by_game_id(&self, game_id: &str) -> Result<Option<PersistedGame>, sqlx::Error> {
        sqlx::query(r#"SELECT * FROM "PersistedGames" WHERE "GameId" = ? LIMIT 1"#)
            .bind(game_id)
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(PersistedGame::from_row)
            .transpose()
    }

    /// `context.PersistedGames.Add(new PersistedGame {...}); SaveChangesAsync()` (GameHub_Other.cs:270).
    /// Returns the new `Id`; `game.id` is ignored.
    pub async fn persisted_games_insert(&self, game: &PersistedGame) -> Result<i32, sqlx::Error> {
        let result = sqlx::query(
            r#"INSERT INTO "PersistedGames" ("CreationDate", "CreatorUserId", "GameId", "GameName", "GameParticipation", "GameState", "HashedPassword", "LastAction", "LastAsyncPlayMessageSent", "ObserversRequirePassword", "StatisticsSent") VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(date_time::format(game.creation_date))
        .bind(game.creator_user_id)
        .bind(&game.game_id)
        .bind(&game.game_name)
        .bind(&game.game_participation)
        .bind(&game.game_state)
        .bind(&game.hashed_password)
        .bind(date_time::format(game.last_action))
        .bind(date_time::format(game.last_async_play_message_sent))
        .bind(game.observers_require_password)
        .bind(game.statistics_sent)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid() as i32)
    }

    /// `context.PersistedGames.Update(persistedGame); SaveChangesAsync()` (GameHub_Other.cs:266): every column.
    pub async fn persisted_games_update_all(&self, game: &PersistedGame) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE "PersistedGames" SET "CreationDate" = ?, "CreatorUserId" = ?, "GameId" = ?, "GameName" = ?, "GameParticipation" = ?, "GameState" = ?, "HashedPassword" = ?, "LastAction" = ?, "LastAsyncPlayMessageSent" = ?, "ObserversRequirePassword" = ?, "StatisticsSent" = ? WHERE "Id" = ?"#,
        )
        .bind(date_time::format(game.creation_date))
        .bind(game.creator_user_id)
        .bind(&game.game_id)
        .bind(&game.game_name)
        .bind(&game.game_participation)
        .bind(&game.game_state)
        .bind(&game.hashed_password)
        .bind(date_time::format(game.last_action))
        .bind(date_time::format(game.last_async_play_message_sent))
        .bind(game.observers_require_password)
        .bind(game.statistics_sent)
        .bind(game.id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// `context.Remove(persistedGame); SaveChangesAsync()` (GameHub_Other.cs:173, :235).
    pub async fn persisted_games_delete(&self, id: i32) -> Result<(), sqlx::Error> {
        sqlx::query(r#"DELETE FROM "PersistedGames" WHERE "Id" = ?"#).bind(id).execute(&self.pool).await?;
        Ok(())
    }
}
