//! Events of the bidding phase: bids, Richese and Ixian auctions, card swaps and Karama powers used while bidding.

mod bid;
pub use bid::*;
mod black_market_bid;
pub use black_market_bid::*;
mod card_given;
pub use card_given::*;
mod emperor_bid_support;
pub use emperor_bid_support::*;
mod emperor_discarded;
pub use emperor_discarded::*;
mod ixian_removed_card_from_auction;
pub use ixian_removed_card_from_auction::*;
mod ixian_swapped_card_on_bid;
pub use ixian_swapped_card_on_bid::*;
mod karma_hand_swap;
pub use karma_hand_swap::*;
mod karma_hand_swap_initiated;
pub use karma_hand_swap_initiated::*;
mod karma_richese_buy;
pub use karma_richese_buy::*;
mod replaced_card_won;
pub use replaced_card_won::*;
mod richese_announces_auction;
pub use richese_announces_auction::*;
mod richese_announces_black_market;
pub use richese_announces_black_market::*;
mod richese_keeps_unsold_card;
pub use richese_keeps_unsold_card::*;
mod richese_specifies_auction;
pub use richese_specifies_auction::*;

/// The legacy way to build these events, which code outside bidding still uses: `X::new(game,
/// faction)` and an `Rc<X>` turned into an `Event` (bots/other.rs, bots/factions.rs,
/// view/actions/other.rs, cards/distrans_used.rs). The submitter goes on the history entry, so
/// `new` ignores its arguments. Goes once those callers write `X::default()` and return `X`.
macro_rules! legacy_constructors {
    ($($name:ident),* $(,)?) => {
        $(
            impl $name {
                pub fn new(_game: &crate::prelude::Game, _initiator: impl Into<Option<crate::prelude::Faction>>) -> Self {
                    Self::default()
                }
            }

            impl From<std::rc::Rc<$name>> for crate::prelude::Event {
                fn from(e: std::rc::Rc<$name>) -> Self {
                    crate::prelude::Event::$name(std::rc::Rc::unwrap_or_clone(e))
                }
            }
        )*
    };
}

legacy_constructors!(
    Bid,
    CardGiven,
    EmperorBidSupport,
    EmperorDiscarded,
    IxianRemovedCardFromAuction,
    IxianSwappedCardOnBid,
    KarmaHandSwap,
    KarmaHandSwapInitiated,
    ReplacedCardWon,
    RicheseAnnouncesAuction,
    RicheseAnnouncesBlackMarket,
    RicheseKeepsUnsoldCard,
    RicheseSpecifiesAuction,
);
