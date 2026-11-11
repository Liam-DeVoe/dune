//! Port of `Events/GameInfo.cs` (a lobby entry; the C# file sits in `Shared/Events`).

use std::cell::RefCell;
use std::rc::Rc;

use dotnet_compat::NetDictionary;
use treachery_engine::cs::Shared;
use treachery_engine::date_time_offset::DateTimeOffset;
use treachery_engine::enumerations::{Faction, MainPhase, Phase, Ruleset};
use treachery_engine::json::{JsonWriter, ToJson};
use treachery_engine::participation::Participation;

use super::available_seat_info::AvailableSeatInfo;

#[derive(Clone, Debug, Default)]
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
    /// C#: `SeatedPlayers = managedGame.Game.Participation.SeatedPlayers` (GameHub_GameManagement.cs:715)
    /// stores a reference to the live dictionary, while every other member is a snapshot. The
    /// cached `RunningGames` array therefore shows seat changes made after the snapshot, until
    /// `RequestAssignSeats` replaces the dictionary object. See [`SeatedPlayersRef`] and the
    /// plan, "Hazards".
    pub seated_players: SeatedPlayersRef,
    pub available_seats: Vec<AvailableSeatInfo>,
}

impl GameInfo {
    /// `CanBeJoined` (`[JsonIgnore]`).
    pub fn can_be_joined(&self) -> bool {
        self.phase == Phase::AwaitingPlayers && (self.seated_players.with(|d| d.len()) as i32) < self.max_players || !self.available_seats.is_empty()
    }

    /// `YourCurrentSeat(userId)`.
    pub fn your_current_seat(&self, user_id: i32) -> i32 {
        self.seated_players.with(|d| d.get(&user_id).copied().unwrap_or(-1))
    }

    /// `YouAreIn(userId)`.
    pub fn you_are_in(&self, user_id: i32) -> bool {
        self.seated_players.with(|d| d.contains_key(&user_id))
    }
}

impl ToJson for GameInfo {
    fn write_json(&self, w: &mut JsonWriter) {
        w.object_value();
        w.member("CreatorId", &self.creator_id);
        w.member("CreatorUsername", &self.creator_username);
        w.member("CreationDate", &self.creation_date);
        w.member("GameId", &self.game_id);
        w.member("HasPassword", &self.has_password);
        w.member("Name", &self.name);
        w.member("MaxPlayers", &self.max_players);
        w.member("MaxTurns", &self.max_turns);
        w.member("MainPhase", &self.main_phase);
        w.member("Phase", &self.phase);
        w.member("Turn", &self.turn);
        w.member("NrOfBots", &self.nr_of_bots);
        w.member("NrOfPlayers", &self.nr_of_players);
        w.member("FactionsInPlay", &self.factions_in_play);
        w.member("Ruleset", &self.ruleset);
        w.member("LastActivity", &self.last_activity);
        w.key("SeatedPlayers");
        self.seated_players.with(|d| crate::json::write_int_dict(w, d));
        w.member("AvailableSeats", &self.available_seats);
        w.end_object();
    }
}

/// A C# reference to a `Participation.SeatedPlayers` dictionary object.
///
/// The engine's `Participation` holds its seat dictionary by value, so the dictionary object
/// C# shares is modelled here: one `SeatedPlayersRef` stands for one dictionary object, and
/// clones share it. While the object is the participation's current dictionary the reference
/// reads the participation (`Live`); when `RequestAssignSeats` replaces the dictionary, the
/// old object keeps the contents it had at that moment (`Detached`). The hub keeps the
/// current reference of each game in `ManagedGame::seated_players_ref`, so every `GameInfo`
/// extracted while the same dictionary is current shares one reference, as in C#.
#[derive(Clone)]
pub struct SeatedPlayersRef(Rc<RefCell<SeatedPlayersSource>>);

enum SeatedPlayersSource {
    /// The participation's current `SeatedPlayers`.
    Live(Shared<Participation>),
    /// A dictionary object no participation holds any more, or a plain value.
    Detached(NetDictionary<i32, i32>),
}

impl SeatedPlayersRef {
    /// A reference to `participation.SeatedPlayers` as it is now and as it will change.
    pub fn live(participation: &Shared<Participation>) -> Self {
        SeatedPlayersRef(Rc::new(RefCell::new(SeatedPlayersSource::Live(participation.clone()))))
    }

    /// A dictionary object that nothing else changes.
    pub fn detached(seated_players: NetDictionary<i32, i32>) -> Self {
        SeatedPlayersRef(Rc::new(RefCell::new(SeatedPlayersSource::Detached(seated_players))))
    }

    /// Whether this reads `participation`'s current dictionary.
    pub fn is_live_for(&self, participation: &Shared<Participation>) -> bool {
        matches!(&*self.0.borrow(), SeatedPlayersSource::Live(p) if p.ptr_eq(participation))
    }

    /// The participation stops holding this dictionary object (it is about to get another
    /// one): from now on this reference, and every clone, keeps the current contents.
    pub fn detach(&self) {
        let contents = self.get();
        *self.0.borrow_mut() = SeatedPlayersSource::Detached(contents);
    }

    /// Reads the dictionary.
    pub fn with<R>(&self, f: impl FnOnce(&NetDictionary<i32, i32>) -> R) -> R {
        match &*self.0.borrow() {
            SeatedPlayersSource::Live(p) => f(&p.borrow().seated_players),
            SeatedPlayersSource::Detached(d) => f(d),
        }
    }

    /// A copy of the dictionary's current contents.
    pub fn get(&self) -> NetDictionary<i32, i32> {
        self.with(|d| d.clone())
    }
}

impl Default for SeatedPlayersRef {
    fn default() -> Self {
        Self::detached(NetDictionary::new())
    }
}

impl std::fmt::Debug for SeatedPlayersRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.with(|d| d.fmt(f))
    }
}
