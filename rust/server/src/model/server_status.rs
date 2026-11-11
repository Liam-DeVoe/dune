//! Port of `Model/ServerStatus.cs`.

use super::game_info::GameInfo;
use super::logged_in_user_info::LoggedInUserInfo;
use super::scheduled_game_info::ScheduledGameInfo;

#[derive(Clone, Debug, Default)]
pub struct ServerStatus {
    pub running_games: Vec<GameInfo>,
    pub own_games: Vec<GameInfo>,
    pub scheduled_games: Vec<ScheduledGameInfo>,
    pub recently_seen_users: Vec<LoggedInUserInfo>,
}

super::to_json_members!(ServerStatus {
    "RunningGames" => running_games,
    "OwnGames" => own_games,
    "ScheduledGames" => scheduled_games,
    "RecentlySeenUsers" => recently_seen_users,
});
