//! Port of `Server/Data/ArchivedGame.cs`. The hub never reads or writes this table; it exists so
//! the schema matches.

use sqlx::sqlite::SqliteRow;
use sqlx::Row;

use super::Db;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ArchivedGame {
    pub id: i32,
    pub game_name: String,
    pub creator_user_id: i32,
    pub game_state: String,
    pub game_participation: String,
}

impl ArchivedGame {
    pub fn from_row(row: &SqliteRow) -> Result<ArchivedGame, sqlx::Error> {
        Ok(ArchivedGame {
            id: row.try_get("Id")?,
            game_name: row.try_get("GameName")?,
            creator_user_id: row.try_get("CreatorUserId")?,
            game_state: row.try_get("GameState")?,
            game_participation: row.try_get("GameParticipation")?,
        })
    }
}

impl Db {
    pub async fn archived_games_all(&self) -> Result<Vec<ArchivedGame>, sqlx::Error> {
        sqlx::query(r#"SELECT * FROM "ArchivedGames""#).fetch_all(&self.pool).await?.iter().map(ArchivedGame::from_row).collect()
    }
}
