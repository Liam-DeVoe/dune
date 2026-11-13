//! The bidding panel: `Bid` in `Phase::Bidding`, `BlackMarketBid` in
//! `Phase::BlackMarketBidding`.
//!
//! The client draws the panel by phase, not by action: every viewer sees the lot (the card or
//! its back, the auction's title, the current bid), and the bidder also gets the inputs.
//! The lot is [`panel`], part of `GameView.bidding`; the inputs are the `Bid` and
//! `BlackMarketBid` action options. Normal, Richese once-around and Richese silent auctions all
//! use `Bid`; black-market auctions (normal, once-around, silent) use `BlackMarketBid`.

use treachery_engine::events::GameEventType;
use treachery_engine::prelude::*;

use crate::build::{event, ResolvedViewer};
use crate::schema::*;
use super::widgets::{picker};

fn name<T: ToString>(v: T) -> String {
    v.to_string()
}

/// `BiddingPanel.IsSilentAuction`.
fn is_silent(game: &Game) -> bool {
    game.current_auction_type == AuctionType::BlackMarketSilent || game.current_auction_type == AuctionType::RicheseSilent
}

/// `BiddingPanel.IsRicheseBidding`.
fn is_richese_bidding(game: &Game) -> bool {
    game.current_auction_type == AuctionType::RicheseOnceAround || game.current_auction_type == AuctionType::RicheseSilent
}

/// The amounts the panel starts with, before the pickers clamp them:
/// `BiddingPanel.OnParametersSetAsync`. Outside silent auctions the form proposes the lowest bid
/// that beats the current one, paid by the ally as far as the ally permits. In silent auctions
/// the fields keep their initializers (0).
fn starting_amounts(game: &Game, p: PlayerIndex) -> (i32, i32) {
    if is_silent(game) {
        return (0, 0);
    }
    let pl = game.player(p);
    let ally_resources = if pl.ally == Faction::None { 0 } else { game.get_permitted_use_of_ally_resources(pl.faction) };
    let player_resources = pl.resources;
    let bid_to_do = game.current_bid.as_ref().map_or(1, |b| b.i_bid_total_amount() + 1);
    let ally = bid_to_do.min(ally_resources);
    let amount = (bid_to_do - ally).min(player_resources);
    (amount, ally)
}

/// What the pickers do to the starting amounts: the player's
/// picker clamps `Amount` into [0, MaxBidAmount]; the ally's picker, shown only when the player
/// has an ally and `MaxAllyBidAmount > 0`, clamps `AllyContributionAmount`, which is 0 without
/// it. Emperor's contribution starts at 0, inside its picker's range.
fn clamped(game: &Game, p: PlayerIndex, (amount, ally): (i32, i32), max_amount: i32, max_ally: i32) -> (i32, i32) {
    let amount = picker(amount, 0, max_amount);
    let ally = if game.player(p).ally != Faction::None && max_ally > 0 { picker(ally, 0, max_ally) } else { 0 };
    (amount, ally)
}

/// The event the panel would confirm before the player touches it (`ConfirmedResult`, with
/// `KarmaCard` null and `UseEmperorSecretAlly` false).
pub fn default_draft(game: &Game, faction: Faction, t: GameEventType) -> Option<GameEvent> {
    let p = game.get_player(Some(faction))?;
    match t {
        GameEventType::Bid => {
            let (amount, ally) = clamped(game, p, starting_amounts(game, p), Bid::valid_max_amount(game, p, false), Bid::valid_max_ally_amount(game, Some(p)));
            let mut e = Bid::new(game, faction);
            e.amount = amount;
            e.ally_contribution_amount = ally;
            Some(event(e))
        }
        GameEventType::BlackMarketBid => {
            let (amount, ally) =
                clamped(game, p, starting_amounts(game, p), BlackMarketBid::valid_max_amount(game, p), BlackMarketBid::valid_max_ally_amount(game, p));
            let mut e = BlackMarketBid::new(game, faction);
            e.amount = amount;
            e.ally_contribution_amount = ally;
            Some(event(e))
        }
        _ => None,
    }
}

/// The panel's inputs for `draft`. Only `Bid`'s maximum depends on the draft (its Karma card).
pub fn options(game: &Game, v: &ResolvedViewer, draft: &GameEvent) -> Option<ActionOptions> {
    let p = v.player?;
    let faction = game.player(p).faction;
    match draft {
        GameEvent::Bid(bid) => {
            let max_amount = Bid::valid_max_amount(game, p, bid.karma_card().is_some());
            let max_ally_amount = Bid::valid_max_ally_amount(game, Some(p));
            let (default_amount, default_ally_amount) =
                clamped(game, p, starting_amounts(game, p), Bid::valid_max_amount(game, p, false), max_ally_amount);
            Some(ActionOptions::Bid {
                max_amount,
                max_ally_amount,
                max_emperor_amount: game.spice_for_bids_emperor_can_pay(faction),
                karma_cards: if Bid::can_karma(game, Some(p)) { Bid::valid_karma_cards(game, Some(p)).map(|c| c.id()).collect() } else { vec![] },
                may_use_emperor_secret_ally: Bid::may_use_emperor_secret_ally(game, p),
                default_amount,
                default_ally_amount,
            })
        }
        GameEvent::BlackMarketBid(_) => {
            let max_amount = BlackMarketBid::valid_max_amount(game, p);
            let max_ally_amount = BlackMarketBid::valid_max_ally_amount(game, p);
            let (default_amount, default_ally_amount) = clamped(game, p, starting_amounts(game, p), max_amount, max_ally_amount);
            Some(ActionOptions::BlackMarketBid {
                max_amount,
                max_ally_amount,
                max_emperor_amount: game.spice_for_bids_emperor_can_pay(faction),
                default_amount,
                default_ally_amount,
            })
        }
        _ => None,
    }
}

/// The lot of the bidding panel, shown in `Phase::Bidding` and `Phase::BlackMarketBidding` to
/// every player, bidding or not.
pub fn panel(game: &Game, v: &ResolvedViewer) -> Option<BiddingPanelView> {
    let black_market = match game.current_phase {
        Phase::Bidding => false,
        Phase::BlackMarketBidding => true,
        _ => return None,
    };
    // Observers, and a host without a seat, get the panel a seated player without prescience
    // sees: the card only in Richese's auctions, where it is public, and no inputs (they have no
    // Bid action).
    let prescience = v.player.filter(|_| !v.is_observer).is_some_and(|p| game.has_bidding_prescience(p));
    let top = game.cards_on_auction.as_ref().and_then(|d| d.top());
    // CardIsVisible: (HasBiddingPrescience(Player) || IsRicheseBidding) && Top != null.
    let card = (prescience || is_richese_bidding(game)).then_some(top).flatten().map(|c| c.id());
    let silent = is_silent(game);
    // NormalBiddingPanel.AuctionDescription: "Card {CardNumber} of {NumberOfCardsOnRegularAuction}"
    // in normal auctions; Richese's auctions and the black market name the auction instead.
    let card_of = (!black_market && game.current_auction_type == AuctionType::Normal)
        .then(|| CardOf { number: game.card_number, of: game.number_of_cards_on_regular_auction() });
    let current_bid = if silent {
        None
    } else {
        game.current_bid.as_ref().map(|b| CurrentBid { faction: name(b.i_bid_initiator()), amount: b.i_bid_total_amount() })
    };
    Some(BiddingPanelView {
        black_market,
        card,
        card_of,
        silent,
        current_bid,
    })
}
