use crate::prelude::*;

/// Implemented by the events that have these members. Not in the prelude, to avoid
/// ambiguity with the base-class traits; `GameEvent` has a dispatching method for each member.
pub trait IBid {
    fn total_amount(&self) -> i32;

    fn amount(&self) -> i32;

    fn ally_contribution_amount(&self) -> i32;

    fn emperor_contribution_amount(&self) -> i32;

    fn uses_emperor_secret_ally(&self) -> bool;

    fn initiator(&self) -> Faction;

    fn player(&self) -> Option<PlayerIndex>;

    fn passed(&self) -> bool;

    fn using_karma_to_remove_bid_limit(&self) -> bool;

    fn karma_card(&self) -> Option<TreacheryCard>;

    fn get_message(&self, game: &Game) -> Message;

    fn karma_bid(&self) -> bool;

}
