use crate::prelude::*;

/// Implemented by the events that have these members. Not in the prelude, to avoid
/// ambiguity with the base-class traits; `GameEvent` has a dispatching method for each member.
pub trait IPlacement {
    fn initiator(&self) -> Faction;

    fn force_locations(&self, game: &Game) -> IndexMap<Location, Battalion>;

    fn to(&self, game: &Game) -> Option<Location>;

}
