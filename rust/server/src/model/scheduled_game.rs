//! Port of `Model/ScheduledGame.cs`.

use dotnet_compat::NetDictionary;
use treachery_engine::date_time_offset::DateTimeOffset;
use treachery_engine::enumerations::{Faction, Ruleset, SubscriptionType};

#[derive(Clone, Debug)]
pub struct ScheduledGame {
    pub scheduled_game_id: String,
    pub date_time: DateTimeOffset,
    pub creator_user_id: i32,
    pub creator_player_name: String,
    /// Initializer 6.
    pub number_of_players: i32,
    /// Initializer 10.
    pub maximum_turns: i32,
    pub ruleset: Option<Ruleset>,
    pub async_play: bool,
    pub allowed_factions_in_play: Vec<Faction>,
    pub subscribed_users: NetDictionary<i32, SubscriptionType>,
}

impl Default for ScheduledGame {
    fn default() -> Self {
        ScheduledGame {
            scheduled_game_id: String::new(),
            date_time: DateTimeOffset::default(),
            creator_user_id: 0,
            creator_player_name: String::new(),
            number_of_players: 6,
            maximum_turns: 10,
            ruleset: None,
            async_play: false,
            allowed_factions_in_play: Vec::new(),
            subscribed_users: NetDictionary::new(),
        }
    }
}
