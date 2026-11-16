//! Focused tests for bidding. They build small game states by hand and check rules that are
//! easy to get wrong.


use treachery_engine::error::catch_panic;
use treachery_engine::prelude::*;

fn game_with(factions: &[Faction]) -> Game {
    let mut g = Game::new();
    for f in factions {
        let p = Player::new(Some(*f));
        g.players.push(p);
    }
    g
}

/// The bidding phase's state, entering the phase (at `Bidding`) first if the game isn't in it.
fn bidding(g: &mut Game) -> &mut BiddingState {
    if g.bidding().is_none() {
        g.enter_bidding(BiddingStep::Bidding);
    }
    g.bidding_mut().unwrap()
}

fn idx(g: &Game, f: Faction) -> PlayerId {
    g.get_player(Some(f)).unwrap()
}

/// A bid by `f`, as the auction keeps it.
fn placed(bid: &Bid, g: &mut Game, f: Faction) -> PlacedBid {
    let by = By::new(g, f);
    bid.placed(g, by)
}

fn outcome<R>(f: impl FnOnce() -> R) -> Result<R, String> {
    catch_panic(f)
}

fn text(m: Result<(), Rejection>) -> Option<String> {
    m.err().map(|r| r.0.plain_text())
}

#[test]
fn silent_bids_cannot_be_passed() {
    let mut g = game_with(&[Faction::Atreides, Faction::Richese]);
    bidding(&mut g).current_auction_type = Some(AuctionType::RicheseSilent);
    let mut bid = Bid::default();
    bid.passed = true;
    let by = By::new(&g, Faction::Atreides);
    assert!(bid.validate(&g, by).is_err());

    bidding(&mut g).current_auction_type = Some(AuctionType::BlackMarketSilent);
    assert!(bid.validate(&g, by).is_err());

    // A BlackMarketBid pass is always valid, even in a silent auction.
    let mut bmb = BlackMarketBid::default();
    bmb.passed = true;
    assert!(bmb.validate(&g, by).is_ok());
}

#[test]
fn bid_total_amount_doesnt_wrap() {
    // C# added ints, so a total above i32::MAX wrapped below zero. The amounts are u16 now.
    let mut bid = Bid::default();
    bid.amount = u16::MAX;
    bid.ally_contribution_amount = 1;
    assert_eq!(bid.total_amount(), 65_536);

    let mut g = game_with(&[Faction::Atreides]);
    bidding(&mut g).current_auction_type = Some(AuctionType::Normal);
    // More than Atreides has.
    assert!(bid.validate(&g, By::new(&g, Faction::Atreides)).is_err());
}

#[test]
fn bid_must_beat_current_bid_except_in_silent_auctions() {
    let mut g = game_with(&[Faction::Atreides, Faction::Harkonnen]);
    g.player_mut(idx(&g, Faction::Atreides)).resources = 10;
    g.player_mut(idx(&g, Faction::Harkonnen)).resources = 10;
    let mut current = Bid::default();
    current.amount = 5;
    let current = placed(&current, &mut g, Faction::Harkonnen);
    bidding(&mut g).current_bid = Some(current);

    let mut bid = Bid::default();
    bid.amount = 5;
    let by = By::new(&g, Faction::Atreides);

    bidding(&mut g).current_auction_type = Some(AuctionType::Normal);
    assert!(bid.validate(&g, by).is_err());

    bidding(&mut g).current_auction_type = Some(AuctionType::RicheseSilent);
    assert!(bid.validate(&g, by).is_ok());
}

#[test]
fn silent_bid_above_resources_is_rejected() {
    let mut g = game_with(&[Faction::Atreides, Faction::Emperor]);
    bidding(&mut g).current_auction_type = Some(AuctionType::RicheseSilent);
    g.player_mut(idx(&g, Faction::Atreides)).resources = 3;
    g.player_mut(idx(&g, Faction::Emperor)).resources = 5;
    // Amount within resources, but Emperor's contribution pushes the total above them.
    let mut bid = Bid::default();
    bid.amount = 3;
    bid.emperor_contribution_amount = 2;
    assert!(bid.validate(&g, By::new(&g, Faction::Atreides)).is_err());
}

#[test]
fn black_market_bid_validation_dereferences_null_player() {
    let mut g = game_with(&[Faction::Atreides]);
    bidding(&mut g).current_auction_type = Some(AuctionType::BlackMarketNormal);
    let mut bmb = BlackMarketBid::default();
    bmb.amount = 1;
    // Initiator not in the game: Player is null.
    let by = By::new(&g, Faction::Emperor);
    let r = outcome(|| bmb.validate(&g, by).is_err());
    assert!(r.is_err());
}

#[test]
fn bid_with_null_player_and_karma_card_is_invalid_not_a_crash() {
    let mut g = game_with(&[Faction::Atreides]);
    bidding(&mut g).current_auction_type = Some(AuctionType::Normal);
    let mut bid = Bid::default();
    bid.amount = 1;
    let karma = TreacheryCardManager::items().iter().copied().find(|c| c.type_() == TreacheryCardType::Karma).unwrap();
    bid.karma_card = Some(karma);
    let by = By::new(&g, Faction::Emperor);
    // UsingKarmaToRemoveBidLimit skips Player.Resources; p == null makes the card invalid.
    let r = outcome(|| text(bid.validate(&g, by)));
    assert!(r.is_ok());
    assert!(r.unwrap().is_some());
}

#[test]
fn emperor_bid_support_message_without_amounts_fails() {
    let g = game_with(&[Faction::Emperor]);
    let mut e2 = EmperorBidSupport::default();
    e2.amounts = None;
    let by = By::new(&g, Faction::Emperor);
    // Missing amounts (fuzz seed 2 recorded this in C#, where `Amounts.Sum(...)` threw).
    assert_eq!(outcome(|| e2.describe(&g, by).plain_text()).unwrap_err(), "amounts");
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

/// Fuzz seed 1 game 2 (rust/docs/escapes.md): everyone passes on a card while the next bidder has a
/// "pass when Atreides or their ally passed" automation rule. The last pass ends the bidding phase,
/// but `Bid` kept passing for the rule's owner. Each automatic pass ended the phase again and logged
/// "everyone passed" again, without end. Now automatic passes stop with the bidding phase.
#[test]
fn automatic_passes_stop_when_bidding_ends() {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut g = game_with(&[Faction::Atreides, Faction::BeneGesserit, Faction::Harkonnen]);
        for (seat, f) in [Faction::Atreides, Faction::BeneGesserit, Faction::Harkonnen].into_iter().enumerate() {
            let p = idx(&g, f);
            g.player_mut(p).seat = Seat::from_number(seat);
        }
        let atreides = idx(&g, Faction::Atreides);
        let bene_gesserit = idx(&g, Faction::BeneGesserit);
        g.player_mut(atreides).ally = Some(Faction::BeneGesserit);
        g.player_mut(bene_gesserit).ally = Some(Faction::Atreides);
        // Atreides has a full hand, so only Bene Gesserit and Harkonnen bid.
        g.player_mut(atreides).treachery_cards = TreacheryCardManager::items().iter().copied().take(4).collect();
        for f in [Faction::BeneGesserit, Faction::Harkonnen] {
            let mut rule = AutomationConfigured::default();
            rule.rule_type = AutomationRuleType::BiddingPassWhenAtreidesOrAtreidesAllyPassed;
            g.rules.automation_rules.push(rule.rule(-1, By::new(&g, f)));
        }
        g.turn.current_main_phase = MainPhase::Bidding;
        g.enter_bidding(BiddingStep::Bidding);
        g.start_bid_sequence_and_auction_type(Some(AuctionType::Normal), None, 1);
        let mut seq = bidding(&mut g).bid_sequence.take().unwrap();
        g.skip_players_that_cant_bid(&mut seq);
        bidding(&mut g).bid_sequence = Some(seq);

        let first = g.bidding().unwrap().bid_sequence.as_ref().unwrap().current_player(&g).unwrap();
        assert_ne!(first, atreides);
        let mut pass = Bid::default();
        pass.passed = true;
        let by = By::new(&g, g.player(first).faction.expect("faction"));
        pass.apply(&mut g, by).unwrap();

        let everyone_passed =
            g.current_report().messages.iter().filter(|m| m.plain_text() == "Bidding ends because everyone passed").count();
        tx.send((g.phase(), everyone_passed)).unwrap();
    });
    // Before the fix the thread never finished and its memory grew without end.
    let (phase, everyone_passed) = rx.recv_timeout(std::time::Duration::from_secs(20)).expect("Bid did not finish");
    assert_eq!(phase, Phase::BiddingReport);
    assert_eq!(everyone_passed, 1);
}

/// Fuzz seed 1 game 176 (rust/docs/escapes.md): the high bidder's hand filled up during the
/// auction, so the bid sequence skipped them and never came back to them to win. The other
/// bidders all passed by automation rule, around and around without end. Now each player passes
/// automatically at most once per bid, and the auction waits for a real pass.
#[test]
fn automatic_passes_stop_when_the_high_bidder_cant_be_reached() {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let factions = [Faction::Richese, Faction::Harkonnen, Faction::Emperor, Faction::Guild];
        let mut g = game_with(&factions);
        for (seat, f) in factions.into_iter().enumerate() {
            let p = idx(&g, f);
            g.player_mut(p).seat = Seat::from_number(seat);
        }
        for f in [Faction::Harkonnen, Faction::Emperor, Faction::Guild] {
            let mut rule = AutomationConfigured::default();
            rule.rule_type = AutomationRuleType::BiddingPassAboveAmount;
            rule.bidding_above_amount = 10;
            g.rules.automation_rules.push(rule.rule(-1, By::new(&g, f)));
        }
        g.turn.current_main_phase = MainPhase::Bidding;
        g.enter_bidding(BiddingStep::Bidding);
        g.start_bid_sequence_and_auction_type(Some(AuctionType::Normal), None, 1);
        let mut high = Bid::default();
        high.amount = 15;
        let high = placed(&high, &mut g, Faction::Richese);
        bidding(&mut g).current_bid = Some(high);
        // Richese's hand filled up after the bid, so the sequence skips Richese from now on.
        let richese = idx(&g, Faction::Richese);
        g.player_mut(richese).treachery_cards = TreacheryCardManager::items().iter().copied().take(5).collect();
        let mut seq = bidding(&mut g).bid_sequence.take().unwrap();
        g.skip_players_that_cant_bid(&mut seq);
        bidding(&mut g).bid_sequence = Some(seq);

        let first = g.bidding().unwrap().bid_sequence.as_ref().unwrap().current_player(&g).unwrap();
        assert_ne!(first, richese);
        let mut pass = Bid::default();
        pass.passed = true;
        let by = By::new(&g, g.player(first).faction.expect("faction"));
        pass.apply(&mut g, by).unwrap();
        tx.send(g.phase()).unwrap();
    });
    // Before the fix the thread never finished.
    let phase = rx.recv_timeout(std::time::Duration::from_secs(20)).expect("Bid did not finish");
    assert_eq!(phase, Phase::Bidding);
}
