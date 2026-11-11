//! Focused tests for package D (storm, spice blow, collection, charity). They build
//! small game states by hand and check rules whose C# behaviour is easy to get wrong.

use treachery_engine::cs;
use treachery_engine::prelude::*;
use treachery_engine::resources_to_be_divided::ResourcesToBeDivided;

fn game_with(factions: &[Faction]) -> Game {
    let mut g = Game::new();
    for f in factions {
        let p = Player::new(&g, *f);
        g.players.push(p);
    }
    g
}

fn idx(g: &Game, f: Faction) -> PlayerIndex {
    g.get_player(Some(f)).unwrap()
}

fn tbd(amount: i32) -> ResourcesToBeDivided {
    ResourcesToBeDivided { amount, first_faction: Faction::Emperor, other_faction: Faction::Atreides, territory: Game::new().map.territories(false).next().unwrap() }
}

#[test]
fn storm_dial_bounds_depend_on_turn() {
    let mut g = game_with(&[Faction::Emperor, Faction::Atreides]);
    g.current_turn = 1;
    assert_eq!(StormDialled::valid_min_amount(&g), 0);
    assert_eq!(StormDialled::valid_max_amount(&g), 20);
    g.current_turn = 2;
    assert_eq!(StormDialled::valid_min_amount(&g), 1);
    assert_eq!(StormDialled::valid_max_amount(&g), 3);

    let mut e = StormDialled::default();
    e.amount = 0;
    assert!(e.validate(&g).is_some());
    e.amount = 3;
    assert!(e.validate(&g).is_none());
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
    let e = cs::catch(|| DivideResources::gained_by_other_faction(None, true, 5)).unwrap_err();
    assert_eq!(e.type_name, "NullReferenceException");
}

#[test]
fn natural_numbers() {
    assert_eq!(Game::natural(1), "first");
    assert_eq!(Game::natural(10), "tenth");
    // C# parity: no special case for 11-13 or 21.
    assert_eq!(Game::natural(11), "11th");
    assert_eq!(Game::natural(21), "21th");
}

#[test]
fn testing_station_accepts_only_plus_or_minus_one() {
    let g = game_with(&[Faction::Emperor]);
    let mut e = TestingStationUsed::default();
    for (v, ok) in [(-2, false), (-1, true), (0, false), (1, true), (2, false)] {
        e.value_added = v;
        assert_eq!(e.validate(&g).is_none(), ok, "value {v}");
    }
}

#[test]
fn storm_spell_range() {
    let g = game_with(&[Faction::Emperor]);
    let mut e = StormSpellPlayed::default();
    for (v, ok) in [(-1, false), (0, true), (10, true), (11, false)] {
        e.move_amount = v;
        assert_eq!(e.validate(&g).is_none(), ok, "value {v}");
    }
}

#[test]
fn divide_resources_accepted_needs_a_pending_division() {
    let mut g = game_with(&[Faction::Emperor, Faction::Atreides]);
    g.current_phase = Phase::AcceptingResourceDivision;
    let atreides = idx(&g, Faction::Atreides);
    assert!(!DivideResourcesAccepted::is_applicable(&g, atreides));
    g.collected_resources_to_be_divided.push(tbd(4));
    assert!(DivideResourcesAccepted::is_applicable(&g, atreides));
    assert!(!DivideResourcesAccepted::is_applicable(&g, idx(&g, Faction::Emperor)));
}

#[test]
fn divide_resources_is_applicable_throws_without_pending_division() {
    // GetResourcesToBeDivided(g).FirstFaction dereferences null in the right phase.
    let mut g = game_with(&[Faction::Emperor]);
    let emperor = idx(&g, Faction::Emperor);
    assert!(!DivideResources::is_applicable(&g, emperor));
    g.current_phase = Phase::DividingCollectedResources;
    let e = cs::catch(|| DivideResources::is_applicable(&g, emperor)).unwrap_err();
    assert_eq!(e.type_name, "NullReferenceException");
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
    g.economics_status = ChoamEconomicsStatus::DoubleFlipped;
    assert_eq!(g.current_charity_multiplier(), 2);
    g.economics_status = ChoamEconomicsStatus::Cancel;
    assert_eq!(g.current_charity_multiplier(), 0);
    assert!(g.charity_is_cancelled());
}

#[test]
fn charity_paid_even_when_choam_cannot_pay() {
    // C# parity: the claimant gets the money before Choam's balance is checked.
    let mut g = game_with(&[Faction::Emperor, Faction::Choam]);
    let emperor = idx(&g, Faction::Emperor);
    let choam = idx(&g, Faction::Choam);
    g.player_mut(emperor).resources = 0;
    g.player_mut(choam).resources = 1;
    g.claim_charity(emperor);
    assert_eq!(g.player(emperor).resources, 2);
    assert_eq!(g.player(choam).resources, 1);
    assert!(g.has_acted_or_passed.contains(&Faction::Emperor));
}

#[test]
fn nexus_card_may_draw() {
    let mut g = game_with(&[Faction::Emperor]);
    let emperor = idx(&g, Faction::Emperor);
    assert!(NexusCardDrawn::may_draw(&g, emperor));
    g.factions_that_drew_nexus_card.push(Faction::Emperor);
    assert!(!NexusCardDrawn::may_draw(&g, emperor));
    // A faction holding its own Nexus card may draw again.
    g.player_mut(emperor).nexus = Faction::Emperor;
    assert!(NexusCardDrawn::may_draw(&g, emperor));
}
