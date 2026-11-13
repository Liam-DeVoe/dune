//! The objects the server sends (protocol.md, Payload objects). Member names are PascalCase,
//! enums are numbers, maps keyed by user id are `[id, value]` pairs, and times are RFC 3339.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::time::{serde_time, Time};

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct LoginInfo {
    pub user_id: i32,
    pub token: String,
    pub player_name: String,
    pub user_name: String,
    pub email: String,
}

/// A game in the lobby.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct GameInfo {
    pub creator_id: i32,
    pub creator_username: String,
    #[serde(with = "serde_time")]
    pub creation_date: Time,
    pub game_id: String,
    pub has_password: bool,
    pub name: String,
    pub max_players: i32,
    pub max_turns: i32,
    pub main_phase: i32,
    pub phase: i32,
    pub turn: i32,
    pub nr_of_bots: i32,
    pub nr_of_players: i32,
    pub factions_in_play: Vec<i32>,
    pub ruleset: i32,
    #[serde(with = "serde_time")]
    pub last_activity: Time,
    pub seated_players: Vec<(i32, i32)>,
    pub available_seats: Vec<AvailableSeat>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct AvailableSeat {
    pub seat: i32,
    pub faction: i32,
    pub is_bot: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ScheduledGameInfo {
    pub scheduled_game_id: String,
    #[serde(with = "serde_time")]
    pub date_time: Time,
    pub creator_user_id: i32,
    pub creator_name: String,
    pub number_of_players: i32,
    pub maximum_turns: i32,
    pub ruleset: Option<i32>,
    pub async_play: bool,
    pub allowed_factions_in_play: Vec<i32>,
    pub subscribers: Vec<(i32, i32)>,
    pub subscriber_names: Vec<(i32, String)>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct SeenUser {
    pub id: i32,
    pub name: String,
    pub status: i32,
    #[serde(with = "serde_time")]
    pub last_seen: Time,
}

/// The lobby.
#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "PascalCase")]
pub struct ServerStatus {
    pub running_games: Vec<GameInfo>,
    pub own_games: Vec<GameInfo>,
    pub scheduled_games: Vec<ScheduledGameInfo>,
    pub recently_seen_users: Vec<SeenUser>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ServerInfo {
    #[serde(with = "serde_time")]
    pub scheduled_maintenance: Time,
    pub admin_name: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct AdminInfo {
    pub users: Vec<UserInfo>,
    /// Sessions.
    pub users_by_user_token_count: i32,
    /// Users with an open connection.
    pub connection_info_by_user_id_count: i32,
    /// Running games.
    pub games_by_game_id_count: i32,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct UserInfo {
    pub id: i32,
    pub username: String,
    pub email: String,
    #[serde(with = "serde_time")]
    pub last_login: Time,
    pub player_name: String,
}

/// The reply to creating, joining, observing or reconnecting to a game, and the `LoadGame`
/// notification.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct GameInitInfo {
    pub game_id: String,
    pub game_name: String,
    /// `crate::participation::to_wire`.
    pub participation: Value,
    pub phase_control: Option<PhaseControlInfo>,
}

/// What the current pause between phases waits for (protocol.md, Playing).
#[derive(Serialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct PhaseControlInfo {
    /// False: the host alone continues this pause.
    pub active: bool,
    /// Seats.
    pub waiting_for: Vec<i32>,
    pub countdown: bool,
    #[serde(serialize_with = "optional_time")]
    pub countdown_ends: Option<Time>,
    /// Seats.
    pub holding: Vec<i32>,
}

fn optional_time<S: serde::Serializer>(t: &Option<Time>, s: S) -> Result<S::Ok, S::Error> {
    match t {
        Some(t) => s.serialize_str(&super::time::format(*t)),
        None => s.serialize_none(),
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "PascalCase", default)]
pub struct GameChatMessage {
    /// Negative: everyone.
    pub target_user_id: i32,
    pub source_user_id: i32,
    pub body: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "PascalCase", default)]
pub struct GlobalChatMessage {
    pub source_user_id: i32,
    pub body: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodings() {
        let t = super::super::time::parse("2026-11-20T19:00:00Z").unwrap();
        let info = PhaseControlInfo { active: true, waiting_for: vec![1, 2], countdown: true, countdown_ends: Some(t), holding: vec![] };
        assert_eq!(
            serde_json::to_string(&info).unwrap(),
            r#"{"Active":true,"WaitingFor":[1,2],"Countdown":true,"CountdownEnds":"2026-11-20T19:00:00.000000Z","Holding":[]}"#
        );
        let seen = SeenUser { id: 3, name: "A".into(), status: 10, last_seen: t };
        assert_eq!(serde_json::to_string(&seen).unwrap(), r#"{"Id":3,"Name":"A","Status":10,"LastSeen":"2026-11-20T19:00:00.000000Z"}"#);
        let m: GameChatMessage = serde_json::from_str(r#"{"Body":"hi","SourceUserId":2}"#).unwrap();
        assert_eq!(m, GameChatMessage { target_user_id: 0, source_user_id: 2, body: Some("hi".into()) });
    }
}
