//! Focused tests for storm, spice blow, collection and charity. They build small game states by
//! hand and check rules that are easy to get wrong.

use treachery_engine::error::catch_panic;
use treachery_engine::prelude::*;
use treachery_engine::game::ResourcesToBeDivided;

fn game_with(factions: &[Faction]) -> Game {
    let mut g = Game::new();
    for f in factions {
        let p = Player::new(Some(*f));
        g.players.push(p);
    }
    g
}

fn idx(g: &Game, f: Faction) -> PlayerId {
    g.get_player(Some(f)).unwrap()
}

fn tbd(amount: i32) -> ResourcesToBeDivided {
    ResourcesToBeDivided { amount, first_faction: Faction::Emperor, other_faction: Faction::Atreides, territory: Game::new().board.map.territories(false).next().unwrap() }
}

#[test]
fn storm_dial_bounds_depend_on_turn() {
    let mut g = game_with(&[Faction::Emperor, Faction::Atreides]);
    g.turn.current_turn = Turn::new(1);
    assert_eq!(StormDialled::valid_min_amount(&g), 0);
    assert_eq!(StormDialled::valid_max_amount(&g), 20);
    g.turn.current_turn = Turn::new(2);
    assert_eq!(StormDialled::valid_min_amount(&g), 1);
    assert_eq!(StormDialled::valid_max_amount(&g), 3);

    let mut e = StormDialled::default();
    e.amount = 0;
    assert!(e.validate(&g, By::NOBODY).is_err());
    e.amount = 3;
    assert!(e.validate(&g, By::NOBODY).is_ok());
}

#[test]
fn unagreed_division_truncates_half_as_float() {
    // (int)(0.5f * Amount): truncation toward zero, so an odd amount favours the other faction.
    let t = tbd(7);
    assert_eq!(DivideResources::gained_by_first_faction(Some(&t), false, 100), 3);
    assert_eq!(DivideResources::gained_by_other_faction(Some(&t), false, 100), 4);
    let neg = tbd(-3);
    assert_eq!(DivideResources::gained_by_first_faction(Some(&neg), false, 0), -1);
    assert_eq!(DivideResources::gained_by_other_faction(Some(&neg), false, 0), -2);
    // Agreed: the proposal is taken as is, even when it exceeds the amount.
    assert_eq!(DivideResources::gained_by_other_faction(Some(&t), true, 9), -2);
}

#[test]
fn agreed_division_of_nothing_reads_tbd_only_for_the_other_faction() {
    assert_eq!(DivideResources::gained_by_first_faction(None, true, 5), 5);
    let e = catch_panic(|| DivideResources::gained_by_other_faction(None, true, 5)).unwrap_err();
    assert_eq!(e, "resources to be divided");
}

#[test]
fn natural_numbers() {
    assert_eq!(Game::natural(1), "first");
    assert_eq!(Game::natural(10), "tenth");
    assert_eq!(Game::natural(11), "11th");
    assert_eq!(Game::natural(13), "13th");
    assert_eq!(Game::natural(21), "21st");
    assert_eq!(Game::natural(22), "22nd");
    assert_eq!(Game::natural(23), "23rd");
    assert_eq!(Game::natural(111), "111th");
}

#[test]
fn testing_station_accepts_only_plus_or_minus_one() {
    let g = game_with(&[Faction::Emperor]);
    let mut e = TestingStationUsed::default();
    for (v, ok) in [(-2, false), (-1, true), (0, false), (1, true), (2, false)] {
        e.value_added = v;
        assert_eq!(e.validate(&g, By::NOBODY).is_ok(), ok, "value {v}");
    }
}

#[test]
fn storm_spell_range() {
    let g = game_with(&[Faction::Emperor]);
    let mut e = StormSpellPlayed::default();
    for (v, ok) in [(0, true), (10, true), (11, false)] {
        e.move_amount = v;
        assert_eq!(e.validate(&g, By::NOBODY).is_ok(), ok, "value {v}");
    }
}

#[test]
fn divide_resources_accepted_needs_a_pending_division() {
    let mut g = game_with(&[Faction::Emperor, Faction::Atreides]);
    g.enter_collection(CollectionStep::Accepting);
    let atreides = idx(&g, Faction::Atreides);
    assert!(!DivideResourcesAccepted::is_applicable(&g, atreides));
    g.collection_mut().unwrap().collected_resources_to_be_divided.push(tbd(4));
    assert!(DivideResourcesAccepted::is_applicable(&g, atreides));
    assert!(!DivideResourcesAccepted::is_applicable(&g, idx(&g, Faction::Emperor)));
}

#[test]
fn divide_resources_is_applicable_throws_without_pending_division() {
    // GetResourcesToBeDivided(g).FirstFaction dereferences null in the right phase.
    let mut g = game_with(&[Faction::Emperor]);
    let emperor = idx(&g, Faction::Emperor);
    assert!(!DivideResources::is_applicable(&g, emperor));
    g.enter_collection(CollectionStep::Dividing);
    let e = catch_panic(|| DivideResources::is_applicable(&g, emperor)).unwrap_err();
    assert_eq!(e, "resources to be divided");
}

#[test]
fn monster_targets_exclude_protected_territories() {
    let g = game_with(&[Faction::Fremen]);
    let targets: Vec<_> = FremenSentMonster::valid_targets(&g).collect();
    assert!(!targets.is_empty());
    assert!(targets.iter().all(|t| !t.is_protected_from_worm()));
    assert_eq!(targets, KarmaMonster::valid_targets(&g).collect::<Vec<_>>());
}

#[test]
fn charity_multiplier_and_cancellation() {
    let mut g = game_with(&[Faction::Emperor]);
    assert_eq!(g.current_charity_multiplier(), 1);
    g.board.choam_economics_status = Some(ChoamEconomicsStatus::DoubleFlipped);
    assert_eq!(g.current_charity_multiplier(), 2);
    g.board.choam_economics_status = Some(ChoamEconomicsStatus::Cancel);
    assert_eq!(g.current_charity_multiplier(), 0);
    assert!(g.charity_is_cancelled());
}

#[test]
fn charity_paid_even_when_choam_cannot_pay() {
    // Known quirk (known-issues.md): the claimant gets the money before Choam's balance is
    // checked, so it is paid even though the log says it can't claim it.
    let mut g = game_with(&[Faction::Emperor, Faction::Choam]);
    g.enter_charity(CharityStep::Claiming);
    let emperor = idx(&g, Faction::Emperor);
    let choam = idx(&g, Faction::Choam);
    g.player_mut(emperor).resources = 0;
    g.player_mut(choam).resources = 1;
    g.claim_charity(emperor);
    assert_eq!(i32::from(g.player(emperor).resources), 2);
    assert_eq!(i32::from(g.player(choam).resources), 1);
    assert!(g.turn.has_acted_or_passed.contains(&Faction::Emperor));
}

#[test]
fn nexus_card_may_draw() {
    let mut g = game_with(&[Faction::Emperor]);
    let emperor = idx(&g, Faction::Emperor);
    assert!(NexusCardDrawn::may_draw(&g, emperor));
    g.enter_blow(BlowStep::NexusCards);
    g.blow_mut().unwrap().factions_that_drew_nexus_card.push(Faction::Emperor);
    assert!(!NexusCardDrawn::may_draw(&g, emperor));
    // A faction holding its own Nexus card may draw again.
    g.player_mut(emperor).nexus = Some(Faction::Emperor);
    assert!(NexusCardDrawn::may_draw(&g, emperor));
}
