use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::prelude::*;

/// A seat at the table, numbered from 0. `EstablishPlayers` draws a seat for each player as it
/// creates them, so every player has one (`Player::seated`). The server seats its users on
/// them, and a user may have none yet (`Option<Seat>`).
///
/// A seat is written as its number. Where a seat can be absent, the lobby protocol, the views
/// and stored participations write -1 for none (`seat_or_minus_one`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Seat(u8);

impl Seat {
    pub const fn new(number: u8) -> Seat {
        Seat(number)
    }

    pub fn number(self) -> i32 {
        i32::from(self.0)
    }

    /// The seat with this number, or `None` for a number no seat has.
    pub fn from_number(number: impl TryInto<u8>) -> Option<Seat> {
        number.try_into().ok().map(Seat)
    }

    /// The seat's number, -1 for none.
    pub fn number_or_minus_one(seat: Option<Seat>) -> i32 {
        seat.map_or(-1, Seat::number)
    }

    /// The seat with this number, `None` for -1, or an error for any other number no seat has.
    pub fn from_number_or_minus_one(number: i64) -> Result<Option<Seat>, String> {
        match number {
            -1 => Ok(None),
            n => Seat::from_number(n).map(Some).ok_or_else(|| format!("{n} is not a seat")),
        }
    }
}

impl std::fmt::Display for Seat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// A plain `number` in TypeScript, like the integer it is written as.
impl ts_rs::TS for Seat {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn name(_: &ts_rs::Config) -> String {
        "number".to_string()
    }

    fn inline(cfg: &ts_rs::Config) -> String {
        Self::name(cfg)
    }
}

/// `#[serde(with = "seat_or_minus_one")]` writes an `Option<Seat>` as its number, -1 for none.
pub mod seat_or_minus_one {
    use super::*;

    pub fn serialize<S: Serializer>(seat: &Option<Seat>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_i32(Seat::number_or_minus_one(*seat))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Seat>, D::Error> {
        Seat::from_number_or_minus_one(i64::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

impl Player {
    /// The player's seat. Every player of a game has one: `EstablishPlayers` draws them right
    /// after it creates the players.
    pub fn seated(&self) -> Seat {
        self.seat.expect("a player has a seat")
    }
}

/// Finding players by faction and seat, and which seats bots play.
impl Game {
    pub fn is_playing(&self, faction: impl Into<Option<Faction>>) -> bool {
        let faction: Option<Faction> = faction.into();
        self.players.iter().any(|p| p.faction == faction)
    }

    pub fn get_player(&self, f: Option<Faction>) -> Option<PlayerId> {
        self.players.iter().position(|p| p.faction == f).map(|i| self.player_at(i))
    }

    pub fn get_ally(&self, f: Option<Faction>) -> Option<Faction> {
        self.get_player(f).map(|p| self.player(p).ally).flatten()
    }

    /// Lazy: callers may stop early, so filters run only on the elements they reach.
    pub fn players_other_than(&self, p: PlayerId) -> impl Iterator<Item = Faction> {
        self.players.iter().filter(move |x| x.faction != self.player(p).faction).filter_map(move |x| x.faction)
    }

    pub fn get_player_by_seat(&self, seat: Seat) -> Option<PlayerId> {
        self.players.iter().position(|p| p.seat == Some(seat)).map(|i| self.player_at(i))
    }

    /// Whether a bot plays this player: whether no user sits in its seat (`human_seats`).
    pub fn is_bot(&self, p: PlayerId) -> bool {
        !self.player(p).seat.is_some_and(|s| self.human_seats.contains(&s))
    }

    pub fn number_of_bots(&self) -> i32 {
        self.player_indices().filter(|p| self.is_bot(*p)).count() as i32
    }
}
