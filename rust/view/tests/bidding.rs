//! Package 7a: the bidding panel (`BiddingPanel.razor`, `NormalBiddingPanel`,
//! `BlackMarketBiddingPanel`). Over every state of every fixture and sample bot game where a
//! player may bid, the `Bid` and `BlackMarketBid` options equal the engine's helpers (the ports
//! of the C# helpers the panels call), the default draft is the panel's starting input, and the
//! lot (`bidding.panel`) shows the card exactly to the players the C# panel shows it to.

mod common;

use std::collections::BTreeMap;
use std::path::PathBuf;

use common::*;
use treachery_engine::compat::cs::Shared;
use treachery_engine::events::GameEventType;
use treachery_engine::game::GameState;
use treachery_engine::game::Participation;
use treachery_engine::prelude::*;
use treachery_view::actions;
use treachery_view::build::ResolvedViewer;
use treachery_view::describe::default_describer;
use treachery_view::schema::*;

fn resolved(game: &Game, p: Option<PlayerIndex>) -> ResolvedViewer {
    ResolvedViewer { player: p, faction: p.map(|p| game.player(p).faction), is_host: false, is_observer: p.is_none() }
}

fn silent(game: &Game) -> bool {
    matches!(game.current_auction_type, AuctionType::BlackMarketSilent | AuctionType::RicheseSilent)
}

/// `BiddingPanel.OnParametersSetAsync`, then the pickers' clamps, written out from the razor
/// file independently of `actions/bidding.rs`.
fn expected_defaults(game: &Game, p: PlayerIndex, max_amount: i32, max_ally: i32) -> (i32, i32) {
    let pl = game.player(p);
    let (mut amount, mut ally) = (0, 0);
    if !silent(game) {
        let ally_resources = if pl.ally == Faction::None { 0 } else { game.get_permitted_use_of_ally_resources(pl.faction) };
        let bid_to_do = match &game.current_bid {
            Some(b) => b.i_bid_total_amount() + 1,
            None => 1,
        };
        ally = bid_to_do.min(ally_resources);
        amount = (bid_to_do - ally).min(pl.resources);
    }
    let clamp = |v: i32, max: i32| if v < 0 { 0 } else if v > max { max } else { v };
    amount = clamp(amount, max_amount);
    ally = if pl.ally != Faction::None && max_ally > 0 { clamp(ally, max_ally) } else { 0 };
    (amount, ally)
}

#[derive(Default)]
struct Seen {
    /// Bidders checked, per event type and auction type.
    bidders: BTreeMap<String, usize>,
    karma: usize,
    ally_pays: usize,
    emperor_pays: usize,
    visible_cards: usize,
    hidden_cards: usize,
    /// Observer lots checked (deviation 5).
    observer_panels: usize,
}

/// Checks one game state: every player's bidding options and every viewer's lot.
fn check_state(game: &mut Game, d: &dyn treachery_view::describe::Describer, seen: &mut Seen, at: &str) {
    if game.current_main_phase != MainPhase::Bidding {
        return;
    }
    let players: Vec<PlayerIndex> = game.player_indices().collect();
    for &p in &players {
        let types = game.get_applicable_events(Some(p), false);
        let v = resolved(game, Some(p));
        let faction = game.player(p).faction;
        for t in [GameEventType::Bid, GameEventType::BlackMarketBid] {
            if !types.contains(&t) {
                continue;
            }
            let view = actions::action_view(game, &v, t, d);
            let options = view.options.unwrap_or_else(|| panic!("{at}: no options for {t:?}"));
            let emperor = game.spice_for_bids_emperor_can_pay(faction);
            let (expected, default_event) = if t == GameEventType::Bid {
                let max_amount = Bid::valid_max_amount(game, p, false);
                let max_ally = Bid::valid_max_ally_amount(game, Some(p));
                let (amount, ally) = expected_defaults(game, p, max_amount, max_ally);
                let karma_cards: Vec<i32> = if Bid::can_karma(game, Some(p)) { Bid::valid_karma_cards(game, Some(p)).map(|c| c.id()).collect() } else { vec![] };
                if !karma_cards.is_empty() {
                    seen.karma += 1;
                }
                let mut e = Bid::new(game, faction);
                e.amount = amount;
                e.ally_contribution_amount = ally;
                (
                    ActionOptions::Bid {
                        max_amount,
                        max_ally_amount: max_ally,
                        max_emperor_amount: emperor,
                        karma_cards,
                        may_use_emperor_secret_ally: Bid::may_use_emperor_secret_ally(game, p),
                        default_amount: amount,
                        default_ally_amount: ally,
                    },
                    GameEvent::from(std::rc::Rc::new(e)),
                )
            } else {
                let max_amount = BlackMarketBid::valid_max_amount(game, p);
                let max_ally = BlackMarketBid::valid_max_ally_amount(game, p);
                let (amount, ally) = expected_defaults(game, p, max_amount, max_ally);
                let mut e = BlackMarketBid::new(game, faction);
                e.amount = amount;
                e.ally_contribution_amount = ally;
                (
                    ActionOptions::BlackMarketBid { max_amount, max_ally_amount: max_ally, max_emperor_amount: emperor, default_amount: amount, default_ally_amount: ally },
                    GameEvent::from(std::rc::Rc::new(e)),
                )
            };
            assert_eq!(options, expected, "{at}: {faction} {t:?}");
            assert_eq!(view.validation, treachery_view::build::validate(game, default_event.clone(), d), "{at}: {faction} {t:?} validation");
            // Outside silent auctions the starting bid is the lowest that beats the current one,
            // so it is valid whenever the player and ally can pay for it.
            if !silent(game) {
                let to_beat = game.current_bid.as_ref().map_or(0, |b| b.i_bid_total_amount());
                let (amount, ally) = match &default_event {
                    GameEvent::Bid(b) => (b.amount, b.ally_contribution_amount),
                    GameEvent::BlackMarketBid(b) => (b.amount, b.ally_contribution_amount),
                    _ => unreachable!(),
                };
                if amount + ally == to_beat + 1 {
                    assert_eq!(view.validation, None, "{at}: {faction} {t:?}: the starting bid {amount}+{ally} is refused");
                }
                if ally > 0 {
                    seen.ally_pays += 1;
                }
            }
            if emperor > 0 {
                seen.emperor_pays += 1;
            }
            *seen.bidders.entry(format!("{t:?} {}", game.current_auction_type)).or_default() += 1;
        }
    }

    // The lot, for every player and an observer.
    let in_panel = matches!(game.current_phase, Phase::Bidding | Phase::BlackMarketBidding);
    let top = game.cards_on_auction.as_ref().and_then(|d| d.top());
    for viewer in players.iter().map(|p| Some(*p)).chain([None]) {
        let v = resolved(game, viewer);
        let panel = actions::bidding::panel(game, &v);
        // Observers get the panel of a player without prescience (deviation 5; the C# one throws
        // for them).
        assert_eq!(panel.is_some(), in_panel, "{at}");
        let Some(panel) = panel else { continue };
        assert_eq!(panel.black_market, game.current_phase == Phase::BlackMarketBidding);
        let richese = matches!(game.current_auction_type, AuctionType::RicheseOnceAround | AuctionType::RicheseSilent);
        // An observer has no prescience but sees the card in Richese's auctions, as everyone does.
        let prescience = viewer.is_some_and(|p| game.has_bidding_prescience(p));
        let expected_card = (prescience || richese).then_some(top).flatten().map(|c| c.id());
        if viewer.is_none() {
            seen.observer_panels += 1;
        }
        assert_eq!(panel.card, expected_card, "{at}: card for {:?}", v.faction);
        if viewer.is_some() {
            if panel.card.is_some() {
                seen.visible_cards += 1;
            } else {
                seen.hidden_cards += 1;
            }
        }
        assert_eq!(panel.silent, silent(game));
        let expected_bid = if silent(game) { None } else { game.current_bid.as_ref().map(|b| (b.i_bid_initiator().to_string(), b.i_bid_total_amount())) };
        assert_eq!(panel.current_bid.map(|b| (b.faction, b.amount)), expected_bid, "{at}");
        if panel.black_market || game.current_auction_type != AuctionType::Normal {
            assert_eq!(panel.card_of, None);
        } else {
            let c = panel.card_of.expect("normal auction");
            assert_eq!((c.number, c.of), (game.card_number, game.number_of_cards_on_regular_auction()));
            assert!(c.number >= 1 && c.number <= c.of, "{at}: card {} of {}", c.number, c.of);
        }
    }
}

fn fixtures() -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/fixtures");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|e| e == "json")).collect();
    files.sort();
    files
}

#[test]
fn bidding_options_equal_the_helpers_in_every_state() {
    let d = default_describer();
    let mut seen = Seen::default();
    for path in fixtures() {
        let state = GameState::load(&std::fs::read_to_string(&path).unwrap());
        let participation = Shared::new(Participation::default());
        let mut game = Game::new_int32_participation(state.version, &participation);
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        for (i, e) in state.events.as_ref().unwrap().iter().enumerate() {
            e.initialize_game(&game);
            assert!(e.execute(&mut game, false, false).is_none(), "{name}: event {i}");
            check_state(&mut game, d.as_ref(), &mut seen, &format!("{name} after event {i}"));
        }
    }
    for path in sample_bot_traces() {
        let trace = read_trace(&path);
        let participation = Shared::new(Participation::default());
        let mut r = treachery_view::replica::ViewReplica::new(trace.version, &participation);
        for (i, e) in trace.events.iter().enumerate() {
            r.apply(e).unwrap();
            check_state(&mut r.game, d.as_ref(), &mut seen, &format!("{} after event {i}", trace.name));
        }
    }
    eprintln!(
        "bidders checked: {:?}; with Karma {}, ally paying {}, Emperor paying {}; lot shown {} hidden {}",
        seen.bidders, seen.karma, seen.ally_pays, seen.emperor_pays, seen.visible_cards, seen.hidden_cards
    );
    // The corpus reaches each kind of auction and each optional input.
    for kind in ["Bid Normal", "Bid RicheseOnceAround", "Bid RicheseSilent"] {
        assert!(seen.bidders.contains_key(kind), "no state with a {kind} bidder");
    }
    assert!(seen.bidders.keys().any(|k| k.starts_with("BlackMarketBid")), "no black-market bidder");
    assert!(seen.karma > 0 && seen.ally_pays > 0 && seen.visible_cards > 0 && seen.hidden_cards > 0 && seen.observer_panels > 0);
}

/// The bid fixture: Atreides sees the card (bidding prescience), the others the card back; the
/// view's actions carry the options and the panel the lot.
#[test]
fn bid_fixture_view() {
    let mut r = load("bid", 0);
    let mut bidders = 0;
    for f in r.game.players.iter().map(|p| p.faction).collect::<Vec<_>>() {
        let view = r.view(viewer_of(&r, f));
        let panel = view.bidding.as_ref().and_then(|b| b.panel.clone()).expect("bidding panel");
        let p = r.game.get_player(Some(f)).unwrap();
        assert_eq!(panel.card.is_some(), r.game.has_bidding_prescience(p), "{f}");
        if let Some(a) = view.actions.iter().find(|a| a.event_type == "Bid") {
            bidders += 1;
            assert!(matches!(a.options, Some(ActionOptions::Bid { .. })));
        }
    }
    assert_eq!(bidders, 1, "one bidder at a time in a normal auction");
    // The C# panel throws for observers (HasBiddingPrescience(null)). They get the lot of a
    // player without prescience instead (deviation 5): the card back, "card n of m", the bid, and
    // no Bid action.
    let view = r.view(OBSERVER);
    let panel = view.bidding.unwrap().panel.expect("observer's bidding panel");
    assert_eq!(panel.card, None);
    assert!(!panel.black_market);
    let c = panel.card_of.expect("normal auction");
    assert_eq!((c.number, c.of), (r.game.card_number, r.game.number_of_cards_on_regular_auction()));
    assert!(view.actions.iter().all(|a| a.event_type != "Bid" && a.event_type != "BlackMarketBid"));
}

/// The maximum bid depends on the draft's Karma card (`Bid.ValidMaxAmount(Player, KarmaCard !=
/// null)`), so the client asks again when one is picked. With it, bidding above one's spice is
/// valid.
#[test]
fn karma_card_lifts_the_limit_in_a_draft_query() {
    let mut r = first_bot_state("a bidder who may use Karma", |g| {
        g.current_phase == Phase::Bidding
            && g.current_auction_type == AuctionType::Normal
            && g.bid_sequence.as_ref().and_then(|s| s.current_player(g)).is_some_and(|p| Bid::can_karma(g, Some(p)))
    });
    let p = r.game.bid_sequence.as_ref().unwrap().current_player(&r.game).unwrap();
    let f = r.game.player(p).faction;
    let viewer = viewer_of(&r, f);
    let view = r.view(viewer);
    let a = view.actions.iter().find(|a| a.event_type == "Bid").expect("Bid applicable");
    let Some(ActionOptions::Bid { max_amount, karma_cards, .. }) = &a.options else { panic!() };
    assert_eq!(*max_amount, r.game.player(p).resources);
    let card = karma_cards[0];
    let over = r.game.player(p).resources + 5;
    let draft = |karma: i32| {
        format!(r#"{{"$type":"Bid","Amount":{over},"AllyContributionAmount":0,"EmperorContributionAmount":0,"UsesEmperorSecretAlly":false,"KarmaBid":false,"_karmaCardId":{karma},"Passed":false,"Initiator":0,"Time":"2026-01-01T00:00:00+00:00"}}"#)
    };
    let with = r.query(viewer, &draft(card)).unwrap();
    assert!(matches!(with.options, Some(ActionOptions::Bid { max_amount: 100, .. })));
    let without = r.query(viewer, &draft(-1)).unwrap();
    assert!(without.validation.is_some(), "bidding more than one has needs Karma");
    if over > r.game.current_bid.as_ref().map_or(0, |b| b.i_bid_total_amount()) {
        assert_eq!(with.validation, None);
    }
}
