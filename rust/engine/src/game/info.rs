use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct GameInfo {
    pub creator_id: i32,
    pub creator_username: String,
    pub creation_date: DateTimeOffset,
    pub game_id: String,
    pub has_password: bool,
    pub name: String,
    pub max_players: i32,
    pub max_turns: i32,
    pub main_phase: MainPhase,
    pub phase: Phase,
    pub turn: i32,
    pub nr_of_bots: i32,
    pub nr_of_players: i32,
    pub factions_in_play: Vec<Faction>,
    pub ruleset: Ruleset,
    pub last_activity: Option<DateTimeOffset>,
    pub seated_players: IndexMap<i32, i32>,
    pub available_seats: Vec<() /* C#: Treachery.Shared.AvailableSeatInfo */>,
}

impl Default for GameInfo {
    fn default() -> Self {
        GameInfo {
            creator_id: Default::default(),
            creator_username: String::new(),
            creation_date: Default::default(),
            game_id: String::new(),
            has_password: Default::default(),
            name: String::new(),
            max_players: Default::default(),
            max_turns: Default::default(),
            main_phase: Default::default(),
            phase: Default::default(),
            turn: Default::default(),
            nr_of_bots: Default::default(),
            nr_of_players: Default::default(),
            factions_in_play: Default::default(),
            ruleset: Default::default(),
            last_activity: Default::default(),
            seated_players: Default::default(),
            available_seats: Default::default(),
        }
    }
}

impl GameInfo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn can_be_joined(&self) -> bool {
        (self.phase == Phase::AwaitingPlayers && (self.seated_players.len() as i32) < self.max_players) || !self.available_seats.is_empty()
    }

    pub fn your_current_seat(&self, user_id: i32) -> i32 {
        self.seated_players.get(&user_id).copied().unwrap_or(-1)
    }

}
