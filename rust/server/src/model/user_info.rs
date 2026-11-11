//! Port of `Model/UserInfo.cs`.

use treachery_engine::date_time_offset::DateTimeOffset;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UserInfo {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub last_login: DateTimeOffset,
    pub player_name: String,
}

super::to_json_members!(UserInfo {
    "Id" => id,
    "Username" => username,
    "Email" => email,
    "LastLogin" => last_login,
    "PlayerName" => player_name,
});
