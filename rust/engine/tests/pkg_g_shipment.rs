//! Focused tests for shipment and movement. They build small game states by hand and check rules
//! that are easy to get wrong.

use treachery_engine::error::catch_panic;
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

fn cost(g: &Game, p: PlayerIndex, forces: i32, specials: i32, to: Location, karma: bool, back: bool, s2s: bool, nf_richese: bool, nf_other: bool) -> i32 {
    Shipment::determine_cost_game_player_int32_int32_location_boolean_boolean_boolean_boolean_boolean(
        g, p, forces, specials, to, karma, back, s2s, nf_richese, nf_other,
    )
}

#[test]
fn shipment_cost_normal_and_reduced() {
    let g = game_with(&[Faction::Emperor, Faction::Atreides]);
    let emperor = idx(&g, Faction::Emperor);
    let flat = g.map.the_great_flat();
    let arrakeen = g.map.arrakeen();
    assert_eq!(cost(&g, emperor, 3, 0, flat, false, false, false, false, false), 6);
    assert_eq!(cost(&g, emperor, 3, 0, arrakeen, false, false, false, false, false), 3);
    // Karma halves, rounding up in double.
    assert_eq!(cost(&g, emperor, 3, 0, arrakeen, true, false, false, false, false), 2);
    assert_eq!(cost(&g, emperor, 3, 2, flat, true, false, false, false, false), 5);
}

#[test]
fn shipment_cost_back_to_reserves_rounds_up_half() {
    let g = game_with(&[Faction::Guild]);
    let guild = idx(&g, Faction::Guild);
    let flat = g.map.the_great_flat();
    assert_eq!(cost(&g, guild, 3, 0, flat, false, true, false, false, false), 2);
    assert_eq!(cost(&g, guild, 4, 0, flat, false, true, false, false, false), 2);
    assert_eq!(cost(&g, guild, 1, 0, flat, false, true, false, false, false), 1);
}

#[test]
fn shipment_cost_guild_discount() {
    let g = game_with(&[Faction::Guild]);
    let guild = idx(&g, Faction::Guild);
    assert_eq!(cost(&g, guild, 3, 0, g.map.the_great_flat(), false, false, false, false, false), 3);
    assert_eq!(cost(&g, guild, 3, 0, g.map.arrakeen(), false, false, false, false, false), 2);
}

#[test]
fn shipment_cost_no_field() {
    let g = game_with(&[Faction::Richese, Faction::Emperor]);
    let richese = idx(&g, Faction::Richese);
    let emperor = idx(&g, Faction::Emperor);
    let flat = g.map.the_great_flat();
    // Richese's own No-Field costs max(forces, 1).
    assert_eq!(cost(&g, richese, 0, 1, flat, false, false, false, true, false), 2);
    assert_eq!(cost(&g, richese, 3, 1, flat, false, false, false, true, false), 6);
    // Others pay for a single force.
    assert_eq!(cost(&g, emperor, 3, 2, flat, false, false, false, false, true), 2);
}

#[test]
fn shipment_cost_product_wraps_before_widening() {
    // C# parity: Math.Abs(totalForces) * 2 is an unchecked int product.
    let g = game_with(&[Faction::Emperor]);
    let emperor = idx(&g, Faction::Emperor);
    let c = cost(&g, emperor, i32::MAX, 0, g.map.the_great_flat(), false, false, false, false, false);
    assert_eq!(c, i32::MAX.wrapping_mul(2));
}

#[test]
fn shipment_type_inference() {
    let g = game_with(&[Faction::Emperor]);
    let mut s = Shipment::default();
    s.shipment_type = ShipmentType::Unknown;
    s.force_amount = -2;
    assert!(s.is_back_to_reserves());
    // The sum wraps: int.MinValue + -1 is positive.
    s.force_amount = i32::MIN;
    s.special_force_amount = -1;
    assert!(!s.is_back_to_reserves());
    s.from_id = -1;
    assert!(!s.is_site_to_site(&g));
    s.from_id = 0;
    assert!(s.is_site_to_site(&g));
}

#[test]
fn shipment_no_field_values_are_offset_by_one() {
    let mut s = Shipment::default();
    assert_eq!(s.no_field_value(), -1);
    assert!(!s.is_no_field());
    s.set_no_field_value(3);
    assert_eq!(s.no_field_value, 4);
    assert!(s.is_no_field());
    s.no_field_value = i32::MIN;
    assert_eq!(s.no_field_value(), i32::MAX);
}

#[test]
fn shipment_parse_force_locations() {
    let g = game_with(&[Faction::Emperor]);
    let d = Shipment::parse_force_locations(&g, Faction::Emperor, Some("5:2|1,-1:7|7,3:0|4"));
    let items: Vec<(i32, i32, i32)> = d.iter().map(|(k, v)| (k.id(), v.amount_of_forces, v.amount_of_special_forces)).collect();
    assert_eq!(items, vec![(5, 2, 1), (3, 0, 4)]);

    assert_eq!(Shipment::parse_force_locations(&g, Faction::Emperor, None).len(), 0);
    assert_eq!(Shipment::parse_force_locations(&g, Faction::Emperor, Some("")).len(), 0);

    let e = catch_panic(|| Shipment::parse_force_locations(&g, Faction::Emperor, Some("5"))).unwrap_err();
    assert!(e.starts_with("index out of bounds"), "{}", e);
    let e = catch_panic(|| Shipment::parse_force_locations(&g, Faction::Emperor, Some("x:1|1"))).unwrap_err();
    assert!(e.starts_with("FormatException"), "{}", e);
    let e = catch_panic(|| Shipment::parse_force_locations(&g, Faction::Emperor, Some("5:1|1,5:2|2"))).unwrap_err();
    assert!(e.starts_with("duplicate key"), "{}", e);
    // A missing location is skipped before its amounts are parsed.
    assert_eq!(Shipment::parse_force_locations(&g, Faction::Emperor, Some("-1:x")).len(), 0);
}

#[test]
fn shipment_valid_no_field_values() {
    let mut g = game_with(&[Faction::Richese, Faction::Emperor]);
    let richese = idx(&g, Faction::Richese);
    let emperor = idx(&g, Faction::Emperor);
    g.latest_revealed_no_field_value = 3;
    g.current_no_field_value = 5;
    assert_eq!(Shipment::valid_no_field_values(&g, richese), vec![0]);
    assert_eq!(Shipment::valid_no_field_values(&g, emperor), Vec::<i32>::new());
    g.current_no_field_value = -1;
    assert_eq!(Shipment::valid_no_field_values(&g, richese), vec![0, 5]);
    assert_eq!(Shipment::valid_cunning_no_field_values(&g, richese, 0), vec![5]);
}

#[test]
fn set_shipment_permission_messages_and_duplicates() {
    let g = game_with(&[Faction::Guild, Faction::Emperor]);
    let mut e = SetShipmentPermission::default();
    e.base.initiator.set(Faction::Guild);
    e.base.player.set(Some(idx(&g, Faction::Guild)));
    // No factions at all is refused (C# threw ArgumentNullException from `Any`).
    e.factions = None;
    assert!(e.validate(&g).is_some());
    e.factions = Some(vec![]);
    assert!(e.validate(&g).is_some());
    e.factions = Some(vec![Faction::Guild]);
    assert!(e.validate(&g).is_some());
    e.factions = Some(vec![Faction::Emperor, Faction::Emperor]);
    assert!(e.validate(&g).is_none());
}

#[test]
fn choam_remove_force_special_force_choices() {
    let g = game_with(&[Faction::Choam, Faction::Emperor]);
    let flat = g.map.the_great_flat();
    assert_eq!(ChoamRemoveForce::valid_special_force_choices(&g, None, Faction::Emperor), vec![false, true]);
    assert_eq!(ChoamRemoveForce::valid_special_force_choices(&g, Some(flat), Faction::Atreides), vec![false, true]);
    assert_eq!(ChoamRemoveForce::valid_special_force_choices(&g, Some(flat), Faction::Emperor), Vec::<bool>::new());
}
