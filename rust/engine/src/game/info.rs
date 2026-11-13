use crate::prelude::*;

#[derive(Clone, Debug, Canon)]
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
    pub seated_players: NetDictionary<i32, i32>,
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
        self.seated_players.get_value_or(&user_id, -1)
    }

    pub fn you_are_in(&self, user_id: i32) -> bool {
        self.seated_players.contains_key(&user_id)
    }

    /// Takes `Option<&GameInfo>`: an object of any other type is unequal, like `None`.
    pub fn equals(&self, obj: Option<&GameInfo>) -> bool {
        matches!(obj, Some(info) if info.game_id == self.game_id)
    }

    pub fn get_hash_code(&self) -> i32 {
        // Not bit-equal to C#: .NET randomizes string hash codes per process, so no
        // port can match it. Only consistency with `equals` matters (FNV-1a over UTF-16).
        let mut h: u32 = 0x811c9dc5;
        for u in self.game_id.encode_utf16() {
            h ^= u as u32;
            h = h.wrapping_mul(0x01000193);
        }
        h as i32
    }

}
