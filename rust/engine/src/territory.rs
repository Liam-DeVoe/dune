//! Port of Model/Territory.cs.
//!
//! C# compares territories by reference. Each `Map` creates every territory
//! exactly once, so within a game reference equality is id equality, which is
//! what this `Copy` handle implements. Territory properties never change after
//! the map is built (`Clone()` exists in C# but nothing calls it), except
//! `IsVisible`, which depends on the attached locations and takes the `Map`.

use std::fmt;

use crate::cs::CsNull;
use crate::enumerations::StrongholdAdvantage;
use crate::i_identifiable::IIdentifiable;
use crate::location::Location;
use crate::map::{Map, TerritoryData};
use crate::map_data::{TERRITORIES, TERRITORY_COUNT};

/// Handle to a territory of the map. Equality is by id, and by map: the second field
/// is 0 for a territory of the map of the game that holds the handle, or the id of
/// another game whose map it came from (read through an event of that game after an
/// undo; engine-conventions.md, "Game identity and undo"). C# compares territories by
/// reference, so a territory of another game's map is never equal to one of this map.
/// `Territory::NULL` is a C# null reference (see `cs::CsNull`).
#[derive(Copy, Clone)]
pub struct Territory(u8, u64);

impl PartialEq for Territory {
    fn eq(&self, other: &Territory) -> bool {
        if self.0 != other.0 {
            return false;
        }
        if self.1 == other.1 {
            return true;
        }
        // Inside `Game::enter_own_game`, the entered game's own objects carry its id.
        let own = crate::game::entered_game_id();
        let norm = |t: u64| if t == own { 0 } else { t };
        norm(self.1) == norm(other.1)
    }
}

impl Eq for Territory {}

impl std::hash::Hash for Territory {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl CsNull for Territory {
    const NULL: Territory = Territory::NULL;
}

impl Territory {
    /// C# null. Every member throws NullReferenceException.
    pub const NULL: Territory = Territory(u8::MAX, 0);

    pub(crate) const fn from_index(i: usize) -> Territory {
        Territory(i as u8, 0)
    }

    /// The territory with this id, if it exists.
    pub fn from_id(id: i32) -> Option<Territory> {
        if id >= 0 && (id as usize) < TERRITORY_COUNT { Some(Territory(id as u8, 0)) } else { None }
    }

    /// This territory as an object of the map of game `game` (see the type's doc).
    pub fn of_game_map(self, game: crate::game::GameId) -> Territory {
        if self.is_null() { self } else { Territory(self.0, game.0) }
    }

    /// The game whose map this territory object belongs to, if it isn't the
    /// map of the game that holds the handle.
    pub fn map_game(self) -> Option<crate::game::GameId> {
        if self.1 == 0 { None } else { Some(crate::game::GameId(self.1)) }
    }

    /// `Territory.Locations` as objects of this territory's map (see `map_game`).
    pub fn locations_of_map(self) -> Vec<Location> {
        match self.map_game() {
            Some(game) => self.locations().iter().map(|l| l.of_game_map(game)).collect(),
            None => self.locations().to_vec(),
        }
    }

    /// The static part of this territory.
    #[track_caller]
    pub fn data(self) -> &'static TerritoryData {
        &TERRITORIES[self.nn().0 as usize]
    }

    pub fn id(self) -> i32 {
        self.data().id
    }

    pub fn skin_id(self) -> i32 {
        self.id()
    }

    pub fn is_homeworld(self) -> bool {
        self.data().is_homeworld
    }

    pub fn has_reduced_shipping_cost(self) -> bool {
        self.data().has_reduced_shipping_cost
    }

    pub fn is_discovery(self) -> bool {
        self.data().is_discovery
    }

    pub fn is_discovered_stronghold(self) -> bool {
        self.is_stronghold() && self.is_discovery()
    }

    /// `IsVisible => Locations.Any(l => l.Visible)`
    pub fn is_visible(self, map: &Map) -> bool {
        self.locations().iter().any(|l| l.visible(map))
    }

    pub fn is_stronghold(self) -> bool {
        self.data().is_stronghold
    }

    pub fn is_protected_from_storm(self) -> bool {
        self.data().is_protected_from_storm
    }

    pub fn is_protected_from_worm(self) -> bool {
        self.data().is_protected_from_worm
    }

    pub fn advantage(self) -> StrongholdAdvantage {
        self.data().advantage
    }

    /// `Locations`, in the order they were created.
    pub fn locations(self) -> &'static [Location] {
        let d = self.data();
        Location::slice(d.first_location as usize, d.location_count as usize)
    }

    /// `locations[(int)(0.5 * locations.Count)]`
    pub fn middle_location(self) -> Location {
        let locations = self.locations();
        locations[(0.5 * locations.len() as f64) as usize]
    }

    /// `Locations.FirstOrDefault(l => l.SpiceBlowAmount > 0)`
    pub fn resource_blow_location(self) -> Option<Location> {
        self.locations().iter().copied().find(|l| l.spice_blow_amount() > 0)
    }

    /// `Locations.FirstOrDefault(l => l.DiscoveryTokenType != DiscoveryTokenType.None)`
    pub fn discovery_token_location(self) -> Option<Location> {
        self.locations()
            .iter()
            .copied()
            .find(|l| l.discovery_token_type() != crate::enumerations::DiscoveryTokenType::None)
    }
}

impl IIdentifiable for Territory {
    fn id(&self) -> i32 {
        Territory::id(*self)
    }
}

impl fmt::Debug for Territory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_null() {
            return f.write_str("Territory(null)");
        }
        write!(f, "Territory({})", self.0)
    }
}

impl serde::Serialize for Territory {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if self.is_null() {
            return s.serialize_none();
        }
        s.serialize_i32(self.id())
    }
}
