//! The setup forms: `EstablishPlayers` for a host without a seat, and the custom
//! setup's `PerformSetup`.

mod common;

use common::*;
use treachery_engine::prelude::*;
use treachery_view::actions::QueryError;
use treachery_view::cache::ViewedGame;
use treachery_view::schema::*;

fn text(s: &str) -> RichText {
    vec![Part::Text { v: s.to_string() }]
}

fn settings(players: i32, factions: &[Faction]) -> String {
    let f: Vec<String> = factions.iter().map(|f| (*f as i32).to_string()).collect();
    format!(
        r#"{{"$type":"EstablishPlayers","Initiator":0,"Settings":{{"NumberOfPlayers":{players},"MaximumTurns":10,"InitialRules":[],"AllowedFactionsInPlay":[{}],"AutoOpenEmptySeats":true,"AsyncPlay":false,"AsyncPlayMessageIntervalMinutes":0}},"Seed":7}}"#,
        f.join(",")
    )
}

/// A new game awaiting players: the creator (user 1) is host, user 2 has joined.
fn awaiting() -> ViewedGame {
    let mut r = ViewedGame::new(Game::LATEST_VERSION);
    r.seating.seated_users = 2;
    r
}

#[test]
fn a_host_without_a_seat_establishes_players() {
    let mut r = awaiting();
    let host = Viewer { user_id: 1, seat: None, is_host: true, is_observer: false };
    let view = r.view(host);
    assert_eq!(view.phase, "AwaitingPlayers");
    let a = view.actions.iter().find(|a| a.event_type == "EstablishPlayers").expect("EstablishPlayers for the host");
    assert_eq!(a.options, Some(ActionOptions::EstablishPlayers(EstablishPlayersOptions { seated_players: 2 })));
    // The component's initial settings (six players, every faction) are valid.
    assert_eq!(a.validation, None);

    // EstablishPlayers.Validate on the host's drafts.
    let two = [Faction::Atreides, Faction::Harkonnen];
    assert_eq!(r.query(host, &settings(2, &two)).unwrap().validation, None);
    assert_eq!(r.query(host, &settings(1, &two)).unwrap().validation, Some(text("At least two players required")));
    assert_eq!(r.query(host, &settings(3, &two)).unwrap().validation, Some(text("More factions required")));

    // A guest has no host actions and can't query them.
    let guest = Viewer { user_id: 2, seat: None, is_host: false, is_observer: false };
    assert!(r.view(guest).actions.is_empty());
    assert_eq!(r.query(guest, &settings(2, &two)), Err(QueryError::NotAPlayer));
}

#[test]
fn perform_setup_offers_the_next_factions_reserves() {
    let mut r = load("setup-host");
    let host = seated_viewer(HOST_SEAT);
    let view = r.view(host);
    let a = view.actions.iter().find(|a| a.event_type == "PerformSetup").expect("PerformSetup for the host");
    let Some(ActionOptions::PerformSetup(o)) = &a.options else { panic!("PerformSetup options: {:?}", a.options) };
    let next = r.game.next_faction_to_perform_custom_setup();
    let p = r.game.player(r.game.get_player(next).unwrap());
    assert_eq!(o.faction, name_or_absent(next));
    assert_eq!(o.forces_in_reserve, p.forces_in_reserve());
    assert_eq!(o.special_forces_in_reserve, p.special_forces_in_reserve());
    assert_eq!(o.may_stage_special, p.has_special_forces(&r.game) && next != Some(Faction::Richese));
    assert_eq!(o.locations.len(), r.game.board.map.locations(false).count());
    assert_eq!(a.validation, None);

    // PerformSetup.Validate: no more forces than the reserves.
    let location = o.locations[0];
    let draft = |n: i32| format!(r#"{{"$type":"PerformSetup","Initiator":0,"Passed":false,"Resources":5,"_forceLocations":"{location}:{n}|0"}}"#);
    assert_eq!(r.query(host, &draft(o.forces_in_reserve)).unwrap().validation, None);
    assert!(r.query(host, &draft(o.forces_in_reserve + 1)).unwrap().validation.is_some());
}
