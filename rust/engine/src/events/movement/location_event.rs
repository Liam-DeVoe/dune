use crate::prelude::*;

/// Implemented by the events that have these members. Not in the prelude, to avoid
/// ambiguity with the base-class traits; `GameEvent` has a dispatching method for each member.
pub trait ILocationEvent {
    fn initiator(&self) -> Faction;

    fn to(&self, game: &Game) -> Option<Location>;

    fn total_amount_of_forces_added_to_location(&self, game: &Game) -> i32;

    fn forces_added_to_location(&self, game: &Game) -> i32;

    fn special_forces_added_to_location(&self, game: &Game) -> i32;

}
