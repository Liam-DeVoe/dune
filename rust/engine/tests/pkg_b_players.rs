//! Focused tests for package B (players, player sequences, battalions, rules). Expected values
//! follow the C# code in Model/Player.cs, Model/PlayerSequence.cs, Model/Battalion.cs and
//! Game_Rules.cs; the comments say which C# behaviour each test pins down.

use treachery_engine::compat::cs;
use treachery_engine::prelude::*;

fn game_with(factions: &[Faction]) -> Game {
    let mut g = Game::new();
    for (seat, f) in factions.iter().enumerate() {
        let mut p = Player::new(&g, *f);
        p.seat = seat as i32;
        g.players.push(p);
    }
    g
}

fn idx(g: &Game, f: Faction) -> PlayerIndex {
    g.get_player(Some(f)).unwrap()
}

fn homeworld(g: &Game, w: World) -> Homeworld {
    g.map.homeworlds().find(|h| h.world() == w).unwrap()
}

fn exception_text<R>(f: impl FnOnce() -> R) -> String {
    match cs::catch(f) {
        Ok(_) => "no exception".to_string(),
        Err(e) => format!("{}: {}", e.type_name, e.message),
    }
}

// ---------------------------------------------------------------- Battalion

#[test]
fn battalion_change_ignores_negative_results() {
    let l = Location::from_id(1).unwrap();
    let mut b = Battalion::new(Faction::Emperor, 2, 1, l);
    b.change_forces(-3); // would go negative: no change, no clamp
    b.change_special_forces(-1);
    assert_eq!((b.amount_of_forces, b.amount_of_special_forces), (2, 0));
    b.change_forces(i32::MAX); // wraps negative: no change
    assert_eq!(b.amount_of_forces, 2);
}

#[test]
fn battalion_take_half_rounds_each_kind_up() {
    let l = Location::from_id(1).unwrap();
    let h = Battalion::new(Faction::Emperor, 3, 5, l).take_half();
    assert_eq!((h.amount_of_forces, h.amount_of_special_forces), (2, 3));
    let t = Battalion::new(Faction::Ixian, 3, 2, l).take(4, true);
    assert_eq!((t.amount_of_forces, t.amount_of_special_forces), (2, 2));
    let t = Battalion::new(Faction::Ixian, 3, 2, l).take(4, false);
    assert_eq!((t.amount_of_forces, t.amount_of_special_forces), (3, 1));
}

// ---------------------------------------------------------------- Player forces

/// `ChangeForces(loc, 0)` on a missing location adds then removes the entry, leaving a free
/// slot that the next new location reuses (hazards §3.2).
#[test]
fn change_forces_zero_leaves_a_hole() {
    let mut g = game_with(&[Faction::Atreides]);
    let me = idx(&g, Faction::Atreides);
    let a = Location::from_id(1).unwrap();
    let b = Location::from_id(2).unwrap();
    let c = Location::from_id(3).unwrap();
    Player::add_forces(&mut g, me, a, 1, false);
    Player::add_forces(&mut g, me, b, 1, false);
    Player::remove_forces(&mut g, me, a, 1); // frees slot 0
    Player::add_forces(&mut g, me, c, 0, false); // takes slot 0, then frees it again
    Player::add_forces(&mut g, me, a, 2, false); // slot 0
    assert_eq!(g.player(me).locations_with_any_forces(), vec![a, b]);
}

/// `ForcesToReserves(Territory, amount, special)` sends `toRemoveInTotal` to reserves at
/// every location and skips the removal when it exceeds the forces there (hazards §18).
#[test]
fn forces_to_reserves_territory_bug() {
    let mut g = game_with(&[Faction::Atreides]);
    let me = idx(&g, Faction::Atreides);
    let hw = homeworld(&g, World::Atreides);
    Player::initialize_homeworld(&mut g, me, hw, 10, 0);
    let t = g.map.territories(false).find(|t| t.locations().len() >= 2).unwrap();
    let mut locs = t.locations().to_vec();
    locs.sort_by_key(|l| l.spice_blow_amount());
    Player::add_forces(&mut g, me, locs[0], 1, false);
    Player::add_forces(&mut g, me, locs[1], 5, false);

    Player::forces_to_reserves_territory_int32_boolean(&mut g, me, t, 3, false);

    let p = g.player(me);
    // First location: 3 to reserves, removal of 3 from 1 is refused. Second location:
    // toRemoveInTotal is now 2, so 2 more to reserves and 2 removed.
    assert_eq!(p.forces_in(Some(locs[0])), 1);
    assert_eq!(p.forces_in(Some(locs[1])), 3);
    assert_eq!(p.forces_in_reserve(), 15);
}

#[test]
fn richese_special_forces_to_reserves_vanish() {
    let mut g = game_with(&[Faction::Richese]);
    let me = idx(&g, Faction::Richese);
    let hw = homeworld(&g, World::Richese);
    Player::initialize_homeworld(&mut g, me, hw, 10, 0);
    let l = Location::from_id(1).unwrap();
    Player::ship_special_forces(&mut g, me, l, 1); // Richese: placed without leaving reserves
    assert_eq!(g.player(me).special_forces_in(Some(l)), 1);
    Player::special_forces_to_reserves(&mut g, me, l, 1);
    assert_eq!(g.player(me).special_forces_in(Some(l)), 0);
    assert_eq!(g.player(me).any_forces_in_reserves(), 10);
}

#[test]
fn salusa_secundus_threshold_tracks_special_forces() {
    let mut g = game_with(&[Faction::Emperor]);
    g.rules.push(Rule::EmperorSpecialForces);
    let me = idx(&g, Faction::Emperor);
    let emperor = homeworld(&g, World::Emperor);
    let star = homeworld(&g, World::SalusaSecundus);
    Player::initialize_homeworld(&mut g, me, emperor, 15, 0);
    Player::initialize_homeworld(&mut g, me, star, 0, 5);
    g.rules.push(Rule::Homeworlds);
    let low_at_start = g.player(me).has_low_threshold(&g, World::SalusaSecundus);
    let l = Location::from_id(1).unwrap();
    // The threshold is 2 (Map.cs:1273). Shipping 3 of 5 leaves 2, which is not below it;
    // shipping one more leaves 1, which is.
    assert_eq!(star.threshold(), 2);
    Player::ship_special_forces(&mut g, me, l, 3);
    let low_at_threshold = g.player(me).has_low_threshold(&g, World::SalusaSecundus);
    Player::ship_special_forces(&mut g, me, l, 1);
    let low_after = g.player(me).has_low_threshold(&g, World::SalusaSecundus);
    assert!(!low_at_start);
    assert!(!low_at_threshold);
    assert!(low_after);
}

#[test]
fn forces_on_planet_skips_homeworlds() {
    let mut g = game_with(&[Faction::Atreides]);
    let me = idx(&g, Faction::Atreides);
    let hw = homeworld(&g, World::Atreides);
    Player::initialize_homeworld(&mut g, me, hw, 10, 0);
    let l = Location::from_id(1).unwrap();
    Player::ship_forces(&mut g, me, l, 4);
    let on_planet: Vec<Location> = g.player(me).forces_on_planet().keys().copied().collect();
    assert_eq!(on_planet, vec![l]);
    assert_eq!(g.player(me).forces_in_reserve(), 6);
}

#[test]
fn private_forces_to_reserves_throws_key_not_found() {
    let mut g = game_with(&[Faction::Atreides]);
    let me = idx(&g, Faction::Atreides);
    let l = Location::from_id(1).unwrap();
    assert_eq!(
        exception_text(|| Player::forces_to_reserves(&mut g, me, l)),
        "KeyNotFoundException: The given key 'Treachery.Shared.Location' was not present in the dictionary."
    );
}

#[test]
fn maximum_number_of_cards() {
    let g = game_with(&[Faction::Harkonnen, Faction::Choam, Faction::Atreides]);
    assert_eq!(g.player(idx(&g, Faction::Harkonnen)).maximum_number_of_cards(&g), 8);
    assert_eq!(g.player(idx(&g, Faction::Choam)).maximum_number_of_cards(&g), 5);
    assert_eq!(g.player(idx(&g, Faction::Atreides)).maximum_number_of_cards(&g), 4);
}

// ---------------------------------------------------------------- PlayerSequence

#[test]
fn determine_first_player_from_storm() {
    let mut g = game_with(&[Faction::Atreides, Faction::Harkonnen, Faction::Emperor, Faction::Fremen, Faction::BeneGesserit, Faction::Ixian]);
    // ceil(sector * 6 / 18f) % 6
    for (sector, seat) in [(0, 0), (1, 1), (3, 1), (4, 2), (17, 0), (16, 0), (15, 5)] {
        g.sector_in_storm = sector;
        let first = PlayerSequence::determine_first_player(&g);
        assert_eq!(g.player(first).seat, seat, "sector {sector}");
    }
}

#[test]
fn determine_first_player_skips_empty_seats_and_throws() {
    let mut g = game_with(&[Faction::Atreides, Faction::Harkonnen]);
    g.player_mut(g.player_at(1)).seat = 4;
    g.sector_in_storm = 6; // starts looking at seat 2
    assert_eq!(PlayerSequence::determine_first_player(&g), g.player_at(1));

    g.settings.as_ref().unwrap().borrow_mut().number_of_players = 0;
    assert_eq!(exception_text(|| PlayerSequence::determine_first_player(&g)), "DivideByZeroException: Attempted to divide by zero.");

    let g = game_with(&[]);
    g.settings.as_ref().unwrap().borrow_mut().number_of_players = 3;
    assert_eq!(exception_text(|| PlayerSequence::determine_first_player(&g)), "Exception: Could not determine the first player.");
}

#[test]
fn sequence_goes_around_and_counts_full_circles() {
    let mut g = game_with(&[Faction::Atreides, Faction::Harkonnen, Faction::Emperor]);
    g.sector_in_storm = 7; // ceil(7*6/18) = 3 -> seat 3 is empty -> wraps to seat 0
    let mut seq = PlayerSequence::new_game(&g);
    let order: Vec<_> = seq.players_in_order(&g).into_iter().map(|p| p.unwrap()).collect();
    assert_eq!(order, vec![g.player_at(0), g.player_at(1), g.player_at(2)]);
    let mut seen = vec![];
    for _ in 0..4 {
        seen.push(seq.current_player(&g).unwrap());
        seq.next_player(&g);
    }
    assert_eq!(seen, vec![g.player_at(0), g.player_at(1), g.player_at(2), g.player_at(0)]);
    assert_eq!(seq.fully_circled, 1);
    seq.next_round(&g);
    assert_eq!(seq.current_player(&g), Some(g.player_at(1)));
}

#[test]
fn reverse_direction_orders_by_negated_seat() {
    let g = game_with(&[Faction::Atreides, Faction::Harkonnen, Faction::Emperor]);
    let seq = PlayerSequence::new(&g, false, -1, Some(g.player_at(1)), true);
    // Players sorted by -seat: 2, 1, 0. Begin next to Harkonnen -> Atreides.
    let order: Vec<_> = seq.players_in_order(&g).into_iter().map(|p| p.unwrap()).collect();
    assert_eq!(order, vec![g.player_at(0), g.player_at(2), g.player_at(1)]);
}

#[test]
fn is_after_uses_first_player() {
    let mut g = game_with(&[Faction::Atreides, Faction::Harkonnen, Faction::Emperor]);
    g.sector_in_storm = 0;
    assert!(PlayerSequence::is_after(&g, g.player_at(2), g.player_at(1)));
    assert!(!PlayerSequence::is_after(&g, g.player_at(0), g.player_at(1)));
}

// ---------------------------------------------------------------- Game_Rules

#[test]
fn ruleset_definition_keeps_duplicate_rule() {
    let d = Game::ruleset_definition();
    let e2 = d.index_with(&Ruleset::Expansion2AdvancedGame, |_| String::new());
    assert_eq!(e2.iter().filter(|r| **r == Rule::StrongholdBonus).count(), 2);
    assert_eq!(e2.len(), 24);
    assert_eq!(d.len(), 11);
}

#[test]
fn determine_approximate_ruleset() {
    assert_eq!(Game::determine_approximate_ruleset(&[], &[], 3), Ruleset::BasicGame);
    assert_eq!(Game::determine_approximate_ruleset(&[Faction::Ecaz], &[Rule::AdvancedCombat], 3), Ruleset::Expansion3AdvancedGame);
    assert_eq!(
        Game::determine_approximate_ruleset(&[Faction::Ixian, Faction::Richese, Faction::Moritani], &[], 3),
        Ruleset::AllExpansionsBasicGame
    );
}

#[test]
fn rules_in_group_in_value_order() {
    let house = Game::get_rules_in_group(RuleGroup::House, 3);
    let mut sorted = house.clone();
    sorted.sort();
    assert_eq!(house, sorted);
    assert!(house.contains(&Rule::HmSwithoutIxian));
    assert!(!Game::get_available_rulesets().contains(&Ruleset::None));
}
