//! Turns, sectors and amounts as unsigned types (rewrite-plan.md, change 2d): event JSON refuses a
//! value out of range, and the rules refuse or report what would go below zero.

use treachery_engine::json::utilities::Utilities;
use treachery_engine::game::amounts;
use treachery_engine::prelude::*;

fn decode(json: &str) -> Result<Event, String> {
    Utilities::deserialize::<HistoryEntry>(json).map(|e| e.expect("an event").event).map_err(|e| e.message)
}

#[test]
fn a_turn_out_of_range_doesnt_decode() {
    let prediction = |turn: i64| decode(&format!(r#"{{"$type":"BeneGesseritPrediction","ToWin":10,"Turn":{turn}}}"#));
    match prediction(5).unwrap() {
        Event::BeneGesseritPrediction(e) => assert_eq!(e.turn, Turn::new(5)),
        _ => unreachable!(),
    }
    assert_eq!(prediction(-1).unwrap_err(), "-1 is not a valid turn");
    assert_eq!(prediction(256).unwrap_err(), "256 is not a valid turn");
    let settings = |turns: i64| decode(&format!(r#"{{"$type":"EstablishPlayers","Settings":{{"MaximumTurns":{turns}}}}}"#));
    assert_eq!(settings(-3).unwrap_err(), "-3 is not a valid turn");
}

#[test]
fn a_game_needs_at_least_one_turn() {
    let g = Game::new();
    let mut e = EstablishPlayers::default();
    let settings = e.settings.as_mut().unwrap();
    settings.number_of_players = 2;
    settings.allowed_factions_in_play = Some(vec![Faction::Atreides, Faction::Harkonnen]);
    assert_eq!(e.validate(&g).map(|m| m.plain_text()), None);
    e.settings.as_mut().unwrap().maximum_turns = Turn::BEFORE_START;
    assert_eq!(e.validate(&g).map(|m| m.plain_text()), Some("At least one turn required".to_string()));
}

#[test]
fn the_storm_moves_modulo_18_sectors() {
    assert_eq!(Sector::first_storm(0), Sector::new(0));
    assert_eq!(Sector::first_storm(20), Sector::new(2));
    assert_eq!(Sector::new(17).moved(1), Sector::new(0));
    assert_eq!(Sector::new(16).moved(3), Sector::new(1));
    assert_eq!(Sector::new(5).moved(36), Sector::new(5));
    assert_eq!(Sector::number_or_none(None), -1);
}

/// Before the first storm, nothing is in the storm. C# compared sector numbers, and a homeworld's
/// sector and the storm's were both -1, so homeworlds counted as in the storm until it was placed.
#[test]
fn nothing_is_in_the_storm_before_the_first_storm() {
    let mut g = Game::new();
    let homeworld = g.board.map.locations(true).find(|l| l.as_homeworld().is_some()).unwrap();
    let sand = g.board.map.the_great_flat();
    assert_eq!(homeworld.sector(&g.board.map), None);
    assert!(!g.is_in_storm(homeworld));
    assert!(!g.is_in_storm(sand));
    g.board.sector_in_storm = sand.sector(&g.board.map);
    assert!(g.is_in_storm(sand));
    assert!(!g.is_in_storm(homeworld));
}

fn game_with(factions: &[Faction]) -> Game {
    let mut g = Game::new();
    g.decks.treachery_discard_pile = Some(Deck::new(Vec::new(), None));
    for f in factions {
        g.players.push(Player::new(Some(*f)));
    }
    g
}

fn refusal(e: Option<Message>) -> Option<String> {
    e.map(|m| m.plain_text())
}

#[test]
fn spending_more_spice_than_there_is_fails_and_changes_nothing() {
    let mut spice: u16 = 3;
    assert!(amounts::spend(&mut spice, 4, "spice").is_err());
    assert!(amounts::spend(&mut spice, -1, "spice").is_err());
    assert!(amounts::gain(&mut spice, -1, "spice").is_err());
    assert_eq!(spice, 3);
    amounts::spend(&mut spice, 3, "spice").unwrap();
    amounts::gain(&mut spice, 5, "spice").unwrap();
    assert_eq!(spice, 5);
}

/// The Emperor could support their own bids, then bid their spice twice. Paying such a bid left
/// them with less than zero.
#[test]
fn the_emperor_supports_only_other_bidders() {
    let g = game_with(&[Faction::Emperor, Faction::Atreides]);
    let mut e = EmperorBidSupport::new(&g, Faction::Emperor);
    e.amounts = Some(IndexMap::from([(Faction::Atreides, 3)]));
    assert_eq!(refusal(e.validate(&g)), None);
    e.amounts = Some(IndexMap::from([(Faction::Emperor, 3)]));
    assert_eq!(refusal(e.validate(&g)).as_deref(), Some("You can't support your own bids"));
    e.amounts = Some(IndexMap::from([(Faction::Atreides, -3)]));
    assert_eq!(refusal(e.validate(&g)).as_deref(), Some("Invalid amount"));
}

/// A pass offers nothing. In silent and once-around auctions the highest bid wins, and a pass
/// counted its amounts, which validation never checks.
#[test]
fn a_pass_offers_nothing() {
    let g = game_with(&[Faction::Emperor, Faction::Atreides]);
    let mut bid = Bid::new(&g, Faction::Atreides);
    bid.amount = 2;
    bid.emperor_contribution_amount = 2;
    assert_eq!(bid.total_amount(), 4);
    bid.passed = true;
    assert_eq!(bid.total_amount(), 0);
    let mut black_market = BlackMarketBid::new(&g, Faction::Atreides);
    black_market.amount = 2;
    black_market.passed = true;
    assert_eq!(black_market.total_amount(), 0);
}

#[test]
fn bids_with_unoffered_emperor_spice_are_refused() {
    let mut g = game_with(&[Faction::Emperor, Faction::Atreides]);
    let atreides = g.get_player(Some(Faction::Atreides)).unwrap();
    let emperor = g.get_player(Some(Faction::Emperor)).unwrap();
    g.player_mut(atreides).resources = 5;
    g.player_mut(emperor).resources = 10;
    let bid = |g: &Game, amount: u16, ally: u16, emperor: u16| {
        let mut b = Bid::new(g, Faction::Atreides);
        b.amount = amount;
        b.ally_contribution_amount = ally;
        b.emperor_contribution_amount = emperor;
        refusal(b.validate(g))
    };
    // The Emperor has the spice but offered none of it.
    assert_eq!(bid(&g, 1, 0, 3).as_deref(), Some("Emperor won't pay that much"));
    g.alliances.permitted_use_of_emperor_spice.as_mut().unwrap().insert(Faction::Atreides, 3);
    assert_eq!(bid(&g, 1, 0, 3), None);

    let mut black_market = BlackMarketBid::new(&g, Faction::Atreides);
    black_market.amount = 6;
    assert_eq!(refusal(black_market.validate(&g)).as_deref(), Some("You can't pay Treachery.Shared.Payment"));
}

fn home(g: &Game, w: World) -> Homeworld {
    g.board.map.homeworlds().find(|h| h.world() == w).unwrap()
}

/// Forces never go below zero. C# ignored a change that would, so a move from a location with
/// too few forces still added them at the destination.
#[test]
fn forces_below_zero_are_a_game_error_and_change_nothing() {
    let mut g = game_with(&[Faction::Atreides]);
    let me = g.get_player(Some(Faction::Atreides)).unwrap();
    let from = g.board.map.arrakeen();
    let to = g.board.map.the_great_flat();
    Player::add_forces(&mut g, me, from, 2, false).unwrap();
    assert!(Player::move_forces(&mut g, me, from, to, 3).is_err());
    assert_eq!((g.player(me).forces_in(Some(from)), g.player(me).forces_in(Some(to))), (2, 0));
    assert!(Player::kill_forces(&mut g, me, from, 3, 0, false).is_err());
    assert_eq!(g.player(me).forces_killed, 0);
    assert!(Player::revive_forces(&mut g, me, 1).is_err());
}

/// Spice on the board never goes below zero either.
#[test]
fn spice_on_the_board_below_zero_is_a_game_error() {
    let mut g = Game::new();
    let l = g.board.map.the_great_flat();
    g.change_resources_on_planet(l, 6).unwrap();
    assert!(g.change_resources_on_planet(l, -7).is_err());
    assert_eq!(g.resources_in(l), 6);
    g.change_resources_on_planet(l, -6).unwrap();
    assert!(!g.board.resources_on_planet.contains_key(&l));
}

/// Reinforcements kill forces in reserve wherever they are. C# took them only from the homeworld
/// of normal forces, and counted them in the tanks even when it held fewer.
#[test]
fn reinforcements_come_from_every_homeworld() {
    let mut g = game_with(&[Faction::Emperor]);
    let me = g.get_player(Some(Faction::Emperor)).unwrap();
    let kaitain = home(&g, World::Emperor);
    let salusa = home(&g, World::SalusaSecundus);
    Player::initialize_homeworld(&mut g, me, kaitain, 1, 0).unwrap();
    Player::initialize_homeworld(&mut g, me, salusa, 0, 5).unwrap();
    Player::change_forces(&mut g, me, salusa.location(), 4).unwrap();
    Player::kill_forces_in_reserves(&mut g, me, 3, false).unwrap();
    let p = g.player(me);
    assert_eq!((p.forces_in(Some(kaitain.location())), p.forces_in(Some(salusa.location()))), (0, 2));
    assert_eq!(p.forces_killed, 3);
    assert!(Player::kill_forces_in_reserves(&mut g, me, 3, false).is_err());
}

/// A pass skipped every check of a Caravan, but the move was still made: from anywhere, with
/// forces the player didn't have.
#[test]
fn a_caravan_cant_be_passed() {
    let g = game_with(&[Faction::Atreides]);
    let mut e = Caravan::default();
    e.passed = true;
    assert_eq!(refusal(e.validate(&g, By::new(&g, Faction::Atreides)).err().map(|r| r.0)).as_deref(), Some("You can't pass a Caravan"));
}

/// Amounts in events are never negative, and an event with one doesn't decode. A negative bid or
/// gift paid by receiving spice, and a negative amount of forces placed, killed or revived moved
/// forces the wrong way. Validation refused most of these; a negative No-Field value meant no
/// No-Field. The exceptions are a shipment's force amounts, negative for a shipment back to
/// reserves, and the values that mean a direction (`Direction`, `ValueAdded`).
#[test]
fn negative_event_amounts_dont_decode() {
    for (event, member) in [
        ("Bid", "Amount"),
        ("Bid", "AllyContributionAmount"),
        ("BlackMarketBid", "EmperorContributionAmount"),
        ("ResourcesTransferred", "Resources"),
        ("TakeLosses", "ForceAmount"),
        ("KarmaFreeRevival", "AmountOfForces"),
        ("Revival", "AmountOfForces"),
        ("Shipment", "SmuggledAmount"),
        ("Shipment", "_noFieldValue"),
        ("NexusPlayed", "TleilaxuForces"),
        ("DealOffered", "Price"),
        ("StormDialled", "Amount"),
    ] {
        let json = |n: i64| format!(r#"{{"$type":"{event}","{member}":{n}}}"#);
        assert!(decode(&json(0)).is_ok(), "{event}.{member} 0");
        assert!(decode(&json(65535)).is_ok(), "{event}.{member} 65535");
        assert_eq!(decode(&json(-1)).unwrap_err(), "-1 is not a valid amount", "{event}.{member}");
        assert_eq!(decode(&json(65536)).unwrap_err(), "65536 is not a valid amount", "{event}.{member}");
    }
    assert!(decode(r#"{"$type":"Shipment","ForceAmount":-2,"SpecialForceAmount":-1}"#).is_ok());
    assert!(decode(r#"{"$type":"RicheseSpecifiesAuction","Direction":-1}"#).is_ok());
    assert!(decode(r#"{"$type":"TestingStationUsed","ValueAdded":-1}"#).is_ok());
}

/// A force list (`_forceLocations`) with an amount outside 0 to 65535 doesn't decode.
#[test]
fn a_force_list_out_of_range_doesnt_decode() {
    let mv = |list: &str| decode(&format!(r#"{{"$type":"Move","_toId":5,"_forceLocations":"{list}"}}"#));
    assert!(mv("12:3|1,14:2|0").is_ok());
    assert_eq!(mv("12:-3|1").unwrap_err(), "-3 is not a valid amount of forces");
    assert_eq!(mv("12:3|65536").unwrap_err(), "65536 is not a valid amount of forces");
    let face_danced = decode(r#"{"$type":"FaceDanced","_targetForceLocations":"12:-1|0"}"#);
    assert_eq!(face_danced.unwrap_err(), "-1 is not a valid amount of forces");
}

/// Spice a player bid in a silent auction can't be given away before the auction ends. It was
/// paid anyway, and the winner ended below zero (fuzz seed 1 game 58).
#[test]
fn spice_bid_in_a_silent_auction_cant_be_given_away() {
    let mut g = game_with(&[Faction::Fremen, Faction::Emperor]);
    let fremen = g.get_player(Some(Faction::Fremen)).unwrap();
    g.player_mut(fremen).resources = 3;
    g.turn.current_phase = Phase::Bidding;
    g.bidding.current_auction_type = Some(AuctionType::RicheseSilent);
    let mut bid = Bid::new(&g, Faction::Fremen);
    bid.amount = 2;
    g.bidding.bids.insert(Faction::Fremen, std::rc::Rc::new(bid).into_event());
    assert_eq!(g.spice_committed_to_bids(fremen), 2);

    let gift = |g: &Game, amount: i32| {
        let mut e = Donated::new(g, Faction::Fremen);
        e.target = Some(Faction::Emperor);
        e.resources = amount;
        refusal(e.validate(g))
    };
    assert_eq!(gift(&g, 1), None);
    assert_eq!(gift(&g, 2).as_deref(), Some("You need that spice to pay your bid"));
}
