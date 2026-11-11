//! Port of `Model/UserConnections.cs`: per user, the connection last added to each game's
//! group. Only that connection is removed by `RemoveFromGroup`; a second tab's connection
//! stays in the group until its socket closes (as in C#).

use dotnet_compat::NetDictionary;
use treachery_engine::date_time_offset::DateTimeOffset;

use crate::clients::ConnectionId;
use crate::net_time::{DateTimeOffsetExt, TimeSpanExt};

#[derive(Clone, Debug, Default)]
pub struct UserConnections {
    connection_info_by_game_id: NetDictionary<String, ConnectionInfo>,
}

#[derive(Clone, Debug)]
pub struct ConnectionInfo {
    /// C# initializer `DateTimeOffset.Now`: pass the hub's clock.
    pub created: DateTimeOffset,
    pub connection_id: ConnectionId,
}

impl UserConnections {
    /// `SetConnectionId(gameId, connectionId)` (UserConnections.cs:9): a new `ConnectionInfo`,
    /// replacing any for the game. A replaced entry keeps its dictionary position (.NET
    /// `Dictionary` indexer set), which `NetDictionary::insert` reproduces.
    pub fn set_connection_id(&mut self, game_id: &str, connection_id: ConnectionId, now: DateTimeOffset) {
        self.connection_info_by_game_id.insert(game_id.to_string(), ConnectionInfo { created: now, connection_id });
    }

    /// `GetGameIdsWithOldConnections(ageInDays)` (UserConnections.cs:14).
    /// The game ids in dictionary order; the caller snapshots them (`.ToArray()`) before it
    /// removes any.
    pub fn get_game_ids_with_old_connections(&self, age_in_days: i32, now: DateTimeOffset) -> Vec<String> {
        self.connection_info_by_game_id
            .iter()
            .filter(|(_, info)| now.subtract(info.created).total_days() > f64::from(age_in_days))
            .map(|(game_id, _)| game_id.clone())
            .collect()
    }

    /// `TryGetConnectionId(gameId, out connectionId)` (UserConnections.cs:22). `None` where C#
    /// returns false (with `string.Empty` in the out parameter, which no caller reads).
    pub fn try_get_connection_id(&self, game_id: &str) -> Option<ConnectionId> {
        self.connection_info_by_game_id.get(&game_id.to_string()).map(|info| info.connection_id)
    }

    /// `RemoveConnectionId(gameId)` (UserConnections.cs:34).
    pub fn remove_connection_id(&mut self, game_id: &str) {
        self.connection_info_by_game_id.remove(&game_id.to_string());
    }
}

impl UserConnections {
    /// Every game id with its connection, in dictionary order. Read-only access for the
    /// equivalence harness (rust/server-equiv), which compares this map with C#'s.
    pub fn entries(&self) -> impl Iterator<Item = (&String, &ConnectionInfo)> {
        self.connection_info_by_game_id.iter()
    }
}
