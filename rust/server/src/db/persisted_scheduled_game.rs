//! Port of `Server/Data/PersistedScheduledGame.cs` and the hub's queries on
//! `context.ScheduledGames`.

use sqlx::sqlite::SqliteRow;
use sqlx::Row;
use treachery_engine::date_time_offset::DateTimeOffset;
use treachery_engine::enumerations::{Faction, Ruleset};

use super::{date_time, get_date_time, Db};

/// One `ScheduledGames` row.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PersistedScheduledGame {
    pub id: i32,
    pub game_id: String,
    pub creator_user_id: i32,
    pub creator_player_name: String,
    pub date_time: DateTimeOffset,
    pub number_of_players: i32,
    pub maximum_turns: i32,
    pub ruleset: Option<Ruleset>,
    pub async_play: bool,
    /// EF primitive collection: the column holds a JSON array of numbers.
    pub allowed_factions_in_play: Vec<Faction>,
    /// `Utilities.Serialize(Dictionary<int, SubscriptionType>)`.
    pub subscribed_users: String,
}

impl PersistedScheduledGame {
    pub fn from_row(row: &SqliteRow) -> Result<PersistedScheduledGame, sqlx::Error> {
        let factions: String = row.try_get("AllowedFactionsInPlay")?;
        let factions: Vec<i32> =
            serde_json::from_str(&factions).map_err(|e| sqlx::Error::Decode(format!("AllowedFactionsInPlay: {e}").into()))?;
        let ruleset: Option<i32> = row.try_get("Ruleset")?;
        Ok(PersistedScheduledGame {
            id: row.try_get("Id")?,
            game_id: row.try_get("GameId")?,
            creator_user_id: row.try_get("CreatorUserId")?,
            creator_player_name: row.try_get("CreatorPlayerName")?,
            date_time: get_date_time(row, "DateTime")?,
            number_of_players: row.try_get("NumberOfPlayers")?,
            maximum_turns: row.try_get("MaximumTurns")?,
            ruleset: ruleset.map(Ruleset),
            async_play: row.try_get("AsyncPlay")?,
            allowed_factions_in_play: factions.into_iter().map(Faction).collect(),
            subscribed_users: row.try_get("SubscribedUsers")?,
        })
    }
}

impl Db {
    /// `context.ScheduledGames.AsNoTracking()` (GameHub_Other.cs:381).
    pub async fn scheduled_games_all(&self) -> Result<Vec<PersistedScheduledGame>, sqlx::Error> {
        sqlx::query(r#"SELECT * FROM "ScheduledGames""#).fetch_all(&self.pool).await?.iter().map(PersistedScheduledGame::from_row).collect()
    }

    /// `context.ScheduledGames.ExecuteDeleteAsync()` (GameHub_Other.cs:201).
    pub async fn scheduled_games_delete_all(&self) -> Result<(), sqlx::Error> {
        sqlx::query(r#"DELETE FROM "ScheduledGames""#).execute(&self.pool).await?;
        Ok(())
    }

    /// `context.ScheduledGames.Add(persisted); SaveChangesAsync()` (GameHub_Other.cs:219).
    /// Returns the new `Id`; `game.id` is ignored.
    pub async fn scheduled_games_insert(&self, game: &PersistedScheduledGame) -> Result<i32, sqlx::Error> {
        let result = sqlx::query(
            r#"INSERT INTO "ScheduledGames" ("AllowedFactionsInPlay", "AsyncPlay", "CreatorPlayerName", "CreatorUserId", "DateTime", "GameId", "MaximumTurns", "NumberOfPlayers", "Ruleset", "SubscribedUsers") VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(primitive_collection(&game.allowed_factions_in_play))
        .bind(game.async_play)
        .bind(&game.creator_player_name)
        .bind(game.creator_user_id)
        .bind(date_time::format(game.date_time))
        .bind(&game.game_id)
        .bind(game.maximum_turns)
        .bind(game.number_of_players)
        .bind(game.ruleset.map(|r| r.0))
        .bind(&game.subscribed_users)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid() as i32)
    }
}

/// An EF Core primitive collection of enums as stored: a JSON array of the numbers, `[1,2]`.
fn primitive_collection(factions: &[Faction]) -> String {
    let items: Vec<String> = factions.iter().map(|f| f.0.to_string()).collect();
    format!("[{}]", items.join(","))
}
