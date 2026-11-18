//! Focused tests for players, player sequences, battalions and rules. The comments say which
//! behavior each test pins down.

use treachery_engine::error::catch_panic;
use treachery_engine::prelude::*;

fn game_with(factions: &[Faction]) -> Game {
    let mut g = Game::new();
    for (seat, f) in factions.iter().enumerate() {
        let mut p = Player::new(Some(*f));
        p.seat = Seat::from_number(seat);
        g.players.push(p);
    }
    g
}

fn idx(g: &Game, f: Faction) -> PlayerId {
    g.get_player(Some(f)).unwrap()
}

fn homeworld(g: &Game, w: World) -> Homeworld {
    g.board.map.homeworlds().find(|h| h.world() == w).unwrap()
}

fn exception_text<R>(f: impl FnOnce() -> R) -> String {
    match catch_panic(f) {
        Ok(_) => "no exception".to_string(),
        Err(e) => e,
    }
}

// ---------------------------------------------------------------- Battalion

/// A change that would leave fewer than zero forces, or more than a u16 holds, is a Game Error
/// and changes nothing.
#[test]
fn battalion_change_below_zero_is_an_error() {
    let l = Location::from_id(1).unwrap();
    let mut b = Battalion::new(Faction::Emperor, 2, 1, l);
    assert!(b.change_forces(-3).is_err());
    b.change_special_forces(-1).unwrap();
    assert_eq!((b.amount_of_forces, b.amount_of_special_forces), (2, 0));
    assert!(b.change_forces(i32::MAX).is_err());
    assert!(b.change_forces(i32::MIN).is_err());
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
/// slot that the next new location reuses.
#[test]
fn change_forces_zero_leaves_a_hole() {
    let mut g = game_with(&[Faction::Atreides]);
    let me = idx(&g, Faction::Atreides);
    let a = Location::from_id(1).unwrap();
    let b = Location::from_id(2).unwrap();
    let c = Location::from_id(3).unwrap();
    Player::add_forces(&mut g, me, a, 1, false).unwrap();
    Player::add_forces(&mut g, me, b, 1, false).unwrap();
    Player::remove_forces(&mut g, me, a, 1).unwrap(); // frees slot 0
    Player::add_forces(&mut g, me, c, 0, false).unwrap(); // takes slot 0, then frees it again
    Player::add_forces(&mut g, me, a, 2, false).unwrap(); // slot 0
    assert_eq!(g.player(me).locations_with_any_forces(), vec![a, b]);
}

/// `some_forces_in_territory_to_reserves` takes from each location what it holds, up to what is
/// left to take, so no forces are made out of nothing (escapes.md, "Forces below zero").
#[test]
fn forces_to_reserves_territory_takes_what_each_location_holds() {
    let mut g = game_with(&[Faction::Atreides]);
    let me = idx(&g, Faction::Atreides);
    let hw = homeworld(&g, World::Atreides);
    Player::initialize_homeworld(&mut g, me, hw, 10, 0).unwrap();
    let t = g.board.map.territories(false).find(|t| t.locations().len() >= 2).unwrap();
    let mut locs = t.locations().to_vec();
    locs.sort_by_key(|l| l.spice_blow_amount());
    Player::add_forces(&mut g, me, locs[0], 1, false).unwrap();
    Player::add_forces(&mut g, me, locs[1], 5, false).unwrap();

    Player::some_forces_in_territory_to_reserves(&mut g, me, t, 3, false).unwrap();

    let p = g.player(me);
    // 1 from the first location and 2 from the second.
    assert_eq!(p.forces_in(Some(locs[0])), 0);
    assert_eq!(p.forces_in(Some(locs[1])), 3);
    assert_eq!(p.forces_in_reserve(), 13);
}

#[test]
fn richese_special_forces_to_reserves_vanish() {
    let mut g = game_with(&[Faction::Richese]);
    let me = idx(&g, Faction::Richese);
    let hw = homeworld(&g, World::Richese);
    Player::initialize_homeworld(&mut g, me, hw, 10, 0).unwrap();
    let l = Location::from_id(1).unwrap();
    Player::ship_special_forces(&mut g, me, l, 1).unwrap(); // Richese: placed without leaving reserves
    assert_eq!(g.player(me).special_forces_in(Some(l)), 1);
    Player::special_forces_to_reserves(&mut g, me, l, 1).unwrap();
    assert_eq!(g.player(me).special_forces_in(Some(l)), 0);
    assert_eq!(g.player(me).any_forces_in_reserves(), 10);
}

#[test]
fn salusa_secundus_threshold_tracks_special_forces() {
    let mut g = game_with(&[Faction::Emperor]);
    g.rules.active.push(Rule::EmperorSpecialForces);
    let me = idx(&g, Faction::Emperor);
    let emperor = homeworld(&g, World::Emperor);
    let star = homeworld(&g, World::SalusaSecundus);
    Player::initialize_homeworld(&mut g, me, emperor, 15, 0).unwrap();
    Player::initialize_homeworld(&mut g, me, star, 0, 5).unwrap();
    g.rules.active.push(Rule::Homeworlds);
    let low_at_start = g.player(me).has_low_threshold(&g, World::SalusaSecundus);
    let l = Location::from_id(1).unwrap();
    // The threshold is 2 (Map.cs:1273). Shipping 3 of 5 leaves 2, which is not below it;
    // shipping one more leaves 1, which is.
    assert_eq!(star.threshold(), 2);
    Player::ship_special_forces(&mut g, me, l, 3).unwrap();
    let low_at_threshold = g.player(me).has_low_threshold(&g, World::SalusaSecundus);
    Player::ship_special_forces(&mut g, me, l, 1).unwrap();
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
    Player::initialize_homeworld(&mut g, me, hw, 10, 0).unwrap();
    let l = Location::from_id(1).unwrap();
    Player::ship_forces(&mut g, me, l, 4).unwrap();
    let on_planet: Vec<Location> = g.player(me).forces_on_planet().keys().copied().collect();
    assert_eq!(on_planet, vec![l]);
    assert_eq!(g.player(me).forces_in_reserve(), 6);
}

#[test]
fn private_forces_to_reserves_panics_without_forces_there() {
    let mut g = game_with(&[Faction::Atreides]);
    let me = idx(&g, Faction::Atreides);
    let l = Location::from_id(1).unwrap();
    // The map lookup's own message.
    assert_eq!(exception_text(|| Player::forces_to_reserves(&mut g, me, l)), "no entry found for key");
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
        g.board.sector_in_storm = Some(Sector::new(sector));
        let first = PlayerSequence::determine_first_player(&g);
        assert_eq!(g.player(first).seat, Seat::from_number(seat), "sector {sector}");
    }
}

#[test]
fn determine_first_player_skips_empty_seats_and_panics() {
    let mut g = game_with(&[Faction::Atreides, Faction::Harkonnen]);
    g.player_mut(g.player_at(1)).seat = Some(Seat::new(4));
    g.board.sector_in_storm = Some(Sector::new(6)); // starts looking at seat 2
    assert_eq!(PlayerSequence::determine_first_player(&g), g.player_at(1));

    g.rules.settings.as_mut().unwrap().number_of_players = 0;
    assert_eq!(exception_text(|| PlayerSequence::determine_first_player(&g)), "attempt to calculate the remainder with a divisor of zero");

    let mut g = game_with(&[]);
    g.rules.settings.as_mut().unwrap().number_of_players = 3;
    // No player in any seat: a plain panic.
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| PlayerSequence::determine_first_player(&g))).is_err());
}

#[test]
fn sequence_goes_around_and_counts_full_circles() {
    let mut g = game_with(&[Faction::Atreides, Faction::Harkonnen, Faction::Emperor]);
    g.board.sector_in_storm = Some(Sector::new(7)); // ceil(7*6/18) = 3 -> seat 3 is empty -> wraps to seat 0
    let mut seq = PlayerSequence::starting_with_first_player(&g);
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
    g.board.sector_in_storm = Some(Sector::new(0));
    assert!(PlayerSequence::is_after(&g, g.player_at(2), g.player_at(1)));
    assert!(!PlayerSequence::is_after(&g, g.player_at(0), g.player_at(1)));
}

// ---------------------------------------------------------------- Game_Rules

#[test]
fn ruleset_definition_lists_each_rule_once() {
    let d = Game::ruleset_definition();
    let e2 = &d[&Ruleset::Expansion2AdvancedGame];
    assert_eq!(e2.iter().filter(|r| **r == Rule::StrongholdBonus).count(), 1);
    assert_eq!(e2.len(), 23);
    assert_eq!(d.len(), 11);
    for (ruleset, rules) in &d {
        for r in rules {
            assert_eq!(rules.iter().filter(|x| *x == r).count(), 1, "{ruleset:?} lists {r:?} more than once");
        }
    }
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
    let house = Game::get_rules_in_group(Some(RuleGroup::House), 3);
    let mut sorted = house.clone();
    sorted.sort();
    assert_eq!(house, sorted);
    assert!(house.contains(&Rule::HmSwithoutIxian));
}
