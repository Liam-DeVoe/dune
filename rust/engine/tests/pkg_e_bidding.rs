//! Focused tests for package E (bidding). They build small game states by hand
//! and check rules whose C# behaviour is easy to get wrong.

use std::rc::Rc;

use treachery_engine::compat::cs;
use treachery_engine::prelude::*;

fn game_with(factions: &[Faction]) -> Game {
    let mut g = Game::new();
    for f in factions {
        let p = Player::new(*f);
        g.players.push(p);
    }
    g
}

fn idx(g: &Game, f: Faction) -> PlayerIndex {
    g.get_player(Some(f)).unwrap()
}

fn as_player<E: GameEventMethods>(e: E, g: &Game, f: Faction) -> E {
    e.base().initiator.set(f);
    e.base().player.set(g.get_player(Some(f)));
    e
}

fn outcome<R>(f: impl FnOnce() -> R) -> Result<R, String> {
    cs::catch(f).map_err(|e| format!("{}: {}", e.type_name, e.message))
}

fn text(m: Option<Message>) -> Option<String> {
    m.map(|m| m.to_cs_string())
}

#[test]
fn silent_bids_cannot_be_passed() {
    let mut g = game_with(&[Faction::Atreides, Faction::Richese]);
    g.current_auction_type = AuctionType::RicheseSilent;
    let mut bid = Bid::default();
    bid.passed = true;
    let bid = as_player(bid, &g, Faction::Atreides);
    assert!(bid.validate(&g).is_some());

    g.current_auction_type = AuctionType::BlackMarketSilent;
    assert!(bid.validate(&g).is_some());

    // A BlackMarketBid pass is always valid, even in a silent auction.
    let mut bmb = BlackMarketBid::default();
    bmb.passed = true;
    let bmb = as_player(bmb, &g, Faction::Atreides);
    assert!(bmb.validate(&g).is_none());
}

#[test]
fn bid_total_amount_wraps() {
    let mut bid = Bid::default();
    bid.amount = i32::MAX;
    bid.ally_contribution_amount = 1;
    assert_eq!(bid.total_amount(), i32::MIN);

    let mut g = game_with(&[Faction::Atreides]);
    g.current_auction_type = AuctionType::Normal;
    let bid = as_player(bid, &g, Faction::Atreides);
    // The wrapped total is below 1.
    assert!(bid.validate(&g).is_some());
}

#[test]
fn bid_must_beat_current_bid_except_in_silent_auctions() {
    let mut g = game_with(&[Faction::Atreides, Faction::Harkonnen]);
    g.player_mut(idx(&g, Faction::Atreides)).resources = 10;
    g.player_mut(idx(&g, Faction::Harkonnen)).resources = 10;
    let mut current = Bid::default();
    current.amount = 5;
    let current = as_player(current, &g, Faction::Harkonnen);
    g.current_bid = Some(Rc::new(current).into_event());

    let mut bid = Bid::default();
    bid.amount = 5;
    let bid = as_player(bid, &g, Faction::Atreides);

    g.current_auction_type = AuctionType::Normal;
    assert!(bid.validate(&g).is_some());

    g.current_auction_type = AuctionType::RicheseSilent;
    assert!(bid.validate(&g).is_none());
}

#[test]
fn silent_bid_above_resources_is_rejected() {
    let mut g = game_with(&[Faction::Atreides, Faction::Emperor]);
    g.current_auction_type = AuctionType::RicheseSilent;
    g.player_mut(idx(&g, Faction::Atreides)).resources = 3;
    g.player_mut(idx(&g, Faction::Emperor)).resources = 5;
    // Amount within resources, but Emperor's contribution pushes the total above them.
    let mut bid = Bid::default();
    bid.amount = 3;
    bid.emperor_contribution_amount = 2;
    let bid = as_player(bid, &g, Faction::Atreides);
    assert!(bid.validate(&g).is_some());
}

#[test]
fn black_market_bid_validation_dereferences_null_player() {
    let mut g = game_with(&[Faction::Atreides]);
    g.current_auction_type = AuctionType::BlackMarketNormal;
    let mut bmb = BlackMarketBid::default();
    bmb.amount = 1;
    // Initiator not in the game: Player is null.
    let bmb = as_player(bmb, &g, Faction::Emperor);
    let r = outcome(|| bmb.validate(&g).is_some());
    assert!(r.unwrap_err().starts_with("NullReferenceException"));
}

#[test]
fn bid_with_null_player_and_karma_card_is_invalid_not_a_crash() {
    let mut g = game_with(&[Faction::Atreides]);
    g.current_auction_type = AuctionType::Normal;
    let mut bid = Bid::default();
    bid.amount = 1;
    let karma = TreacheryCardManager::items().iter().copied().find(|c| c.type_() == TreacheryCardType::Karma).unwrap();
    bid.set_karma_card(Some(karma));
    let bid = as_player(bid, &g, Faction::Emperor);
    // UsingKarmaToRemoveBidLimit skips Player.Resources; p == null makes the card invalid.
    let r = outcome(|| text(bid.validate(&g)));
    assert!(r.is_ok());
    assert!(r.unwrap().is_some());
}

#[test]
fn emperor_bid_support_message_sum_is_checked() {
    let g = game_with(&[Faction::Emperor]);
    let mut e = EmperorBidSupport::default();
    let mut amounts = NetDictionary::new();
    amounts.insert(Faction::Atreides, i32::MAX);
    amounts.insert(Faction::Harkonnen, 1);
    e.amounts = Some(amounts);
    let e = as_player(e, &g, Faction::Emperor);
    let r = outcome(|| e.get_message(&g).to_cs_string());
    assert!(r.unwrap_err().starts_with("OverflowException"));

    let mut e2 = EmperorBidSupport::default();
    e2.amounts = None;
    let e2 = as_player(e2, &g, Faction::Emperor);
    // `Amounts.Sum(...)` on null: LINQ checks its source (fuzz seed 2 recorded this in C#).
    assert_eq!(
        outcome(|| e2.get_message(&g).to_cs_string()).unwrap_err(),
        "ArgumentNullException: Value cannot be null. (Parameter 'source')"
    );
}

#[test]
fn karma_hand_swap_returned_cards_skip_unknown_ids() {
    let mut e = KarmaHandSwap::default();
    let cards: Vec<TreacheryCard> = TreacheryCardManager::items().iter().copied().take(2).collect();
    e.set_returned_cards(&cards);
    assert_eq!(e.returned_cards(), cards);

    e.card_ids = Some(format!("{};-5;{}", cards[0].id(), cards[1].id()));
    assert_eq!(e.returned_cards(), cards);

    e.card_ids = Some(String::new());
    assert!(e.returned_cards().is_empty());

    e.card_ids = Some("x".to_string());
    assert!(outcome(|| e.returned_cards()).unwrap_err().starts_with("FormatException"));
}

#[test]
fn distrans_targets_exclude_self_and_full_hands() {
    let mut g = game_with(&[Faction::Atreides, Faction::Harkonnen, Faction::Emperor]);
    let atreides = idx(&g, Faction::Atreides);
    let emperor = idx(&g, Faction::Emperor);
    let cards: Vec<TreacheryCard> = TreacheryCardManager::items().iter().copied().filter(|c| c.type_() != TreacheryCardType::Distrans).take(4).collect();
    g.player_mut(emperor).treachery_cards = cards;
    assert_eq!(DistransUsed::valid_targets(&g, atreides).collect::<Vec<_>>(), vec![Faction::Harkonnen]);
}
