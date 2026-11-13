//! Territories are compared by identity. Each `Map` has every territory
//! exactly once, so within a game identity is id equality, which is what this
//! `Copy` handle implements. Territory properties never change after the map
//! is built, except `IsVisible`, which depends on the attached locations and
//! takes the `Map`.

use std::fmt;

use crate::compat::cs::CsNull;
use crate::enumerations::StrongholdAdvantage;
use crate::identity::IIdentifiable;
use crate::map::location::Location;
use crate::map::{Map, TerritoryData};
use crate::map::data::{TERRITORIES, TERRITORY_COUNT};

/// Handle to a territory of the map. Equality is by id.
/// `Territory::NULL` is the null handle (see `cs::CsNull`).
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Territory(u8);

impl CsNull for Territory {
    const NULL: Territory = Territory::NULL;
}

impl Territory {
    /// The null handle. Every member throws NullReferenceException.
    pub const NULL: Territory = Territory(u8::MAX);

    pub(crate) const fn from_index(i: usize) -> Territory {
        Territory(i as u8)
    }

    /// The territory with this id, if it exists.
    pub fn from_id(id: i32) -> Option<Territory> {
        if id >= 0 && (id as usize) < TERRITORY_COUNT { Some(Territory(id as u8)) } else { None }
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
