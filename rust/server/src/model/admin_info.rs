//! Port of `Model/AdminInfo.cs`.

use super::user_info::UserInfo;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AdminInfo {
    /// C# `List<UserInfo>`, declared without an initializer (null until set).
    pub users: Option<Vec<UserInfo>>,
    pub users_by_user_token_count: i32,
    pub connection_info_by_user_id_count: i32,
    pub games_by_game_id_count: i32,
}

super::to_json_members!(AdminInfo {
    "Users" => users,
    "UsersByUserTokenCount" => users_by_user_token_count,
    "ConnectionInfoByUserIdCount" => connection_info_by_user_id_count,
    "GamesByGameIdCount" => games_by_game_id_count,
});
