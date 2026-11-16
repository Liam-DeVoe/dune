use crate::events::PlacementEvent;
use crate::prelude::*;

/// The force list of a converted placement event (`DiscoveryEntered`, `FremenRidesMonster`,
/// `PerformFremenSetup`, `PerformSetup`), which keeps it as the text of its JSON member
/// `_forceLocations`. Converted events have no submitter of their own, so reading the list as
/// battalions takes the player whose forces they are.
pub struct PlacedForces;

impl PlacedForces {
    /// The battalions of player `p` in `force_locations`. Panics when the list is missing, as C#
    /// threw reading its length.
    pub fn of(game: &Game, p: PlayerId, force_locations: &Option<String>) -> IndexMap<Location, Battalion> {
        let faction = game.player(p).faction;
        let s = force_locations.as_deref().expect("field force locations");
        PlacementEvent::parse_force_locations(game, faction.expect("faction"), s)
    }

    /// The text of `value`, for the event's force list.
    pub fn text(game: &Game, value: &IndexMap<Location, Battalion>) -> Option<String> {
        Some(PlacementEvent::force_locations_string(game, value))
    }

    /// The forces and special forces in the list, which don't depend on whose they are. The
    /// list is parsed twice, as C# did.
    pub fn amounts(game: &Game, force_locations: &Option<String>) -> (i32, i32) {
        let s = force_locations.as_deref().expect("field force locations");
        // The faction only labels the battalions; the counts don't read it.
        let parsed = PlacementEvent::parse_force_locations(game, Faction::Fremen, s);
        let forces = parsed.values().map(|b| b.forces()).sum::<i32>();
        let special_forces = parsed.values().map(|b| b.special_forces()).sum::<i32>();
        (forces, special_forces)
    }
}
