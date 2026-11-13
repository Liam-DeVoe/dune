use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct GameSettings {
    pub number_of_players: i32,
    pub maximum_turns: i32,
    pub initial_rules: Option<Vec<Rule>>,
    pub allowed_factions_in_play: Option<Shared<Vec<Faction>>>,
    pub auto_open_empty_seats: bool,
    pub async_play: bool,
    pub async_play_message_interval_minutes: i32,
}

impl Default for GameSettings {
    fn default() -> Self {
        GameSettings {
            number_of_players: 6,
            maximum_turns: 10,
            initial_rules: Some(Default::default()),
            allowed_factions_in_play: Some(Shared::new(Default::default())),
            auto_open_empty_seats: true,
            async_play: Default::default(),
            async_play_message_interval_minutes: Default::default(),
        }
    }
}

impl GameSettings {
    pub fn new() -> Self {
        Self::default()
    }

}
