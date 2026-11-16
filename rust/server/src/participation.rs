//! A game's participation: who plays which seat, observers, hosts, open seats, kicked users and
//! the bot speed. The engine knows only which seats users play (`Game::human_seats`), so every
//! change of seats here updates the game too.
//!
//! Also its JSON in the database, with integer-keyed maps as objects (`{"7": 0}`) and -1 for a
//! user without a seat, the form older databases hold, and what clients get of it
//! (`ParticipationInfo`).

use std::collections::BTreeMap;

use serde_json::{json, Map, Value};
use treachery_engine::enumerations::Phase;
use treachery_engine::game::{Game, PlayerId, Seat};
use treachery_engine::prelude::{IndexMap, IndexSet};
use treachery_protocol::{ParticipationInfo, UserId};
use treachery_view::build::{Seating, Viewer};

#[derive(Clone, Debug)]
pub struct Participation {
    pub player_names: IndexMap<UserId, String>,
    /// Each player's seat, if they have one yet.
    pub seated_players: IndexMap<UserId, Option<Seat>>,
    pub observers: IndexSet<UserId>,
    pub hosts: IndexSet<UserId>,
    pub available_seats: IndexSet<Seat>,
    pub kicked: IndexSet<UserId>,
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

    pub fn is_player(&self, user: UserId) -> bool {
        self.seated_players.contains_key(&user)
    }

    pub fn is_host(&self, user: UserId) -> bool {
        self.hosts.contains(&user)
    }

    pub fn is_observer(&self, user: UserId) -> bool {
        self.observers.contains(&user)
    }

    pub fn is_participant(&self, user: UserId) -> bool {
        self.player_names.contains_key(&user)
    }

    pub fn was_kicked(&self, user: UserId) -> bool {
        self.kicked.contains(&user)
    }

    /// The user's seat, if they have one.
    pub fn seat_of(&self, user: UserId) -> Option<Seat> {
        self.seated_players.get(&user).copied().flatten()
    }

    /// The first user in the seat.
    pub fn user_in_seat(&self, seat: Seat) -> Option<UserId> {
        self.seated_players.iter().find(|(_, s)| **s == Some(seat)).map(|(user, _)| *user)
    }

    pub fn seat_is_available(&self, seat: Seat) -> bool {
        self.available_seats.contains(&seat)
    }

    /// Whether a user may join on this seat, or on none: any seat or none while the game waits
    /// for players and has room, otherwise only a seat the host opened.
    pub fn is_open(&self, game: &Game, seat: Option<Seat>) -> bool {
        game.turn.current_phase == Phase::AwaitingPlayers && (self.seated_players.len() as i32) < game.maximum_players() || seat.is_some_and(|s| self.seat_is_available(s))
    }

    pub fn number_of_hosts(&self) -> i32 {
        self.hosts.len() as i32
    }

    pub fn number_of_observers(&self) -> i32 {
        self.observers.len() as i32
    }

    pub fn number_of_seated_players(&self) -> i32 {
        self.seated_players.len() as i32
    }

    /// The user's name, "?" for someone not in the game.
    pub fn name_of(&self, user: UserId) -> String {
        self.player_names.get(&user).cloned().unwrap_or_else(|| "?".to_string())
    }

    /// The player in the user's seat.
    pub fn player_of(&self, game: &Game, user: UserId) -> Option<PlayerId> {
        self.seat_of(user).and_then(|s| game.get_player_by_seat(s))
    }

    /// The seats users play, which is all the engine knows of the participation.
    pub fn human_seats(&self) -> IndexSet<Seat> {
        self.seated_players.values().copied().flatten().collect()
    }

    /// Tells the game which seats users play. Every change of seats below calls it.
    pub fn seat_humans(&self, game: &mut Game) {
        game.human_seats = self.human_seats();
    }

    /// What the views show of the seating.
    pub fn seating(&self) -> Seating {
        let mut names = BTreeMap::new();
        for (user, seat) in &self.seated_players {
            if let Some(seat) = seat {
                names.entry(*seat).or_insert_with(|| self.name_of(*user));
            }
        }
        Seating { names, seated_users: self.number_of_seated_players(), bots_speed: self.bots_speed }
    }

    /// Who the user is in the game's views.
    pub fn viewer(&self, user: UserId) -> Viewer {
        Viewer { user_id: user.0, seat: self.seat_of(user), is_host: self.is_host(user), is_observer: self.is_observer(user) }
    }

    /// The name views and mails give a player: "Bot", the user's name, or "?".
    pub fn player_name(&self, game: &Game, p: PlayerId) -> String {
        self.seating().player_name(game, p)
    }

    /// Seats a user, or lets them wait for a seat (`None`). Taking over a player's seat is
    /// logged.
    pub fn add_player(&mut self, game: &mut Game, user: UserId, name: &str, seat: Option<Seat>) {
        if let Some(seat) = seat {
            if let Some(player) = game.get_player_by_seat(seat) {
                let faction = game.player(player).faction;
                game.log(vec![name.to_string().into(), " now controls ".into(), faction.into()]);
            }
            self.available_seats.shift_remove(&seat);
        }
        self.seated_players.insert(user, seat);
        self.player_names.insert(user, name.to_string());
        self.observers.shift_remove(&user);
        self.seat_humans(game);
    }

    pub fn add_observer(&mut self, game: &mut Game, user: UserId, name: &str) {
        game.log(vec![name.to_string().into(), " is now observing".into()]);
        self.observers.insert(user);
        self.player_names.insert(user, name.to_string());
    }

    /// Takes a user out of the game, for good if `kick`. Logged.
    pub fn remove_user(&mut self, game: &mut Game, user: UserId, kick: bool) {
        let name = self.name_of(user);
        game.log(vec![name.into(), (if kick { " was kicked from" } else { " left" }).into(), " the game".into()]);
        self.seated_players.shift_remove(&user);
        self.observers.shift_remove(&user);
        self.player_names.shift_remove(&user);
        self.hosts.shift_remove(&user);
        if kick {
            self.kicked.insert(user);
        }
        self.seat_humans(game);
    }

    /// Seats exactly these users, each on the seat given.
    pub fn assign_seats(&mut self, game: &mut Game, seats: &[(UserId, Option<Seat>)]) {
        self.seated_players = seats.iter().copied().collect();
        self.seat_humans(game);
    }

    /// Takes every seated user off their seat.
    pub fn reset_seats(&mut self, game: &mut Game) {
        for seat in self.seated_players.values_mut() {
            *seat = None;
        }
        self.seat_humans(game);
    }

    pub fn open_or_close_seat(&mut self, seat: Seat) {
        if !self.available_seats.shift_remove(&seat) {
            self.available_seats.insert(seat);
        }
    }

    pub fn set_or_unset_host(&mut self, user: UserId) {
        if !self.hosts.shift_remove(&user) {
            self.hosts.insert(user);
        }
    }
}

fn numbers<T: serde::Serialize>(set: impl IntoIterator<Item = T>) -> Value {
    Value::Array(set.into_iter().map(|i| json!(i)).collect())
}

/// What clients get (protocol.md, Lobby objects).
pub fn to_wire(p: &Participation) -> ParticipationInfo {
    ParticipationInfo {
        player_names: p.player_names.iter().map(|(k, v)| (*k, v.clone())).collect(),
        seated_players: p.seated_players.iter().map(|(k, v)| (*k, *v)).collect(),
        observers: p.observers.iter().copied().collect(),
        hosts: p.hosts.iter().copied().collect(),
        available_seats: p.available_seats.iter().copied().collect(),
        kicked: p.kicked.iter().copied().collect(),
        bots_speed: p.bots_speed,
    }
}

/// The stored form.
pub fn to_json(p: &Participation) -> String {
    let object = |pairs: Vec<(String, Value)>| Value::Object(pairs.into_iter().collect::<Map<_, _>>());
    json!({
        "PlayerNames": object(p.player_names.iter().map(|(k, v)| (k.to_string(), json!(v))).collect()),
        "SeatedPlayers": object(p.seated_players.iter().map(|(k, v)| (k.to_string(), json!(Seat::number_or_minus_one(*v)))).collect()),
        "Observers": numbers(&p.observers),
        "Hosts": numbers(&p.hosts),
        "AvailableSeats": numbers(&p.available_seats),
        "Kicked": numbers(&p.kicked),
        "BotsSpeed": p.bots_speed,
    })
    .to_string()
}

/// Reads the stored form. Missing members keep their defaults.
pub fn from_json(text: &str) -> Result<Participation, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let Value::Object(map) = v else { return Err("participation: not an object".into()) };
    let integer = |v: &Value| v.as_i64().ok_or_else(|| format!("not an int: {v}"));
    let seat = |v: &Value| integer(v).and_then(|n| Seat::from_number(n).ok_or_else(|| format!("not a seat: {v}")));
    let pairs = |v: &Value| -> Result<Vec<(UserId, Value)>, String> {
        match v {
            Value::Object(m) => m.iter().map(|(k, v)| Ok((UserId(k.parse().map_err(|_| format!("bad key {k}"))?), v.clone()))).collect(),
            _ => Err(format!("not an object: {v}")),
        }
    };
    let array = |v: &Value| -> Result<Vec<Value>, String> { v.as_array().cloned().ok_or_else(|| format!("not an array: {v}")) };
    let users = |v: &Value| -> Result<Vec<UserId>, String> { array(v)?.iter().map(|u| integer(u).map(UserId)).collect() };
    let mut p = Participation::new();
    for (k, v) in &map {
        match k.as_str() {
            "PlayerNames" => {
                for (id, name) in pairs(v)? {
                    p.player_names.insert(id, name.as_str().unwrap_or_default().to_string());
                }
            }
            "SeatedPlayers" => {
                for (id, s) in pairs(v)? {
                    p.seated_players.insert(id, Seat::from_number_or_minus_one(integer(&s)?)?);
                }
            }
            "Observers" => p.observers.extend(users(v)?),
            "Hosts" => p.hosts.extend(users(v)?),
            "AvailableSeats" => {
                for s in array(v)? {
                    p.available_seats.insert(seat(&s)?);
                }
            }
            "Kicked" => p.kicked.extend(users(v)?),
            "BotsSpeed" => p.bots_speed = integer(v).and_then(|n| i32::try_from(n).map_err(|_| format!("not an int: {v}")))?,
            _ => {}
        }
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_keeps_order() {
        let text = r#"{"PlayerNames":{"7":"Alice","3":"Bob"},"SeatedPlayers":{"7":0,"3":-1},"Observers":[9],"Hosts":[7],"AvailableSeats":[2],"Kicked":[],"BotsSpeed":2}"#;
        let p = from_json(text).unwrap();
        assert_eq!(to_json(&p), text);
        let wire = to_wire(&p);
        assert_eq!(wire.player_names, vec![(UserId(7), "Alice".to_string()), (UserId(3), "Bob".to_string())]);
        assert_eq!(wire.seated_players, vec![(UserId(7), Some(Seat::new(0))), (UserId(3), None)]);
        assert_eq!(wire.bots_speed, 2);
        assert_eq!(from_json("{}").unwrap().bots_speed, Participation::new().bots_speed);
        assert!(from_json("[]").is_err());
    }

    #[test]
    fn refuses_numbers_that_are_no_seat() {
        assert!(from_json(r#"{"SeatedPlayers":{"7":-2}}"#).is_err());
        assert!(from_json(r#"{"AvailableSeats":[-1]}"#).is_err());
        assert!(from_json(r#"{"AvailableSeats":[256]}"#).is_err());
    }
}
