//! A `Location` is a `Copy` handle holding the location's id; equality and
//! hashing use the id. Its
//! static properties come from the table in `data.rs`; the properties that
//! change during a game (`Sector` and `Visible` of attached locations, every
//! `Neighbours` list) live in [`Map`] and take it as an argument.

use std::fmt;

use crate::map::attached_location::AttachedLocation;
use crate::compat::cs::CsNull;
use crate::map::discovery::DiscoveredLocation;
use crate::enumerations::{DiscoveryTokenType, StrongholdAdvantage};
use crate::map::hidden_mobile_stronghold::HiddenMobileStronghold;
use crate::map::homeworld::Homeworld;
use crate::identity::IIdentifiable;
use crate::map::{LocationData, LocationKind, Map};
use crate::map::data::{LOCATIONS, LOCATION_COUNT};
use crate::map::territory::Territory;

/// Handle to a location of the map. Equality is by id.
/// `Location::NULL` is the null handle (see `cs::CsNull`).
///
/// The second field says which game's map the location belongs to: 0 for the map of
/// the game that holds the handle, otherwise the id of another game (a location passed
/// to a player of another game after an undo; engine-conventions.md, "Game identity
/// and undo"). Only the state dump reads it (`"foreign":true`); equality ignores it.
#[derive(Copy, Clone)]
pub struct Location(u8, u64);

impl PartialEq for Location {
    fn eq(&self, other: &Location) -> bool {
        self.0 == other.0
    }
}

impl Eq for Location {}

impl std::hash::Hash for Location {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl CsNull for Location {
    const NULL: Location = Location::NULL;
}

static ALL_LOCATIONS: [Location; LOCATION_COUNT] = {
    let mut a = [Location(0, 0); LOCATION_COUNT];
    let mut i = 0;
    while i < LOCATION_COUNT {
        a[i] = Location(i as u8, 0);
        i += 1;
    }
    a
};

impl Location {
    /// The null handle. Every member throws NullReferenceException.
    pub const NULL: Location = Location(u8::MAX, 0);

    pub(crate) const fn from_index(i: usize) -> Location {
        Location(i as u8, 0)
    }

    /// Every location in `Map._locations` order.
    pub(crate) fn all() -> &'static [Location] {
        &ALL_LOCATIONS
    }

    /// Contiguous run of locations, used for `Territory.Locations`.
    pub(crate) fn slice(first: usize, count: usize) -> &'static [Location] {
        &ALL_LOCATIONS[first..first + count]
    }

    #[track_caller]
    pub(crate) fn index(self) -> usize {
        self.nn().0 as usize
    }

    /// The location with this id, if it exists. (`Map.LocationLookup.Find`
    /// throws instead; see [`Map::location_lookup`].)
    pub fn from_id(id: i32) -> Option<Location> {
        if id >= 0 && (id as usize) < LOCATION_COUNT { Some(Location(id as u8, 0)) } else { None }
    }

    /// This location as an object of the map of game `game` (see the type's doc).
    pub fn of_game_map(self, game: crate::game::GameId) -> Location {
        if self.is_null() { self } else { Location(self.0, game.0) }
    }

    /// The game whose map this location object belongs to, if it isn't the map of
    /// the game that holds the handle.
    pub fn map_game(self) -> Option<crate::game::GameId> {
        if self.1 == 0 { None } else { Some(crate::game::GameId(self.1)) }
    }

    /// The static part of this location.
    #[track_caller]
    pub fn data(self) -> &'static LocationData {
        &LOCATIONS[self.nn().0 as usize]
    }

    pub fn id(self) -> i32 {
        self.data().id
    }

    pub fn territory(self) -> Territory {
        // Location.Territory: a territory of the same map as this location.
        match self.map_game() {
            Some(game) => self.data().territory.of_game_map(game),
            None => self.data().territory,
        }
    }

    /// Whether both handles refer to the same object: the same id on the same
    /// game's map. (`==` compares ids only.)
    pub fn is_same_object(self, other: Location) -> bool {
        if self.0 != other.0 {
            return false;
        }
        let own = crate::game::entered_game_id();
        let norm = |t: u64| if t == own { 0 } else { t };
        norm(self.1) == norm(other.1)
    }

    /// Changes which game's map this location object belongs to: `from` and `to` are
    /// tags (0 is the map of the game holding the handle).
    pub(crate) fn retag(&mut self, from: u64, to: u64) {
        if self.1 == from && !self.is_null() {
            self.1 = to;
        }
    }

    /// `Sector`. Attached locations take their host's sector once a game has
    /// pointed them at it, else -1.
    pub fn sector(self, map: &Map) -> i32 {
        match self.data().kind {
            LocationKind::HiddenMobileStronghold | LocationKind::Discovered { .. } => map.attached_sector(self),
            _ => self.data().sector,
        }
    }

    /// `Visible`: attached locations are visible once pointed at something.
    pub fn visible(self, map: &Map) -> bool {
        match self.as_attached_location() {
            Some(a) => a.attached_to_location(map).is_some(),
            None => true,
        }
    }

    /// `Neighbours`. For attached locations this is the host (or nothing);
    /// `Map` keeps that list up to date.
    pub fn neighbours(self, map: &Map) -> &[Location] {
        map.neighbours_of(self)
    }

    pub fn orientation(self) -> &'static str {
        self.data().orientation
    }

    pub fn spice_blow_amount(self) -> i32 {
        self.data().spice_blow_amount
    }

    pub fn discovery_token_type(self) -> DiscoveryTokenType {
        self.data().discovery_token_type
    }

    pub fn is_stronghold(self) -> bool {
        self.territory().is_stronghold()
    }

    pub fn is_homeworld(self) -> bool {
        self.territory().is_homeworld()
    }

    pub fn is_protected_from_storm(self) -> bool {
        self.territory().is_protected_from_storm()
    }

    pub fn advantage(self) -> StrongholdAdvantage {
        self.territory().advantage()
    }

    /// `l is Homeworld hw`
    pub fn as_homeworld(self) -> Option<Homeworld> {
        match self.data().kind {
            LocationKind::Homeworld(_) => Some(Homeworld::from_location(self)),
            _ => None,
        }
    }

    /// `l is DiscoveredLocation ds`
    pub fn as_discovered_location(self) -> Option<DiscoveredLocation> {
        match self.data().kind {
            LocationKind::Discovered { .. } => Some(DiscoveredLocation::from_location(self)),
            _ => None,
        }
    }

    /// `l is HiddenMobileStronghold hms`
    pub fn as_hidden_mobile_stronghold(self) -> Option<HiddenMobileStronghold> {
        match self.data().kind {
            LocationKind::HiddenMobileStronghold => Some(HiddenMobileStronghold::from_location(self)),
            _ => None,
        }
    }

    /// `l is AttachedLocation al` (hidden mobile stronghold or discovered location)
    pub fn as_attached_location(self) -> Option<AttachedLocation> {
        match self.data().kind {
            LocationKind::HiddenMobileStronghold | LocationKind::Discovered { .. } => {
                Some(AttachedLocation::from_location(self))
            }
            _ => None,
        }
    }

    /// The name of the location's runtime type (`Location`, `Homeworld`, ...).
    pub fn type_name(self) -> &'static str {
        match self.data().kind {
            LocationKind::Normal => "Location",
            LocationKind::Homeworld(_) => "Homeworld",
            LocationKind::Discovered { .. } => "DiscoveredLocation",
            LocationKind::HiddenMobileStronghold => "HiddenMobileStronghold",
        }
    }
}

/// The text a location prints as when no describer is set (the engine never sets
/// one): "Homeworld" for a homeworld, the full .NET type name for every other
/// location. Used in dictionary exception messages.
impl dotnet_compat::exception::CsToString for Location {
    fn cs_to_string(&self) -> String {
        match self.data().kind {
            LocationKind::Homeworld(_) => "Homeworld".to_string(),
            _ => format!("Treachery.Shared.{}", self.type_name()),
        }
    }
}

impl IIdentifiable for Location {
    fn id(&self) -> i32 {
        Location::id(*self)
    }
}

impl fmt::Debug for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_null() {
            return f.write_str("Location(null)");
        }
        write!(f, "Location({})", self.0)
    }
}

/// Serialized as its id, which is how JSON refers to locations.
impl serde::Serialize for Location {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if self.is_null() {
            return s.serialize_none();
        }
        s.serialize_i32(self.id())
    }
}
