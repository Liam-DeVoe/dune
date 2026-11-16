//! Turns, sectors and amounts as unsigned types (rewrite-plan.md, change 2d): event JSON refuses a
//! value out of range, and the rules refuse or report what would go below zero.

use treachery_engine::json::utilities::Utilities;
use treachery_engine::game::amounts;
use treachery_engine::prelude::*;

fn decode(json: &str) -> Result<GameEvent, String> {
    Utilities::deserialize::<GameEvent>(json).map(|e| e.expect("an event")).map_err(|e| e.message)
}

#[test]
fn a_turn_out_of_range_doesnt_decode() {
    let prediction = |turn: i64| decode(&format!(r#"{{"$type":"BeneGesseritPrediction","ToWin":10,"Turn":{turn}}}"#));
    match prediction(5).unwrap() {
        GameEvent::BeneGesseritPrediction(e) => assert_eq!(e.turn, Turn::new(5)),
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
fn bids_with_negative_amounts_or_unoffered_emperor_spice_are_refused() {
    let mut g = game_with(&[Faction::Emperor, Faction::Atreides]);
    let atreides = g.get_player(Some(Faction::Atreides)).unwrap();
    let emperor = g.get_player(Some(Faction::Emperor)).unwrap();
    g.player_mut(atreides).resources = 5;
    g.player_mut(emperor).resources = 10;
    let bid = |g: &Game, amount: i32, ally: i32, emperor: i32| {
        let mut b = Bid::new(g, Faction::Atreides);
        b.amount = amount;
        b.ally_contribution_amount = ally;
        b.emperor_contribution_amount = emperor;
        refusal(b.validate(g))
    };
    assert_eq!(bid(&g, -1, 0, 5).as_deref(), Some("Invalid amount"));
    assert_eq!(bid(&g, 2, -1, 0).as_deref(), Some("Invalid amount"));
    // The Emperor has the spice but offered none of it.
    assert_eq!(bid(&g, 1, 0, 3).as_deref(), Some("Emperor won't pay that much"));
    g.alliances.permitted_use_of_emperor_spice.as_mut().unwrap().insert(Faction::Atreides, 3);
    assert_eq!(bid(&g, 1, 0, 3), None);

    let mut black_market = BlackMarketBid::new(&g, Faction::Atreides);
    black_market.amount = 6;
    assert_eq!(refusal(black_market.validate(&g)).as_deref(), Some("You can't pay Treachery.Shared.Payment"));
}

#[test]
fn spice_given_away_is_never_negative() {
    let mut g = game_with(&[Faction::Emperor, Faction::Atreides]);
    let atreides = g.get_player(Some(Faction::Atreides)).unwrap();
    let card = TreacheryCardManager::items()[0];
    g.player_mut(atreides).resources = 5;
    g.player_mut(atreides).treachery_cards.push(card);
    let mut gift = Donated::new(&g, Faction::Atreides);
    gift.target = Some(Faction::Emperor);
    gift.card = Some(card);
    gift.resources = -3;
    assert_eq!(refusal(gift.validate(&g)).as_deref(), Some("Invalid amount"));

    let mut transfer = ResourcesTransferred::new(&g, Faction::Atreides);
    transfer.resources = -3;
    assert_eq!(refusal(transfer.validate(&g)).as_deref(), Some("Invalid amount"));
}
