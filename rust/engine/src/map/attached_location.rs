//! The hidden mobile stronghold and the discovered strongholds are locations
//! that float: `PointAt` attaches them to a host location. Their mutable state
//! (`AttachedToLocation`, `Game`) lives in [`Map`]; this is a handle.

use std::ops::Deref;

use crate::map::location::Location;
use crate::map::Map;

/// Handle to an attached location (`l is AttachedLocation`).
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct AttachedLocation(Location);

impl AttachedLocation {
    /// Only call for a location whose kind is attached; use
    /// [`Location::as_attached_location`] otherwise.
    pub(crate) fn from_location(l: Location) -> Self {
        AttachedLocation(l)
    }

    pub fn location(self) -> Location {
        self.0
    }

    /// `AttachedToLocation`
    pub fn attached_to_location(self, map: &Map) -> Option<Location> {
        map.attached_to_location(self.0)
    }

    /// `PointAt(game, newLocation)`. C# passes the game only to read its
    /// version, which is always the latest:
    ///
    /// ```ignore
    /// // Game.Map.HiddenMobileStronghold.PointAt(Game, Target);
    /// game.map.hidden_mobile_stronghold().point_at(&mut game.map, target);
    /// ```
    pub fn point_at(self, map: &mut Map, new_location: Location) {
        map.point_at(self.0, new_location);
    }
}

impl Deref for AttachedLocation {
    type Target = Location;
    fn deref(&self) -> &Location {
        &self.0
    }
}

impl From<AttachedLocation> for Location {
    fn from(a: AttachedLocation) -> Location {
        a.0
    }
}
