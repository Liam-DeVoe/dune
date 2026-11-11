//! Port of `Server/Data/PersistedUserToken.cs` and the hub's queries on `db.UserTokens`.

use sqlx::sqlite::SqliteRow;
use sqlx::Row;
use treachery_engine::date_time_offset::DateTimeOffset;

use super::{date_time, get_date_time, Db};

/// One `UserTokens` row: a login token that survives a restart.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PersistedUserToken {
    pub token: String,
    pub user_id: i32,
    pub created: DateTimeOffset,
}

impl PersistedUserToken {
    pub fn from_row(row: &SqliteRow) -> Result<PersistedUserToken, sqlx::Error> {
        Ok(PersistedUserToken { token: row.try_get("Token")?, user_id: row.try_get("UserId")?, created: get_date_time(row, "Created")? })
    }
}

impl Db {
    /// `context.UserTokens.AsNoTracking()` (GameHub_Other.cs:421) and `db.UserTokens.AsEnumerable()`
    /// (GameHub_Other.cs:52).
    pub async fn user_tokens_all(&self) -> Result<Vec<PersistedUserToken>, sqlx::Error> {
        sqlx::query(r#"SELECT * FROM "UserTokens""#).fetch_all(&self.pool).await?.iter().map(PersistedUserToken::from_row).collect()
    }

    /// `db.UserTokens.Add(...); await db.SaveChangesAsync();` (GameHub_Authentication.cs:87).
    pub async fn user_tokens_insert(&self, token: &PersistedUserToken) -> Result<(), sqlx::Error> {
        sqlx::query(r#"INSERT INTO "UserTokens" ("Token", "UserId", "Created") VALUES (?, ?, ?)"#)
            .bind(&token.token)
            .bind(token.user_id)
            .bind(date_time::format(token.created))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// `db.UserTokens.RemoveRange(expired); await db.SaveChangesAsync();` (GameHub_Other.cs:55).
    pub async fn user_tokens_delete(&self, tokens: &[String]) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        for token in tokens {
            sqlx::query(r#"DELETE FROM "UserTokens" WHERE "Token" = ?"#).bind(token).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
