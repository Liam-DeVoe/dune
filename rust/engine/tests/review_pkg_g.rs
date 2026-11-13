//! Package G review (shipment and movement): probes of C# behaviour at the edges
//! of the shipment cost and force-location code. See docs/reviews/pkg-g.md.
//!
//! Expected values come from the C# probe:
//!
//!     cd rust/engine/csharp-ref && dotnet run -c Release -- review-g
//!
//! Each assertion names the probe line it checks.

use treachery_engine::cs;
use treachery_engine::enumerations::{Faction, ShipmentType};
use treachery_engine::events::shipment::Shipment;
use treachery_engine::events::GameEvent;
use treachery_engine::events::game_event::GameEventMethods;
use treachery_engine::game::Game;
use treachery_engine::json;
use treachery_engine::participation::Participation;

// Same text as ReviewA.Establish (step 1 of fuzz-2026-00018).
const ESTABLISH: &str = r#"{"$type":"EstablishPlayers","Settings":{"NumberOfPlayers":3,"MaximumTurns":8,"InitialRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"AllowedFactionsInPlay":[50,10,120,60,40,100,20,90,70,80,30,110],"AutoOpenEmptySeats":true,"AsyncPlay":false,"AsyncPlayMessageIntervalMinutes":0},"Seed":1073548723,"MaximumNumberOfPlayers":3,"MaximumTurns":8,"ApplicableRules":[301,300,65,60,20,1013,15,700,105,80,85,604,90,998,30,701,40,107,10,70,50,25,209,999,112],"_players":"","_factionsInPlay":"Guild,Fremen,Moritani,BeneGesserit,Emperor,Richese,Atreides,Choam,Ixian,Tleilaxu,Harkonnen,Ecaz","Initiator":0,"Time":"2024-01-01T12:02:17+00:00"}"#;

fn setup() -> Game {
    let mut p = Participation::new();
    p.player_names.insert(100, "Human100".to_string());
    p.seated_players.insert(100, 2);
    let mut g = Game::new_int32_participation(Game::LATEST_VERSION, &treachery_engine::cs::Shared::new(p));
    let e: GameEvent = json::deserialize(ESTABLISH).expect("event json");
    e.initialize_game(&g);
    assert!(e.execute(&mut g, true, true).is_none());
    g
}

fn show<T: ToString>(f: impl FnOnce() -> T) -> String {
    match cs::catch(f) {
        Ok(v) => v.to_string(),
        Err(e) => format!("{}: {}", e.type_name, e.message),
    }
}

fn parse(g: &Game, s: &str) -> String {
    show(|| {
        let d = Shipment::parse_force_locations(g, Faction::Emperor, Some(s));
        d.iter()
            .map(|(k, v)| format!("{}:{}|{}", k.id(), v.amount_of_forces, v.amount_of_special_forces))
            .collect::<Vec<_>>()
            .join(",")
    })
}

#[test]
fn force_location_strings_throw_like_csharp() {
    let g = setup();
    assert_eq!(parse(&g, "abc:1|1"), "FormatException: The input string 'abc' was not in a correct format.");
    assert_eq!(parse(&g, "42:1|0,42:2|0"), "ArgumentException: An item with the same key has already been added. Key: Treachery.Shared.Location");
    assert_eq!(parse(&g, "42:1"), "IndexOutOfRangeException: Index was outside the bounds of the array.");
    let aoor = "ArgumentOutOfRangeException: Index was out of range. Must be non-negative and less than the size of the collection. (Parameter 'index')";
    assert_eq!(parse(&g, "9999:1|0"), aoor);
    assert_eq!(parse(&g, "-5:1|0,42:3|4"), aoor);
    assert_eq!(parse(&g, "42:99999999999|0"), "OverflowException: Value was either too large or too small for an Int32.");
}

#[test]
fn shipment_costs_match_csharp_at_the_edges() {
    let g = setup();
    let pl = g.player_indices().find(|p| g.player(*p).faction != Faction::Fremen).unwrap();
    assert_eq!(g.player(pl).faction, Faction::Ecaz, "probe: player0 = Ecaz");
    let to = g
        .map
        .locations(false)
        .find(|l| l.id() != 0 && !l.territory().has_reduced_shipping_cost() && !l.territory().is_stronghold())
        .unwrap();
    assert_eq!(to.id(), 1, "probe: to = 1");
    let cost = |f: i32, s: i32, karma: bool, back: bool| {
        show(|| Shipment::determine_cost_game_player_int32_int32_location_boolean_boolean_boolean_boolean_boolean(&g, pl, f, s, to, karma, back, false, false, false))
    };
    assert_eq!(cost(i32::MAX, 1, false, false), "OverflowException: Negating the minimum value of a twos complement number is invalid.");
    assert_eq!(cost(1073741824, 0, false, false), "-2147483648");
    assert_eq!(cost(16777217, 0, false, true), "8388608");
    assert_eq!(cost(i32::MAX, 0, false, true), "1073741824");
    assert_eq!(cost(-3, 0, false, true), "-1");
    assert_eq!(cost(3, 0, true, false), "3");

    let faction = g.player(pl).faction;
    let mut s = Shipment::default();
    s.shipment_type = ShipmentType::ShipmentBack;
    s.force_amount = i32::MIN;
    s.to_id = to.id();
    s.initialize(&g, faction);
    assert_eq!(show(|| Shipment::determine_cost(&g, pl, &s)), "OverflowException: Negating the minimum value of a twos complement number is invalid.");
    assert_eq!(show(|| s.validate(&g).map(|m| m.to_cs_string()).unwrap_or("null".into())), "You can't ship back to reserves");

    let mut s2 = Shipment::default();
    s2.shipment_type = ShipmentType::Unknown;
    s2.force_amount = i32::MAX;
    s2.special_force_amount = 1;
    s2.to_id = to.id();
    s2.initialize(&g, faction);
    assert_eq!(show(|| s2.validate(&g).map(|m| m.to_cs_string()).unwrap_or("null".into())), "You can't ship back to reserves");

    let valid: Vec<_> = Shipment::valid_shipment_locations(&g, pl, false, false).collect();
    assert_eq!(valid.len(), 85, "probe: validLocations.count");
    assert_eq!(valid[0].id(), 1, "probe: validLocations.first");
}

/// `ChoamRemoveForce` with no location, executed without validation (as
/// `Game.TryLoad(..., performValidation: false, ...)` does for savegames on the server).
/// C# passes the null location into `Player.ForcesToReserves`/`SpecialForcesToReserves`
/// (`ChoamRemoveForce.cs:116-119`), which call `AddForcesToReserves` and then
/// `ForcesInLocations.TryGetValue(null)`: ArgumentNullException. The Rust port
/// threw NullReferenceException before the call when this review started; the null
/// handle work fixed it while the review ran, so this test is a guard. C# prints the
/// stack trace in the technical description; the canonical form drops it.
///
/// Probe: `choamRemoveForce.normal.result`, `choamRemoveForce.special.result`.
#[test]
fn choam_remove_force_without_location_throws_argument_null() {
    let expected = "Game Error: Value cannot be null. (Parameter 'key'). Technical description: System.ArgumentNullException: Value cannot be null. (Parameter 'key').";
    for special in [false, true] {
        let mut g = setup();
        let json = format!(
            r#"{{"$type":"ChoamRemoveForce","Target":10,"SpecialForce":{special},"_locationId":-1,"Initiator":110,"Time":"2024-01-01T12:03:00+00:00"}}"#
        );
        let e: GameEvent = json::deserialize(&json).expect("event json");
        e.initialize_game(&g);
        let r = e.execute(&mut g, false, true).map(|m| m.to_cs_string());
        assert_eq!(r.as_deref(), Some(expected), "SpecialForce = {special}");
    }
}
