//! Port of Model/DiscoveryStronghold.cs (class `DiscoveredLocation`).

use std::ops::Deref;

use crate::attached_location::AttachedLocation;
use crate::enumerations::DiscoveryToken;
use crate::location::Location;
use crate::map::{LocationKind, Map};

/// Handle to a discovered location (`l is DiscoveredLocation ds`).
/// Derefs to [`Location`]; `.attached()` gives the [`AttachedLocation`] API.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct DiscoveredLocation(Location);

impl DiscoveredLocation {
    pub(crate) fn from_location(l: Location) -> Self {
        DiscoveredLocation(l)
    }

    pub fn location(self) -> Location {
        self.0
    }

    pub fn attached(self) -> AttachedLocation {
        AttachedLocation::from_location(self.0)
    }

    pub fn discovery(self) -> DiscoveryToken {
        match self.0.data().kind {
            LocationKind::Discovered { discovery } => discovery,
            _ => unreachable!("DiscoveredLocation handle on a non-discovered location"),
        }
    }

    /// `AttachedToLocation`
    pub fn attached_to_location(self, map: &Map) -> Option<Location> {
        self.attached().attached_to_location(map)
    }

    /// `PointAt(game, newLocation)`; see [`AttachedLocation::point_at`].
    pub fn point_at(self, map: &mut Map, game_version: i32, new_location: Location) {
        self.attached().point_at(map, game_version, new_location)
    }
}

impl Deref for DiscoveredLocation {
    type Target = Location;
    fn deref(&self) -> &Location {
        &self.0
    }
}

impl From<DiscoveredLocation> for Location {
    fn from(a: DiscoveredLocation) -> Location {
        a.0
    }
}
