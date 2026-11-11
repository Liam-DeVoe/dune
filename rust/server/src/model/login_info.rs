//! Port of `Model/LoginInfo.cs`.

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LoginInfo {
    pub user_id: i32,
    pub token: String,
    pub player_name: String,
    pub user_name: String,
    pub email: String,
}

super::to_json_members!(LoginInfo {
    "UserId" => user_id,
    "Token" => token,
    "PlayerName" => player_name,
    "UserName" => user_name,
    "Email" => email,
});
