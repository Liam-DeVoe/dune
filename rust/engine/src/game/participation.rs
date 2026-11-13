use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct Participation {
    pub player_names: IndexMap<i32, String>,
    pub seated_players: IndexMap<i32, i32>,
    pub observers: IndexSet<i32>,
    pub hosts: IndexSet<i32>,
    pub available_seats: IndexSet<i32>,
    pub kicked: IndexSet<i32>,
    pub bots_speed: i32,
}

impl Default for Participation {
    fn default() -> Self {
        Participation {
            player_names: Default::default(),
            seated_players: Default::default(),
            observers: Default::default(),
            hosts: Default::default(),
            available_seats: Default::default(),
            kicked: Default::default(),
            bots_speed: 1,
        }
    }
}

impl Participation {
    pub fn new() -> Self {
        Self::default()
    }

}

impl Game {
    pub fn is_playing(&self, faction: Faction) -> bool {
        self.players.iter().any(|p| p.faction == faction)
    }

    pub fn get_player(&self, f: Option<Faction>) -> Option<PlayerIndex> {
        self.players.iter().position(|p| Some(p.faction) == f).map(|i| self.player_at(i))
    }

    pub fn get_player_in_seat(&self, seat: i32) -> Option<PlayerIndex> {
        self.players.iter().position(|p| p.seat == seat).map(|i| self.player_at(i))
    }

    pub fn get_faction_in_seat(&self, seat: i32) -> Option<Faction> {
        self.get_player_in_seat(seat).map(|p| self.player(p).faction)
    }

    pub fn get_ally(&self, f: Option<Faction>) -> Option<Faction> {
        self.get_player(f).map(|p| self.player(p).ally)
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn players_other_than(&self, p: PlayerIndex) -> impl Iterator<Item = Faction> {
        self.players.iter().filter(move |x| x.faction != self.player(p).faction).map(move |x| x.faction)
    }

    pub fn get_user_id_of_player(&self, player: PlayerIndex) -> i32 {
        self.user_id_in_seat(self.player(player).seat)
    }

    pub fn get_player_name(&self, user_id: i32) -> String {
        self.participation.borrow().player_names.get(&user_id).cloned().unwrap_or_else(|| "?".to_string())
    }

    pub fn get_player_name_player(&self, player: PlayerIndex) -> String {
        if self.player(player).is_bot(self) {
            "Bot".to_string()
        } else {
            self.participation.borrow().player_names.get(&self.user_id_in_seat(self.player(player).seat)).cloned().unwrap_or_else(|| "?".to_string())
        }
    }

    pub fn get_player_by_user_id(&self, user_id: Option<i32>) -> Option<PlayerIndex> {
        match user_id {
            None => None,
            Some(id) => self.get_player_by_seat(self.seat_of(id)),
        }
    }

    pub fn get_player_by_name(&self, name: &str) -> Option<PlayerIndex> {
        (0..self.players.len())
            .map(|i| self.player_at(i))
            .find(|p| self.get_player_name_player(*p) == name)
    }

    pub fn get_user_id_by_name(&self, name: &str) -> i32 {
        self.participation.borrow().player_names.iter().find(|u| u.1 == name).map(|u| *u.0).unwrap_or_default()
    }

    pub fn get_player_by_seat(&self, seat_nr: i32) -> Option<PlayerIndex> {
        self.players.iter().position(|p| p.seat == seat_nr).map(|i| self.player_at(i))
    }

    pub fn user_id_in_seat(&self, seat: i32) -> i32 {
        for (key, value) in self.participation.borrow().seated_players.iter() {
            if *value == seat {
                return *key;
            }
        }

        -1
    }

    pub fn seat_of(&self, user_id: i32) -> i32 {
        self.participation.borrow().seated_players.get(&user_id).copied().unwrap_or(-1)
    }

    pub fn is_open(&self, seat: i32) -> bool {
        // (a && b) || c, as in C#
        self.current_phase == Phase::AwaitingPlayers && (self.participation.borrow().seated_players.len() as i32) < self.maximum_players()
            || self.seat_is_available(seat)
    }

    pub fn is_bot(&self, p: PlayerIndex) -> bool {
        !self.participation.borrow().seated_players.values().any(|s| *s == self.player(p).seat)
    }

    pub fn seat_is_available(&self, seat: i32) -> bool {
        self.participation.borrow().available_seats.contains(&seat)
    }

    pub fn number_of_hosts(&self) -> i32 {
        self.participation.borrow().hosts.len() as i32
    }

    pub fn number_of_bots(&self) -> i32 {
        self.player_indices().filter(|p| self.is_bot(*p)).count() as i32
    }

    pub fn number_of_observers(&self) -> i32 {
        self.participation.borrow().observers.len() as i32
    }

    pub fn number_of_seated_players(&self) -> i32 {
        self.participation.borrow().seated_players.len() as i32
    }

    pub fn player_names(&self) -> Vec<String> {
        // C# returns a lazy query over Participation; callers enumerate it right away.
        let participation = self.participation.borrow();
        participation
            .player_names
            .iter()
            .filter(|id_and_name| !participation.observers.contains(id_and_name.0))
            .map(|id_and_name| id_and_name.1.clone())
            .collect()
    }

    pub fn observer_names(&self) -> Vec<String> {
        // C# returns a lazy query over Participation; callers enumerate it right away.
        let participation = self.participation.borrow();
        participation
            .player_names
            .iter()
            .filter(|id_and_name| participation.observers.contains(id_and_name.0))
            .map(|id_and_name| id_and_name.1.clone())
            .collect()
    }

    pub fn is_player(&self, user_id: i32) -> bool {
        self.participation.borrow().seated_players.contains_key(&user_id)
    }

    pub fn is_host(&self, user_id: Option<i32>) -> bool {
        match user_id {
            Some(id) => self.participation.borrow().hosts.contains(&id),
            None => false,
        }
    }

    pub fn is_observer(&self, user_id: Option<i32>) -> bool {
        match user_id {
            Some(id) => self.participation.borrow().observers.contains(&id),
            None => false,
        }
    }

    pub fn is_participant(&self, user_id: i32) -> bool {
        self.participation.borrow().player_names.contains_key(&user_id)
    }

    pub fn was_kicked(&self, user_id: i32) -> bool {
        self.participation.borrow().kicked.contains(&user_id)
    }

    pub fn add_player(&mut self, user_id: i32, player_name: &str, seat: i32) {
        if seat >= 0 {
            let player = self.get_player_in_seat(seat);

            if let Some(player) = player {
                let faction = self.player(player).faction;
                self.log(vec![player_name.into(), " now controls ".into(), faction.into()]);
            }
        }

        self.participation.borrow_mut().seated_players.insert(user_id, seat);
        self.participation.borrow_mut().player_names.insert(user_id, player_name.to_string());
        self.participation.borrow_mut().available_seats.shift_remove(&seat);
        self.participation.borrow_mut().observers.shift_remove(&user_id);
    }

    pub fn add_observer(&mut self, user_id: i32, observer_name: &str) {
        self.log(vec![observer_name.into(), " is now observing".into()]);
        self.participation.borrow_mut().observers.insert(user_id);
        self.participation.borrow_mut().player_names.insert(user_id, observer_name.to_string());
    }

    pub fn remove_user(&mut self, user_id: i32, kick: bool) {
        let name = self.get_player_name(user_id);
        self.log(vec![name.into(), (if kick { " was kicked from" } else { " left" }).into(), " the game".into()]);

        self.participation.borrow_mut().seated_players.shift_remove(&user_id);
        self.participation.borrow_mut().observers.shift_remove(&user_id);
        self.participation.borrow_mut().player_names.shift_remove(&user_id);
        self.participation.borrow_mut().hosts.shift_remove(&user_id);

        if kick {
            self.participation.borrow_mut().kicked.insert(user_id);
        }
    }

    pub fn open_or_close_seat(&mut self, seat: i32) {
        if self.seat_is_available(seat) {
            self.participation.borrow_mut().available_seats.shift_remove(&seat);
        } else {
            self.participation.borrow_mut().available_seats.insert(seat);
        }
    }

    pub fn set_or_unset_host(&mut self, user_id: i32) {
        if self.is_host(Some(user_id)) {
            self.participation.borrow_mut().hosts.shift_remove(&user_id);
        } else {
            self.participation.borrow_mut().hosts.insert(user_id);
        }
    }

    pub fn reset_seats(&mut self) {
        // Overwriting existing keys does not invalidate the enumerator in .NET Core 3.0+.
        let keys: Vec<i32> = self.participation.borrow().seated_players.keys().copied().collect();
        for user_id in keys {
            self.participation.borrow_mut().seated_players.insert(user_id, -1);
        }
    }

}
