use std::ops::Deref;

use crate::map::attached_location::AttachedLocation;
use crate::map::location::Location;
use crate::map::Map;

/// Handle to the hidden mobile stronghold (`l is HiddenMobileStronghold`).
/// Derefs to [`Location`]; `.attached()` gives the [`AttachedLocation`] API.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct HiddenMobileStronghold(Location);

impl HiddenMobileStronghold {
    pub(crate) fn from_location(l: Location) -> Self {
        HiddenMobileStronghold(l)
    }

    pub fn location(self) -> Location {
        self.0
    }

    pub fn attached(self) -> AttachedLocation {
        AttachedLocation::from_location(self.0)
    }

    /// `AttachedToLocation`
    pub fn attached_to_location(self, map: &Map) -> Option<Location> {
        self.attached().attached_to_location(map)
    }

    /// `PointAt(game, newLocation)`; see [`AttachedLocation::point_at`].
    pub fn point_at(self, map: &mut Map, new_location: Location) {
        self.attached().point_at(map, new_location)
    }
}

impl Deref for HiddenMobileStronghold {
    type Target = Location;
    fn deref(&self) -> &Location {
        &self.0
    }
}

impl From<HiddenMobileStronghold> for Location {
    fn from(a: HiddenMobileStronghold) -> Location {
        a.0
    }
}
