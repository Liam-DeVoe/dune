//! Turns, sectors and amounts as unsigned types (rewrite-plan.md, change 2d): event JSON refuses a
//! value out of range, and the rules refuse or report what would go below zero.

use treachery_engine::json::utilities::Utilities;
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
