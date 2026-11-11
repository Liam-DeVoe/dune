//! Port of `Model/ScheduledGameInfo.cs`.

use dotnet_compat::NetDictionary;
use treachery_engine::date_time_offset::DateTimeOffset;
use treachery_engine::enumerations::{Faction, Ruleset, SubscriptionType};
use treachery_engine::json::{JsonWriter, ToJson};

#[derive(Clone, Debug, Default)]
pub struct ScheduledGameInfo {
    /// C# `string` without an initializer.
    pub scheduled_game_id: Option<String>,
    pub date_time: DateTimeOffset,
    pub creator_user_id: i32,
    /// C# `string` without an initializer.
    pub creator_name: Option<String>,
    /// Initializer 6.
    pub number_of_players: Option<i32>,
    /// Initializer 10.
    pub maximum_turns: Option<i32>,
    pub ruleset: Option<Ruleset>,
    pub async_play: bool,
    /// C#: `AllowedFactionsInPlay = g.AllowedFactionsInPlay` shares the scheduled game's list.
    pub allowed_factions_in_play: Vec<Faction>,
    /// C#: `Subscribers = g.SubscribedUsers` shares the live dictionary
    /// (GameHub_GameManagement.cs:637). Every C# change to it forces a status update, so a
    /// snapshot is equivalent unless the harness finds otherwise.
    pub subscribers: NetDictionary<i32, SubscriptionType>,
    pub subscriber_names: NetDictionary<i32, String>,
}

impl ToJson for ScheduledGameInfo {
    fn write_json(&self, w: &mut JsonWriter) {
        w.object_value();
        w.member("ScheduledGameId", &self.scheduled_game_id);
        w.member("DateTime", &self.date_time);
        w.member("CreatorUserId", &self.creator_user_id);
        w.member("CreatorName", &self.creator_name);
        w.member("NumberOfPlayers", &self.number_of_players);
        w.member("MaximumTurns", &self.maximum_turns);
        w.member("Ruleset", &self.ruleset);
        w.member("AsyncPlay", &self.async_play);
        w.member("AllowedFactionsInPlay", &self.allowed_factions_in_play);
        w.key("Subscribers");
        crate::json::write_int_dict(w, &self.subscribers);
        w.key("SubscriberNames");
        crate::json::write_int_dict(w, &self.subscriber_names);
        w.end_object();
    }
}
